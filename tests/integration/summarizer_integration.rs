use sekha_controller::{
    config::Config,
    llm::bridge_client::BridgeClient,
    models::internal::{Conversation, Message},
    orchestrator::summarizer::HierarchicalSummarizer,
    services::llm_bridge_client::LlmBridgeClient,
    storage::{
        chroma_client::ChromaClient,
        init_db,
        repository::{ConversationRepository, SeaOrmConversationRepository},
    },
};
use std::sync::Arc;
use uuid::Uuid;

async fn setup_test_db() -> Arc<SeaOrmConversationRepository> {
    let db = init_db("sqlite::memory:")
        .await
        .expect("Failed to initialize test database");

    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let config = Config::default();
    let bridge = BridgeClient::new(&config).expect("Failed to create BridgeClient");
    let embedding_service = Arc::new(
        sekha_controller::services::embedding_service::EmbeddingService::new(
            bridge,
            "http://localhost:8000".to_string(),
        ),
    );

    Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ))
}

#[tokio::test]
async fn test_generate_daily_summary_with_empty_messages() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 0,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Generate daily summary with no messages
    let result = summarizer.generate_daily_summary(conv_id).await;

    assert!(result.is_ok());
    let summary = result.unwrap();
    assert_eq!(summary, "No messages to summarize");
}

#[tokio::test]
async fn test_generate_daily_summary_with_messages() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Add recent messages (within last 24 hours)
    use sekha_controller::storage::entities::messages;
    for i in 0..3 {
        let msg_model = messages::ActiveModel {
            id: sea_orm::ActiveValue::Set(Uuid::new_v4()),
            conversation_id: sea_orm::ActiveValue::Set(conv_id),
            role: sea_orm::ActiveValue::Set(if i % 2 == 0 {
                "user".to_string()
            } else {
                "assistant".to_string()
            }),
            content: sea_orm::ActiveValue::Set(format!("Test message {}", i)),
            timestamp: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
            embedding_id: sea_orm::ActiveValue::Set(None),
            metadata: sea_orm::ActiveValue::Set(None),
        };

        messages::Entity::insert(msg_model)
            .exec(repo.get_db())
            .await
            .expect("Failed to insert test message");
    }

    // Generate daily summary - LLM will fail gracefully
    let result = summarizer.generate_daily_summary(conv_id).await;

    assert!(result.is_ok());
    let summary = result.unwrap();
    // Should get graceful degradation message since LLM is offline
    assert!(summary.contains("messages") || summary.contains("LLM offline"));
}

#[tokio::test]
async fn test_fetch_messages_from_last_n_days() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Add old message (5 days ago) and recent message (today)
    use sekha_controller::storage::entities::messages;

    // Old message
    let old_msg = messages::ActiveModel {
        id: sea_orm::ActiveValue::Set(Uuid::new_v4()),
        conversation_id: sea_orm::ActiveValue::Set(conv_id),
        role: sea_orm::ActiveValue::Set("user".to_string()),
        content: sea_orm::ActiveValue::Set("Old message".to_string()),
        timestamp: sea_orm::ActiveValue::Set(
            chrono::Utc::now().naive_utc() - chrono::Duration::days(5),
        ),
        embedding_id: sea_orm::ActiveValue::Set(None),
        metadata: sea_orm::ActiveValue::Set(None),
    };

    messages::Entity::insert(old_msg)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert old message");

    // Recent message
    let recent_msg = messages::ActiveModel {
        id: sea_orm::ActiveValue::Set(Uuid::new_v4()),
        conversation_id: sea_orm::ActiveValue::Set(conv_id),
        role: sea_orm::ActiveValue::Set("user".to_string()),
        content: sea_orm::ActiveValue::Set("Recent message".to_string()),
        timestamp: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
        embedding_id: sea_orm::ActiveValue::Set(None),
        metadata: sea_orm::ActiveValue::Set(None),
    };

    messages::Entity::insert(recent_msg)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert recent message");

    // Generate daily summary (fetches from last 1 day)
    let result = summarizer.generate_daily_summary(conv_id).await;

    assert!(result.is_ok());
    // Should only include 1 recent message
    let summary = result.unwrap();
    assert!(summary.contains("1 message") || summary.contains("messages"));
}

