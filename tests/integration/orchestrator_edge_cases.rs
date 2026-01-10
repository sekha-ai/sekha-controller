// Integration tests for orchestrator edge cases
use sekha_controller::{
    orchestrator::context_assembly::ContextAssembler,
    storage::repository::Repository,
};
use std::sync::Arc;
use tokio;

/// Test context assembly with empty database
#[tokio::test]
async fn test_assembly_empty_database() {
    // Setup: Create repository with empty database
    let repo = Arc::new(Repository::new_in_memory().await.unwrap());
    let assembler = ContextAssembler::new(repo.clone());
    
    // Test: Assemble context with no conversations in DB
    let query = "test query";
    let budget = 2000;
    let excluded_folders = vec![];
    
    let result = assembler.assemble(
        query,
        budget,
        &[],  // preferred_labels
        &excluded_folders,
    ).await;
    
    // Should return empty context, not error
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

/// Test context assembly with huge dataset (performance)
#[tokio::test]
#[ignore]  // Ignore by default - slow test
async fn test_assembly_large_dataset() {
    use std::time::Instant;
    
    // Setup: Create repository with many conversations
    let repo = Arc::new(Repository::new_in_memory().await.unwrap());
    
    // Insert 10,000 test conversations
    for i in 0..10_000 {
        let _ = repo.create_conversation(
            format!("Conversation {}", i),
            format!("/test/folder{}", i % 100),
            vec![
                serde_json::json!({"role": "user", "content": format!("Test message {}", i)})
            ],
            None,
        ).await;
    }
    
    let assembler = ContextAssembler::new(repo.clone());
    
    // Test: Measure performance
    let start = Instant::now();
    let result = assembler.assemble(
        "test query",
        4000,
        &[],
        &[],
    ).await;
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 500, "Should be < 500ms, was {}ms", duration.as_millis());
}

/// Test token budget edge cases
#[tokio::test]
async fn test_budget_edge_cases() {
    let repo = Arc::new(Repository::new_in_memory().await.unwrap());
    
    // Add a test conversation
    let _ = repo.create_conversation(
        "Test".to_string(),
        "/test".to_string(),
        vec![
            serde_json::json!({"role": "user", "content": "Hello world"})
        ],
        None,
    ).await;
    
    let assembler = ContextAssembler::new(repo.clone());
    
    // Test: Budget = 0 (should return empty)
    let result = assembler.assemble("test", 0, &[], &[]).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
    
    // Test: Negative budget (should treat as 0 or error gracefully)
    let result = assembler.assemble("test", -100, &[], &[]).await;
    assert!(result.is_ok());
    // Should either return empty or handle gracefully
    
    // Test: Huge budget
    let result = assembler.assemble("test", 1_000_000, &[], &[]).await;
    assert!(result.is_ok());
}

/// Test privacy filtering with nested folders
#[tokio::test]
async fn test_privacy_nested_folders() {
    let repo = Arc::new(Repository::new_in_memory().await.unwrap());
    
    // Create conversations in nested folders
    let _ = repo.create_conversation(
        "Public".to_string(),
        "/work/project".to_string(),
        vec![serde_json::json!({"role": "user", "content": "Public info"})],
        None,
    ).await;
    
    let _ = repo.create_conversation(
        "Private".to_string(),
        "/private/secrets/deep".to_string(),
        vec![serde_json::json!({"role": "user", "content": "Secret info"})],
        None,
    ).await;
    
    let assembler = ContextAssembler::new(repo.clone());
    
    // Test: Exclude /private (should exclude all subfolders too)
    let result = assembler.assemble(
        "info",
        4000,
        &[],
        &["/private".to_string()],
    ).await.unwrap();
    
    // Should only return public conversation
    assert!(result.len() > 0);
    for msg in result {
        assert!(!msg.folder.starts_with("/private"));
    }
}

/// Test message truncation when hitting budget
#[tokio::test]
async fn test_message_truncation() {
    let repo = Arc::new(Repository::new_in_memory().await.unwrap());
    
    // Create conversations with known token counts
    let long_content = "a".repeat(1000);  // ~250 tokens
    
    for i in 0..5 {
        let _ = repo.create_conversation(
            format!("Long {}", i),
            "/test".to_string(),
            vec![serde_json::json!({"role": "user", "content": long_content})],
            None,
        ).await;
    }
    
    let assembler = ContextAssembler::new(repo.clone());
    
    // Test: Small budget should limit results
    let result = assembler.assemble(
        "test",
        500,  // ~2 messages
        &[],
        &[],
    ).await.unwrap();
    
    // Should have fewer than 5 messages due to budget
    assert!(result.len() < 5);
    assert!(result.len() > 0);
}

/// Test with unicode and emoji content
#[tokio::test]
async fn test_unicode_content() {
    let repo = Arc::new(Repository::new_in_memory().await.unwrap());
    
    // Create conversation with unicode
    let _ = repo.create_conversation(
        "Unicode Test".to_string(),
        "/test".to_string(),
        vec![
            serde_json::json!({
                "role": "user",
                "content": "Hello 🌍 世界 مرحبا"
            })
        ],
        None,
    ).await;
    
    let assembler = ContextAssembler::new(repo.clone());
    
    // Test: Should handle unicode correctly
    let result = assembler.assemble(
        "hello",
        4000,
        &[],
        &[],
    ).await;
    
    assert!(result.is_ok());
}
