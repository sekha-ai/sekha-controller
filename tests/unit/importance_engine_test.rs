use async_trait::async_trait;
use mockall::mock;
use mockall::predicate::*;
use sekha_controller::models::internal::Message;
use sekha_controller::orchestrator::importance_engine::ImportanceEngine;
use sekha_controller::services::llm_bridge_client::LlmBridgeClient;
use sekha_controller::storage::repository::{ConversationRepository, RepositoryError};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Manually define the mock since automock doesn't work for integration tests
mock! {
    pub ConversationRepo {}

    #[async_trait]
    impl ConversationRepository for ConversationRepo {
        async fn create(&self, conv: sekha_controller::models::internal::Conversation) -> Result<Uuid, RepositoryError>;
        async fn create_with_messages(&self, conv: sekha_controller::models::internal::NewConversation) -> Result<Uuid, RepositoryError>;
        async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
        async fn count_by_label(&self, label: &str) -> Result<u64, RepositoryError>;
        async fn count_by_folder(&self, folder: &str) -> Result<u64, RepositoryError>;
        async fn count_all(&self) -> Result<u64, RepositoryError>;
        async fn find_by_id(&self, id: Uuid) -> Result<Option<sekha_controller::models::internal::Conversation>, RepositoryError>;
        async fn find_by_label(&self, label: &str, limit: u64, offset: u64) -> Result<Vec<sekha_controller::models::internal::Conversation>, RepositoryError>;
        async fn get_conversation_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, RepositoryError>;
        async fn find_message_by_id(&self, id: Uuid) -> Result<Option<Message>, RepositoryError>;
        async fn find_recent_messages(&self, conversation_id: Uuid, limit: usize) -> Result<Vec<Message>, RepositoryError>;
        async fn find_with_filters(&self, filter: Option<String>, limit: usize, offset: u32) -> Result<(Vec<sekha_controller::models::internal::Conversation>, u64), RepositoryError>;
        async fn update_label(&self, id: Uuid, new_label: &str, new_folder: &str) -> Result<(), RepositoryError>;
        async fn get_message_list(&self, conversation_id: Uuid) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>;
        async fn get_stats(&self, folder: Option<String>) -> Result<sekha_controller::storage::repository::Stats, Box<dyn std::error::Error>>;
        async fn get_stats_by_folder(&self, folder: Option<String>) -> Result<sekha_controller::storage::repository::Stats, Box<dyn std::error::Error>>;
        async fn get_stats_by_label(&self, label: Option<String>) -> Result<sekha_controller::storage::repository::Stats, Box<dyn std::error::Error>>;
        async fn get_all_folders(&self) -> Result<Vec<String>, RepositoryError>;
        async fn find_by_folder(&self, folder: &str, limit: u64, offset: u64) -> Result<Vec<sekha_controller::models::internal::Conversation>, RepositoryError>;
        async fn update_status(&self, id: Uuid, status: &str) -> Result<(), RepositoryError>;
        async fn update_importance(&self, id: Uuid, score: i32) -> Result<(), RepositoryError>;
        async fn count_messages_in_conversation(&self, conversation_id: Uuid) -> Result<u64, RepositoryError>;
        async fn full_text_search(&self, query: &str, limit: usize) -> Result<Vec<Message>, RepositoryError>;
        async fn semantic_search(&self, query: &str, limit: usize, filters: Option<serde_json::Value>) -> Result<Vec<sekha_controller::storage::repository::SearchResult>, RepositoryError>;
        async fn get_all_labels(&self) -> Result<Vec<String>, RepositoryError>;
        fn get_db(&self) -> &sea_orm::DatabaseConnection;
    }
}

#[tokio::test]
async fn test_calculate_score_basic_message() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let mut mock_repo = MockConversationRepo::new();
    let message_id = Uuid::new_v4();

    // Create test message with correct fields
    let test_message = Message {
        id: message_id,
        conversation_id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "This is a short message".to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        embedding_id: None,
        metadata: None,
    };

    mock_repo
        .expect_find_message_by_id()
        .with(eq(message_id))
        .times(1)
        .returning(move |_| Ok(Some(test_message.clone())));

    // Mock LLM score response
    Mock::given(method("POST"))
        .and(path("/score_importance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "score": 0.6,
            "reasoning": "Normal importance",
            "model": "llama3.1:8b"
        })))
        .mount(&mock_server)
        .await;

    let engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    let score = engine.calculate_score(message_id).await.unwrap();

    // Score should be weighted: (heuristic * 0.3) + (llm_score * 0.7)
    // heuristic ~5.0, llm 0.6 -> (5.0 * 0.3) + (0.6 * 0.7) = 1.5 + 0.42 = 1.92
    assert!(score > 1.0);
    assert!(score < 3.0);
}

