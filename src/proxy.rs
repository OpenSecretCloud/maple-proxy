use crate::config::{Config, OpenAIError};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse},
    Json,
};
use dashmap::DashMap;
use futures::Stream;
use opensecret::{
    ChatCompletionChunk, ChatCompletionRequest, EmbeddingRequest, EmbeddingResponse,
    ModelsResponse, OpenSecretClient, Result as OpenSecretResult,
};
use std::{
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::OnceCell;
use tracing::{debug, error};

const CLIENT_CACHE_MAX_ENTRIES: usize = 1024;
const CLIENT_CACHE_ENTRY_TTL: Duration = Duration::from_secs(60 * 60);

struct CachedClientEntry {
    cell: OnceCell<Arc<OpenSecretClient>>,
    created_at: Instant,
}

impl CachedClientEntry {
    fn new(created_at: Instant) -> Self {
        Self {
            cell: OnceCell::new(),
            created_at,
        }
    }

    fn is_expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.created_at) >= CLIENT_CACHE_ENTRY_TTL
    }
}

#[derive(Clone)]
pub struct ProxyState {
    pub config: Config,
    clients: DashMap<String, Arc<CachedClientEntry>>,
}

impl ProxyState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            clients: DashMap::new(),
        }
    }

    fn client_entry_for_api_key(&self, api_key: &str) -> Arc<CachedClientEntry> {
        let now = Instant::now();

        if let Some(entry) = self.clients.get(api_key) {
            if !entry.is_expired(now) {
                return Arc::clone(entry.value());
            }
        }

        self.clients
            .remove_if(api_key, |_, entry| entry.is_expired(now));
        self.evict_expired_clients(now);
        self.evict_oldest_client_if_needed();

        self.clients
            .entry(api_key.to_string())
            .or_insert_with(|| Arc::new(CachedClientEntry::new(now)))
            .clone()
    }

    async fn client_for_api_key(
        &self,
        api_key: &str,
    ) -> Result<Arc<OpenSecretClient>, OpenAIError> {
        let cache_key = api_key.to_string();
        let client_entry = self.client_entry_for_api_key(&cache_key);
        let backend_url = self.config.backend_url.clone();
        let request_timeout = self.config.request_timeout();
        let init_api_key = cache_key.clone();

        let client = client_entry
            .cell
            .get_or_try_init(|| async move {
                debug!(
                    "Creating OpenSecret client for API key: {}...",
                    &init_api_key[..8.min(init_api_key.len())]
                );
                create_client_with_auth(&backend_url, &init_api_key, request_timeout)
                    .await
                    .map(Arc::new)
            })
            .await;

        match client {
            Ok(client) => Ok(Arc::clone(client)),
            Err(error) => {
                self.remove_client_entry_if_same(&cache_key, &client_entry);
                Err(error)
            }
        }
    }

    fn remove_client_entry_if_same(&self, api_key: &str, client_entry: &Arc<CachedClientEntry>) {
        self.clients
            .remove_if(api_key, |_, entry| Arc::ptr_eq(entry, client_entry));
    }

    fn evict_expired_clients(&self, now: Instant) {
        self.clients.retain(|_, entry| !entry.is_expired(now));
    }

    fn evict_oldest_client_if_needed(&self) {
        while self.clients.len() >= CLIENT_CACHE_MAX_ENTRIES {
            let oldest_key = self
                .clients
                .iter()
                .min_by_key(|entry| entry.value().created_at)
                .map(|entry| entry.key().clone());

            let Some(oldest_key) = oldest_key else {
                break;
            };

            self.clients.remove(&oldest_key);
        }
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
    request_timeout: Duration,
) -> Result<OpenSecretClient, OpenAIError> {
    let client = OpenSecretClient::new_with_api_key(backend_url, api_key.to_string())
        .map_err(|e| OpenAIError::server_error(format!("Failed to create client: {}", e)))?;

    // Perform attestation handshake
    tokio::time::timeout(request_timeout, client.perform_attestation_handshake())
        .await
        .map_err(|_| {
            error!(
                "Attestation handshake timed out after {} seconds",
                request_timeout.as_secs()
            );
            OpenAIError::server_error(format!(
                "Timed out establishing secure connection with Maple backend after {} seconds",
                request_timeout.as_secs()
            ))
        })?
        .map_err(|e| {
            error!("Attestation handshake failed: {}", e);
            OpenAIError::server_error("Failed to establish secure connection with Maple backend")
        })?;

    Ok(client)
}

