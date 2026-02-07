// Line 18-35 replacement for test function
#[cfg(test)]
use crate::config::Config;
#[cfg(test)]
use crate::llm::bridge_client::BridgeClient;

#[tokio::test]
async fn test_create_message_with_fts_indexing() {
    // Setup: Create in-memory DB and repository with graceful degradation
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = init_db(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();

    // Use invalid URLs so embedding fails gracefully (creates message but no embedding)
    let chroma = Arc::new(ChromaClient::new("http://localhost:1".to_string()));
    let config = Config::default();
    let bridge = BridgeClient::new(&config).expect("Failed to create BridgeClient");
    let embedding_service = Arc::new(EmbeddingService::new(
        bridge,
        "http://localhost:1".to_string(),
    ));

    let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);

    // ... rest of test unchanged
