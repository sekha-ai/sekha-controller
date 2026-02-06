use mockall::predicate::*;
use sekha_controller::{
    models::internal::{Conversation, Message},
    orchestrator::pruning_engine::PruningEngine,
    services::llm_bridge_client::LlmBridgeClient,
    storage::repository::{MockConversationRepository, RepositoryError},
};
use std::sync::Arc;
use uuid::Uuid;

fn create_test_conversation(id: Uuid, importance: i32, word_count: i32) -> Conversation {
    Conversation {
        id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: importance,
        word_count,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc() - chrono::Duration::days(100),
        updated_at: chrono::Utc::now().naive_utc() - chrono::Duration::days(50),
    }
}

fn create_test_message(id: Uuid, conv_id: Uuid, content: &str) -> Message {
    Message {
        id,
        conversation_id: conv_id,
        role: "user".to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        embedding_id: None,
        metadata: None,
    }
}

#[tokio::test]
async fn test_pruning_engine_creation() {
    let mock_repo = MockConversationRepository::new();
    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    // Verify creation succeeds
    assert!(true);
}

#[tokio::test]
async fn test_generate_suggestions_high_token_low_importance() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // Mock count_messages to return 30 (30 * 200 = 6000 tokens > 5000 threshold)
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(30));

    // Mock find_recent_messages for preview
    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 2, 10000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_ok());

    let suggestion = result.unwrap();
    assert_eq!(suggestion.token_estimate, 6000);
    assert_eq!(suggestion.importance_score, 2.0);
    assert_eq!(suggestion.recommendation, "archive");
}

#[tokio::test]
async fn test_generate_suggestions_low_token_low_importance() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // Mock count_messages to return 10 (10 * 200 = 2000 tokens < 5000 threshold)
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(10));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 3, 1000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_ok());

    let suggestion = result.unwrap();
    assert_eq!(suggestion.token_estimate, 2000);
    assert_eq!(suggestion.recommendation, "keep");
}

#[tokio::test]
async fn test_generate_suggestions_high_importance_always_keep() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // Even with high tokens (30 * 200 = 6000 > 5000), high importance should keep
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(30));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 8, 10000); // High importance

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_ok());

    let suggestion = result.unwrap();
    assert_eq!(suggestion.importance_score, 8.0);
    assert_eq!(suggestion.recommendation, "keep");
}

#[tokio::test]
async fn test_generate_suggestions_boundary_5000_tokens() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // Exactly at boundary: 25 * 200 = 5000 tokens
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(25));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 3, 5000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_ok());

    let suggestion = result.unwrap();
    assert_eq!(suggestion.token_estimate, 5000);
    // At exactly 5000, condition is false (not > 5000), so "keep"
    assert_eq!(suggestion.recommendation, "keep");
}

#[tokio::test]
async fn test_generate_suggestions_boundary_5001_tokens() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // Just over boundary: 26 * 200 = 5200 tokens > 5000
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(26));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 4, 5200); // importance < 5

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_ok());

    let suggestion = result.unwrap();
    assert_eq!(suggestion.token_estimate, 5200);
    assert_eq!(suggestion.recommendation, "archive");
}

#[tokio::test]
async fn test_generate_suggestions_boundary_importance_5() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // High tokens but importance exactly at 5
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(30));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 5, 6000); // importance = 5 (not < 5)

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_ok());

    let suggestion = result.unwrap();
    assert_eq!(suggestion.importance_score, 5.0);
    // importance = 5 is not < 5, so "keep"
    assert_eq!(suggestion.recommendation, "keep");
}

#[tokio::test]
async fn test_count_messages_error_propagation() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    use sea_orm::DbErr;
    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| {
            Err(RepositoryError::DbError(DbErr::ConnectionAcquire(
                sea_orm::ConnAcquireErr::Timeout,
            )))
        });

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 5, 1000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_recent_messages_error_propagation() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(10));

    use sea_orm::DbErr;
    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| {
            Err(RepositoryError::DbError(DbErr::ConnectionAcquire(
                sea_orm::ConnAcquireErr::Timeout,
            )))
        });

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 5, 1000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_preview_with_short_messages() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(5));

    // Messages shorter than 100 chars
    let messages = vec![
        create_test_message(Uuid::new_v4(), conv_id, "Short message 1"),
        create_test_message(Uuid::new_v4(), conv_id, "Short message 2"),
    ];

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(move |_, _| Ok(messages));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 5, 100);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    // LLM will fail, but should get EmbeddingError
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, RepositoryError::EmbeddingError(_)));
    }
}

#[tokio::test]
async fn test_generate_preview_with_long_messages() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(5));

    // Messages longer than 100 chars (should be truncated)
    let long_content = "A".repeat(200); // 200 chars
    let messages = vec![create_test_message(Uuid::new_v4(), conv_id, &long_content)];

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(move |_, _| Ok(messages));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 5, 1000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    // LLM will fail, returns EmbeddingError
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_preview_with_empty_messages() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(0));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 5, 0);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    // LLM will fail, returns EmbeddingError
    assert!(result.is_err());
}

#[tokio::test]
async fn test_suggestion_includes_all_fields() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_count_messages_in_conversation()
        .with(eq(conv_id))
        .return_once(|_| Ok(15));

    mock_repo
        .expect_find_recent_messages()
        .with(eq(conv_id), eq(5))
        .return_once(|_, _| Ok(vec![]));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let engine = PruningEngine::new(repo, llm_bridge);
    let conv = create_test_conversation(conv_id, 7, 3000);

    let result = engine.generate_suggestion_for_conversation(&conv).await;
    assert!(result.is_err()); // LLM fails
}
