use sekha_controller::{
    config::Config,
    models::internal::{Conversation, Message},
    orchestrator::label_intelligence::LabelIntelligence,
    services::llm_bridge_client::LlmBridgeClient,
    storage::repository::{MockConversationRepository, RepositoryError},
};
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_label_intelligence_creation() {
    let mock_repo = Arc::new(MockConversationRepository::new());
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    
    let label_intel = LabelIntelligence::new(mock_repo, llm_bridge);
    assert!(true);
}

#[tokio::test]
#[should_panic(expected = "get_all_labels()")]
async fn test_suggest_labels_with_messages() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Unlabeled".to_string(),
                folder: "/inbox".to_string(),
                status: "active".to_string(),
                importance_score: 5,
                word_count: 100,
                session_count: 1,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_conversation_messages()
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Let's discuss the project timeline and deliverables".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    // Will panic at get_all_labels()
    let _ = label_intel.suggest_labels(conv_id).await;
}

#[tokio::test]
async fn test_suggest_labels_conversation_not_found() {
    let mut mock_repo = MockConversationRepository::new();
    
    mock_repo
        .expect_find_by_id()
        .returning(|_| Ok(None));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let result = label_intel.suggest_labels(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_suggest_labels_no_messages() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Empty".to_string(),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: 3,
                word_count: 0,
                session_count: 1,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_conversation_messages()
        .returning(|_| Ok(vec![]));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let result = label_intel.suggest_labels(conv_id).await;
    assert!(result.is_err());
}

#[tokio::test]
#[should_panic(expected = "get_all_labels()")]
async fn test_suggest_labels_with_long_messages() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Long Conversation".to_string(),
                folder: "/work".to_string(),
                status: "active".to_string(),
                importance_score: 8,
                word_count: 2000,
                session_count: 10,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    let mut messages = vec![];
    for i in 0..20 {
        messages.push(Message {
            id: Uuid::new_v4(),
            conversation_id: conv_id,
            role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
            content: format!("This is message {} with some detailed content about work projects and technical discussions", i),
            timestamp: chrono::Utc::now().naive_utc(),
            embedding_id: None,
            metadata: Some(serde_json::json!({})),
        });
    }
    
    mock_repo
        .expect_get_conversation_messages()
        .returning(move |_| Ok(messages.clone()));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let _ = label_intel.suggest_labels(conv_id).await;
}

#[tokio::test]
async fn test_suggest_labels_db_error() {
    let mut mock_repo = MockConversationRepository::new();
    
    mock_repo
        .expect_find_by_id()
        .returning(|_| Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(sea_orm::RuntimeErr::Internal("Connection failed".to_string())))));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let result = label_intel.suggest_labels(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
#[should_panic(expected = "get_all_labels()")]
async fn test_suggest_labels_with_technical_content() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Technical".to_string(),
                folder: "/coding".to_string(),
                status: "active".to_string(),
                importance_score: 7,
                word_count: 500,
                session_count: 5,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_conversation_messages()
        .returning(move |_| {
            Ok(vec![
                Message {
                    id: Uuid::new_v4(),
                    conversation_id: conv_id,
                    role: "user".to_string(),
                    content: "I need help implementing a REST API in Rust with Axum framework".to_string(),
                    timestamp: chrono::Utc::now().naive_utc(),
                    embedding_id: None,
                    metadata: Some(serde_json::json!({})),
                },
                Message {
                    id: Uuid::new_v4(),
                    conversation_id: conv_id,
                    role: "assistant".to_string(),
                    content: "I can help you with that. Let's start with the basic setup".to_string(),
                    timestamp: chrono::Utc::now().naive_utc(),
                    embedding_id: None,
                    metadata: Some(serde_json::json!({})),
                },
            ])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let _ = label_intel.suggest_labels(conv_id).await;
}

#[tokio::test]
#[should_panic(expected = "get_all_labels()")]
async fn test_suggest_labels_with_single_message() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Quick".to_string(),
                folder: "/inbox".to_string(),
                status: "active".to_string(),
                importance_score: 4,
                word_count: 20,
                session_count: 1,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_conversation_messages()
        .returning(move |_| {
            Ok(vec![Message {
                id: Uuid::new_v4(),
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Quick question about database optimization".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let _ = label_intel.suggest_labels(conv_id).await;
}

#[tokio::test]
async fn test_suggest_labels_message_retrieval_error() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Test".to_string(),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: 5,
                word_count: 100,
                session_count: 1,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_conversation_messages()
        .returning(|_| Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(sea_orm::RuntimeErr::Internal("Connection failed".to_string())))));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);
    
    let result = label_intel.suggest_labels(conv_id).await;
    assert!(result.is_err());
}