#[tokio::test]
async fn test_calculate_score_important_message_with_code() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let mut mock_repo = MockConversationRepo::new();
    let message_id = Uuid::new_v4();

    // Important message with keywords and code
    let test_message = Message {
        id: message_id,
        conversation_id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "This is a critical decision with code: ```rust\nfn main() {}\n``` - over 100 characters long to trigger length bonus".to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        embedding_id: None,
        metadata: Some(json!({"type": "important"})),
    };

    mock_repo
        .expect_find_message_by_id()
        .with(eq(message_id))
        .times(1)
        .returning(move |_| Ok(Some(test_message.clone())));

    // Mock high LLM score
    Mock::given(method("POST"))
        .and(path("/score_importance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "score": 0.9,
            "reasoning": "High importance",
            "model": "llama3.1:8b"
        })))
        .mount(&mock_server)
        .await;

    let engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    let score = engine.calculate_score(message_id).await.unwrap();

    // Should have high score due to:
    // - "critical" keyword (+1.0)
    // - Length > 100 (+1.0)
    // - Code block ``` (+2.0)
    // Base 5.0 + 4.0 = 9.0 heuristic
    // (9.0 * 0.3) + (0.9 * 0.7) = 2.7 + 0.63 = 3.33
    assert!(
        score > 3.0,
        "Score should be high for important message, got {}",
        score
    );
}

#[tokio::test]
async fn test_calculate_score_question_with_urgent_keyword() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let mut mock_repo = MockConversationRepo::new();
    let message_id = Uuid::new_v4();

    let test_message = Message {
        id: message_id,
        conversation_id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "What is the urgent issue here?".to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        embedding_id: None,
        metadata: None,
    };

    mock_repo
        .expect_find_message_by_id()
        .with(eq(message_id))
        .times(1)
        .returning(move |_| Ok(Some(test_message.clone())));

    Mock::given(method("POST"))
        .and(path("/score_importance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "score": 0.7,
            "reasoning": "Question with urgent keyword",
            "model": "llama3.1:8b"
        })))
        .mount(&mock_server)
        .await;

    let engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    let score = engine.calculate_score(message_id).await.unwrap();

    // Heuristic: 5.0 + 1.0 (urgent) + 0.5 (question mark) = 6.5
    // (6.5 * 0.3) + (0.7 * 0.7) = 1.95 + 0.49 = 2.44
    assert!(score > 2.0, "Score: {}", score);
}

#[tokio::test]
async fn test_calculate_score_message_not_found() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let mut mock_repo = MockConversationRepo::new();
    let message_id = Uuid::new_v4();

    mock_repo
        .expect_find_message_by_id()
        .with(eq(message_id))
        .times(1)
        .returning(|_| Ok(None));

    let engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    let result = engine.calculate_score(message_id).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Message not found"));
}

#[tokio::test]
async fn test_calculate_score_llm_error() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let mut mock_repo = MockConversationRepo::new();
    let message_id = Uuid::new_v4();

    let test_message = Message {
        id: message_id,
        conversation_id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "Test message".to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        embedding_id: None,
        metadata: None,
    };

    mock_repo
        .expect_find_message_by_id()
        .with(eq(message_id))
        .times(1)
        .returning(move |_| Ok(Some(test_message.clone())));

    // Mock LLM error (500)
    Mock::given(method("POST"))
        .and(path("/score_importance"))
        .respond_with(ResponseTemplate::new(500).set_body_string("LLM unavailable"))
        .mount(&mock_server)
        .await;

    let engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    let result = engine.calculate_score(message_id).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("LLM Bridge error"));
}

#[tokio::test]
async fn test_heuristic_score_all_keywords() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let mut mock_repo = MockConversationRepo::new();
    let message_id = Uuid::new_v4();

    // Message with ALL heuristic bonuses
    let test_message = Message {
        id: message_id,
        conversation_id: Uuid::new_v4(),
        role: "user".to_string(),
        content: "This is a critical important urgent decision with code ```js\nconsole.log('test')\n``` that requires immediate attention?".to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        embedding_id: None,
        metadata: None,
    };

    mock_repo
        .expect_find_message_by_id()
        .with(eq(message_id))
        .times(1)
        .returning(move |_| Ok(Some(test_message.clone())));

    Mock::given(method("POST"))
        .and(path("/score_importance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "score": 0.95,
            "reasoning": "Maximum importance",
            "model": "llama3.1:8b"
        })))
        .mount(&mock_server)
        .await;

    let engine = ImportanceEngine::new(Arc::new(mock_repo), llm_bridge);
    let score = engine.calculate_score(message_id).await.unwrap();

    // Heuristic: 5.0 + 1.0 (length) + 2.0 (code) + 0.5 (question) + 3.0 (keywords) = 11.5
    // But clamped to 10.0
    // (10.0 * 0.3) + (0.95 * 0.7) = 3.0 + 0.665 = 3.665
    assert!(score > 3.5, "Score should be maximum, got {}", score);
}
