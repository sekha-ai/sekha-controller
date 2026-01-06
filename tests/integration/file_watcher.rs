// tests/integration/file_watcher.rs
//! Minimal integration tests for file watcher
//! Tests end-to-end flow, skips if external dependencies unavailable

use super::{create_test_services, ConversationRepository, Arc};
use sekha_controller::{
    services::file_watcher::ImportProcessor,
    storage::{init_db, SeaOrmConversationRepository},
};
use std::fs;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

// ============================================
// Integration Tests
// ============================================

#[tokio::test]
async fn test_file_watcher_end_to_end_chatgpt() {
    let temp_dir = TempDir::new().unwrap();
    let import_file = temp_dir.path().join("chatgpt_export.json");

    // Sample ChatGPT export
    let chatgpt_json = r#"{
        "title": "Integration Test",
        "create_time": 1703073600.0,
        "update_time": 1703077200.0,
        "mapping": {
            "root": {
                "id": "root",
                "message": null,
                "parent": null,
                "children": ["msg1"]
            },
            "msg1": {
                "id": "msg1",
                "message": {
                    "id": "msg1",
                    "author": {"role": "user"},
                    "create_time": 1703073600.0,
                    "content": {
                        "content_type": "text",
                        "parts": ["Integration test message"]
                    }
                },
                "parent": "root",
                "children": []
            }
        }
    }"#;

    fs::write(&import_file, chatgpt_json).unwrap();

    // Setup repository
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let processor = ImportProcessor::new(repo.clone());

    // Process file
    let result = processor.process_file(&import_file).await;
    
    // Graceful degradation: success or specific error types are acceptable
    match result {
        Ok(_) => {
            // Verify import
            let count = repo.count_by_label("Integration Test").await.unwrap();
            assert_eq!(count, 1, "Should have imported 1 conversation");
        }
        Err(e) if e.to_string().contains("embedding") || e.to_string().contains("Chroma") => {
            // Acceptable: external service unavailable in CI
            eprintln!("Skipping due to external service: {}", e);
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_file_watcher_multiple_conversations_integration() {
    let temp_dir = TempDir::new().unwrap();
    let import_file = temp_dir.path().join("multi_export.json");

    // ChatGPT array format
    let chatgpt_json = r#"[{"title":"Conv 1","create_time":1703073600.0,"update_time":1703077200.0,"mapping":{"root":{"id":"root","message":null,"parent":null,"children":["msg1"]},"msg1":{"id":"msg1","message":{"id":"msg1","author":{"role":"user"},"create_time":1703073600.0,"content":{"content_type":"text","parts":["First"]}},"parent":"root","children":[]}}},{"title":"Conv 2","create_time":1703073700.0,"update_time":1703077300.0,"mapping":{"root":{"id":"root","message":null,"parent":null,"children":["msg1"]},"msg1":{"id":"msg1","message":{"id":"msg1","author":{"role":"assistant"},"create_time":1703073700.0,"content":{"content_type":"text","parts":["Second"]}},"parent":"root","children":[]}}}]"#;

    fs::write(&import_file, chatgpt_json).unwrap();

    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let processor = ImportProcessor::new(repo.clone());
    let result = processor.process_file(&import_file).await;

    match result {
        Ok(_) => {
            let count1 = repo.count_by_label("Conv 1").await.unwrap();
            let count2 = repo.count_by_label("Conv 2").await.unwrap();
            assert_eq!(count1, 1);
            assert_eq!(count2, 1);
        }
        Err(e) if e.to_string().contains("embedding") || e.to_string().contains("Chroma") => {
            eprintln!("Skipping due to external service: {}", e);
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}