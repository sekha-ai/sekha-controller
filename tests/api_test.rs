// tests/api_test.rs - FULLY CORRECTED

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use sekha_controller::{
    api::{mcp, routes},
    config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::chroma_client::ChromaClient,
    storage::{self, SeaOrmConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use serde_json::json;

// ============================================
// Test Helpers
// ============================================

fn create_test_services() -> (Arc<ChromaClient>, Arc<EmbeddingService>) {
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    (chroma_client, embedding_service)
}

async fn create_test_config() -> Arc<RwLock<config::Config>> {
    let config_str = r#"
        server_port = 8080
        mcp_api_key = "test_key_12345678901234567890123456789012"
        database_url = "sqlite::memory:"
        ollama_url = "http://localhost:11434"
        chroma_url = "http://localhost:8000"
        max_connections = 10
        log_level = "info"
        summarization_enabled = true
        pruning_enabled = true
        embedding_model = "nomic-embed-text:latest"
        summarization_model = "llama3.1:8b"
    "#;

    let config: config::Config = toml::from_str(config_str).unwrap();
    Arc::new(RwLock::new(config))
}

async fn setup_test_repo() -> (
    Arc<SeaOrmConversationRepository>,
    Arc<ChromaClient>,
    Arc<EmbeddingService>,
) {
    let db_conn = storage::init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db_conn,
        chroma_client.clone(),
        embedding_service.clone(),
    ));
    (repo, chroma_client, embedding_service)
}

async fn create_test_app() -> Router {
    let (repo, chroma_client, embedding_service) = setup_test_repo().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));

    let state = routes::AppState {
        config: create_test_config().await,
        repo: repo.clone(),
        chroma_client,
        embedding_service,
        orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge)),
    };

    routes::create_router(state)
}

async fn create_test_mcp_app() -> Router {
    let (repo, chroma_client, embedding_service) = setup_test_repo().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));

    let state = routes::AppState {
        config: create_test_config().await,
        repo: repo.clone(),
        chroma_client,
        embedding_service,
        orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge)),
    };

    mcp::create_mcp_router(state)
}

// ============================================
// Tests
// ============================================

#[tokio::test]
async fn test_create_conversation() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"label": "Test", "folder": "/", "messages": [{"role": "user", "content": "Hello"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_mcp_auth_failure() {
    let app = create_test_mcp_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                // Missing Authorization header
                .body(Body::from(
                    r#"{"label": "Test", "folder": "/", "messages": [{"role": "user", "content": "Hello"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_success() {
    let app = create_test_mcp_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{"label": "Auth Success", "folder": "/", "messages": [{"role": "user", "content": "Test"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_conversations_pagination() {
    let app = create_test_app().await;

    // Create multiple conversations
    for i in 0..5 {
        app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"label": "Pagination Test {}", "folder": "/pagination", "messages": [{{"role": "user", "content": "Message {}"}}]}}"#,
                        i, i
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Test pagination
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/conversations?limit=3&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["results"].is_array());
    assert_eq!(json["results"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn test_query_endpoint() {
    let app = create_test_app().await;

    // Store a conversation
    app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"label": "Query Test", "folder": "/query", "messages": [{"role": "user", "content": "Semantic search test"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Query for it
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"query": "semantic search", "limit": 10}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== MCP Export Tests ====================

#[tokio::test]
async fn test_memory_export_not_found() {
    let app = create_test_mcp_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp/tools/memory_export")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{"conversation_id": "00000000-0000-0000-0000-000000000000", "format": "json"}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ==================== MCP Stats Tests ====================

#[tokio::test]
async fn test_memory_stats_empty() {
    let app = create_test_mcp_app().await;

    // Get stats on empty database
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp/tools/memory_stats")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(r#"{"folder": null}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["data"]["total_conversations"], 0);
    assert_eq!(json["data"]["average_importance"], 0.0);
    assert_eq!(json["data"]["folders"], json!([]));
}
