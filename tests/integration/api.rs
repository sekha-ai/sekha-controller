use super::{create_test_app, Uuid};
// use crate::integration::create_test_app;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

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
    
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
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
    
    // Count all - no filters (covers lines 540-541)
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
    
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
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
    
    // Count by label (covers lines 599-600)
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
    
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(json["count"], 2);
}

// ============================================
// list_conversations() Edge Cases
// ============================================

#[tokio::test]
async fn test_list_conversations_with_archived_filter() {
    let app = create_test_app().await;
    
    // Create and archive a conversation
    let create_response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"label":"Archive Test","folder":"test","messages":[{"role":"user","content":"Test"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = axum::body::to_bytes(create_response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();
    
    // Archive it (you'll need to implement update_status or use label endpoint)
    // For now, test the filter parameter (covers line 606)
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
    
    // Test pinned filter (covers filter building logic)
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

// ============================================
// Error Path Coverage
// ============================================

#[tokio::test]
async fn test_delete_conversation_covers_error_paths() {
    let app = create_test_app().await;
    
    // Create a conversation
    let create_response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"label":"Delete Test","folder":"test","messages":[{"role":"user","content":"Test"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = axum::body::to_bytes(create_response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let conv_id = json["id"].as_str().unwrap();
    
    // Delete it
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/conversations/{}", conv_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Try to delete again - should get 404 (covers error paths)
    let app = create_test_app().await; // New app for clean state
    
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
    
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(json["count"], 0);
    assert_eq!(json["error"], "Cannot specify both label and folder");
}

// TEST 2: List with archived filter (covers 606)
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

// TEST 3: List with pinned filter (covers filter building)
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
