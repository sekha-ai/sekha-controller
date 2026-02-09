use mockall::predicate::*;
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
async fn test_suggest_labels_conversation_not_found() {
    let mut mock_repo = MockConversationRepository::new();

    mock_repo.expect_find_by_id().returning(|_| Ok(None));

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

    mock_repo.expect_find_by_id().returning(move |_| {
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
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_suggest_labels_db_error() {
    let mut mock_repo = MockConversationRepository::new();

    mock_repo.expect_find_by_id().returning(|_| {
        Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(
            sea_orm::ConnAcquireErr::Timeout,
        )))
    });

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    let result = label_intel.suggest_labels(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_suggest_labels_message_retrieval_error() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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

    mock_repo.expect_get_conversation_messages().returning(|_| {
        Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(
            sea_orm::ConnAcquireErr::Timeout,
        )))
    });

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    let result = label_intel.suggest_labels(conv_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_suggest_labels_llm_unavailable_with_existing_labels() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test message content".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["work".to_string(), "personal".to_string()]));

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    // LLM is offline by default in tests, should use graceful degradation
    let result = label_intel.suggest_labels(conv_id).await.unwrap();

    // Should return suggestions based on first existing label
    assert!(result.len() > 0);
    assert_eq!(result[0].label, "work");
    assert!(result[0].is_existing);
    assert_eq!(result[0].confidence, 0.9);
}

#[tokio::test]
async fn test_suggest_labels_llm_unavailable_no_existing_labels() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test message content".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo.expect_get_all_labels().returning(|| Ok(vec![]));

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    // LLM is offline by default in tests, should use graceful degradation
    let result = label_intel.suggest_labels(conv_id).await.unwrap();

    // Should return generic suggestions when no existing labels
    assert!(result.len() > 0);
    let labels: Vec<String> = result.iter().map(|s| s.label.clone()).collect();
    assert!(
        labels.contains(&"general".to_string())
            || labels.contains(&"conversation".to_string())
            || labels.contains(&"note".to_string())
    );
}

#[tokio::test]
async fn test_suggest_labels_response_parsing_multiple_labels() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["existing".to_string(), "old".to_string()]));

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    let result = label_intel.suggest_labels(conv_id).await.unwrap();

    // Verify suggestions structure
    assert!(result.len() > 0);
    for suggestion in &result {
        assert!(!suggestion.label.is_empty());
        assert!(suggestion.confidence > 0.0);
        assert_eq!(suggestion.reason, "Suggested based on conversation content");
    }
}

#[tokio::test]
async fn test_suggest_labels_long_content_truncation() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
        Ok(Some(Conversation {
            id: conv_id,
            label: "Long".to_string(),
            folder: "/test".to_string(),
            status: "active".to_string(),
            importance_score: 5,
            word_count: 3000,
            session_count: 1,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }))
    });

    let long_content = "x".repeat(5000);
    mock_repo
        .expect_get_conversation_messages()
        .returning(move |_| {
            Ok(vec![Message {
                id: Uuid::new_v4(),
                conversation_id: conv_id,
                role: "user".to_string(),
                content: long_content.clone(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["test".to_string()]));

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    // Should handle long content (truncated to 2000 chars)
    let result = label_intel.suggest_labels(conv_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_auto_label_above_threshold() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["work".to_string()]));

    mock_repo
        .expect_update_label()
        .with(eq(conv_id), eq("work"), eq("/personal"))
        .returning(|_, _, _| Ok(()));

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    // Threshold of 0.8, should match confidence of 0.9 for existing label
    let result = label_intel.auto_label(conv_id, 0.8).await.unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap(), "work");
}

#[tokio::test]
async fn test_auto_label_below_threshold() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["work".to_string()]));

    // update_label should NOT be called if confidence is below threshold

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    // Threshold of 0.95 (above the 0.9 confidence for existing labels)
    let result = label_intel.auto_label(conv_id, 0.95).await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_auto_label_no_suggestions() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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

    let result = label_intel.auto_label(conv_id, 0.8).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_auto_label_update_error() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["work".to_string()]));

    mock_repo.expect_update_label().returning(|_, _, _| {
        Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(
            sea_orm::ConnAcquireErr::Timeout,
        )))
    });

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    let result = label_intel.auto_label(conv_id, 0.8).await;
    assert!(result.is_err());
}

#[test]
fn test_infer_folder_with_colon() {
    let mock_repo = Arc::new(MockConversationRepository::new());
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(mock_repo, llm_bridge);

    // Test with colon (work folder)
    let folder = label_intel.infer_folder("project:backend");
    assert_eq!(folder, "/work");

    let folder = label_intel.infer_folder("client:acme");
    assert_eq!(folder, "/work");
}

#[test]
fn test_infer_folder_without_colon() {
    let mock_repo = Arc::new(MockConversationRepository::new());
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(mock_repo, llm_bridge);

    // Test without colon (personal folder)
    let folder = label_intel.infer_folder("shopping");
    assert_eq!(folder, "/personal");

    let folder = label_intel.infer_folder("family");
    assert_eq!(folder, "/personal");

    let folder = label_intel.infer_folder("");
    assert_eq!(folder, "/personal");
}

#[tokio::test]
async fn test_suggest_labels_existing_vs_new_confidence() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    let msg_id = Uuid::new_v4();

    mock_repo.expect_find_by_id().returning(move |_| {
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
        .returning(move |_| {
            Ok(vec![Message {
                id: msg_id,
                conversation_id: conv_id,
                role: "user".to_string(),
                content: "Test".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                embedding_id: None,
                metadata: Some(serde_json::json!({})),
            }])
        });

    // Return existing labels
    mock_repo
        .expect_get_all_labels()
        .returning(|| Ok(vec!["coding".to_string(), "personal".to_string()]));

    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let label_intel = LabelIntelligence::new(repo, llm_bridge);

    let result = label_intel.suggest_labels(conv_id).await.unwrap();

    // Verify existing labels have higher confidence (0.9) vs new labels (0.6)
    for suggestion in &result {
        if suggestion.is_existing {
            assert_eq!(suggestion.confidence, 0.9);
        } else {
            assert_eq!(suggestion.confidence, 0.6);
        }
    }
}
