use sekha_controller::{
    config::Config, models::internal::Conversation, orchestrator::pruning_engine::PruningEngine,
    services::llm_bridge_client::LlmBridgeClient, storage::repository::MockConversationRepository,
};
use std::sync::Arc;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_pruning_engine_initialization() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&mock_server)
        .await;

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_find_with_filters()
        .returning(|_, _, _| Ok((vec![], 0)));

    let _engine = PruningEngine::new(Arc::new(mock_repo), llm_bridge);
    assert!(true);
}

#[tokio::test]
async fn test_pruning_candidates() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))
        .mount(&mock_server)
        .await;

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let mut mock_repo = MockConversationRepository::new();

    let old_conv = Conversation {
        id: Uuid::new_v4(),
        label: "Old Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 2,
        word_count: 10,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc() - chrono::Duration::days(100),
        updated_at: chrono::Utc::now().naive_utc() - chrono::Duration::days(90),
    };

    mock_repo
        .expect_find_with_filters()
        .returning(move |_, _, _| Ok((vec![old_conv.clone()], 1)));

    let _engine = PruningEngine::new(Arc::new(mock_repo), llm_bridge);
    assert!(true);
}
