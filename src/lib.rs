pub mod config;
pub mod proxy;

pub use config::{Config, OpenAIError, OpenAIErrorDetails};
use proxy::{create_chat_completion, create_embeddings, health_check, list_models, ProxyState};

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

#[cfg(feature = "l402")]
use toll_booth::{
    FreeTierConfig, L402Rail, L402RailConfig, MemoryStorage, PricingEntry, TollBoothConfig,
    TollBoothEngine, TollBoothLayer,
};

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
        .route("/v1/embeddings", post(create_embeddings))
        .with_state(state)
        .layer(
            ServiceBuilder::new().layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            ),
        );

    // Add L402 payment gating if enabled
    #[cfg(feature = "l402")]
    {
        if config.l402_enabled() {
            let root_key = config.root_key.clone().unwrap();
            let storage: std::sync::Arc<dyn toll_booth::StorageBackend> =
                std::sync::Arc::new(MemoryStorage::new());

            let l402_rail = L402Rail::new(L402RailConfig {
                root_key: root_key.clone(),
                storage: storage.clone(),
                default_amount: config.price_sats,
                backend: None, // No Lightning backend yet - generates test invoices
                service_name: Some("maple-proxy".to_string()),
            });

            let mut pricing = std::collections::HashMap::new();
            pricing.insert(
                "/v1/chat/completions".to_string(),
                PricingEntry::Simple(config.price_sats),
            );
            pricing.insert(
                "/v1/embeddings".to_string(),
                PricingEntry::Simple(config.price_sats),
            );
            // /v1/models is free (discovery endpoint)

            let mut tb_config = TollBoothConfig {
                storage,
                pricing,
                upstream: config.backend_url.clone(),
                root_key,
                rails: vec![Box::new(l402_rail)],
                ..Default::default()
            };

            if config.free_requests > 0 {
                tb_config.free_tier = Some(FreeTierConfig::Requests(config.free_requests));
            }

            let engine = TollBoothEngine::new(tb_config)
                .expect("Failed to create toll-booth engine");

            app = app.layer(TollBoothLayer::new(engine));
            tracing::info!(
                "L402 payment gating enabled ({} sats/request, {} free/day)",
                config.price_sats,
                config.free_requests
            );
        }
    }

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
