use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use sekha_controller::{
    config::Config,
    models::internal::{Conversation, Message},
    orchestrator::context_assembly::ContextAssembler,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{
        chroma_client::ChromaClient, entities::conversations, entities::messages, init_db,
        repository::SeaOrmConversationRepository,
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

async fn setup_test_db() -> (
    Arc<SeaOrmConversationRepository>,
    sea_orm::DatabaseConnection,
) {
    let db = init_db("sqlite::memory:")
        .await
        .expect("Failed to initialize test database");

    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));

    let repo = Arc::new(SeaOrmConversationRepository::new(
        db.clone(),
        chroma_client,
        embedding_service,
    ));

    (repo, db)
}

async fn create_test_conversation_in_db(
    db: &sea_orm::DatabaseConnection,
    label: &str,
    folder: &str,
    importance: i32,
) -> Uuid {
    let conv_id = Uuid::new_v4();

    let conversation = conversations::ActiveModel {
        id: Set(conv_id),
        label: Set(label.to_string()),
        folder: Set(folder.to_string()),
        status: Set("active".to_string()),
        importance_score: Set(importance),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        word_count: Set(0),
        session_count: Set(1),
    };

    conversation.insert(db).await.unwrap();
    conv_id
}

async fn create_test_message_in_db(
    db: &sea_orm::DatabaseConnection,
    conversation_id: Uuid,
    content: &str,
    timestamp: chrono::NaiveDateTime,
) -> Uuid {
    let msg_id = Uuid::new_v4();

    let message = messages::ActiveModel {
        id: Set(msg_id),
        conversation_id: Set(conversation_id),
        role: Set("user".to_string()),
        content: Set(content.to_string()),
        timestamp: Set(timestamp),
        embedding_id: Set(None),
        metadata: Set(None),
    };

    message.insert(db).await.unwrap();
    msg_id
}

#[tokio::test]
async fn test_get_pinned_messages_integration() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create pinned conversation (importance >= 10)
    let pinned_conv_id = create_test_conversation_in_db(&db, "Pinned", "/pinned", 10).await;

    // Create message in pinned conversation
    let now = Utc::now().naive_utc();
    create_test_message_in_db(&db, pinned_conv_id, "Pinned message content", now).await;

    // Create non-pinned conversation for contrast
    let normal_conv_id = create_test_conversation_in_db(&db, "Normal", "/normal", 5).await;
    create_test_message_in_db(&db, normal_conv_id, "Normal message", now).await;

    // Test get_pinned_messages
    let pinned = assembler.get_pinned_messages().await.unwrap();

    // Should only get messages from pinned conversation
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].conversation_id, pinned_conv_id);
    assert_eq!(pinned[0].label, "Pinned");
    assert!(pinned[0].is_pinned);
    assert_eq!(pinned[0].importance, 10.0);
    assert_eq!(pinned[0].score, 10.0);
}

#[tokio::test]
async fn test_get_pinned_messages_multiple_messages() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create pinned conversation with multiple messages
    let pinned_conv_id = create_test_conversation_in_db(&db, "Multi-Pin", "/pins", 15).await;

    let now = Utc::now().naive_utc();
    create_test_message_in_db(&db, pinned_conv_id, "Message 1", now).await;
    create_test_message_in_db(&db, pinned_conv_id, "Message 2", now).await;
    create_test_message_in_db(&db, pinned_conv_id, "Message 3", now).await;

    let pinned = assembler.get_pinned_messages().await.unwrap();

    assert_eq!(pinned.len(), 3);
    for candidate in &pinned {
        assert_eq!(candidate.conversation_id, pinned_conv_id);
        assert!(candidate.is_pinned);
    }
}

#[tokio::test]
async fn test_get_pinned_messages_no_pinned() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create only non-pinned conversations
    let conv_id = create_test_conversation_in_db(&db, "Normal", "/normal", 5).await;
    create_test_message_in_db(&db, conv_id, "Not pinned", Utc::now().naive_utc()).await;

    let pinned = assembler.get_pinned_messages().await.unwrap();
    assert_eq!(pinned.len(), 0);
}

#[tokio::test]
async fn test_get_recent_labeled_messages_integration() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create conversation with specific label
    let conv_id = create_test_conversation_in_db(&db, "TestLabel", "/test", 5).await;

    // Create recent message (within 7 days)
    let recent = Utc::now().naive_utc();
    create_test_message_in_db(&db, conv_id, "Recent message", recent).await;

    // Create old message (outside 7 days)
    let old = Utc::now().naive_utc() - chrono::Duration::days(10);
    create_test_message_in_db(&db, conv_id, "Old message", old).await;

    // Get recent messages for this label
    let recent_msgs = assembler
        .get_recent_labeled_messages(&["TestLabel".to_string()], 7)
        .await
        .unwrap();

    // Should only get the recent message
    assert_eq!(recent_msgs.len(), 1);
    assert_eq!(recent_msgs[0].label, "TestLabel");
    assert!(!recent_msgs[0].is_pinned);
    assert_eq!(recent_msgs[0].importance, 5.0);
}

