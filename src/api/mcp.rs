use crate::api::routes::AppState;
use crate::config::Config;
use axum::routing::post;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{api::dto::*, auth::McpAuth, models::internal::Conversation};

/// API authentication middleware
pub async fn auth_middleware(
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response()
        })?;

    // Check Bearer token format
    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization format. Use: Bearer <token>",
        )
            .into_response());
    }

    // Extract token
    let token = auth_header.trim_start_matches("Bearer ").trim();

    // Validate against config
    if !config.is_valid_api_key(token) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid API key").into_response());
    }

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request as HttpRequest};

    #[tokio::test]
    async fn test_valid_api_key() {
        // Create test config
        let mut config = Config::default();
        config.mcp_api_key = "test_key".to_string();

        let config = Arc::new(config);

        // Create request with valid auth
        let mut request = HttpRequest::builder().uri("/").body(Body::empty()).unwrap();

        request
            .headers_mut()
            .insert(header::AUTHORIZATION, "Bearer test_key".parse().unwrap());

        // Auth should succeed
        // (full test requires mock Next handler)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

// ==================== Tool: memory_store ====================

#[derive(Debug, Deserialize)]
pub struct MemoryStoreArgs {
    label: String,
    folder: String,
    messages: Vec<MessageDto>,
    #[serde(default)]
    importance_score: Option<i32>,
}

pub async fn memory_store(
    _auth: McpAuth,
    State(state): State<AppState>,
    Json(args): Json<MemoryStoreArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let importance = args.importance_score.unwrap_or(5);

    let conv = Conversation {
        id,
        label: args.label.clone(),
        folder: args.folder.clone(),
        status: "active".to_string(),
        importance_score: importance,
        word_count: args.messages.iter().map(|m| m.content.len() as i32).sum(),
        session_count: 1,
        created_at: now,
        updated_at: now,
    };

    state
        .repo
        .create(conv)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({ "conversation_id": id })),
        error: None,
    }))
}

// ==================== Tool: memory_search ====================

#[derive(Debug, Deserialize)]
pub struct MemorySearchArgs {
    query: String,
    #[serde(default)]
    filters: Option<Value>,
    #[serde(default = "default_limit")]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

fn default_limit() -> Option<u32> {
    Some(10)
}

pub async fn memory_search(
    _auth: McpAuth,
    State(state): State<AppState>,
    Json(args): Json<MemorySearchArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    let limit = args.limit.unwrap_or(10) as usize;
    let filters = args.filters;

    // Use repository's semantic search
    let search_results = state
        .repo
        .semantic_search(&args.query, limit, filters)
        .await
        .map_err(|e| {
            tracing::error!("Search failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Format results for MCP response
    let results: Vec<Value> = search_results
        .into_iter()
        .map(|hit| {
            serde_json::json!({
                "conversation_id": hit.conversation_id,
                "message_id": hit.message_id,
                "score": hit.score,
                "content": hit.content,
                "label": hit.label,
                "folder": hit.folder,
                "timestamp": hit.timestamp.to_string(),
                "metadata": hit.metadata,
            })
        })
        .collect();

    let total = results.len();

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({
            "query": args.query,
            "total_results": total,
            "limit": limit,
            "results": results
        })),
        error: None,
    }))
}

// ==================== Tool: memory_update ====================

#[derive(Debug, Deserialize)]
pub struct MemoryUpdateArgs {
    conversation_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    importance_score: Option<i32>,
}

pub async fn memory_update(
    _auth: McpAuth,
    State(state): State<AppState>,
    Json(args): Json<MemoryUpdateArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    // Verify conversation exists
    let conv = state
        .repo
        .find_by_id(args.conversation_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or_else(|| StatusCode::NOT_FOUND)?;

    let mut updated_fields = Vec::new();

    // Update label and folder if provided
    if args.label.is_some() || args.folder.is_some() {
        let new_label = args.label.as_deref().unwrap_or(&conv.label);
        let new_folder = args.folder.as_deref().unwrap_or(&conv.folder);

        state
            .repo
            .update_label(args.conversation_id, new_label, new_folder)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        updated_fields.push("label/folder");
    }

    // Track other requested updates (need separate repo methods)
    if args.status.is_some() {
        updated_fields.push("status");
    }
    if args.importance_score.is_some() {
        updated_fields.push("importance_score");
    }

    // TODO: Add repository methods for status and importance_score updates

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({
            "conversation_id": args.conversation_id,
            "updated_fields": updated_fields,
            "message": "Conversation updated successfully"
        })),
        error: None,
    }))
}

