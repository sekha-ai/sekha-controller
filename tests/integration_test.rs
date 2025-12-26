use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use sekha_controller::{
    api::routes::{create_router, AppState},
    config::Config,
    models::internal::{NewConversation, NewMessage},
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{
        chroma_client::ChromaClient, init_db, ConversationRepository, SeaOrmConversationRepository,
    },
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================
// Test Fixtures and Helpers
// ============================================

fn create_test_services() -> (Arc<ChromaClient>, Arc<EmbeddingService>) {
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    (chroma_client, embedding_service)
}

async fn create_test_config() -> Arc<RwLock<Config>> {
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

async fn create_test_app() -> Router {
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

async fn create_test_mcp_app() -> Router {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db.clone(),
        chroma_client.clone(),     // Add clone
        embedding_service.clone(), // Add clone
    ));

    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:11434".to_string()));

    let state = AppState {
        config: create_test_config().await,
        repo: repo.clone(),
        chroma_client,     // ADD THIS
        embedding_service, // ADD THIS
        orchestrator: Arc::new(sekha_controller::orchestrator::MemoryOrchestrator::new(
            repo, llm_bridge,
        )),
    };

    sekha_controller::api::mcp::create_mcp_router(state)
}

fn create_test_conversation() -> NewConversation {
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

// ============================================
// Module 4: Storage Layer Tests
// ============================================

#[tokio::test]
async fn test_repository_create_with_messages() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let conv = create_test_conversation();
    let result = repo.create_with_messages(conv).await;

    assert!(
        result.is_ok(),
        "Failed to create conversation with messages: {:?}",
        result
    );
}

#[tokio::test]
async fn test_repository_semantic_search() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create a conversation
    let conv = create_test_conversation();
    let conv_id = repo.create_with_messages(conv).await.unwrap();

    // Search for it
    let results = repo
        .semantic_search("test message", 10, None)
        .await
        .unwrap();

    assert!(!results.is_empty(), "Search should return results");
    assert_eq!(results[0].conversation_id, conv_id);
}

