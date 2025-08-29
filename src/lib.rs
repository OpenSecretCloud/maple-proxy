pub mod config;
pub mod proxy;

pub use config::{Config, OpenAIError, OpenAIErrorDetails};
use proxy::{create_chat_completion, health_check, list_models, ProxyState};

use axum::{
    http::Method,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

/// Create the Axum application with the given configuration
pub fn create_app(config: Config) -> Router {
    let state = Arc::new(ProxyState::new(config.clone()));

    let mut app = Router::new()
        // Health check endpoints
        .route("/health", get(health_check))
        .route("/", get(health_check))
        // OpenAI-compatible endpoints
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(create_chat_completion))
        .with_state(state)
        .layer(
            ServiceBuilder::new().layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            ),
        );

    // Add CORS if enabled
    if config.enable_cors {
        app = app.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(Any),
        );
    }

    app
}
