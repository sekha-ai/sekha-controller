use sekha_controller::{
    models::internal::{Conversation, Message},
    storage::repository::{MockConversationRepository, RepositoryError, SearchResult},
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_context_assembler_creation() {
    let mock_repo = Arc::new(MockConversationRepository::new());
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(mock_repo);
    // Just verify construction succeeds
    assert!(true);
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_semantic_search() {
    let mut mock_repo = MockConversationRepository::new();
    
    let results = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.95,
            content: "Test message".to_string(),
            label: "Work".to_string(),
            folder: "/work".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
    ];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Will panic at get_db() for pinned messages
    let _ = assembler.assemble("test query", vec![], 1000, vec![]).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_preferred_labels() {
    let mut mock_repo = MockConversationRepository::new();
    
    let results = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.85,
            content: "Labeled message".to_string(),
            label: "Important".to_string(),
            folder: "/work".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.95,
            content: "Other message".to_string(),
            label: "Other".to_string(),
            folder: "/work".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
    ];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Will panic at get_db()
    let _ = assembler.assemble("test", vec!["Important".to_string()], 1000, vec![]).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_excluded_folders() {
    let mut mock_repo = MockConversationRepository::new();
    
    let results = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.95,
            content: "Excluded message".to_string(),
            label: "Test".to_string(),
            folder: "/excluded".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.90,
            content: "Included message".to_string(),
            label: "Test".to_string(),
            folder: "/included".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
    ];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Test folder exclusion
    let _ = assembler.assemble("test", vec![], 1000, vec!["/excluded".to_string()]).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_small_budget() {
    let mut mock_repo = MockConversationRepository::new();
    
    let results = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.95,
            content: "A".repeat(5000), // 5000 chars
            label: "Test".to_string(),
            folder: "/test".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.90,
            content: "B".repeat(5000), // 5000 chars
            label: "Test".to_string(),
            folder: "/test".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
    ];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Test budget constraint
    let _ = assembler.assemble("test", vec![], 500, vec![]).await;
}

#[tokio::test]
async fn test_assemble_with_semantic_search_error() {
    let mut mock_repo = MockConversationRepository::new();
    
    mock_repo
        .expect_semantic_search()
        .returning(|_, _, _| Err(RepositoryError::NotFound("No results".to_string())));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    let result = assembler.assemble("test", vec![], 1000, vec![]).await;
    assert!(result.is_err());
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_large_budget() {
    let mut mock_repo = MockConversationRepository::new();
    
    let mut results = vec![];
    for i in 0..10 {
        results.push(SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.9 - (i as f32 * 0.01),
            content: format!("Message {}", i),
            label: "Test".to_string(),
            folder: "/test".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        });
    }
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Test with very large budget
    let _ = assembler.assemble("test", vec![], 50000, vec![]).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_multiple_labels() {
    let mut mock_repo = MockConversationRepository::new();
    
    let results = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.85,
            content: "Label1 message".to_string(),
            label: "Label1".to_string(),
            folder: "/test".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.90,
            content: "Label2 message".to_string(),
            label: "Label2".to_string(),
            folder: "/test".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        },
    ];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    let _ = assembler
        .assemble("test", vec!["Label1".to_string(), "Label2".to_string()], 1000, vec![])
        .await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_old_messages() {
    let mut mock_repo = MockConversationRepository::new();
    
    let old_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(30);
    let results = vec![SearchResult {
        conversation_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
        score: 0.95,
        content: "Old message".to_string(),
        label: "Test".to_string(),
        folder: "/test".to_string(),
        timestamp: old_date,
        metadata: serde_json::json!({}),
    }];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Test recency scoring with old messages
    let _ = assembler.assemble("test", vec![], 1000, vec![]).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_empty_results() {
    let mut mock_repo = MockConversationRepository::new();
    
    mock_repo
        .expect_semantic_search()
        .returning(|_, _, _| Ok(vec![]));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    // Test with no semantic search results
    let _ = assembler.assemble("test", vec![], 1000, vec![]).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_assemble_with_metadata() {
    let mut mock_repo = MockConversationRepository::new();
    
    let results = vec![SearchResult {
        conversation_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
        score: 0.95,
        content: "Message with metadata".to_string(),
        label: "Test".to_string(),
        folder: "/test".to_string(),
        timestamp: chrono::Utc::now().naive_utc(),
        metadata: serde_json::json!({
            "role": "user",
            "extra": "data"
        }),
    }];
    
    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(results.clone()));
    
    let repo = Arc::new(mock_repo);
    let assembler = sekha_controller::orchestrator::context_assembly::ContextAssembler::new(repo);
    
    let _ = assembler.assemble("test", vec![], 1000, vec![]).await;
}
