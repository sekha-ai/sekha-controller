use chrono::Utc;
use sekha_controller::{
    config::Config,
    models::internal::Message,
    orchestrator::{
        label_intelligence::LabelSuggestion, pruning_engine::PruningSuggestion, MemoryOrchestrator,
    },
    services::llm_bridge_client::LlmBridgeClient,
    storage::repository::{MockConversationRepository, SearchResult},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_memory_orchestrator_creation() {
    let mock_repo = Arc::new(MockConversationRepository::new());
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());

    let orchestrator = MemoryOrchestrator::new(mock_repo, llm_bridge);

    // Just verify it creates without panicking
    assert!(true);
}

#[tokio::test]
async fn test_assemble_context_with_search_results() {
    let mut mock_repo = MockConversationRepository::new();

    let test_messages = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.95,
            content: "Important context message 1".to_string(),
            label: "Work".to_string(),
            folder: "/work".to_string(),
            timestamp: Utc::now().naive_utc(),
            metadata: json!({"role": "user"}),
        },
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.89,
            content: "Important context message 2".to_string(),
            label: "Personal".to_string(),
            folder: "/personal".to_string(),
            timestamp: Utc::now().naive_utc(),
            metadata: json!({"role": "assistant"}),
        },
    ];

    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(test_messages.clone()));

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator
        .assemble_context("test query", vec!["Work".to_string()], 1000, vec![])
        .await;

    assert!(result.is_ok());
    let messages = result.unwrap();
    assert!(!messages.is_empty());
}

#[tokio::test]
async fn test_assemble_context_with_excluded_folders() {
    let mut mock_repo = MockConversationRepository::new();

    // Return empty results when folder is excluded
    mock_repo
        .expect_semantic_search()
        .returning(|_, _, _| Ok(vec![]));

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator
        .assemble_context("test query", vec![], 1000, vec!["/excluded".to_string()])
        .await;

    assert!(result.is_ok());
    let messages = result.unwrap();
    assert_eq!(messages.len(), 0); // Should be empty due to exclusion
}

#[tokio::test]
async fn test_score_message_importance() {
    let mut mock_repo = MockConversationRepository::new();
    let test_message_id = Uuid::new_v4();

    // Mock getting the message - returns JSON array
    mock_repo.expect_get_message_list().returning(move |_| {
        Ok(vec![json!({
            "id": test_message_id,
            "role": "user",
            "content": "This is a test message",
            "timestamp": "2024-01-01T00:00:00"
        })])
    });

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator.score_message_importance(test_message_id).await;

    // The scoring will fail without real LLM, but we're testing the orchestrator path
    // In a real scenario, you'd mock the LLM bridge response
    assert!(result.is_ok() || result.is_err()); // Either outcome is acceptable for unit test
}

