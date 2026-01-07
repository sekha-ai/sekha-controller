// tests/integration/mod.rs

// ============================================
// Re-export commonly used types
// ============================================
pub use serde_json::json; // ✅ Macro
pub use std::sync::Arc; // ✅ Arc
pub use uuid::Uuid; // ✅ Uuid

// tests/integration/mod.rs
use axum::Router;
use sekha_controller::ConversationRepository;
use sekha_controller::{
    api::routes::{create_router, AppState},
    config::Config,
    models::internal::{NewConversation, NewMessage},
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, init_db, SeaOrmConversationRepository},
};
use tokio::sync::RwLock;

// ============================================
// Public modules (test files)
// ============================================
pub mod api;
pub mod concurrency;
pub mod file_watcher;
pub mod mcp;
pub mod orchestrator;
pub mod repository;
// pub mod search;

// ============================================
// Shared Test Helpers
// ============================================

pub fn create_test_services() -> (Arc<ChromaClient>, Arc<EmbeddingService>) {
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    (chroma_client, embedding_service)
}

pub async fn is_llm_bridge_running() -> bool {
    let client = reqwest::Client::new();
    let result = client
        .get("http://localhost:11434")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;

    result.is_ok()
}

pub async fn is_chroma_running() -> bool {
    let client = reqwest::Client::new();
    let result = client
        .get("http://localhost:8000/api/v1/heartbeat")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;

    result.is_ok()
}

pub async fn create_test_config() -> Arc<RwLock<Config>> {
    Arc::new(RwLock::new(Config {
        server_port: 8080,
        mcp_api_key: "test_key_12345678901234567890123456789012".to_string(),
        rest_api_key: Some("rest_test_key_123456789012345678901234".to_string()),
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
    }))
}

pub async fn create_test_app() -> Router {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db.clone(),
        chroma_client.clone(),
        embedding_service.clone(),
    ));

    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));

    let state = AppState {
        config: create_test_config().await,
        repo: repo.clone(),
        chroma_client,
        embedding_service,
        orchestrator: Arc::new(sekha_controller::orchestrator::MemoryOrchestrator::new(
            repo, llm_bridge,
        )),
    };

    create_router(state)
}

pub async fn create_test_mcp_app() -> Router {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db.clone(),
        chroma_client.clone(),
        embedding_service.clone(),
    ));

    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));

    let state = AppState {
        config: create_test_config().await,
        repo: repo.clone(),
        chroma_client,
        embedding_service,
        orchestrator: Arc::new(sekha_controller::orchestrator::MemoryOrchestrator::new(
            repo, llm_bridge,
        )),
    };

    sekha_controller::api::mcp::create_mcp_router(state)
}

pub fn create_test_conversation() -> NewConversation {
    NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Test Conversation".to_string(),
        folder: "/tests".to_string(),
        messages: vec![
            NewMessage {
                role: "user".to_string(),
                content: "Hello, this is a test message".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                metadata: json!({"source": "test"}),
            },
            NewMessage {
                role: "assistant".to_string(),
                content: "This is a response to the test".to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
                metadata: json!({"source": "test"}),
            },
        ],
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        importance_score: Some(5),
        status: "active".to_string(),
        word_count: 42,
        updated_at: chrono::Utc::now().naive_utc(),
    }
}

// Feature flag for tests requiring external services
#[cfg(test)]
pub fn requires_chroma() -> bool {
    std::env::var("CHROMA_URL").is_ok()
}

#[cfg(test)]
pub fn requires_ollama() -> bool {
    std::env::var("OLLAMA_URL").is_ok()
}
