use super::{
    create_test_services, 
    create_test_conversation, 
    is_chroma_running,
    ConversationRepository,  // ✅ Import trait
    Arc,                     // ✅ Import Arc
    json,                    // ✅ Import json macro
};
use sekha_controller::{
    storage::{init_db, SeaOrmConversationRepository},
    models::internal::NewMessage,  // ✅ Import NewMessage
};
use uuid::Uuid;

// ============================================
// Storage Layer Tests
// ============================================

#[tokio::test]
async fn test_repository_create_with_messages() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let conv = create_test_conversation();
    let result = repo.create_with_messages(conv).await;

    assert!(
        result.is_ok(),
        "Failed to create conversation with messages: {:?}",
        result
    );
}

#[tokio::test]
async fn test_repository_semantic_search() {
    if !is_chroma_running().await {
        eprintln!("⚠️  Skipping test_repository_semantic_search - Chroma not running");
        return;
    }
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create a conversation
    let conv = create_test_conversation();
    let conv_id = repo.create_with_messages(conv).await.unwrap();

    // Search for it
    let results = repo
        .semantic_search("test message", 10, None)
        .await
        .unwrap();

    assert!(!results.is_empty(), "Search should return results");
    assert_eq!(results[0].conversation_id, conv_id);
}

#[tokio::test]
async fn test_repository_delete_cascades() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create conversation
    let conv = create_test_conversation();
    let conv_id = repo.create_with_messages(conv).await.unwrap();

    // Verify it exists
    assert!(repo.find_by_id(conv_id).await.unwrap().is_some());

    // Delete it
    repo.delete(conv_id).await.unwrap();

    // Verify it's gone
    assert!(repo.find_by_id(conv_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_repository_count_by_label() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create multiple conversations with same label
    for _i in 0..3 {
        let mut conv = create_test_conversation();
        conv.label = "count_test".to_string();
        conv.id = Some(Uuid::new_v4());
        repo.create_with_messages(conv).await.unwrap();
    }

    let count = repo.count_by_label("count_test").await.unwrap();
    assert_eq!(count, 3);
}

// ============================================
// Storage Edge Cases
// ============================================

#[tokio::test]
async fn test_repository_empty_messages() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let mut conv = create_test_conversation();
    conv.messages = vec![]; // Empty messages

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_repository_very_long_message() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let mut conv = create_test_conversation();
    conv.messages = vec![NewMessage {
        role: "user".to_string(),
        content: "A".repeat(10000), // Very long message
        timestamp: chrono::Utc::now().naive_utc(),
        metadata: json!({}),
    }];

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_inserts() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let mut handles = vec![];

    // Spawn 10 concurrent inserts
    for i in 0..10 {
        let repo_clone = repo.clone();
        let handle: tokio::task::JoinHandle<Result<uuid::Uuid, _>> = tokio::spawn(async move {
            let mut conv = create_test_conversation();
            conv.label = format!("Concurrent {}", i);
            conv.id = Some(Uuid::new_v4());
            repo_clone.create_with_messages(conv).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    // Verify all were created
    let count = repo.count_by_label("Concurrent").await.unwrap();
    assert_eq!(count, 10);
}

#[tokio::test]
async fn test_updated_at_trigger() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create a conversation
    let mut conv = create_test_conversation();
    conv.id = Some(Uuid::new_v4());
    let conv_id = repo.create_with_messages(conv).await.unwrap();

    // Get initial updated_at
    let initial_conv = repo.find_by_id(conv_id).await.unwrap().unwrap();
    let initial_updated_at = initial_conv.updated_at;

    // Wait a moment to ensure timestamp would be different
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Update the conversation
    repo.update_label(conv_id, "Updated Label", "/updated")
        .await
        .unwrap();

    // Get updated conversation
    let updated_conv = repo.find_by_id(conv_id).await.unwrap().unwrap();

    // Verify updated_at changed automatically
    assert!(
        updated_conv.updated_at > initial_updated_at,
        "updated_at should be automatically updated by trigger"
    );
    assert_eq!(updated_conv.label, "Updated Label");
}

#[tokio::test]
async fn test_fts_auto_indexing() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create a conversation with a searchable message
    let mut conv = create_test_conversation();
    conv.messages = vec![
        NewMessage {
            role: "user".to_string(),
            content: "The quick brown fox jumps over the lazy dog".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: json!({}),
        },
    ];
    
    let conv_id = repo.create_with_messages(conv).await.unwrap();

    // Search using FTS - should find the message immediately
    let results = repo.full_text_search("quick brown fox", 10).await.unwrap();

    assert!(!results.is_empty(), "FTS should find the indexed message");
    assert_eq!(results[0].conversation_id, conv_id);
    assert!(results[0].content.contains("quick brown fox"));
}

#[tokio::test]
async fn test_fts_update_trigger() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db.clone(), chroma_client, embedding_service);

    // Create conversation
    let mut conv = create_test_conversation();
    conv.messages = vec![
        NewMessage {
            role: "user".to_string(),
            content: "Original content here".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: json!({}),
        },
    ];
    
    repo.create_with_messages(conv).await.unwrap();

    // Update message content directly (simulating an update)
    use sea_orm::ConnectionTrait;
    db.execute_unprepared(
        "UPDATE messages SET content = 'Updated searchable content' WHERE content = 'Original content here'"
    ).await.unwrap();

    // Search for updated content - trigger should have updated FTS index
    let results = repo.full_text_search("searchable", 10).await.unwrap();

    assert!(!results.is_empty(), "FTS should find updated content");
    assert!(results[0].content.contains("searchable"));
}

#[tokio::test]
async fn test_fts_performance() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create 100 conversations with unique words
    for i in 0..100 {
        let mut conv = create_test_conversation();
        conv.id = Some(Uuid::new_v4());
        conv.messages = vec![NewMessage {
            role: "user".to_string(),
            content: format!("Message with unique word number{}", i),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: json!({}),
        }];
        repo.create_with_messages(conv).await.unwrap();
    }

    // FTS should find ONLY the matching message
    let results = repo.full_text_search("number42", 10).await.unwrap();
    
    assert_eq!(results.len(), 1, "Should find exactly one message");
    assert!(results[0].content.contains("number42"));
}

