use crate::config::{Config, OpenAIError};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures::Stream;
use opensecret::{
    ChatCompletionChunk, ChatCompletionRequest, EmbeddingRequest, EmbeddingResponse,
    ModelsResponse, OpenSecretClient, Result as OpenSecretResult,
};
use std::{convert::Infallible, sync::Arc};
use tracing::{debug, error};

#[derive(Clone)]
pub struct ProxyState {
    pub config: Config,
}

impl ProxyState {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "maple-proxy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

fn extract_api_key(
    headers: &HeaderMap,
    default_key: &Option<String>,
) -> Result<String, OpenAIError> {
    // Try to get API key from Authorization header first
    if let Some(auth_header) = headers.get("authorization") {
        let auth_str = auth_header.to_str().map_err(|_| {
            OpenAIError::authentication_error("Invalid Authorization header format")
        })?;

        if let Some(key) = auth_str.strip_prefix("Bearer ") {
            return Ok(key.to_string());
        }
    }

    // Fall back to default API key from config
    default_key
        .as_ref()
        .cloned()
        .ok_or_else(|| OpenAIError::authentication_error("No API key provided. Set MAPLE_API_KEY environment variable or provide Authorization header"))
}

async fn create_client_with_auth(
    backend_url: &str,
    api_key: &str,
) -> Result<OpenSecretClient, OpenAIError> {
    let client = OpenSecretClient::new_with_api_key(backend_url, api_key.to_string())
        .map_err(|e| OpenAIError::server_error(format!("Failed to create client: {}", e)))?;

    // Perform attestation handshake
    client.perform_attestation_handshake().await.map_err(|e| {
        error!("Attestation handshake failed: {}", e);
        OpenAIError::server_error("Failed to establish secure connection with Maple backend")
    })?;

    Ok(client)
}

pub async fn list_models(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
) -> Result<Json<ModelsResponse>, (StatusCode, Json<OpenAIError>)> {
    let api_key = extract_api_key(&headers, &state.config.default_api_key)
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e)))?;

    debug!(
        "Listing models for API key: {}...",
        &api_key[..8.min(api_key.len())]
    );

    let client = create_client_with_auth(&state.config.backend_url, &api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))?;

    let models = client.get_models().await.map_err(|e| {
        error!("Failed to get models: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OpenAIError::server_error(format!(
                "Failed to retrieve models: {}",
                e
            ))),
        )
    })?;

    debug!("Successfully retrieved {} models", models.data.len());
    Ok(Json(models))
}

pub async fn create_chat_completion(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    Json(mut request): Json<ChatCompletionRequest>,
) -> Result<Response, (StatusCode, Json<OpenAIError>)> {
    let api_key = extract_api_key(&headers, &state.config.default_api_key)
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e)))?;

    debug!(
        "Chat completion request for model: {}, stream: {:?}",
        request.model,
        request.stream.unwrap_or(false)
    );

    let client = create_client_with_auth(&state.config.backend_url, &api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))?;

    // Check if streaming is requested
    if request.stream.unwrap_or(false) {
        // Handle streaming response
        let stream = client
            .create_chat_completion_stream(request)
            .await
            .map_err(|e| {
                error!("Failed to create streaming chat completion: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(OpenAIError::server_error(format!(
                        "Failed to create streaming completion: {}",
                        e
                    ))),
                )
            })?;

        let sse_stream = create_sse_stream(stream);
        Ok(Sse::new(sse_stream).into_response())
    } else {
        // Handle non-streaming response
        request.stream = Some(false); // Ensure it's explicitly false

        let response = client.create_chat_completion(request).await.map_err(|e| {
            error!("Failed to create chat completion: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OpenAIError::server_error(format!(
                    "Failed to create completion: {}",
                    e
                ))),
            )
        })?;

        debug!("Successfully created chat completion: {}", response.id);
        Ok(Json(response).into_response())
    }
}

fn create_sse_stream(
    mut stream: std::pin::Pin<Box<dyn Stream<Item = OpenSecretResult<ChatCompletionChunk>> + Send>>,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>> {
    async_stream::stream! {
        use futures::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    match serde_json::to_string(&chunk) {
                        Ok(json) => {
                            let event = axum::response::sse::Event::default()
                                .data(json);
                            yield Ok(event);
                        }
                        Err(e) => {
                            error!("Failed to serialize chunk: {}", e);
                            let error_event = axum::response::sse::Event::default()
                                .data(format!(r#"{{"error": "Failed to serialize chunk: {}"}}"#, e));
                            yield Ok(error_event);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    let error_event = axum::response::sse::Event::default()
                        .data(format!(r#"{{"error": "Stream error: {}"}}"#, e));
                    yield Ok(error_event);
                    break;
                }
            }
        }

        // Send [DONE] event to indicate end of stream
        let done_event = axum::response::sse::Event::default()
            .data("[DONE]");
        yield Ok(done_event);
    }
}

pub async fn create_embeddings(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    Json(request): Json<EmbeddingRequest>,
) -> Result<Json<EmbeddingResponse>, (StatusCode, Json<OpenAIError>)> {
    let api_key = extract_api_key(&headers, &state.config.default_api_key)
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e)))?;

    debug!("Embeddings request for model: {}", request.model);

    let client = create_client_with_auth(&state.config.backend_url, &api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))?;

    let response = client.create_embeddings(request).await.map_err(|e| {
        error!("Failed to create embeddings: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OpenAIError::server_error(format!(
                "Failed to create embeddings: {}",
                e
            ))),
        )
    })?;

    debug!(
        "Successfully created embeddings with {} vectors",
        response.data.len()
    );
    Ok(Json(response))
}