#[tokio::test]
async fn test_repository_delete_cascades() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create conversation
    let conv = create_test_conversation();
    let conv_id = repo.create_with_messages(conv).await.unwrap();

    // Verify it exists
    assert!(repo.find_by_id(conv_id).await.unwrap().is_some());

    // Delete it
    repo.delete(conv_id).await.unwrap();

    // Verify it's gone
    assert!(repo.find_by_id(conv_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_repository_count_by_label() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    // Create multiple conversations with same label
    for _i in 0..3 {
        let mut conv = create_test_conversation();
        conv.label = "count_test".to_string();
        conv.id = Some(Uuid::new_v4());
        repo.create_with_messages(conv).await.unwrap();
    }

    let count = repo.count_by_label("count_test").await.unwrap();
    assert_eq!(count, 3);
}

// ============================================
// Module 3: REST API Tests
// ============================================

#[tokio::test]
async fn test_api_create_conversation() {
    let app = create_test_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "API Test", "folder": "/api", "messages": [{"role": "user", "content": "Hello"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("API Test"));
    assert!(body_str.contains("conversation_id"));
}

#[tokio::test]
async fn test_api_get_conversation() {
    let app = create_test_app().await;

    // First create a conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Get Test", "folder": "/get", "messages": [{"role": "user", "content": "Test"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Now retrieve it
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/conversations/{}", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_update_conversation_label() {
    let app = create_test_app().await;

    // Create conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Original", "folder": "/original", "messages": [{"role": "user", "content": "Test"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Update label
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/label", conv_id))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Updated", "folder": "/updated" }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_delete_conversation() {
    let app = create_test_app().await;

    // Create conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Delete Test", "folder": "/delete", "messages": [{"role": "user", "content": "Test"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Delete it
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/conversations/{}", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // Verify it's gone
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/conversations/{}", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_count_conversations() {
    let app = create_test_app().await;

    // Create multiple conversations with same label
    for _i in 0..3 {
        app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{ "label": "count_test", "folder": "/count", "messages": [{"role": "user", "content": "Test"}] }"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Count them
    let count_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations/count?label=count_test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(count_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(count_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["count"], 3);
}

#[tokio::test]
async fn test_api_query_semantic_search() {
    let app = create_test_app().await;

    // Create a conversation with searchable content
    app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Search Test", "folder": "/search", "messages": [{"role": "user", "content": "What is the capital of France?"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Search for it
    let search_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "query": "capital France", "limit": 10 }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(search_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["results"].is_array());
}

// ============================================
// Module 6: MCP Protocol Tests
// ============================================

#[tokio::test]
async fn test_mcp_memory_store() {
    let app = create_test_mcp_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{ "label": "MCP Store", "folder": "/mcp", "messages": [{"role": "user", "content": "MCP test"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["conversation_id"].is_string());
}

#[tokio::test]
async fn test_mcp_memory_search() {
    let app = create_test_mcp_app().await;

    // First store a conversation
    app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{ "label": "MCP Search", "folder": "/mcp", "messages": [{"role": "user", "content": "Searchable content about Rust programming"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Now search for it
    let search_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_search")
                .header("Content-Type", "application/json")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(
                    r#"{ "query": "Rust programming", "limit": 10 }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(search_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["results"].is_array());
}

#[tokio::test]
async fn test_mcp_memory_update() {
    let app = create_test_mcp_app().await;

    // First store a conversation
    let store_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{ "label": "Original Label", "folder": "/original", "messages": [{"role": "user", "content": "Test"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(store_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["data"]["conversation_id"].as_str().unwrap();

    // Update the conversation
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_update")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "label": "Updated Label", "folder": "/updated" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(update_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["updated_fields"].is_array());
}

#[tokio::test]
async fn test_mcp_memory_get_context() {
    let app = create_test_mcp_app().await;

    // Store a conversation
    let store_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{ "label": "Context Test", "folder": "/context", "messages": [{"role": "user", "content": "Test context"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(store_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["data"]["conversation_id"].as_str().unwrap();

    // Get context
    let context_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_get_context")
                .header("Content-Type", "application/json")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(context_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(context_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["data"]["label"], "Context Test");
}

#[tokio::test]
async fn test_mcp_memory_prune() {
    let app = create_test_mcp_app().await;

    // Note: This test requires the orchestrator and LLM bridge to be fully functional
    // It may need to be marked as ignored in CI if LLM is not available
    let prune_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_prune")
                .header("Content-Type", "application/json")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(
                    r#"{ "threshold_days": 30, "importance_threshold": 5.0 }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should succeed even with empty database
    assert_eq!(prune_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(prune_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["suggestions"].is_array());
}

// ============================================
// Module 3: Authentication Tests
// ============================================

#[tokio::test]
async fn test_mcp_auth_failure() {
    let app = create_test_mcp_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer wrong_key")
                .body(Body::from(
                    r#"{ "label": "Auth Test", "folder": "/auth", "messages": [] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_missing() {
    let app = create_test_mcp_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                // No Authorization header
                .body(Body::from(
                    r#"{ "label": "Auth Test", "folder": "/auth", "messages": [] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================
// Module 3: Error Handling Tests
// ============================================

#[tokio::test]
async fn test_api_get_nonexistent_conversation() {
    let app = create_test_app().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .clone()
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
async fn test_api_delete_nonexistent_conversation() {
    let app = create_test_app().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .clone()
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
async fn test_mcp_update_nonexistent_conversation() {
    let app = create_test_mcp_app().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_update")
                .header("Content-Type", "application/json")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "label": "Updated" }}"#,
                    fake_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================
// Module 4: Storage Edge Cases
// ============================================

#[tokio::test]
async fn test_repository_empty_messages() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let mut conv = create_test_conversation();
    conv.messages = vec![]; // Empty messages

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_repository_very_long_message() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = SeaOrmConversationRepository::new(db, chroma_client, embedding_service);

    let mut conv = create_test_conversation();
    conv.messages = vec![NewMessage {
        role: "user".to_string(),
        content: "A".repeat(10000), // Very long message
        timestamp: chrono::Utc::now().naive_utc(),
        metadata: json!({}),
    }];

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_inserts() {
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let mut handles = vec![];

    // Spawn 10 concurrent inserts
    for i in 0..10 {
        let repo_clone = repo.clone();
        let handle = tokio::spawn(async move {
            let mut conv = create_test_conversation();
            conv.label = format!("Concurrent {}", i);
            conv.id = Some(Uuid::new_v4());
            repo_clone.create_with_messages(conv).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    // Verify all were created
    let count = repo.count_by_label("Concurrent").await.unwrap();
    assert_eq!(count, 10);
}

// ============================================
// Module 6: Discovery Tests
// ============================================

#[tokio::test]
async fn test_mcp_tools_discovery() {
    // This test verifies that all 6 MCP tools are registered correctly
    let app = create_test_mcp_app().await;

    // Try to call each tool - if router is misconfigured, this will fail

    // memory_store
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(
                    r#"{ "label": "Discovery", "folder": "/", "messages": [] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // memory_search
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_search")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(r#"{ "query": "test", "limit": 10 }"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // memory_update
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_update")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{ "conversation_id": "00000000-0000-0000-0000-000000000000", "label": "Test" }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND); // Should fail but route exists

    // memory_get_context
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_get_context")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(
                    r#"{ "conversation_id": "00000000-0000-0000-0000-000000000000" }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND); // Should fail but route exists

    // memory_prune
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_prune")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(r#"{ "threshold_days": 30 }"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // memory_query (deprecated but should still work)
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_query")
                .header(
                    "Authorization",
                    "Bearer test_key_12345678901234567890123456789012",
                )
                .body(Body::from(r#"{ "query": "test" }"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================
// Module 5: Memory Orchestration Tests
// ============================================

#[tokio::test]
async fn test_orchestrator_context_assembly() {
    let app = create_test_app().await;

    // Create a conversation first
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Context Test", "folder": "/test", "messages": [
                        {"role": "user", "content": "What is Rust programming language?"},
                        {"role": "assistant", "content": "Rust is a systems programming language focused on safety and performance."}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Test context assembly
    let assemble_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/context/assemble")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ 
                        "query": "Rust programming", 
                        "preferred_labels": ["Context Test"], 
                        "context_budget": 4000 
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(assemble_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(assemble_response.into_body(), 8192)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Should return array of messages
    assert!(json.is_array(), "Response should be an array of messages");
}

#[tokio::test]
async fn test_orchestrator_daily_summary() {
    let app = create_test_app().await;

    // Create a conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Summary Test", "folder": "/test", "messages": [
                        {"role": "user", "content": "Discuss the benefits of using Rust for systems programming"},
                        {"role": "assistant", "content": "Rust offers memory safety without garbage collection, zero-cost abstractions, and fearless concurrency"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Generate daily summary
    let summary_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "level": "daily" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(summary_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(summary_response.into_body(), 8192)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["level"], "daily");
    assert!(json["summary"].is_string());
    assert_eq!(json["conversation_id"], conv_id);
    assert!(json["generated_at"].is_string());
}

#[tokio::test]
async fn test_orchestrator_weekly_monthly_summaries() {
    let app = create_test_app().await;

    // Create a conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Multi-level Summary", "folder": "/test", "messages": [
                        {"role": "user", "content": "Test weekly summary"},
                        {"role": "assistant", "content": "This is a test response"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Test weekly summary (should fall back to daily if no daily summaries exist)
    let weekly_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "level": "weekly" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(weekly_response.status(), StatusCode::OK);

    // Test monthly summary (should fall back to weekly)
    let monthly_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "level": "monthly" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(monthly_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_orchestrator_invalid_summary_level() {
    let app = create_test_app().await;

    let conv_id = Uuid::new_v4();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "level": "invalid_level" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Invalid level"));
}

#[tokio::test]
async fn test_orchestrator_pruning_dry_run() {
    let app = create_test_app().await;

    // Create an old conversation (simulated by creation, then we'll test the endpoint)
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Old Conversation", "folder": "/archive", "messages": [
                        {"role": "user", "content": "This is old content that might be pruned"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Request pruning suggestions
    let prune_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/prune/dry-run")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "threshold_days": 90 }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(prune_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(prune_response.into_body(), 8192)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["suggestions"].is_array());
    assert!(json["total"].is_number());
}

#[tokio::test]
async fn test_orchestrator_pruning_execute() {
    let app = create_test_app().await;

    // Create a conversation to prune
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "To Be Pruned", "folder": "/prune", "messages": [
                        {"role": "user", "content": "This will be archived"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Execute pruning
    let execute_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/prune/execute")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_ids": ["{}"] }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(execute_response.status(), StatusCode::OK);

    // Verify conversation was archived (not deleted)
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/conversations/{}", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(get_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "archived");
}

#[tokio::test]
async fn test_orchestrator_label_suggestions() {
    let app = create_test_app().await;

    // Create a conversation with content suitable for labeling
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Unlabeled", "folder": "/", "messages": [
                        {"role": "user", "content": "I need help with Rust async programming and tokio runtime"},
                        {"role": "assistant", "content": "Let's discuss Rust async features and the tokio ecosystem"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Get label suggestions
    let suggest_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labels/suggest")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(suggest_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(suggest_response.into_body(), 8192)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["conversation_id"], conv_id);
    assert!(json["suggestions"].is_array());

    // Verify suggestion structure
    if let Some(suggestions) = json["suggestions"].as_array() {
        if !suggestions.is_empty() {
            let first = &suggestions[0];
            assert!(first["label"].is_string());
            assert!(first["confidence"].is_number());
            assert!(first["is_existing"].is_boolean());
            assert!(first["reason"].is_string());
        }
    }
}

#[tokio::test]
async fn test_orchestrator_label_suggest_empty_conversation() {
    let app = create_test_app().await;

    // Create conversation with no messages
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Empty", "folder": "/", "messages": [] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();

    // Should still succeed but return empty suggestions
    let suggest_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labels/suggest")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}" }}"#,
                    conv_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(suggest_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_orchestrator_context_assembly_large_budget() {
    let app = create_test_app().await;

    // Create multiple conversations
    for i in 0..5 {
        app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{ "label": "Test {}", "folder": "/test", "messages": [
                            {{"role": "user", "content": "Message {} about testing context assembly"}},
                            {{"role": "assistant", "content": "Response {} with relevant context"}}
                        ]}}"#,
                        i, i, i
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Test with large context budget
    let assemble_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/context/assemble")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ 
                        "query": "testing context", 
                        "preferred_labels": [], 
                        "context_budget": 16000 
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(assemble_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(assemble_response.into_body(), 65536)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Should return multiple messages within budget
    assert!(json.is_array());
}

#[tokio::test]
async fn test_orchestrator_pruning_with_different_thresholds() {
    let app = create_test_app().await;

    // Test with different threshold values
    let thresholds = vec![30, 60, 90, 180];

    for threshold in thresholds {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/prune/dry-run")
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{ "threshold_days": {} }}"#,
                        threshold
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_orchestrator_error_handling_nonexistent_conversation() {
    let app = create_test_app().await;
    let fake_id = Uuid::new_v4();

    // Test summarize with nonexistent conversation
    let summary_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "level": "daily" }}"#,
                    fake_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error (500 or 404 depending on implementation)
    assert!(
        summary_response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || summary_response.status() == StatusCode::NOT_FOUND
    );

    // Test label suggest with nonexistent conversation
    let label_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labels/suggest")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}" }}"#,
                    fake_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        label_response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || label_response.status() == StatusCode::NOT_FOUND
    );
}

// ============================================
// Module 6: Serialization Tests
// ============================================

#[tokio::test]
async fn test_json_serialization_edge_cases() {
    let app = create_test_app().await;

    // Test with special characters
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Special \"Chars\" \n \t \\ Test", "folder": "/", "messages": [{"role": "user", "content": "Line1\nLine2\tTabbed"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

// ============================================
// Module 6: File Watcher Tests
// ============================================

#[tokio::test]
async fn test_file_watcher_chatgpt_import() {
    use sekha_controller::services::file_watcher::ImportProcessor;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let import_file = temp_dir.path().join("chatgpt_export.json");

    // Create sample ChatGPT export
    let chatgpt_json = r#"{
        "title": "Test Import",
        "create_time": 1703073600.0,
        "update_time": 1703073600.0,
        "mapping": {
            "root": {
                "id": "root",
                "message": null,
                "parent": null,
                "children": ["msg1"]
            },
            "msg1": {
                "id": "msg1",
                "message": {
                    "id": "msg1",
                    "author": {"role": "user"},
                    "create_time": 1703073600.0,
                    "content": {
                        "content_type": "text",
                        "parts": ["What is Rust?"]
                    }
                },
                "parent": "root",
                "children": ["msg2"]
            },
            "msg2": {
                "id": "msg2",
                "message": {
                    "id": "msg2",
                    "author": {"role": "assistant"},
                    "create_time": 1703073660.0,
                    "content": {
                        "content_type": "text",
                        "parts": ["Rust is a systems programming language."]
                    }
                },
                "parent": "msg1",
                "children": []
            }
        }
    }"#;

    tokio::fs::write(&import_file, chatgpt_json).await.unwrap();

    // Create repository and processor
    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let processor = ImportProcessor::new(repo.clone());

    // Process file
    let result = processor.process_file(&import_file).await;
    assert!(
        result.is_ok(),
        "Failed to process ChatGPT export: {:?}",
        result
    );

    // Verify import
    let count = repo.count_by_label("Test Import").await.unwrap();
    assert_eq!(count, 1, "Should have imported 1 conversation");
}

#[tokio::test]
async fn test_file_watcher_multiple_conversations() {
    use sekha_controller::services::file_watcher::ImportProcessor;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let import_file = temp_dir.path().join("multi_export.json");

    // Array of ChatGPT conversations
    let chatgpt_json = r#"[
        {
            "title": "Conversation 1",
            "create_time": 1703073600.0,
            "update_time": 1703073600.0,
            "mapping": {
                "root": {"id": "root", "message": null, "parent": null, "children": ["msg1"]},
                "msg1": {
                    "id": "msg1",
                    "message": {
                        "id": "msg1",
                        "author": {"role": "user"},
                        "create_time": 1703073600.0,
                        "content": {"content_type": "text", "parts": ["First conversation"]}
                    },
                    "parent": "root",
                    "children": []
                }
            }
        },
        {
            "title": "Conversation 2",
            "create_time": 1703073700.0,
            "update_time": 1703073700.0,
            "mapping": {
                "root": {"id": "root", "message": null, "parent": null, "children": ["msg1"]},
                "msg1": {
                    "id": "msg1",
                    "message": {
                        "id": "msg1",
                        "author": {"role": "user"},
                        "create_time": 1703073700.0,
                        "content": {"content_type": "text", "parts": ["Second conversation"]}
                    },
                    "parent": "root",
                    "children": []
                }
            }
        }
    ]"#;

    tokio::fs::write(&import_file, chatgpt_json).await.unwrap();

    let db = init_db("sqlite::memory:").await.unwrap();
    let (chroma_client, embedding_service) = create_test_services();
    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service,
    ));

    let processor = ImportProcessor::new(repo.clone());
    processor.process_file(&import_file).await.unwrap();

    // Verify both imports
    let count1 = repo.count_by_label("Conversation 1").await.unwrap();
    let count2 = repo.count_by_label("Conversation 2").await.unwrap();

    assert_eq!(count1, 1);
    assert_eq!(count2, 1);
}
