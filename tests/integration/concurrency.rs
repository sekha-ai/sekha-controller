// tests/integration/concurrency.rs
use super::{create_test_services, Arc, ConversationRepository};
use sekha_controller::models::internal::{NewConversation, NewMessage};
use sekha_controller::storage::{init_db, SeaOrmConversationRepository};
use serde_json::json;

#[tokio::test]
async fn test_concurrent_conversation_creation() {
    // Test that multiple conversations can be created concurrently
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo: Arc<dyn ConversationRepository + Send + Sync> = Arc::new(
        SeaOrmConversationRepository::new(db, chroma_client, embedding_service),
    );

    // Spawn 10 concurrent conversation creations
    let mut handles = vec![];

    for i in 0..10 {
        let repo_clone = repo.clone();
        let handle = tokio::spawn(async move {
            let new_conv = NewConversation {
                id: None,
                label: format!("Concurrent Test {}", i),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: Some(5),
                word_count: 10,
                session_count: Some(1),
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                messages: vec![NewMessage {
                    role: "user".to_string(),
                    content: format!("Message {}", i),
                    timestamp: chrono::Utc::now().naive_utc(),
                    metadata: json!({}),
                }],
            };

            repo_clone.create_with_messages(new_conv).await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    // Verify all 10 were created
    let (conversations, count) = repo.find_with_filters(None, 100, 0).await.unwrap();
    assert_eq!(count, 10);
    assert_eq!(conversations.len(), 10);
}
