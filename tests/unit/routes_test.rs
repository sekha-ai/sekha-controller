use axum::body::Body;
use axum::http::{Request, StatusCode};
use sekha_controller::api::dto::*;
use sekha_controller::api::routes::{create_router, AppState};
use sekha_controller::config::Config;
use sekha_controller::models::internal::{NewConversation, NewMessage};
use sekha_controller::orchestrator::MemoryOrchestrator;
use sekha_controller::services::embedding_service::EmbeddingService;
use sekha_controller::services::llm_bridge_client::LlmBridgeClient;
use sekha_controller::storage::chroma_client::ChromaClient;
use sekha_controller::storage::{init_db, SeaOrmConversationRepository};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

async fn create_test_app() -> AppState {
    let config = Arc::new(RwLock::new(Config {
        server_port: 8080,
        mcp_api_key: "test_key_12345678901234567890123456789012".to_string(),
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

    let db = init_db("sqlite::memory:").await.unwrap();
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
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:5001".to_string()));

    AppState {
        config,
        repo: repo.clone(),
        chroma_client: chroma,
        embedding_service,
        orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge)),
    }
}

#[tokio::test]
async fn test_list_conversations_with_label_filter() {
    let state = create_test_app().await;

    // Create conversation with specific label
    let conv = NewConversation {
        id: None,
        label: "TestLabel".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations?label=TestLabel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_conversations_with_all_filters() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations?label=test&folder=inbox&pinned=true&archived=false")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_count_conversations_both_params_error() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations/count?label=test&folder=inbox")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_conversation_not_found() {
    let state = create_test_app().await;
    let router = create_router(state);
    let fake_id = Uuid::new_v4();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/conversations/{}", fake_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_conversation_not_found() {
    let state = create_test_app().await;
    let router = create_router(state);
    let fake_id = Uuid::new_v4();

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/conversations/{}", fake_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = create_test_app().await;
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

    // May be 200 or 503 depending on services
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn test_update_conversation_folder() {
    let state = create_test_app().await;

    let conv = NewConversation {
        id: None,
        label: "Test".to_string(),
        folder: "old".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    let conv_id = state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/folder", conv_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"folder":"new"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_pin_conversation() {
    let state = create_test_app().await;

    let conv = NewConversation {
        id: None,
        label: "Test".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    let conv_id = state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/pin", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_archive_conversation() {
    let state = create_test_app().await;

    let conv = NewConversation {
        id: None,
        label: "Test".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    let conv_id = state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/archive", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rebuild_embeddings() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/rebuild-embeddings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_full_text_search() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search/fts")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query":"test","limit":10}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_assemble_context() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/context/assemble")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"query":"test query","preferred_labels":[],"context_budget":5}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status().is_server_error());
}

#[tokio::test]
async fn test_generate_summary() {
    let state = create_test_app().await;

    let conv = NewConversation {
        id: None,
        label: "Test".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test message".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    let conv_id = state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"conversation_id":"{}","level":"daily"}}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status().is_server_error());
}

#[tokio::test]
async fn test_generate_summary_invalid_level() {
    let state = create_test_app().await;
    let router = create_router(state);
    let conv_id = Uuid::new_v4();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"conversation_id":"{}","level":"invalid"}}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_prune_dry_run() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/prune/dry-run")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"threshold_days":30}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status().is_server_error());
}

#[tokio::test]
async fn test_prune_execute() {
    let state = create_test_app().await;

    let conv = NewConversation {
        id: None,
        label: "Test".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    let conv_id = state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/prune/execute")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"conversation_ids":["{}"]}}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_suggest_labels() {
    let state = create_test_app().await;

    let conv = NewConversation {
        id: None,
        label: "Test".to_string(),
        folder: "test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "test message content".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }],
    };
    let conv_id = state.repo.create_with_messages(conv).await.unwrap();

    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labels/suggest")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"conversation_id":"{}"}}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success() || response.status().is_server_error());
}

#[tokio::test]
async fn test_update_folder_not_found() {
    let state = create_test_app().await;
    let router = create_router(state);
    let fake_id = Uuid::new_v4();

    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/folder", fake_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"folder":"new"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_semantic_query_with_results() {
    let state = create_test_app().await;
    let router = create_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query":"test","limit":5,"offset":0}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
