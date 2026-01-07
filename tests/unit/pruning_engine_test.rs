use chrono::Utc;
use sekha_controller::models::internal::{NewConversation, NewMessage};
use sekha_controller::orchestrator::pruning_engine::PruningEngine;
use sekha_controller::services::embedding_service::EmbeddingService;
use sekha_controller::services::llm_bridge_client::LlmBridgeClient;
use sekha_controller::storage::chroma_client::ChromaClient;
use sekha_controller::storage::repository::ConversationRepository;
use sekha_controller::storage::SeaOrmConversationRepository;
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_generate_suggestions_with_active_conversation() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let db = sekha_controller::storage::init_db("sqlite::memory:")
        .await
        .unwrap();
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        mock_server.uri(),
        "http://localhost:8000".to_string(),
    ));
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma,
        embedding_service,
    ));

    let conv = NewConversation {
        id: None,
        label: "Test Conversation".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(3),
        word_count: 1000,
        session_count: Some(1),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "Test message content here".to_string(),
            metadata: json!({}),
            timestamp: Utc::now().naive_utc(),
        }],
    };

    let conv_id = repo.create_with_messages(conv).await.unwrap();

    Mock::given(method("POST"))
        .and(path("/summarize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "summary": "Test conversation summary",
            "level": "daily",
            "model": "llama3.1:8b",
            "tokens_used": 25
        })))
        .mount(&mock_server)
        .await;

    let engine = PruningEngine::new(repo.clone(), llm_bridge);
    let suggestions = engine.generate_suggestions(0, 5.0).await.unwrap();

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].conversation_id, conv_id);
    assert_eq!(suggestions[0].conversation_label, "Test Conversation");
    assert_eq!(suggestions[0].importance_score, 3.0);
    assert!(suggestions[0].preview.contains("Test conversation"));
    assert_eq!(suggestions[0].recommendation, "keep");
}

#[tokio::test]
async fn test_generate_suggestions_filters_by_date_threshold() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let db = sekha_controller::storage::init_db("sqlite::memory:")
        .await
        .unwrap();
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        mock_server.uri(),
        "http://localhost:8000".to_string(),
    ));
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma,
        embedding_service,
    ));

    let conv = NewConversation {
        id: None,
        label: "Recent".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "Recent message".to_string(),
            metadata: json!({}),
            timestamp: Utc::now().naive_utc(),
        }],
    };

    repo.create_with_messages(conv).await.unwrap();

    let engine = PruningEngine::new(repo.clone(), llm_bridge);
    let suggestions = engine.generate_suggestions(1000, 5.0).await.unwrap();

    assert_eq!(suggestions.len(), 0);
}

#[tokio::test]
async fn test_generate_suggestions_empty_database() {
    let mock_server = MockServer::start().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(mock_server.uri()));

    let db = sekha_controller::storage::init_db("sqlite::memory:")
        .await
        .unwrap();
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        mock_server.uri(),
        "http://localhost:8000".to_string(),
    ));
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma,
        embedding_service,
    ));

    let engine = PruningEngine::new(repo.clone(), llm_bridge);
    let suggestions = engine.generate_suggestions(50, 5.0).await.unwrap();

    assert_eq!(suggestions.len(), 0);
}