#[tokio::test]
async fn test_get_recent_labeled_messages_multiple_labels() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create conversations with different labels
    let conv1 = create_test_conversation_in_db(&db, "Label1", "/test1", 5).await;
    let conv2 = create_test_conversation_in_db(&db, "Label2", "/test2", 7).await;
    let conv3 = create_test_conversation_in_db(&db, "Label3", "/test3", 3).await;

    let recent = Utc::now().naive_utc();
    create_test_message_in_db(&db, conv1, "Message 1", recent).await;
    create_test_message_in_db(&db, conv2, "Message 2", recent).await;
    create_test_message_in_db(&db, conv3, "Message 3", recent).await;

    // Get messages for Label1 and Label2 only
    let recent_msgs = assembler
        .get_recent_labeled_messages(&["Label1".to_string(), "Label2".to_string()], 7)
        .await
        .unwrap();

    assert_eq!(recent_msgs.len(), 2);
    let labels: Vec<String> = recent_msgs.iter().map(|m| m.label.clone()).collect();
    assert!(labels.contains(&"Label1".to_string()));
    assert!(labels.contains(&"Label2".to_string()));
    assert!(!labels.contains(&"Label3".to_string()));
}

#[tokio::test]
async fn test_get_recent_labeled_messages_date_cutoff() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let conv_id = create_test_conversation_in_db(&db, "DateTest", "/date", 5).await;

    // Create messages at different dates
    let day1 = Utc::now().naive_utc();
    let day5 = Utc::now().naive_utc() - chrono::Duration::days(5);
    let day10 = Utc::now().naive_utc() - chrono::Duration::days(10);

    create_test_message_in_db(&db, conv_id, "Recent", day1).await;
    create_test_message_in_db(&db, conv_id, "Medium", day5).await;
    create_test_message_in_db(&db, conv_id, "Old", day10).await;

    // Get messages within 7 days
    let recent_msgs = assembler
        .get_recent_labeled_messages(&["DateTest".to_string()], 7)
        .await
        .unwrap();

    // Should get 2 messages (day1 and day5, not day10)
    assert_eq!(recent_msgs.len(), 2);
}

#[tokio::test]
async fn test_get_recent_labeled_messages_empty_results() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create conversation with label but no recent messages
    let conv_id = create_test_conversation_in_db(&db, "OldLabel", "/old", 5).await;
    let old = Utc::now().naive_utc() - chrono::Duration::days(30);
    create_test_message_in_db(&db, conv_id, "Old message", old).await;

    let recent_msgs = assembler
        .get_recent_labeled_messages(&["OldLabel".to_string()], 7)
        .await
        .unwrap();

    assert_eq!(recent_msgs.len(), 0);
}

#[tokio::test]
async fn test_get_recent_labeled_messages_empty_labels() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Test with empty labels array
    let recent_msgs = assembler
        .get_recent_labeled_messages(&[], 7)
        .await
        .unwrap();

    assert_eq!(recent_msgs.len(), 0);
}

#[tokio::test]
async fn test_fetch_message_integration() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let conv_id = create_test_conversation_in_db(&db, "Test", "/test", 5).await;
    let msg_id =
        create_test_message_in_db(&db, conv_id, "Test message content", Utc::now().naive_utc())
            .await;

    // Fetch the message
    let message = assembler.fetch_message(msg_id).await.unwrap();

    assert!(message.is_some());
    let msg = message.unwrap();
    assert_eq!(msg.id, msg_id);
    assert_eq!(msg.conversation_id, conv_id);
    assert_eq!(msg.content, "Test message content");
    assert_eq!(msg.role, "user");
}

#[tokio::test]
async fn test_fetch_message_not_found() {
    let (repo, _db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let fake_id = Uuid::new_v4();
    let message = assembler.fetch_message(fake_id).await.unwrap();

    assert!(message.is_none());
}

#[tokio::test]
async fn test_fetch_message_with_metadata() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let conv_id = create_test_conversation_in_db(&db, "Meta", "/meta", 5).await;
    let msg_id = Uuid::new_v4();

    // Create message with metadata
    let message = messages::ActiveModel {
        id: Set(msg_id),
        conversation_id: Set(conv_id),
        role: Set("assistant".to_string()),
        content: Set("Content with metadata".to_string()),
        timestamp: Set(Utc::now().naive_utc()),
        embedding_id: Set(None),
        metadata: Set(Some(serde_json::json!({"key": "value"}))),
    };

    message.insert(&db).await.unwrap();

    // Fetch and verify metadata
    let fetched = assembler.fetch_message(msg_id).await.unwrap().unwrap();

    assert!(fetched.metadata.is_some());
    assert_eq!(fetched.metadata.unwrap()["key"], "value");
}

