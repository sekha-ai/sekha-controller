use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::routes::AppState;
use crate::config::Config;

// Change from unit struct to holding validated token
#[derive(Clone)]
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
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Invalid API key" })),
            )
                .into_response())
        }
    }
}
