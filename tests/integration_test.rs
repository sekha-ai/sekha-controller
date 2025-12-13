use sekha_controller::{
    storage::{init_db, SeaOrmConversationRepository, ConversationRepository},
    models::internal::Conversation,
    api::routes::{create_router, AppState},
    config::Config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;

async fn create_test_config() -> Arc<RwLock<Config>> {
    Arc::new(RwLock::new(Config {
        server_port: 8080,
        mcp_api_key: "test_key_12345678901234567890123456789012".to_string(),
        database_url: "sqlite::memory:".to_string(),
        ollama_url: "http://localhost:11434".to_string(),
        max_connections: 10,
        log_level: "info".to_string(),
        summarization_enabled: true,
        pruning_enabled: true,
    }))
}

#[tokio::test]
async fn test_repository_create_and_retrieve() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let repo = SeaOrmConversationRepository::new(db);
    
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
    let repo = SeaOrmConversationRepository::new(db);
    
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
    let repo = Arc::new(SeaOrmConversationRepository::new(db));
    
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
    
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("API Test"));
}

#[tokio::test]
async fn test_api_query_endpoint() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let repo = Arc::new(SeaOrmConversationRepository::new(db));
    
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
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}