#[tokio::test]
async fn test_pinned_messages_inactive_conversations_excluded() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create pinned but inactive conversation
    let conv_id = Uuid::new_v4();
    let conversation = conversations::ActiveModel {
        id: Set(conv_id),
        label: Set("Inactive Pinned".to_string()),
        folder: Set("/inactive".to_string()),
        status: Set("archived".to_string()),
        importance_score: Set(10),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        word_count: Set(0),
        session_count: Set(1),
    };
    conversation.insert(&db).await.unwrap();

    create_test_message_in_db(&db, conv_id, "Message", Utc::now().naive_utc()).await;

    // Should not return inactive pinned conversations
    let pinned = assembler.get_pinned_messages().await.unwrap();
    assert_eq!(pinned.len(), 0);
}

#[tokio::test]
async fn test_recent_labeled_messages_inactive_excluded() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    // Create inactive conversation
    let conv_id = Uuid::new_v4();
    let conversation = conversations::ActiveModel {
        id: Set(conv_id),
        label: Set("InactiveLabel".to_string()),
        folder: Set("/inactive".to_string()),
        status: Set("archived".to_string()),
        importance_score: Set(5),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        word_count: Set(0),
        session_count: Set(1),
    };
    conversation.insert(&db).await.unwrap();

    create_test_message_in_db(&db, conv_id, "Message", Utc::now().naive_utc()).await;

    // Should not return messages from inactive conversations
    let recent = assembler
        .get_recent_labeled_messages(&["InactiveLabel".to_string()], 7)
        .await
        .unwrap();
    assert_eq!(recent.len(), 0);
}

#[tokio::test]
async fn test_get_recent_labeled_messages_importance_preserved() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let conv_id = create_test_conversation_in_db(&db, "ImportTest", "/test", 8).await;
    create_test_message_in_db(&db, conv_id, "Message", Utc::now().naive_utc()).await;

    let recent = assembler
        .get_recent_labeled_messages(&["ImportTest".to_string()], 7)
        .await
        .unwrap();

    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].importance, 8.0);
}

#[tokio::test]
async fn test_assemble_context_with_token_budget() {
    let (repo, db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let conv_id = create_test_conversation_in_db(&db, "Test", "/test", 5).await;
    
    // Create messages with known sizes
    let msg1_id = create_test_message_in_db(
        &db,
        conv_id,
        "x".repeat(100).as_str(), // ~25 tokens
        Utc::now().naive_utc(),
    ).await;
    
    let msg2_id = create_test_message_in_db(
        &db,
        conv_id,
        "y".repeat(100).as_str(), // ~25 tokens
        Utc::now().naive_utc(),
    ).await;

    let msg3_id = create_test_message_in_db(
        &db,
        conv_id,
        "z".repeat(100).as_str(), // ~25 tokens
        Utc::now().naive_utc(),
    ).await;

    // Create candidates
    let mut candidates = vec![
        sekha_controller::orchestrator::context_assembly::CandidateMessage {
            message_id: msg1_id,
            conversation_id: conv_id,
            score: 10.0,
            timestamp: Utc::now().naive_utc(),
            label: "test".to_string(),
            is_pinned: false,
            importance: 5.0,
        },
        sekha_controller::orchestrator::context_assembly::CandidateMessage {
            message_id: msg2_id,
            conversation_id: conv_id,
            score: 8.0,
            timestamp: Utc::now().naive_utc(),
            label: "test".to_string(),
            is_pinned: false,
            importance: 5.0,
        },
        sekha_controller::orchestrator::context_assembly::CandidateMessage {
            message_id: msg3_id,
            conversation_id: conv_id,
            score: 6.0,
            timestamp: Utc::now().naive_utc(),
            label: "test".to_string(),
            is_pinned: false,
            importance: 5.0,
        },
    ];

    // Test with small budget (should only fit 1-2 messages)
    let context = assembler
        .assemble_context(&mut candidates, 50)
        .await
        .unwrap();

    // Should fit at least 1 message within budget
    assert!(context.len() > 0);
    assert!(context.len() <= 2); // Budget should limit to 1-2 messages
}

#[tokio::test]
async fn test_assemble_context_empty_candidates() {
    let (repo, _db) = setup_test_db().await;
    let assembler = ContextAssembler::new(repo.clone());

    let mut candidates = vec![];
    let context = assembler
        .assemble_context(&mut candidates, 4000)
        .await
        .unwrap();

    assert_eq!(context.len(), 0);
}
