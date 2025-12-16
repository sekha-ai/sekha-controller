use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::future::Future;

use crate::api::routes::AppState;

pub struct McpAuth;

impl FromRequestParts<AppState> for McpAuth {
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        // Clone what we need BEFORE async block
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let config = state.config.clone();

        async move {
            let auth_header = auth_header.ok_or_else(|| {
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

            let expected_key = config.read().await.mcp_api_key.clone();

            if token == expected_key && token.len() >= 32 {
                Ok(McpAuth)
            } else {
                Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "Invalid API key" })),
                )
                    .into_response())
            }
        }
    }
}