fn timeout_response(operation: &str, timeout: Duration) -> (StatusCode, Json<OpenAIError>) {
    error!(
        "{} timed out after {} seconds",
        operation,
        timeout.as_secs()
    );
    (
        StatusCode::GATEWAY_TIMEOUT,
        Json(OpenAIError::server_error(format!(
            "{} timed out after {} seconds",
            operation,
            timeout.as_secs()
        ))),
    )
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

    let client = state
        .client_for_api_key(&api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))?;

    let request_timeout = state.config.request_timeout();
    let models = tokio::time::timeout(request_timeout, client.get_models())
        .await
        .map_err(|_| timeout_response("List models request", request_timeout))?
        .map_err(|e| {
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

    let client = state
        .client_for_api_key(&api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))?;

    // Check if streaming is requested
    if request.stream.unwrap_or(false) {
        // Handle streaming response
        let request_timeout = state.config.request_timeout();
        let stream = tokio::time::timeout(
            request_timeout,
            client.create_chat_completion_stream(request),
        )
        .await
        .map_err(|_| timeout_response("Streaming chat completion request", request_timeout))?
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

        let sse_stream = create_sse_stream(stream, state.config.stream_idle_timeout());
        Ok(Sse::new(sse_stream).into_response())
    } else {
        // Handle non-streaming response
        request.stream = Some(false); // Ensure it's explicitly false

        let request_timeout = state.config.request_timeout();
        let response =
            tokio::time::timeout(request_timeout, client.create_chat_completion(request))
                .await
                .map_err(|_| timeout_response("Chat completion request", request_timeout))?
                .map_err(|e| {
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
    stream_idle_timeout: Duration,
) -> impl Stream<Item = Result<axum::response::sse::Event, Infallible>> {
    async_stream::stream! {
        use futures::StreamExt;

        loop {
            let chunk_result = match tokio::time::timeout(stream_idle_timeout, stream.next()).await {
                Ok(Some(chunk_result)) => chunk_result,
                Ok(None) => break,
                Err(_) => {
                    error!(
                        "Streaming chat completion idle timed out after {} seconds",
                        stream_idle_timeout.as_secs()
                    );
                    let error_event = axum::response::sse::Event::default()
                        .data(format!(
                            r#"{{"error": "Stream idle timeout after {} seconds"}}"#,
                            stream_idle_timeout.as_secs()
                        ));
                    yield Ok(error_event);
                    break;
                }
            };

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

    let client = state
        .client_for_api_key(&api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))?;

    let request_timeout = state.config.request_timeout();
    let response = tokio::time::timeout(request_timeout, client.create_embeddings(request))
        .await
        .map_err(|_| timeout_response("Embeddings request", request_timeout))?
        .map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            host: "127.0.0.1".to_string(),
            port: 0,
            backend_url: "http://localhost:3000".to_string(),
            default_api_key: None,
            debug: false,
            enable_cors: false,
            request_timeout_secs: 300,
            stream_idle_timeout_secs: 300,
        }
    }

    #[test]
    fn reuses_client_cell_for_same_api_key() {
        let state = ProxyState::new(test_config());

        let first = state.client_entry_for_api_key("key-a");
        let second = state.client_entry_for_api_key("key-a");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(state.clients.len(), 1);
    }

    #[test]
    fn keeps_client_cells_separate_by_api_key() {
        let state = ProxyState::new(test_config());

        let first = state.client_entry_for_api_key("key-a");
        let second = state.client_entry_for_api_key("key-b");

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(state.clients.len(), 2);
    }

    #[test]
    fn evicts_old_client_cell_at_capacity() {
        let state = ProxyState::new(test_config());

        for index in 0..CLIENT_CACHE_MAX_ENTRIES {
            state.client_entry_for_api_key(&format!("key-{}", index));
        }

        state.client_entry_for_api_key("new-key");

        assert!(state.clients.contains_key("new-key"));
        assert_eq!(state.clients.len(), CLIENT_CACHE_MAX_ENTRIES);
    }

    #[test]
    fn replaces_expired_client_cell() {
        let state = ProxyState::new(test_config());
        let expired_at = Instant::now()
            .checked_sub(CLIENT_CACHE_ENTRY_TTL + Duration::from_secs(1))
            .unwrap();
        let expired = Arc::new(CachedClientEntry::new(expired_at));

        state
            .clients
            .insert("key-a".to_string(), Arc::clone(&expired));

        let fresh = state.client_entry_for_api_key("key-a");

        assert!(!Arc::ptr_eq(&expired, &fresh));
        assert_eq!(state.clients.len(), 1);
    }

    #[test]
    fn removes_failed_initialization_cell() {
        let state = ProxyState::new(test_config());
        let entry = state.client_entry_for_api_key("key-a");

        state.remove_client_entry_if_same("key-a", &entry);

        assert!(!state.clients.contains_key("key-a"));
    }

    #[tokio::test]
    async fn sse_stream_emits_error_when_idle_timeout_expires() {
        use futures::{stream, StreamExt};

        let backend_stream = Box::pin(stream::pending::<OpenSecretResult<ChatCompletionChunk>>());
        let sse_stream = create_sse_stream(backend_stream, Duration::from_millis(1));
        futures::pin_mut!(sse_stream);

        assert!(
            tokio::time::timeout(Duration::from_secs(1), sse_stream.next())
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            tokio::time::timeout(Duration::from_secs(1), sse_stream.next())
                .await
                .unwrap()
                .is_some()
        );
    }
}
