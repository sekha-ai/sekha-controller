use sekha_controller::{
    api::routes::AppState,
    config::Config,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, init_db, repository::SeaOrmConversationRepository},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

mod api;
mod context_assembly_integration;
mod orchestrator_edge_cases;
mod orchestrator_integration;
mod pruning_engine_integration;
mod summarizer_integration;

#[allow(dead_code)]
pub fn create_test_conversation(label: &str, folder: &str) -> Value {
    json!({
        "label": label,
        "folder": folder,
        "messages": [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"}
        ]
    })
}

#[allow(dead_code)]
pub async fn create_test_services() -> AppState {
    let config = Arc::new(RwLock::new(Config::default()));

    // Use in-memory SQLite for integration tests
    let db = init_db("sqlite::memory:")
        .await
        .expect("Failed to initialize test database");

    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));

    // Create real repository with test database
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client.clone(),
        embedding_service.clone(),
    ));

    let config_ref = config.read().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(&*config_ref).unwrap());
    drop(config_ref);

    AppState {
        config,
        orchestrator: Arc::new(sekha_controller::orchestrator::MemoryOrchestrator::new(
            repo.clone(),
            llm_bridge.clone(),
        )),
        repo,
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    }
}

#[allow(dead_code)]
pub async fn create_test_state_with_data() -> (AppState, Vec<Uuid>) {
    let state = create_test_services().await;
    (state, vec![])
}
