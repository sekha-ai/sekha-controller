use super::{create_test_services, Uuid};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sekha_controller::api::routes::create_router;
use tower::ServiceExt;

// Helper to create app for tests
async fn create_test_app() -> axum::Router {
    let state = create_test_services().await;
    create_router(state)
}

// ============================================
// REST API Tests
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
async fn test_api_update_conversation_folder_success() {
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
                    r#"{ "label": "Folder Test", "folder": "/old", "messages": [{"role": "user", "content": "Test"}] }"#,
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

    // Update folder
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/folder", conv_id))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "folder": "/new" }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_update_conversation_folder_not_found() {
    let app = create_test_app().await;

    let fake_id = Uuid::new_v4();
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/folder", fake_id))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "folder": "/new" }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_pin_conversation() {
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
                    r#"{ "label": "Pin Test", "folder": "/pin", "messages": [{"role": "user", "content": "Test"}] }"#,
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

    // Pin it
    let pin_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/pin", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(pin_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_archive_conversation() {
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
                    r#"{ "label": "Archive Test", "folder": "/archive", "messages": [{"role": "user", "content": "Test"}] }"#,
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

    // Archive it
    let archive_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/conversations/{}/archive", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(archive_response.status(), StatusCode::OK);
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

#[tokio::test]
async fn test_api_full_text_search() {
    let app = create_test_app().await;

    // Create conversation
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "FTS Test", "folder": "/fts", "messages": [{"role": "user", "content": "The quick brown fox jumps over the lazy dog"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Search using FTS
    let search_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search/fts")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "query": "fox", "limit": 10 }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_rebuild_embeddings() {
    let app = create_test_app().await;

    let response = app
        .clone()
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
async fn test_api_assemble_context() {
    let app = create_test_app().await;

    // Create conversation first
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Context Test", "folder": "/context", "messages": [{"role": "user", "content": "Test message for context"}] }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assemble context
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/context/assemble")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "query": "test", "context_budget": 1000 }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_generate_summary_daily() {
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
                    r#"{ "label": "Summary Test", "folder": "/summary", "messages": [{"role": "user", "content": "Test"}] }"#,
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
    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_generate_summary_weekly() {
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
                    r#"{ "label": "Summary Weekly", "folder": "/summary", "messages": [{"role": "user", "content": "Test"}] }"#,
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

    // Generate weekly summary
    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_generate_summary_monthly() {
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
                    r#"{ "label": "Summary Monthly", "folder": "/summary", "messages": [{"role": "user", "content": "Test"}] }"#,
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

    // Generate monthly summary
    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_generate_summary_invalid_level() {
    let app = create_test_app().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/summarize")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{ "conversation_id": "{}", "level": "invalid" }}"#,
                    fake_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_api_prune_dry_run() {
    let app = create_test_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/prune/dry-run")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "threshold_days": 30 }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_prune_execute() {
    let app = create_test_app().await;

    // Create conversation to prune
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{ "label": "Prune Test", "folder": "/prune", "messages": [{"role": "user", "content": "Test"}] }"#,
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

    // Execute prune
    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_suggest_labels() {
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
                    r#"{ "label": "Label Test", "folder": "/labels", "messages": [{"role": "user", "content": "Test"}] }"#,
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
    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================
// Error Handling Tests
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
// count_conversations() Coverage Tests
// ============================================

#[tokio::test]
async fn test_count_conversations_by_folder() {
    let app = create_test_app().await;

    // Create conversations in different folders
    for i in 0..3 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"label":"Folder Test {}","folder":"work","messages":[{{"role":"user","content":"Test"}}]}}"#,
                        i
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Create one in different folder
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"label":"Other","folder":"personal","messages":[{"role":"user","content":"Test"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Count by folder
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations/count?folder=work")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["count"], 3);
}

#[tokio::test]
async fn test_count_conversations_no_filters() {
    let app = create_test_app().await;

    // Create several conversations
    for i in 0..5 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"label":"Test {}","folder":"test","messages":[{{"role":"user","content":"Test"}}]}}"#,
                        i
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Count all - no filters
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations/count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["count"].as_u64().unwrap() >= 5);
}

#[tokio::test]
async fn test_count_conversations_by_label() {
    let app = create_test_app().await;

    // Create conversations with same label
    for _ in 0..2 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"label":"UniqueLabel","folder":"test","messages":[{"role":"user","content":"Test"}]}"#
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Count by label
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations/count?label=UniqueLabel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["count"], 2);
}

#[tokio::test]
async fn test_list_conversations_with_archived_filter() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations?archived=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_conversations_with_pinned_filter() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations?pinned=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_delete_conversation_covers_error_paths() {
    let app = create_test_app().await;

    let fake_id = "00000000-0000-0000-0000-000000000000";
    let response = app
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
async fn test_count_conversations_both_filters_error() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations/count?label=test&folder=work")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["count"], 0);
    assert_eq!(json["error"], "Cannot specify both label and folder");
}

#[tokio::test]
async fn test_list_conversations_archived_filter() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations?archived=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_conversations_pinned_filter() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/conversations?pinned=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
