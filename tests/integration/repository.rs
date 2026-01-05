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