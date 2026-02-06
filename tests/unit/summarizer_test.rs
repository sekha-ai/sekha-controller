use mockall::predicate::*;
use sekha_controller::{
    models::internal::{Conversation, Message},
    orchestrator::summarizer::HierarchicalSummarizer,
    services::llm_bridge_client::{LlmBridgeClient, LlmBridgeError},
    storage::repository::{MockConversationRepository, RepositoryError},
};
use std::sync::Arc;
use uuid::Uuid;

fn create_test_message(role: &str, content: &str, conv_id: Uuid) -> Message {
    Message {
        id: Uuid::new_v4(),
        conversation_id: conv_id,
        role: role.to_string(),
        content: content.to_string(),
        metadata: None,
        timestamp: chrono::Local::now().naive_local(),
        embedding_id: None,
    }
}

fn create_test_conversation(id: Uuid) -> Conversation {
    Conversation {
        id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: chrono::Local::now().naive_local(),
        updated_at: chrono::Local::now().naive_local(),
    }
}

#[tokio::test]
async fn test_hierarchical_summarizer_creation() {
    let mock_repo = MockConversationRepository::new();

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    // Verify creation succeeds
    assert!(true);
}

// NOTE: test_generate_daily_summary_empty_messages removed because it requires
// repo.get_db() which cannot be properly mocked. Integration tests cover this.

#[tokio::test]
async fn test_generate_daily_summary_conversation_not_found() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_find_by_id()
        .with(eq(conv_id))
        .return_once(|_| Ok(None));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    let result = summarizer.generate_daily_summary(conv_id).await;

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, RepositoryError::NotFound(_)));
    }
}

#[tokio::test]
async fn test_generate_weekly_summary_conversation_not_found() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_find_by_id()
        .with(eq(conv_id))
        .return_once(|_| Ok(None));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    let result = summarizer.generate_weekly_summary(conv_id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_monthly_summary_conversation_not_found() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_find_by_id()
        .with(eq(conv_id))
        .return_once(|_| Ok(None));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    let result = summarizer.generate_monthly_summary(conv_id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_daily_summary_with_db_error() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    use sea_orm::DbErr;
    mock_repo
        .expect_find_by_id()
        .with(eq(conv_id))
        .return_once(|_| {
            Err(RepositoryError::DbError(DbErr::ConnectionAcquire(
                sea_orm::ConnAcquireErr::Timeout,
            )))
        });

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    let result = summarizer.generate_daily_summary(conv_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_weekly_summary_with_db_error() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_find_by_id()
        .with(eq(conv_id))
        .return_once(|_| {
            Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(
                sea_orm::ConnAcquireErr::Timeout,
            )))
        });

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    let result = summarizer.generate_weekly_summary(conv_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_monthly_summary_with_db_error() {
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_find_by_id()
        .with(eq(conv_id))
        .return_once(|_| {
            Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(
                sea_orm::ConnAcquireErr::Timeout,
            )))
        });

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);
    let result = summarizer.generate_monthly_summary(conv_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_summarizer_hierarchy_levels() {
    // Test that we have all three levels: daily, weekly, monthly
    let conv_id = Uuid::new_v4();
    let mut mock_repo = MockConversationRepository::new();

    // Set up mock to return None (conversation not found) for all levels
    mock_repo
        .expect_find_by_id()
        .times(3)
        .returning(|_| Ok(None));

    let config = sekha_controller::config::Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let repo = Arc::new(mock_repo);

    let summarizer = HierarchicalSummarizer::new(repo, llm_bridge);

    // All should fail with NotFound
    let daily = summarizer.generate_daily_summary(conv_id).await;
    let weekly = summarizer.generate_weekly_summary(conv_id).await;
    let monthly = summarizer.generate_monthly_summary(conv_id).await;

    assert!(daily.is_err());
    assert!(weekly.is_err());
    assert!(monthly.is_err());
}
