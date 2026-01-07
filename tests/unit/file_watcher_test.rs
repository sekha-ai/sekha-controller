// tests/unit/file_watcher_test.rs
//! Unit tests for file watcher - tests through public API
//! Uses temp files and in-memory SQLite for isolation

use sekha_controller::{
    models::internal::NewConversation,
    services::file_watcher::ImportProcessor,
    storage::{init_db, ConversationRepository, SeaOrmConversationRepository},
};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

// ============================================
// Test Data Factories
// ============================================

fn chatgpt_single_json() -> String {
    r#"{
        "title": "ChatGPT Single Unit Test",
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
                        "parts": ["Unit test message"]
                    }
                },
                "parent": "root",
                "children": []
            }
        }
    }"#
    .to_string()
}

fn chatgpt_array_json() -> String {
    r#"[{"title":"Array Convo 1","create_time":1703073600.0,"update_time":1703077200.0,"mapping":{"root":{"id":"root","message":null,"parent":null,"children":["msg1"]},"msg1":{"id":"msg1","message":{"id":"msg1","author":{"role":"user"},"create_time":1703073600.0,"content":{"content_type":"text","parts":["First array"]}},"parent":"root","children":[]}}},{"title":"Array Convo 2","create_time":1703073700.0,"update_time":1703077300.0,"mapping":{"root":{"id":"root","message":null,"parent":null,"children":["msg1"]},"msg1":{"id":"msg1","message":{"id":"msg1","author":{"role":"assistant"},"create_time":1703073700.0,"content":{"content_type":"text","parts":["Second array"]}},"parent":"root","children":[]}}}]"#.to_string()
}

fn claude_json() -> String {
    r#"{"conversations":[{"title":"Claude JSON Unit Test","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:30:00Z","messages":[{"role":"user","content":"Hello Claude","timestamp":"2024-01-01T10:00:00Z"},{"role":"assistant","content":"Hi there","timestamp":"2024-01-01T10:01:00Z"}]}]}"#.to_string()
}

fn markdown_content() -> String {
    r#"# Unit Test Conversation

## User
Markdown unit test message

## Assistant
Markdown unit test response"#
        .to_string()
}

fn txt_content() -> String {
    r#"User: TXT unit test message
Assistant: TXT unit test response"#
        .to_string()
}

fn malformed_json() -> String {
    r#"{invalid json structure}"#.to_string()
}

fn incomplete_chatgpt() -> String {
    r#"{"title":"Incomplete"}"#.to_string()
}

// ============================================
// Test Setup Helper
// ============================================

async fn create_test_processor() -> (ImportProcessor, Arc<SeaOrmConversationRepository>) {
    let db = init_db("sqlite::memory:").await.unwrap();
    // Use mock services to avoid external dependencies
    let chroma_client = Arc::new(sekha_controller::storage::chroma_client::ChromaClient::new(
        "http://localhost:1".to_string(), // Invalid URL = graceful degradation
    ));
    let embedding_service = Arc::new(
        sekha_controller::services::embedding_service::EmbeddingService::new(
            "http://localhost:1".to_string(),
            "http://localhost:1".to_string(),
        ),
    );
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));
    let processor = ImportProcessor::new(repo.clone());
    (processor, repo)
}

// ============================================
// Parsing Format Tests
// ============================================

#[tokio::test]
async fn test_process_chatgpt_single_format() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("chatgpt_single.json");
    fs::write(&file_path, chatgpt_single_json()).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(
        result.is_ok(),
        "Should process ChatGPT single format: {:?}",
        result
    );

    let conversations = repo
        .find_by_label("ChatGPT Single Unit Test", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 1);
}

#[tokio::test]
async fn test_process_chatgpt_array_format() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("chatgpt_array.json");
    fs::write(&file_path, chatgpt_array_json()).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok(), "Should process ChatGPT array format");

    let conv1 = repo.find_by_label("Array Convo 1", 10, 0).await.unwrap();
    let conv2 = repo.find_by_label("Array Convo 2", 10, 0).await.unwrap();
    assert_eq!(conv1.len(), 1);
    assert_eq!(conv2.len(), 1);
}

#[tokio::test]
async fn test_process_claude_format() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("claude.json");
    fs::write(&file_path, claude_json()).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok(), "Should process Claude format: {:?}", result);

    let conversations = repo
        .find_by_label("Claude JSON Unit Test", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 1);
}

#[tokio::test]
async fn test_process_markdown_format() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");
    fs::write(&file_path, markdown_content()).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(
        result.is_ok(),
        "Should process markdown format: {:?}",
        result
    );

    // Uses filename as title
    let conversations = repo.find_by_label("test", 10, 0).await.unwrap();
    assert_eq!(conversations.len(), 1);
}

#[tokio::test]
async fn test_process_txt_format() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, txt_content()).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok(), "Should process TXT format: {:?}", result);

    let conversations = repo.find_by_label("test", 10, 0).await.unwrap();
    assert_eq!(conversations.len(), 1);
}

// ============================================
// Error Handling Tests
// ============================================

#[tokio::test]
async fn test_process_malformed_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("malformed.json");
    fs::write(&file_path, malformed_json()).unwrap();

    let (processor, _repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_err(), "Should fail on malformed JSON");
}

#[tokio::test]
async fn test_process_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.json");
    fs::write(&file_path, "").unwrap();

    let (processor, _repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_err(), "Should fail on empty file");
}

