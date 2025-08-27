use clap::Parser;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Parser, Debug, Clone)]
#[command(name = "maple-proxy")]
#[command(about = "Lightweight OpenAI-compatible proxy server for Maple/OpenSecret")]
pub struct Config {
    /// Host to bind the server to
    #[arg(long, env = "MAPLE_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind the server to
    #[arg(short, long, env = "MAPLE_PORT", default_value = "8080")]
    pub port: u16,

    /// OpenSecret/Maple backend URL
    #[arg(
        long,
        env = "MAPLE_BACKEND_URL",
        default_value = "https://enclave.trymaple.ai"
    )]
    pub backend_url: String,

    /// Default API key for Maple/OpenSecret (can be overridden by client Authorization header)
    #[arg(long, env = "MAPLE_API_KEY")]
    pub default_api_key: Option<String>,

    /// Enable debug logging
    #[arg(short, long, env = "MAPLE_DEBUG")]
    pub debug: bool,

    /// Enable CORS for all origins (useful for web clients)
    #[arg(long, env = "MAPLE_ENABLE_CORS")]
    pub enable_cors: bool,
}

impl Config {
    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let addr = format!("{}:{}", self.host, self.port);
        addr.parse()
            .map_err(|e| anyhow::anyhow!("Invalid socket address '{}': {}", addr, e))
    }

    pub fn load() -> Self {
        // Load from .env file if it exists
        let _ = dotenvy::dotenv();

        Config::parse()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIError {
    pub error: OpenAIErrorDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIErrorDetails {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

impl OpenAIError {
    pub fn new(message: impl Into<String>, error_type: impl Into<String>) -> Self {
        Self {
            error: OpenAIErrorDetails {
                message: message.into(),
                error_type: error_type.into(),
                param: None,
                code: None,
            },
        }
    }

    pub fn authentication_error(message: impl Into<String>) -> Self {
        Self::new(message, "invalid_request_error")
    }

    pub fn server_error(message: impl Into<String>) -> Self {
        Self::new(message, "server_error")
    }
}
