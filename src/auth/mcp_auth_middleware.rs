use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;

/// Middleware to validate MCP API key from Bearer token
/// Used for /mcp/tools/* endpoints called by sekha-mcp server
pub async fn mcp_auth_middleware(
    State(config): State<Arc<RwLock<Config>>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Missing authorization header"
                })),
            )
                .into_response()
        })?;

    // Extract Bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Invalid authorization format. Use: Bearer <token>"
            })),
        )
            .into_response()
    })?;

    // Validate token against config
    let config_guard = config.read().await;
    let expected_key = &config_guard.mcp_api_key;

    if token != expected_key || token.len() < 32 {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "error": "Invalid API key"
            })),
        )
            .into_response());
    }

    drop(config_guard);

    // Authentication successful - proceed to handler
    Ok(next.run(request).await)
}
