use axum::{
    body::Body,
    extract::{FromRef, FromRequestParts, Request, State},
    http::{request::Parts, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::routes::AppState;
use crate::config::Config;

// ==================== EXTRACTOR (kept for reference, not currently used) ====================

// Change from unit struct to holding validated token
#[derive(Clone, Debug)]
pub struct McpAuth {
    pub token: String,
}

// Implement FromRef to allow AppState to be extracted from router state
impl FromRef<AppState> for Arc<RwLock<Config>> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

// Correct Axum 0.8 implementation
impl FromRequestParts<AppState> for McpAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                let body = Json(json!({
                    "error": "Missing authorization header"
                }));
                (StatusCode::UNAUTHORIZED, body).into_response()
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            let body = Json(json!({
                "error": "Invalid authorization format"
            }));
            (StatusCode::BAD_REQUEST, body).into_response()
        })?;

        // Get config through the state
        let expected_key = state.config.read().await.mcp_api_key.clone();

        if token == expected_key && token.len() >= 32 {
            Ok(McpAuth {
                token: token.to_string(),
            })
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid API key" })),
            )
                .into_response())
        }
    }
}

// ==================== MIDDLEWARE (actively used for /mcp/tools/*) ====================

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
