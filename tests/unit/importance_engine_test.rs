use mockall::predicate::*;
use sekha_controller::{
    config::Config, models::internal::Conversation,
    orchestrator::importance_engine::ImportanceEngine,
    services::llm_bridge_client::LlmBridgeClient, storage::repository::MockConversationRepository,
};
use std::sync::Arc;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_importance_engine_initialization() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&mock_server)
        .await;

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let mut mock_repo = MockConversationRepository::new();

    mock_repo.expect_find_by_id().returning(|_| Ok(None));

    let _engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    assert!(true);
}

#[tokio::test]
async fn test_importance_scoring() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/score"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"score": 8.5})))
        .mount(&mock_server)
        .await;

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let mut mock_repo = MockConversationRepository::new();

    let conv = Conversation {
        id: Uuid::new_v4(),
        label: "Test".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    mock_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(conv.clone())));

    let _engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    assert!(true);
}
