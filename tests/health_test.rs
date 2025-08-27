use axum::http::StatusCode;
use axum_test::TestServer;
use maple_proxy::{create_app, Config};
use serde_json::Value;

#[tokio::test]
async fn test_health_check_endpoint() {
    // Create test config
    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 0, // Use random port for testing
        backend_url: "http://localhost:3000".to_string(),
        default_api_key: None,
        debug: false,
        enable_cors: false,
    };

    // Create test server
    let app = create_app(config);
    let server = TestServer::new(app).unwrap();

    // Test /health endpoint
    let response = server.get("/health").await;
    response.assert_status(StatusCode::OK);

    let json: Value = response.json();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "maple-proxy");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_root_health_check() {
    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        backend_url: "http://localhost:3000".to_string(),
        default_api_key: None,
        debug: false,
        enable_cors: false,
    };

    let app = create_app(config);
    let server = TestServer::new(app).unwrap();

    // Test root endpoint also returns health check
    let response = server.get("/").await;
    response.assert_status(StatusCode::OK);

    let json: Value = response.json();
    assert_eq!(json["status"], "ok");
}
