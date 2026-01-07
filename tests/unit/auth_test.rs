use axum::extract::FromRequestParts;
use axum::http::{Request, StatusCode};
use sekha_controller::api::routes::AppState;
use sekha_controller::auth::McpAuth;
use sekha_controller::config::Config;
use sekha_controller::orchestrator::MemoryOrchestrator;
use sekha_controller::services::embedding_service::EmbeddingService;
use sekha_controller::services::llm_bridge_client::LlmBridgeClient;
use sekha_controller::storage::chroma_client::ChromaClient;
use sekha_controller::storage::SeaOrmConversationRepository;
use std::sync::Arc;
use tokio::sync::RwLock;

async fn create_test_state(api_key: String) -> AppState {
    let config = Arc::new(RwLock::new(Config {
        server_port: 8080,
        mcp_api_key: api_key,
        rest_api_key: None,
        database_url: "sqlite::memory:".to_string(),
        ollama_url: "http://localhost:11434".to_string(),
        chroma_url: "http://localhost:8000".to_string(),
        additional_api_keys: vec![],
        cors_enabled: true,
        rate_limit_per_minute: 60,
        max_connections: 10,
        log_level: "info".to_string(),
        summarization_enabled: true,
        pruning_enabled: true,
        embedding_model: "nomic-embed-text:latest".to_string(),
        summarization_model: "llama3.1:8b".to_string(),
    }));

    let db = sekha_controller::storage::init_db("sqlite::memory:")
        .await
        .unwrap();
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma.clone(),
        embedding_service.clone(),
    ));
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));

    AppState {
        config,
        repo: repo.clone(),
        chroma_client: chroma,
        embedding_service,
        orchestrator: Arc::new(MemoryOrchestrator::new(repo.clone(), llm_bridge)),
    }
}

#[tokio::test]
async fn test_valid_auth_token() {
    let state = create_test_state("test_key_12345678901234567890123456789012".to_string()).await;

    let mut req = Request::builder()
        .header(
            "authorization",
            "Bearer test_key_12345678901234567890123456789012",
        )
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().token,
        "test_key_12345678901234567890123456789012"
    );
}

#[tokio::test]
async fn test_missing_authorization_header() {
    let state = create_test_state("test_key_12345678901234567890123456789012".to_string()).await;

    let mut req = Request::builder().body(()).unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_authorization_format() {
    let state = create_test_state("test_key_12345678901234567890123456789012".to_string()).await;

    let mut req = Request::builder()
        .header(
            "authorization",
            "InvalidFormat test_key_12345678901234567890123456789012",
        )
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_api_key() {
    let state = create_test_state("test_key_12345678901234567890123456789012".to_string()).await;

    let mut req = Request::builder()
        .header(
            "authorization",
            "Bearer wrong_key_12345678901234567890123456789012",
        )
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_short_api_key() {
    let state = create_test_state("test_key_12345678901234567890123456789012".to_string()).await;

    let mut req = Request::builder()
        .header("authorization", "Bearer short")
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
}
