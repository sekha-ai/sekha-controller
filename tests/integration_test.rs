use sekha_controller::{
    config::Config,
    storage::{init_db, SeaOrmConversationRepository, ConversationRepository},
    models::internal::Conversation,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

#[tokio::test]
async fn test_create_conversation() {
    // Setup
    let db = init_db("sqlite::memory:").await.unwrap();
    let repo = SeaOrmConversationRepository::new(db);
    
    // Create conversation
    let conv = Conversation {
        id: Uuid::new_v4(),
        label: "Test".to_string(),
        folder: "/".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 10,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };
    
    // Test
    let result = repo.create(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_api_create_conversation() {
    // Setup config
    let config = Arc::new(RwLock::new(Config::load().unwrap()));
    let db = init_db("sqlite::memory:").await.unwrap();
    let repo = Arc::new(SeaOrmConversationRepository::new(db));
    
    let state = sekha_controller::api::routes::AppState {
        config,
        repo: repo.clone(),
    };
    
    let app = sekha_controller::api::routes::create_router(state);
    
    let response = app
        .oneshot(
            Request::builder()
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
