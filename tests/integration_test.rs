use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sekha_controller::{
    api::routes::{create_router, AppState},
    config::Config,
    models::internal::Conversation,
    services::embedding_service::EmbeddingService,
    storage::{
        chroma_client::ChromaClient, init_db, ConversationRepository, SeaOrmConversationRepository,
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

// Helper to create test services
fn create_test_services() -> (Arc<ChromaClient>, Arc<EmbeddingService>) {
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    (chroma_client, embedding_service)
}

async fn create_test_config() -> Arc<RwLock<Config>> {
    Arc::new(RwLock::new(Config {
        server_port: 8080,
        mcp_api_key: "test_key_12345678901234567890123456789012".to_string(),
        database_url: "sqlite::memory:".to_string(),
        ollama_url: "http://localhost:11434".to_string(),
        chroma_url: "http://localhost:8000".to_string(),
        max_connections: 10,
        log_level: "info".to_string(),
        summarization_enabled: true,
        pruning_enabled: true,
        embedding_model: "nomic-embed-text:latest".to_string(),
        summarization_model: "llama3.1:8b".to_string(),
    }))
}

#[tokio::test]
async fn test_repository_create_and_retrieve() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let conv = Conversation {
        id: Uuid::new_v4(),
        label: "Integration Test".to_string(),
        folder: "/tests".to_string(),
        status: "active".to_string(),
        importance_score: 8,
        word_count: 42,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let id = repo.create(conv).await.unwrap();

    let retrieved = repo.find_by_id(id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.label, "Integration Test");
    assert_eq!(retrieved.folder, "/tests");
}

#[tokio::test]
async fn test_repository_update_label() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let conv = Conversation {
        id: Uuid::new_v4(),
        label: "Original".to_string(),
        folder: "/original".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 10,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let id = repo.create(conv).await.unwrap();

    repo.update_label(id, "Updated", "/updated").await.unwrap();

    let retrieved = repo.find_by_id(id).await.unwrap();
    assert_eq!(retrieved.unwrap().label, "Updated");
}

#[tokio::test]
async fn test_api_create_conversation() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let state = AppState {
        config: create_test_config().await,
        repo: repo.clone(),
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"label": "API Test", "folder": "/api", "messages": [{"role": "user", "content": "Hello"}]}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("API Test"));
}

#[tokio::test]
async fn test_api_query_endpoint() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let state = AppState {
        config: create_test_config().await,
        repo: repo.clone(),
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"query": "test", "limit": 10}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_memory_orchestrator_integration() {
    // Setup
    let repo = setup_test_repo().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:5001".to_string()));
    let orchestrator = MemoryOrchestrator::new(repo, llm_bridge);

    // Test context assembly
    let context = orchestrator
        .assemble_context("test", vec![], 1000)
        .await
        .unwrap();
    assert!(!context.is_empty());

    // Test importance scoring
    let score = orchestrator
        .score_message_importance(context[0].id)
        .await
        .unwrap();
    assert!(score >= 1.0 && score <= 10.0);

    // Test summarization
    let summary = orchestrator
        .generate_daily_summary(context[0].conversation_id)
        .await
        .unwrap();
    assert!(!summary.is_empty());
}