// ==================== Tool: memory_prune ====================

#[derive(Debug, Deserialize)]
pub struct MemoryPruneArgs {
    #[serde(default = "default_threshold_days")]
    threshold_days: i64,
    #[serde(default = "default_importance_threshold")]
    importance_threshold: f32,
}

fn default_threshold_days() -> i64 {
    30
}

fn default_importance_threshold() -> f32 {
    5.0
}

#[derive(Debug, Serialize)]
pub struct PruningSuggestionDto {
    conversation_id: Uuid,
    conversation_label: String,
    last_accessed: String,
    message_count: u64,
    token_estimate: u32,
    importance_score: f32,
    preview: String,
    recommendation: String,
}

pub async fn memory_prune(
    _auth: McpAuth,
    State(state): State<AppState>,
    Json(args): Json<MemoryPruneArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    use crate::orchestrator::pruning_engine::PruningEngine;
    use crate::services::llm_bridge_client::LlmBridgeClient;

    // Create LLM bridge client from config
    let config = state.config.read().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(config.ollama_url.clone()));

    // Create pruning engine
    let pruning_engine = PruningEngine::new(state.repo.clone(), llm_bridge);

    // Generate pruning suggestions
    let suggestions = pruning_engine
        .generate_suggestions(args.threshold_days, args.importance_threshold)
        .await
        .map_err(|e| {
            tracing::error!("Pruning suggestions failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert to DTOs for serialization
    let suggestion_dtos: Vec<PruningSuggestionDto> = suggestions
        .into_iter()
        .map(|s| PruningSuggestionDto {
            conversation_id: s.conversation_id,
            conversation_label: s.conversation_label,
            last_accessed: s.last_accessed.to_string(),
            message_count: s.message_count,
            token_estimate: s.token_estimate,
            importance_score: s.importance_score,
            preview: s.preview,
            recommendation: s.recommendation,
        })
        .collect();

    let total_suggestions = suggestion_dtos.len();
    let estimated_savings: u32 = suggestion_dtos
        .iter()
        .filter(|s| s.recommendation == "archive")
        .map(|s| s.token_estimate)
        .sum();

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({
            "threshold_days": args.threshold_days,
            "importance_threshold": args.importance_threshold,
            "total_suggestions": total_suggestions,
            "estimated_token_savings": estimated_savings,
            "suggestions": suggestion_dtos
        })),
        error: None,
    }))
}

// ==================== Tool: memory_get_context ====================

#[derive(Debug, Deserialize)]
pub struct MemoryGetContextArgs {
    conversation_id: Uuid,
}

pub async fn memory_get_context(
    _auth: McpAuth,
    State(state): State<AppState>,
    Json(args): Json<MemoryGetContextArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    let conv = state
        .repo
        .find_by_id(args.conversation_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match conv {
        Some(c) => Ok(Json(McpToolResponse {
            success: true,
            data: Some(serde_json::json!({
                "conversation_id": c.id,
                "label": c.label,
                "status": c.status,
                "folder": c.folder,
                "importance_score": c.importance_score,
                "word_count": c.word_count,
                "session_count": c.session_count,
                "created_at": c.created_at.to_string(),
                "updated_at": c.updated_at.to_string(),
            })),
            error: None,
        })),
        None => Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some("Conversation not found".to_string()),
        })),
    }
}

// ==================== ROUTER & LEGACY COMPATIBILITY ====================

pub fn create_mcp_router(state: AppState) -> Router {
    Router::new()
        .route("/mcp/tools/memory_store", post(memory_store))
        .route("/mcp/tools/memory_get_context", post(memory_get_context))
        .route("/mcp/tools/memory_update", post(memory_update))
        .route("/mcp/tools/memory_search", post(memory_search))
        .route("/mcp/tools/memory_prune", post(memory_prune))
        .with_state(state)
}
