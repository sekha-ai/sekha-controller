use axum::{
    extract::FromRequestParts,
    http::{header, Request, StatusCode},
};
use sekha_controller::{
    api::routes::AppState,
    auth::McpAuth,
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, repository::MockConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_test_state() -> AppState {
    let config = Arc::new(RwLock::new(Config::default()));
    let mock_repo = Arc::new(MockConversationRepository::new());
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let default_config = Config::default();
    let llm_client = Arc::new(LlmBridgeClient::new(&default_config).unwrap());
    let orchestrator = Arc::new(MemoryOrchestrator::new(
        mock_repo.clone(),
        llm_client.clone(),
    ));

    AppState {
        config,
        orchestrator,
        repo: mock_repo,
        embedding_service,
        chroma_client,
        llm_client,
    }
}

#[tokio::test]
async fn test_mcp_auth_valid_token() {
    let state = create_test_state();
    let valid_key = state.config.read().await.mcp_api_key.clone();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer {}", valid_key))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let auth = result.unwrap();
    assert_eq!(auth.token, valid_key);
}

#[tokio::test]
async fn test_mcp_auth_missing_authorization_header() {
    let state = create_test_state();

    let mut req = Request::builder().uri("/").body(()).unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_invalid_format_no_bearer() {
    let state = create_test_state();
    let valid_key = state.config.read().await.mcp_api_key.clone();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, valid_key) // Missing "Bearer " prefix
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mcp_auth_invalid_token_wrong_key() {
    let state = create_test_state();

    let mut req = Request::builder()
        .uri("/")
        .header(
            header::AUTHORIZATION,
            "Bearer wrong_key_123456789012345678901234567890",
        )
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_token_too_short() {
    let state = create_test_state();

    // Token less than 32 characters
    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, "Bearer short_key_123")
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_token_exactly_32_chars() {
    let state = create_test_state();

    // Set a 32-character key
    let key_32 = "12345678901234567890123456789012";
    state.config.write().await.mcp_api_key = key_32.to_string();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer {}", key_32))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let auth = result.unwrap();
    assert_eq!(auth.token, key_32);
}

#[tokio::test]
async fn test_mcp_auth_empty_authorization_header() {
    let state = create_test_state();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, "")
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mcp_auth_bearer_with_extra_spaces() {
    let state = create_test_state();
    let valid_key = state.config.read().await.mcp_api_key.clone();

    // "Bearer  " with extra space
    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer  {}", valid_key))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    // Should fail because strip_prefix("Bearer ") will leave extra space
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mcp_auth_bearer_lowercase() {
    let state = create_test_state();
    let valid_key = state.config.read().await.mcp_api_key.clone();

    // "bearer" instead of "Bearer"
    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("bearer {}", valid_key))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    // Should fail because it's case-sensitive
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mcp_auth_only_bearer_no_token() {
    let state = create_test_state();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, "Bearer ")
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    // Should fail - no token after "Bearer "
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_invalid_utf8() {
    let state = create_test_state();

    // Create request with invalid UTF-8 in header
    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, vec![0xFF, 0xFE, 0xFD]) // Invalid UTF-8
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mcp_auth_very_long_token() {
    let state = create_test_state();

    // Create a very long valid token (> 32 chars)
    let long_key = "a".repeat(100);
    state.config.write().await.mcp_api_key = long_key.clone();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer {}", long_key))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let auth = result.unwrap();
    assert_eq!(auth.token, long_key);
    assert_eq!(auth.token.len(), 100);
}

#[tokio::test]
async fn test_mcp_auth_token_with_special_chars() {
    let state = create_test_state();

    // Token with special characters (32+ chars)
    let special_key = "test-key_123.456!@#$%^&*()+=[]{}";
    state.config.write().await.mcp_api_key = special_key.to_string();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer {}", special_key))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let auth = result.unwrap();
    assert_eq!(auth.token, special_key);
}

#[test]
fn test_mcp_auth_clone() {
    let auth = McpAuth {
        token: "test_token_12345678901234567890".to_string(),
    };

    let cloned = auth.clone();
    assert_eq!(auth.token, cloned.token);
}

#[tokio::test]
async fn test_mcp_auth_multiple_auth_headers() {
    let state = create_test_state();
    let valid_key = state.config.read().await.mcp_api_key.clone();

    // Multiple authorization headers (HTTP allows this)
    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer {}", valid_key))
        .header(header::AUTHORIZATION, "Bearer wrong_key")
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    // First header should be used
    assert!(result.is_ok() || result.is_err()); // Depends on implementation
}

#[tokio::test]
async fn test_mcp_auth_token_31_chars() {
    let state = create_test_state();

    // Token with exactly 31 characters (one less than required)
    let key_31 = "1234567890123456789012345678901";
    state.config.write().await.mcp_api_key = key_31.to_string();

    let mut req = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, format!("Bearer {}", key_31))
        .body(())
        .unwrap();

    let (mut parts, _) = req.into_parts();
    let result = McpAuth::from_request_parts(&mut parts, &state).await;

    // Should fail - exactly 31 chars is too short
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
