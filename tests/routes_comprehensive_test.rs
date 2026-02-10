//! Comprehensive integration tests for API routes
//! Tests all endpoints restored in Module 1-5

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sekha_controller::{
    api::routes::{self, AppState},
    config::Config,
    llm::bridge_client::BridgeClient,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, repository::MockConversationRepository},
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

/// Create test AppState with mock repository
async fn create_test_state() -> AppState {
    let config = Arc::new(RwLock::new(Config::default()));
    let mock_repo = Arc::new(MockConversationRepository::new());

    let config_ref = config.read().await;

    let bridge = BridgeClient::new(&*config_ref).expect("Failed to create BridgeClient");
    let embedding_service = Arc::new(EmbeddingService::new(
        bridge,
        "http://localhost:8000".to_string(),
    ));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let llm_bridge = Arc::new(LlmBridgeClient::new(&*config_ref).unwrap());
    drop(config_ref);

    routes::AppState {
        config,
        repo: mock_repo.clone(),
        orchestrator: Arc::new(MemoryOrchestrator::new(mock_repo, llm_bridge.clone())),
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    }
}

// ==================== HEALTH & METRICS ====================

#[tokio::test]
async fn test_health_endpoint() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
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

// ==================== CONVERSATION CRUD ====================

#[tokio::test]
async fn test_create_conversation() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let payload = json!({
        "label": "Test Conversation",
        "folder": "/work",
        "messages": [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi!"}
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/conversations")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_list_conversations() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/conversations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_conversations_with_pagination() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/conversations?page=1&page_size=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_conversations_with_filters() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/conversations?label=Test&folder=/work")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_count_conversations() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/conversations/count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_count_conversations_by_label() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/conversations/count?label=Project:AI")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== QUERY ENDPOINTS ====================

#[tokio::test]
async fn test_semantic_query() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let payload = json!({
        "query": "authentication",
        "limit": 10,
        "offset": 0
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/query")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_full_text_search() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let payload = json!({
        "query": "test",
        "limit": 10
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/search/fts")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rebuild_embeddings() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rebuild-embeddings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

// ==================== ORCHESTRATION ENDPOINTS ====================

#[tokio::test]
async fn test_assemble_context() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let payload = json!({
        "query": "test query",
        "preferred_labels": [],
        "context_budget": 4000,
        "excluded_folders": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/context/assemble")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_prune_dry_run() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let payload = json!({
        "threshold_days": 90
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/prune/dry-run")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== ERROR CASES ====================

#[tokio::test]
async fn test_create_conversation_invalid_json() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/conversations")
                .header("content-type", "application/json")
                .body(Body::from("{invalid json}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_semantic_query_missing_fields() {
    let state = create_test_state().await;
    let app = routes::create_router(state);

    let payload = json!({});

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/query")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