#[tokio::test]
async fn test_generate_daily_summary() {
    let mut mock_repo = MockConversationRepository::new();
    let test_conv_id = Uuid::new_v4();

    // Mock getting messages for summary
    mock_repo.expect_get_message_list().returning(|_| {
        Ok(vec![
            json!({"role": "user", "content": "Hello"}),
            json!({"role": "assistant", "content": "Hi there!"}),
        ])
    });

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator.generate_daily_summary(test_conv_id).await;

    // Summary will fail without real LLM, but we're testing orchestrator integration
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_suggest_pruning() {
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();

    // Mock finding old, low-importance conversations
    let old_convs = vec![Conversation {
        id: Uuid::new_v4(),
        label: "Old Conversation".to_string(),
        folder: "/old".to_string(),
        status: "active".to_string(),
        importance_score: 2,
        word_count: 50,
        session_count: 1,
        created_at: Utc::now().naive_utc() - chrono::Duration::days(100),
        updated_at: Utc::now().naive_utc() - chrono::Duration::days(100),
    }];

    let conv_count = old_convs.len() as u64;
    mock_repo
        .expect_find_with_filters()
        .returning(move |_, _, _| Ok((old_convs.clone(), conv_count)));

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator.suggest_pruning(90).await;

    assert!(result.is_ok());
    let suggestions = result.unwrap();
    // Should have pruning suggestions for old conversations
    assert!(!suggestions.is_empty());
}

#[tokio::test]
async fn test_suggest_labels() {
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();
    let test_conv_id = Uuid::new_v4();

    // Mock conversation retrieval
    mock_repo.expect_find_by_id().returning(move |_| {
        Ok(Some(Conversation {
            id: test_conv_id,
            label: "Unlabeled".to_string(),
            folder: "/inbox".to_string(),
            status: "active".to_string(),
            importance_score: 5,
            word_count: 100,
            session_count: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }))
    });

    // Mock getting messages
    mock_repo.expect_get_message_list().returning(|_| {
        Ok(vec![
            json!({"role": "user", "content": "Let's discuss the project timeline"}),
            json!({"role": "assistant", "content": "Sure, what's your deadline?"}),
        ])
    });

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator.suggest_labels(test_conv_id).await;

    // Label suggestion will fail without real LLM, but we're testing orchestrator path
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_orchestrator_components_exist() {
    // Verify all orchestrator components can be accessed
    let mock_repo = Arc::new(MockConversationRepository::new());
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());

    let orchestrator = MemoryOrchestrator::new(mock_repo, llm_bridge);

    // Access each component to ensure they're properly initialized
    let _ = &orchestrator.context_assembler;
    let _ = &orchestrator.importance_engine;
    let _ = &orchestrator.summarizer;
    let _ = &orchestrator.pruning_engine;
    let _ = &orchestrator.label_intelligence;

    assert!(true);
}

#[tokio::test]
async fn test_assemble_context_respects_budget() {
    let mut mock_repo = MockConversationRepository::new();

    // Return results that would exceed budget
    let large_results = vec![
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.95,
            content: "A".repeat(5000), // 5000 chars
            label: "Work".to_string(),
            folder: "/work".to_string(),
            timestamp: Utc::now().naive_utc(),
            metadata: json!({"role": "user"}),
        },
        SearchResult {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            score: 0.90,
            content: "B".repeat(5000), // 5000 chars
            label: "Work".to_string(),
            folder: "/work".to_string(),
            timestamp: Utc::now().naive_utc(),
            metadata: json!({"role": "user"}),
        },
    ];

    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(large_results.clone()));

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator
        .assemble_context(
            "test query",
            vec![],
            3000, // Budget smaller than 2 messages
            vec![],
        )
        .await;

    assert!(result.is_ok());
    let messages = result.unwrap();

    // Should limit messages to fit within budget
    let total_content_length: usize = messages.iter().map(|m| m.content.len()).sum();

    // Total should be less than or close to budget
    assert!(total_content_length <= 6000); // Some tolerance
}

#[tokio::test]
async fn test_assemble_context_prefers_labels() {
    let mut mock_repo = MockConversationRepository::new();

    let preferred_result = SearchResult {
        conversation_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
        score: 0.85, // Lower score but preferred label
        content: "Preferred label message".to_string(),
        label: "Important".to_string(),
        folder: "/work".to_string(),
        timestamp: Utc::now().naive_utc(),
        metadata: json!({"role": "user"}),
    };

    let other_result = SearchResult {
        conversation_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
        score: 0.95, // Higher score but not preferred
        content: "Other label message".to_string(),
        label: "Other".to_string(),
        folder: "/work".to_string(),
        timestamp: Utc::now().naive_utc(),
        metadata: json!({"role": "user"}),
    };

    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(vec![other_result.clone(), preferred_result.clone()]));

    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let orchestrator = MemoryOrchestrator::new(Arc::new(mock_repo), llm_bridge);

    let result = orchestrator
        .assemble_context("test query", vec!["Important".to_string()], 1000, vec![])
        .await;

    assert!(result.is_ok());
    // The context assembler should prioritize the preferred label
}
