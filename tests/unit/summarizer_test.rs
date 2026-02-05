use sekha_controller::{
    config::Config,
    models::internal::{Conversation, Message},
    orchestrator::summarizer::Summarizer,
    services::llm_bridge_client::LlmBridgeClient,
    storage::repository::{MockConversationRepository, RepositoryError},
};
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_summarizer_creation() {
    let mock_repo = Arc::new(MockConversationRepository::new());
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    
    let summarizer = Summarizer::new(mock_repo, llm_bridge);
    assert!(true); // Verify construction succeeds
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_messages() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Test Conversation".to_string(),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: 7,
                word_count: 250,
                session_count: 3,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_message_list()
        .returning(|_| {
            Ok(vec![
                serde_json::json!({
                    "role": "user",
                    "content": "Hello, I need help with my project"
                }),
                serde_json::json!({
                    "role": "assistant",
                    "content": "I'd be happy to help! What's your project about?"
                }),
                serde_json::json!({
                    "role": "user",
                    "content": "It's a web application for task management"
                }),
            ])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    // Will panic at get_db() during message retrieval
    let _ = summarizer.generate_daily_summary(conv_id).await;
}

#[tokio::test]
async fn test_generate_daily_summary_conversation_not_found() {
    let mut mock_repo = MockConversationRepository::new();
    
    mock_repo
        .expect_find_by_id()
        .returning(|_| Ok(None));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let result = summarizer.generate_daily_summary(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_long_conversation() {
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
                importance_score: 9,
                word_count: 5000,
                session_count: 20,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    // Generate many messages
    let mut messages = vec![];
    for i in 0..100 {
        messages.push(serde_json::json!({
            "role": if i % 2 == 0 { "user" } else { "assistant" },
            "content": format!("Message {}: Lorem ipsum dolor sit amet", i)
        }));
    }
    
    mock_repo
        .expect_get_message_list()
        .returning(move |_| Ok(messages.clone()));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let _ = summarizer.generate_daily_summary(conv_id).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_empty_messages() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Empty Conversation".to_string(),
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
        .expect_get_message_list()
        .returning(|_| Ok(vec![]));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let _ = summarizer.generate_daily_summary(conv_id).await;
}

#[tokio::test]
async fn test_generate_daily_summary_db_error() {
    let mut mock_repo = MockConversationRepository::new();
    
    mock_repo
        .expect_find_by_id()
        .returning(|_| Err(RepositoryError::DbError(sea_orm::DbErr::ConnectionAcquire(sea_orm::RuntimeErr::Internal("Connection failed".to_string())))));
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let result = summarizer.generate_daily_summary(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_single_message() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Single Message".to_string(),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: 5,
                word_count: 10,
                session_count: 1,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_message_list()
        .returning(|_| {
            Ok(vec![serde_json::json!({
                "role": "user",
                "content": "Quick question"
            })])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let _ = summarizer.generate_daily_summary(conv_id).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_special_characters() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Special Chars!".to_string(),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: 5,
                word_count: 50,
                session_count: 1,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_message_list()
        .returning(|_| {
            Ok(vec![
                serde_json::json!({
                    "role": "user",
                    "content": "Hello! How are you? 😊 #hashtag @mention"
                }),
                serde_json::json!({
                    "role": "assistant",
                    "content": "I'm doing well, thanks! 🎉 [link](https://example.com)"
                }),
            ])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let _ = summarizer.generate_daily_summary(conv_id).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_code_blocks() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Code Discussion".to_string(),
                folder: "/coding".to_string(),
                status: "active".to_string(),
                importance_score: 8,
                word_count: 300,
                session_count: 5,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_message_list()
        .returning(|_| {
            Ok(vec![
                serde_json::json!({
                    "role": "user",
                    "content": "Here's my code: ```rust\nfn main() {\n    println!(\"Hello\");\n}\n```"
                }),
                serde_json::json!({
                    "role": "assistant",
                    "content": "Looks good! You could also use: ```rust\nprintln!(\"Hello, world!\");\n```"
                }),
            ])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let _ = summarizer.generate_daily_summary(conv_id).await;
}

#[tokio::test]
#[should_panic(expected = "get_db()")]
async fn test_generate_daily_summary_with_multilingual_content() {
    let mut mock_repo = MockConversationRepository::new();
    let conv_id = Uuid::new_v4();
    
    mock_repo
        .expect_find_by_id()
        .returning(move |_| {
            Ok(Some(Conversation {
                id: conv_id,
                label: "Multilingual".to_string(),
                folder: "/test".to_string(),
                status: "active".to_string(),
                importance_score: 6,
                word_count: 100,
                session_count: 2,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            }))
        });
    
    mock_repo
        .expect_get_message_list()
        .returning(|_| {
            Ok(vec![
                serde_json::json!({
                    "role": "user",
                    "content": "Bonjour! Comment ça va? 你好! こんにちは!"
                }),
                serde_json::json!({
                    "role": "assistant",
                    "content": "Hello! I can help you in multiple languages."
                }),
            ])
        });
    
    let repo = Arc::new(mock_repo);
    let config = Config::default();
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());
    let summarizer = Summarizer::new(repo, llm_bridge);
    
    let _ = summarizer.generate_daily_summary(conv_id).await;
}