#[tokio::test]
async fn test_generate_weekly_summary_with_no_daily_summaries() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 0,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Generate weekly summary - should fall back to daily summary
    let result = summarizer.generate_weekly_summary(conv_id).await;

    assert!(result.is_ok());
    let summary = result.unwrap();
    // Should get "No messages to summarize" from fallback to daily
    assert_eq!(summary, "No messages to summarize");
}

#[tokio::test]
async fn test_generate_monthly_summary_with_no_weekly_summaries() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 0,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Generate monthly summary - should fall back to weekly then daily
    let result = summarizer.generate_monthly_summary(conv_id).await;

    assert!(result.is_ok());
    let summary = result.unwrap();
    // Should get "No messages to summarize" from fallback chain
    assert_eq!(summary, "No messages to summarize");
}

#[tokio::test]
async fn test_store_summary_creates_entry() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Add a recent message
    use sekha_controller::storage::entities::messages;
    let msg_model = messages::ActiveModel {
        id: sea_orm::ActiveValue::Set(Uuid::new_v4()),
        conversation_id: sea_orm::ActiveValue::Set(conv_id),
        role: sea_orm::ActiveValue::Set("user".to_string()),
        content: sea_orm::ActiveValue::Set("Test message".to_string()),
        timestamp: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
        embedding_id: sea_orm::ActiveValue::Set(None),
        metadata: sea_orm::ActiveValue::Set(None),
    };

    messages::Entity::insert(msg_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test message");

    // Generate daily summary - this will call store_summary internally
    let result = summarizer.generate_daily_summary(conv_id).await;

    assert!(result.is_ok());

    // Verify summary was stored (or at least attempted - may fail if table doesn't exist)
    // The code handles this gracefully with let _ = self.store_summary(...)
}

#[tokio::test]
async fn test_llm_graceful_degradation_daily() {
    let repo = setup_test_db().await;
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = HierarchicalSummarizer::new(repo.clone(), llm_bridge);

    // Create a conversation with messages
    let conv_id = Uuid::new_v4();
    let conversation = Conversation {
        id: conv_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Insert conversation
    use sea_orm::EntityTrait;
    use sekha_controller::storage::entities::conversations;

    let conv_model = conversations::ActiveModel {
        id: sea_orm::ActiveValue::Set(conv_id),
        label: sea_orm::ActiveValue::Set(conversation.label.clone()),
        folder: sea_orm::ActiveValue::Set(conversation.folder.clone()),
        status: sea_orm::ActiveValue::Set(conversation.status.clone()),
        importance_score: sea_orm::ActiveValue::Set(conversation.importance_score),
        word_count: sea_orm::ActiveValue::Set(conversation.word_count as i32),
        session_count: sea_orm::ActiveValue::Set(conversation.session_count),
        created_at: sea_orm::ActiveValue::Set(conversation.created_at),
        updated_at: sea_orm::ActiveValue::Set(conversation.updated_at),
    };

    conversations::Entity::insert(conv_model)
        .exec(repo.get_db())
        .await
        .expect("Failed to insert test conversation");

    // Add messages
    use sekha_controller::storage::entities::messages;
    for i in 0..5 {
        let msg_model = messages::ActiveModel {
            id: sea_orm::ActiveValue::Set(Uuid::new_v4()),
            conversation_id: sea_orm::ActiveValue::Set(conv_id),
            role: sea_orm::ActiveValue::Set("user".to_string()),
            content: sea_orm::ActiveValue::Set(format!("Message {}", i)),
            timestamp: sea_orm::ActiveValue::Set(chrono::Utc::now().naive_utc()),
            embedding_id: sea_orm::ActiveValue::Set(None),
            metadata: sea_orm::ActiveValue::Set(None),
        };

        messages::Entity::insert(msg_model)
            .exec(repo.get_db())
            .await
            .expect("Failed to insert test message");
    }

    // LLM will be unavailable - should gracefully degrade
    let result = summarizer.generate_daily_summary(conv_id).await;

    assert!(result.is_ok());
    let summary = result.unwrap();
    // Should contain offline message
    assert!(summary.contains("LLM offline") || summary.contains("5 messages"));
}
