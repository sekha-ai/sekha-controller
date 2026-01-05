use super::{create_test_services, ConversationRepository, Arc};
use sekha_controller::{
    services::file_watcher::import_chatgpt_conversation,
    storage::{init_db, SeaOrmConversationRepository},
};

// ============================================
// File Watcher Tests
// ============================================

#[tokio::test]
async fn test_file_watcher_chatgpt_import() {
    use sekha_controller::services::file_watcher::ImportProcessor;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let import_file = temp_dir.path().join("chatgpt_export.json");

    // Create sample ChatGPT export
    let chatgpt_json = r#"{
        "title": "Test Import",
        "create_time": 1703073600.0,
        "update_time": 1703073600.0,
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
                        "parts": ["What is Rust?"]
                    }
                },
                "parent": "root",
                "children": ["msg2"]
            },
            "msg2": {
                "id": "msg2",
                "message": {
                    "id": "msg2",
                    "author": {"role": "assistant"},
                    "create_time": 1703073660.0,
                    "content": {
                        "content_type": "text",
                        "parts": ["Rust is a systems programming language."]
                    }
                },
                "parent": "msg1",
                "children": []
            }
        }
    }"#;

    tokio::fs::write(&import_file, chatgpt_json).await.unwrap();

    // Create repository and processor
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    async fn is_chroma_running() -> bool {
        let client = reqwest::Client::new();
        let result = client
            .get("http://localhost:8000/api/v1/heartbeat")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await;
        
        result.is_ok()
    }

    let processor = ImportProcessor::new(repo.clone());

    // Process file
    let result = processor.process_file(&import_file).await;
    assert!(
        result.is_ok(),
        "Failed to process ChatGPT export: {:?}",
        result
    );

    // Verify import
    let count = repo.count_by_label("Test Import").await.unwrap();
    assert_eq!(count, 1, "Should have imported 1 conversation");
}

#[tokio::test]
async fn test_file_watcher_multiple_conversations() {
    use sekha_controller::services::file_watcher::ImportProcessor;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let import_file = temp_dir.path().join("multi_export.json");

    // Array of ChatGPT conversations
    let chatgpt_json = r#"[
        {
            "title": "Conversation 1",
            "create_time": 1703073600.0,
            "update_time": 1703073600.0,
            "mapping": {
                "root": {"id": "root", "message": null, "parent": null, "children": ["msg1"]},
                "msg1": {
                    "id": "msg1",
                    "message": {
                        "id": "msg1",
                        "author": {"role": "user"},
                        "create_time": 1703073600.0,
                        "content": {"content_type": "text", "parts": ["First conversation"]}
                    },
                    "parent": "root",
                    "children": []
                }
            }
        },
        {
            "title": "Conversation 2",
            "create_time": 1703073700.0,
            "update_time": 1703073700.0,
            "mapping": {
                "root": {"id": "root", "message": null, "parent": null, "children": ["msg1"]},
                "msg1": {
                    "id": "msg1",
                    "message": {
                        "id": "msg1",
                        "author": {"role": "user"},
                        "create_time": 1703073700.0,
                        "content": {"content_type": "text", "parts": ["Second conversation"]}
                    },
                    "parent": "root",
                    "children": []
                }
            }
        }
    ]"#;

    tokio::fs::write(&import_file, chatgpt_json).await.unwrap();

    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    async fn is_chroma_running() -> bool {
    let client = reqwest::Client::new();
    let result = client
        .get("http://localhost:8000/api/v1/heartbeat")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;
    
    result.is_ok()
}

    let processor = ImportProcessor::new(repo.clone());
    processor.process_file(&import_file).await.unwrap();

    // Verify both imports
    let count1 = repo.count_by_label("Conversation 1").await.unwrap();
    let count2 = repo.count_by_label("Conversation 2").await.unwrap();

    assert_eq!(count1, 1);
    assert_eq!(count2, 1);
}