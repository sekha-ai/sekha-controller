use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use sekha_controller::api::mcp::create_mcp_router;
use sekha_controller::{
    api::routes::AppState,
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
use uuid::Uuid;

/// Helper function to create test state with proper dependency initialization
async fn create_test_state(mock_repo: MockConversationRepository) -> AppState {
    let base_config = Config::default();
    let config = Arc::new(RwLock::new(base_config.clone()));
    
    // Create BridgeClient first, then pass to EmbeddingService
    let bridge = BridgeClient::new(&base_config).expect("Failed to create BridgeClient");
    let embedding_service = Arc::new(EmbeddingService::new(
        bridge,
        "http://localhost:8000".to_string(),
    ));
    
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let llm_bridge = Arc::new(LlmBridgeClient::new(&base_config).unwrap());
    let repo = Arc::new(mock_repo);
    let orchestrator = Arc::new(MemoryOrchestrator::new(repo.clone(), llm_bridge.clone()));

    AppState {
        config,
        repo,
        orchestrator,
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    }
}

#[tokio::test]
async fn test_memory_store_creates_conversation() {
    let mut mock_repo = MockConversationRepository::new();
    let test_id = Uuid::new_v4();

    mock_repo
        .expect_create_with_messages()
        .returning(move |_| Ok(test_id));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(
                    json!({
                        "label": "Test Memory",
                        "folder": "/test",
                        "messages": [
                            {"role": "user", "content": "Hello"},
                            {"role": "assistant", "content": "Hi there!"}
                        ],
                        "importance_score": 7
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["data"]["conversation_id"].is_string());
}

#[tokio::test]
async fn test_memory_search_returns_results() {
    use chrono::Utc;
    use sekha_controller::storage::repository::SearchResult;

    let mut mock_repo = MockConversationRepository::new();

    let test_results = vec![SearchResult {
        conversation_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
        score: 0.95,
        content: "Test content".to_string(),
        label: "Test".to_string(),
        folder: "/test".to_string(),
        timestamp: Utc::now().naive_utc(),
        metadata: json!({"test": true}),
    }];

    mock_repo
        .expect_semantic_search()
        .returning(move |_, _, _| Ok(test_results.clone()));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_search")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(
                    json!({
                        "query": "test search",
                        "limit": 10
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["total_results"], 1);
    assert!(json["data"]["results"].is_array());
}

#[tokio::test]
async fn test_memory_update_label_and_folder() {
    use chrono::Utc;
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();
    let test_id = Uuid::new_v4();

    let conv = Conversation {
        id: test_id,
        label: "Old Label".to_string(),
        folder: "/old".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    mock_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(conv.clone())));

    mock_repo.expect_update_label().returning(|_, _, _| Ok(()));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_update")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(
                    json!({
                        "conversation_id": test_id,
                        "label": "New Label",
                        "folder": "/new"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
}

#[tokio::test]
async fn test_memory_get_context_returns_conversation() {
    use chrono::Utc;
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();
    let test_id = Uuid::new_v4();

    let conv = Conversation {
        id: test_id,
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: 7,
        word_count: 500,
        session_count: 3,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    mock_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(conv.clone())));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_get_context")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(
                    json!({
                        "conversation_id": test_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["label"], "Test Conversation");
    assert_eq!(json["data"]["importance_score"], 7);
}

#[tokio::test]
async fn test_memory_export_with_messages() {
    use chrono::Utc;
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();
    let test_id = Uuid::new_v4();

    let conv = Conversation {
        id: test_id,
        label: "Export Test".to_string(),
        folder: "/export".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 200,
        session_count: 1,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    mock_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(conv.clone())));

    mock_repo
        .expect_get_message_list()
        .returning(|_| Ok(vec![json!({"role": "user", "content": "test"})]));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_export")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(
                    json!({
                        "conversation_id": test_id,
                        "format": "json",
                        "include_metadata": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["data"]["messages"].is_array());
    assert_eq!(json["data"]["format"], "json");
}

#[tokio::test]
async fn test_memory_stats_by_folder() {
    use chrono::Utc;
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();

    let convs = vec![
        Conversation {
            id: Uuid::new_v4(),
            label: "Test 1".to_string(),
            folder: "/work".to_string(),
            status: "active".to_string(),
            importance_score: 8,
            word_count: 100,
            session_count: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        },
        Conversation {
            id: Uuid::new_v4(),
            label: "Test 2".to_string(),
            folder: "/work".to_string(),
            status: "active".to_string(),
            importance_score: 6,
            word_count: 150,
            session_count: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        },
    ];

    mock_repo
        .expect_find_by_folder()
        .returning(move |_, _, _| Ok(convs.clone()));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_stats")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(
                    json!({
                        "folder": "/work"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["total_conversations"], 2);
    assert_eq!(json["data"]["average_importance"], 7.0);
}

#[tokio::test]
async fn test_memory_stats_global() {
    use chrono::Utc;
    use sekha_controller::models::internal::Conversation;

    let mut mock_repo = MockConversationRepository::new();

    mock_repo
        .expect_get_all_folders()
        .returning(|| Ok(vec!["/work".to_string(), "/personal".to_string()]));

    let convs = vec![Conversation {
        id: Uuid::new_v4(),
        label: "Test".to_string(),
        folder: "/work".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 100,
        session_count: 1,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    }];

    mock_repo
        .expect_find_with_filters()
        .returning(move |_, _, _| Ok((convs.clone(), 1)));

    let state = create_test_state(mock_repo).await;
    let api_key = state.config.read().await.mcp_api_key.clone();
    let app = create_mcp_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_stats")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["data"]["folders"].is_array());
}
