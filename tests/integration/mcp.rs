use super::{create_test_mcp_app, json, Uuid};
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;

// ============================================
// MCP Protocol Tests
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
// Authentication Tests
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
// Error Handling Tests
// ============================================

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
// Discovery Tests
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
                .header("Content-Type", "application/json")
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
                .header("Content-Type", "application/json")
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
                .header("Content-Type", "application/json")
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
                .header("Content-Type", "application/json")
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
                .header("Content-Type", "application/json")
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

}

// ============================================
// MCP Export & Stats Tests
// ============================================

#[tokio::test]
async fn test_mcp_memory_export_success() {
    let app = create_test_mcp_app().await;

    // Create a conversation
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{"label": "Export Test", "folder": "/export", "messages": [{"role": "user", "content": "Hello"}, {"role": "assistant", "content": "Hi there"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conversation_id = json["data"]["conversation_id"].as_str().unwrap();

    // Export it
    let export_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_export")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    format!(r#"{{"conversation_id": "{}", "format": "json"}}"#, conversation_id)
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(export_response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(export_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["messages"].is_array());
    assert_eq!(json["data"]["messages"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_mcp_memory_stats_global() {
    let app = create_test_mcp_app().await;

    // Create multiple conversations
    for i in 0..3 {
        app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp/tools/memory_store")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                    .body(Body::from(
                        format!(r#"{{"label": "Global Test {}", "folder": "/folder{}", "messages": [{{"role": "user", "content": "Test {}"}}]}}"#, i, i % 2, i)
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Get global stats
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
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
    assert_eq!(json["data"]["total_conversations"], 3);
    assert!(json["data"]["folders"].is_array());
    assert_eq!(json["data"]["folders"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_mcp_memory_stats_by_folder() {
    let app = create_test_mcp_app().await;

    // Create conversations in specific folder
    for i in 0..2 {
        app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp/tools/memory_store")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                    .body(Body::from(
                        format!(r#"{{"label": "Folder Test {}", "folder": "/work", "messages": [{{"role": "user", "content": "Work {}"}}]}}"#, i, i)
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Create one outside the folder
    app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_store")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(
                    r#"{"label": "Other Test", "folder": "/personal", "messages": [{"role": "user", "content": "Personal"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Get stats for /work folder only
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/tools/memory_stats")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_key_12345678901234567890123456789012")
                .body(Body::from(r#"{"folder": "/work"}"#))
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
    assert_eq!(json["data"]["total_conversations"], 2);
    assert_eq!(json["data"]["folders"], json!(["/work"]));
}