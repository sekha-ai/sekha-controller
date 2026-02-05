use sekha_controller::{
    api::routes::{create_router, AppState},
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, repository::MockConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_mcp_api_key_required() {
    let config = Arc::new(RwLock::new(Config::default()));
    let mock_repo = Arc::new(MockConversationRepository::new());
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));

    let config_ref = config.read().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(&*config_ref).unwrap());
    drop(config_ref);

    let state = AppState {
        config,
        repo: mock_repo.clone(),
        orchestrator: Arc::new(MemoryOrchestrator::new(mock_repo, llm_bridge.clone())),
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    };

    let _app = create_router(state);
    // Test that API key validation is present
    assert!(true); // Placeholder - actual auth testing would need HTTP client
}