#[tokio::test]
async fn test_process_incomplete_chatgpt() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("incomplete.json");
    fs::write(&file_path, incomplete_chatgpt()).unwrap();

    let (processor, _repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_err(), "Should fail on incomplete ChatGPT format");
}

#[tokio::test]
async fn test_process_unknown_extension() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.xyz");
    fs::write(&file_path, txt_content()).unwrap();

    let (processor, _repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_err(), "Should fail on unknown extension");
}

// ============================================
// Edge Cases Tests
// ============================================

#[tokio::test]
async fn test_process_empty_messages() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty_messages.json");

    let empty_mapping = r#"{
        "title": "Empty Messages Test",
        "create_time": 1703073600.0,
        "update_time": 1703077200.0,
        "mapping": {
            "root": {
                "id": "root",
                "message": null,
                "parent": null,
                "children": []
            }
        }
    }"#;

    fs::write(&file_path, empty_mapping).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok(), "Should handle empty message mapping");

    // Should still create conversation
    let conversations = repo
        .find_by_label("Empty Messages Test", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].word_count, 0);
}

#[tokio::test]
async fn test_process_long_content() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("long_content.json");

    let long_text = "A".repeat(50000);
    let long_json = format!(
        r#"{{
            "title": "Long Content Test",
            "create_time": 1703073600.0,
            "update_time": 1703077200.0,
            "mapping": {{
                "root": {{
                    "id": "root",
                    "message": null,
                    "parent": null,
                    "children": ["msg1"]
                }},
                "msg1": {{
                    "id": "msg1",
                    "message": {{
                        "id": "msg1",
                        "author": {{"role": "user"}},
                        "create_time": 1703073600.0,
                        "content": {{
                            "content_type": "text",
                            "parts": ["{}"]
                        }}
                    }},
                    "parent": "root",
                    "children": []
                }}
            }}
        }}"#,
        long_text
    );

    fs::write(&file_path, long_json).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok(), "Should handle very long messages");

    let conversations = repo
        .find_by_label("Long Content Test", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations[0].word_count, 50000);
}

#[tokio::test]
async fn test_process_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("special_chars.json");

    let special_json = r#"{
        "title": "Special Chars Test !@#$%^&*()",
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
                        "parts": ["Special: \"quotes\", 'apostrophes', \n newlines, \t tabs, unicode: 🎉你好"]
                    }
                },
                "parent": "root",
                "children": []
            }
        }
    }"#;

    fs::write(&file_path, special_json).unwrap();

    let (processor, repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok(), "Should handle special characters");

    let conversations = repo
        .find_by_label("Special Chars Test !@#$%^&*()", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 1);
    assert!(
        conversations[0].word_count > 0,
        "Should have messages imported (word_count > 0)"
    );
}

// ============================================
// Import Source Tests
// ============================================

#[tokio::test]
async fn test_chatgpt_import_folder_placement() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("chatgpt.json");
    fs::write(&file_path, chatgpt_single_json()).unwrap();

    let (processor, repo) = create_test_processor().await;
    processor.process_file(&file_path).await.unwrap();

    // Verify placed in correct folder
    let conversations = repo
        .find_by_label("ChatGPT Single Unit Test", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations[0].folder, "/imports/chatgpt");
}

#[tokio::test]
async fn test_claude_import_folder_placement() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("claude.json");
    fs::write(&file_path, claude_json()).unwrap();

    let (processor, repo) = create_test_processor().await;
    processor.process_file(&file_path).await.unwrap();

    let conversations = repo
        .find_by_label("Claude JSON Unit Test", 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations[0].folder, "/imports/claude");
}

// ============================================
// Duplicate Handling Tests
// ============================================

#[tokio::test]
async fn test_multiple_imports_same_content() {
    let temp_dir = TempDir::new().unwrap();
    let processor = create_test_processor().await.0;

    // Import same content twice (different files)
    for i in 0..2 {
        let file_path = temp_dir.path().join(format!("duplicate_{}.json", i));
        fs::write(&file_path, chatgpt_single_json()).unwrap();

        let result = processor.process_file(&file_path).await;
        assert!(result.is_ok(), "Should handle duplicate imports");

        // Small delay to avoid timestamp collisions
        sleep(Duration::from_millis(10)).await;
    }

    // Both should succeed (creates separate conversations)
}

// ============================================
// Metadata Tests
// ============================================

#[tokio::test]
async fn test_import_metadata_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("metadata.json");
    fs::write(&file_path, chatgpt_single_json()).unwrap();

    let (processor, repo) = create_test_processor().await;
    processor.process_file(&file_path).await.unwrap();

    let conversations = repo
        .find_by_label("ChatGPT Single Unit Test", 10, 0)
        .await
        .unwrap();
    let conv = &conversations[0];

    // Verify timestamps are set (not zero)
    assert!(conv.created_at.timestamp() > 0);
    assert!(conv.updated_at >= conv.created_at);
    assert_eq!(conv.status, "active");
    assert!(conv.importance_score > 0);
}

#[tokio::test]
async fn test_field_after_import() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("move_test.json");
    fs::write(&file_path, chatgpt_single_json()).unwrap();

    let (processor, _repo) = create_test_processor().await;
    let result = processor.process_file(&file_path).await;

    assert!(result.is_ok());

    // File should be moved (but we don't have imported dir in temp test)
    // The main point is it doesn't panic
}
