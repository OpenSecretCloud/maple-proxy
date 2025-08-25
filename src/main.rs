mod config;
mod proxy;

use crate::config::Config;
use crate::proxy::{ProxyState, create_chat_completion, health_check, list_models};
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
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load();

    // Initialize tracing
    let filter = if config.debug {
        EnvFilter::from_default_env().add_directive(Level::DEBUG.into())
    } else {
        EnvFilter::from_default_env().add_directive(Level::INFO.into())
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Maple Proxy Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Backend URL: {}", config.backend_url);
    info!("Binding to: {}", config.socket_addr()?);
    
    if config.default_api_key.is_some() {
        info!("Default API key configured");
    } else {
        info!("No default API key - clients must provide Authorization header");
    }

    let state = Arc::new(ProxyState::new(config.clone()));

    // Build the router
    let mut app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        .route("/", get(health_check))
        
        // OpenAI-compatible endpoints
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(create_chat_completion))
        
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                ),
        );

    // Add CORS if enabled
    if config.enable_cors {
        info!("CORS enabled for all origins");
        app = app.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(Any),
        );
    }

    let listener = tokio::net::TcpListener::bind(config.socket_addr()?).await?;
    
    info!("🚀 Maple Proxy Server started successfully!");
    info!("📋 Available endpoints:");
    info!("   GET  /health              - Health check");
    info!("   GET  /v1/models           - List available models"); 
    info!("   POST /v1/chat/completions - Create chat completions (streaming & non-streaming)");
    info!("");
    info!("💡 Usage:");
    info!("   Set MAPLE_API_KEY environment variable or provide Authorization: Bearer <key> header");
    info!("   Compatible with any OpenAI client library!");
    info!("");
    info!("🔗 Example curl:");
    info!("   curl http://{} \\", config.socket_addr()?);
    info!("     -H \"Authorization: Bearer YOUR_MAPLE_API_KEY\" \\");
    info!("     -H \"Content-Type: application/json\" \\");
    info!("     -d '{{\"model\": \"gpt-4\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello!\"}}]}}'");
    info!("     /v1/chat/completions");

    axum::serve(listener, app).await?;

    Ok(())
}