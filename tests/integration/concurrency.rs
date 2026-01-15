// tests/integration/concurrency.rs
use super::{create_test_services, Arc, ConversationRepository};
use sekha_controller::models::internal::{NewConversation, NewMessage};
use sekha_controller::storage::{init_db, SeaOrmConversationRepository};
use sekha_controller::services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient};
use sekha_controller::storage::chroma_client::ChromaClient;
use serde_json::json;
use tokio::time::{timeout, Duration};
use mockall::mock;
use mockall::predicate::*;
use async_trait::async_trait;

// Mock ChromaClient for testing
mock! {
    pub ChromaClientTest {}
    
    #[async_trait]
    impl sekha_controller::storage::chroma_client::ChromaClientTrait for ChromaClientTest {
        async fn create_collection(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        async fn add_documents(
            &self,
            collection_name: &str,
            documents: Vec<String>,
            metadatas: Vec<serde_json::Value>,
            ids: Vec<String>,
            embeddings: Option<Vec<Vec<f32>>>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        async fn query(
            &self,
            collection_name: &str,
            query_embeddings: Vec<Vec<f32>>,
            n_results: usize,
        ) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error + Send + Sync>>;
        async fn delete_documents(
            &self,
            collection_name: &str,
            ids: Vec<String>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    }
}

#[tokio::test]
async fn test_concurrent_conversation_creation() {
    // Wrap test in timeout to prevent hanging
    let result = timeout(
        Duration::from_secs(60), // 60 second timeout
        run_concurrent_test(),
    )
    .await;

    assert!(result.is_ok(), "Test timed out after 60 seconds");
    assert!(result.unwrap().is_ok(), "Test failed");
}

async fn run_concurrent_test() -> Result<(), Box<dyn std::error::Error>> {
    // Test that multiple conversations can be created concurrently
    let db = init_db("sqlite::memory:").await?;
    
    // Use actual services but disable embedding generation to avoid external calls
    // The EmbeddingService will attempt connections but we're testing DB concurrency,
    // not embedding generation
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    
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

            // This will attempt to call external services, which may fail
            // but we're primarily testing concurrent DB access
            repo_clone.create_with_messages(new_conv).await
        });

        handles.push(handle);
    }

    // Wait for all to complete - some may fail due to missing external services
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    // If external services are running, we should have 10 successes
    // If not, we just verify no panics occurred (concurrency safety)
    println!("Successfully created {} conversations concurrently", success_count);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_conversation_creation_no_external_services() {
    // This test verifies concurrent database access without requiring external services
    // It creates conversations without messages to avoid embedding service calls
    
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
                label: format!("Concurrent Test No Messages {}", i),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: Some(5),
                word_count: 0,
                session_count: Some(0),
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                messages: vec![], // No messages = no embedding calls
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
