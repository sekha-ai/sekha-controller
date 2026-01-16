// tests/integration/concurrency.rs
use super::{create_test_services, Arc, ConversationRepository};
use sekha_controller::models::internal::{NewConversation, NewMessage};
use sekha_controller::storage::{init_db, SeaOrmConversationRepository};
use serde_json::json;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_concurrent_conversation_creation() {
    // This test verifies concurrent database access without requiring external services
    // It creates conversations without messages to avoid embedding service calls
    
    let result = timeout(
        Duration::from_secs(10), // 10 second timeout should be plenty
        run_concurrent_test(),
    )
    .await;

    assert!(result.is_ok(), "Test timed out");
    assert!(result.unwrap().is_ok(), "Test failed");
}

async fn run_concurrent_test() -> Result<(), Box<dyn std::error::Error>> {
    let db = init_db("sqlite::memory:").await?;
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
                word_count: 0,
                session_count: Some(0),
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                messages: vec![], // No messages = no embedding calls = no external service dependency
            };

            repo_clone.create_with_messages(new_conv).await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        assert!(handle.await?.is_ok());
    }

    // Verify all 10 were created
    let (conversations, count) = repo.find_with_filters(None, 100, 0).await?;
    assert_eq!(count, 10);
    assert_eq!(conversations.len(), 10);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires external services (Ollama + ChromaDB)
async fn test_concurrent_conversation_creation_with_messages() {
    // This test requires external services to be running
    // Run with: cargo test -- --ignored --test-threads=1
    
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo: Arc<dyn ConversationRepository + Send + Sync> = Arc::new(
        SeaOrmConversationRepository::new(db, chroma_client, embedding_service),
    );

    let mut handles = vec![];

    for i in 0..10 {
        let repo_clone = repo.clone();
        let handle = tokio::spawn(async move {
            let new_conv = NewConversation {
                id: None,
                label: format!("Concurrent Test With Messages {}", i),
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
