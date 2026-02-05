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
}

#[tokio::test]
async fn test_api_health_check() {
    let app = create_test_app().await;

    let response = app
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
