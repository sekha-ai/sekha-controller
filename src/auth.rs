use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::Config;

pub struct McpAuth;

#[async_trait]
impl<S> FromRequestParts<S> for McpAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get auth header
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

        // Extract Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                let body = Json(json!({
                    "error": "Invalid authorization format"
                }));
                (StatusCode::BAD_REQUEST, body).into_response()
            })?;

        // Get config from extensions
        let config = parts
            .extensions
            .get::<Arc<RwLock<Config>>>()
            .ok_or_else(|| {
                let body = Json(json!({
                    "error": "Config not found"
                }));
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            })?;

        let expected_key = config.read().await.mcp_api_key.clone();
        
        if token == expected_key && token.len() >= 32 {
            Ok(McpAuth)
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Invalid API key" })),
            ).into_response())
        }
    }
}
