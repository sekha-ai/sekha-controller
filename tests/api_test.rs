use sekha_controller::{
    api::{routes, mcp},
    storage,
    config,
    services::embedding_service::EmbeddingService,
    storage::chroma_client::ChromaClient,
};
use axum::{http::StatusCode, body::Body};
use tower::ServiceExt;
use std::sync::Arc;
use tokio::sync::RwLock;

// Helper to create test services
fn create_test_services() -> (Arc<ChromaClient>, Arc<EmbeddingService>) {
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    (chroma_client, embedding_service)
}

async fn create_test_config() -> Arc<RwLock<config::Config>> {
    let config_str = r#"
        server_port = 8080
        mcp_api_key = "test_key_12345678901234567890123456789012"
        database_url = "sqlite::memory:"
        ollama_url = "http://localhost:11434"
        chroma_url = "http://localhost:8000"
        max_connections = 10
        log_level = "info"
        summarization_enabled = true
        pruning_enabled = true
        embedding_model = "nomic-embed-text:latest"
        summarization_model = "llama3.1:8b"
    "#;
    
    let config: config::Config = toml::from_str(config_str).unwrap();
    Arc::new(RwLock::new(config))
}

#[tokio::test]
async fn test_create_conversation() {
    let db_conn = storage::init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(storage::SeaOrmConversationRepository::new(db_conn, chroma_client, embedding_service));
    let test_config = create_test_config().await;
    
    let state = routes::AppState {
        config: test_config,
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
    let db_conn = storage::init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(storage::SeaOrmConversationRepository::new(db_conn, chroma_client, embedding_service));
    let test_config = create_test_config().await;
    
    let state = routes::AppState {
        config: test_config,
        repo: repo.clone(),
    };
    
    let app = mcp::create_mcp_router(state);
    
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"label": "Test", "folder": "/", "messages": [{"role": "user", "content": "Hello"}]}"#))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_smart_query_endpoint() {
    let repo = setup_test_repo().await;
    let orchestrator = Arc::new(MemoryOrchestrator::new(repo.clone()));
    
    let app = create_router(AppState {
        config: Arc::new(RwLock::new(Config::default())),
        repo,
        orchestrator,
    });

    // Create test conversation
    let create_req = CreateConversationRequest {
        label: "Test::SmartQuery".to_string(),
        folder: "/test".to_string(),
        messages: vec![
            MessageDto {
                role: "user".to_string(),
                content: "What is the token limit for Claude?".to_string(),
            }
        ],
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_req).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // Test smart query
    let query_req = QueryRequest {
        query: "token limit".to_string(),
        limit: Some(10),
        offset: Some(0),
        filters: None,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/query/smart")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&query_req).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: QueryResponse = serde_json::from_slice(&body).unwrap();
    assert!(!result.results.is_empty());
}