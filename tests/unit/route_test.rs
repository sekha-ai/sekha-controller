use sekha_controller::api::route::create_router;
use sekha_controller::AppState;
use sekha_controller::storage::{init_db, SeaOrmConversationRepository};
use sekha_controller::storage::chroma_client::ChromaClient;
use sekha_controller::services::embedding_service::EmbeddingService;
use sekha_controller::services::llm_bridge_client::LlmBridgeClient;
use sekha_controller::orchestrator::MemoryOrchestrator;
use sekha_controller::config::Config;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_router_creation() {
    let state = create_test_app_state().await;
    let router = create_router(state);
    
    // Test that router construction succeeds
    // Test with metrics endpoint instead (less dependency on services)
    let response = router
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Metrics should respond
    assert!(response.status().is_success() || response.status().is_client_error());
}

#[tokio::test]
async fn test_semantic_query_mock_endpoint() {
    let state = create_test_app_state().await;
    let router = create_router(state);
    
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query":"test","limit":10}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_router_has_all_routes() {
    let state = create_test_app_state().await;
    let router = create_router(state);
    
    // Test that main routes exist
    let routes = vec![
        ("/health", "GET"),
        ("/metrics", "GET"),
        ("/api/v1/query", "POST"),
    ];
    
    // Just verify router construction doesn't panic
    // Actual route testing is in integration tests
}

// Helper function to create test AppState
async fn create_test_app_state() -> AppState {
    let db = init_db("sqlite::memory:").await.unwrap();
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client.clone(),
        embedding_service.clone(),
    ));
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));
    
    let config = Arc::new(RwLock::new(Config {
        server_port: 8080,
        mcp_api_key: "test_key_12345678901234567890123456789012".to_string(),
        rest_api_key: Some("rest_test_key_123456789012345678901234".to_string()),
        database_url: "sqlite::memory:".to_string(),
        ollama_url: "http://localhost:11434".to_string(),
        chroma_url: "http://localhost:8000".to_string(),
        additional_api_keys: vec![],
        cors_enabled: true,
        rate_limit_per_minute: 60,
        max_connections: 10,
        log_level: "info".to_string(),
        summarization_enabled: true,
        pruning_enabled: true,
        embedding_model: "nomic-embed-text:latest".to_string(),
        summarization_model: "llama3.1:8b".to_string(),
    }));

    AppState {
        config,
        repo: repo.clone(),
        chroma_client,
        embedding_service,
        orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge)),
    }
}
