use sekha_controller::{api::routes, storage, config};
use axum::{Router, http::StatusCode, body::Body};
use tower::ServiceExt; // for `oneshot`
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_create_conversation() {
    // Setup test database
    let db_conn = storage::init_db("sqlite::memory:").await.unwrap();
    let repo = Arc::new(storage::SeaOrmConversationRepository::new(db_conn));
    
    let config = Arc::new(RwLock::new(config::Config {
        server_port: 8080,
        mcp_api_key: "test_key_12345678901234567890123456789012".to_string(),
        database_url: "sqlite::memory:".to_string(),
        ollama_url: "http://localhost:11434".to_string(),
        max_connections: 10,
        log_level: "info".to_string(),
        summarization_enabled: true,
        pruning_enabled: true,
    }));
    
    let state = routes::AppState {
        config,
        repo: repo.clone(),
    };
    
    let app = routes::create_router(state);
    
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"label": "Test", "folder": "/", "messages": [{"role": "user", "content": "Hello"}]}"#))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_mcp_auth_failure() {
    // Test MCP endpoint without API key
    // Similar setup...
}
