use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sekha_controller::api::routes::create_router;
use sekha_controller::{
    api::routes::AppState,
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, repository::MockConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

#[tokio::test]
async fn test_metrics_route() {
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

    let router = create_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_route() {
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

    let router = create_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
