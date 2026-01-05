use super::{create_test_app, is_llm_bridge_running, Uuid};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

// ============================================
// Memory Orchestration Tests
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
    if !is_llm_bridge_running().await {
        eprintln!("⚠️  Skipping test_orchestrator_daily_summary - LLM bridge not running");
        return;
    }
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
    if !is_llm_bridge_running().await {
        eprintln!(
            "⚠️  Skipping test_orchestrator_weekly_monthly_summaries - LLM bridge not running"
        );
        return;
    }
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
    if !is_llm_bridge_running().await {
        eprintln!("⚠️  Skipping test_orchestrator_label_suggestions - LLM bridge not running");
        return;
    }
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
