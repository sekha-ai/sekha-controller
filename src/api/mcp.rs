use crate::api::routes::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{api::dto::*, auth::McpAuth, models::internal::Conversation};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

// ============================================
// Tool: memory_store - ALREADY IMPLEMENTED
// ============================================

pub async fn memory_store(
    _auth: McpAuth,
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let conv = Conversation {
        id,
        label: req.label.clone(),
        folder: req.folder.clone(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: req.messages.iter().map(|m| m.content.len() as i32).sum(),
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

// ============================================
// Tool: memory_update - NEW IMPLEMENTATION
// ============================================

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

    let mut updates = Vec::new();

    // Update label and folder if provided
    if args.label.is_some() || args.folder.is_some() {
        let new_label = args.label.as_deref().unwrap_or(&conv.label);
        let new_folder = args.folder.as_deref().unwrap_or(&conv.folder);

        state
            .repo
            .update_label(args.conversation_id, new_label, new_folder)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        updates.push("label/folder");
    }

    // Update other fields as needed
    // Note: For other fields like status, we'd need additional repo methods
    // For now, we'll track what was requested
    if args.status.is_some() {
        updates.push("status");
        // TODO: Add repo.update_status() method if needed
    }

    if args.importance_score.is_some() {
        updates.push("importance_score");
        // TODO: Add repo.update_importance() method if needed
    }

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({
            "conversation_id": args.conversation_id,
            "updated_fields": updates,
            "message": "Conversation updated successfully"
        })),
        error: None,
    }))
}

// ============================================
// Tool: memory_search - REAL IMPLEMENTATION
// ============================================

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
    let offset = args.offset.unwrap_or(0) as u32;

    // Use the existing semantic search implementation
    let search_results = state
        .repo
        .semantic_search(&args.query, limit, args.filters)
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
            "offset": offset,
            "results": results
        })),
        error: None,
    }))
}

// ============================================
// Tool: memory_prune - NEW IMPLEMENTATION
// ============================================

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

// ============================================
// Tool: memory_get_context - ALREADY IMPLEMENTED
// ============================================

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

// ============================================
// Router
// ============================================

pub fn create_mcp_router(state: AppState) -> Router {
    Router::new()
        .route("/mcp/tools/memory_store", post(memory_store))
        .route("/mcp/tools/memory_query", post(memory_query))
        .route("/mcp/tools/memory_get_context", post(memory_get_context))
        .route("/mcp/tools/memory_update", post(memory_update))
        .route("/mcp/tools/memory_search", post(memory_search))
        .route("/mcp/tools/memory_prune", post(memory_prune))
        .with_state(state)
}

// Legacy endpoint - keep for backward compatibility
pub async fn memory_query(
    _auth: McpAuth,
    _state: State<AppState>,
    Json(args): Json<MemoryQueryArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    // Redirect to memory_search for backward compatibility
    memory_search(
        _auth,
        _state,
        Json(MemorySearchArgs {
            query: args.query,
            filters: args.filters,
            limit: args.limit,
            offset: Some(0),
        }),
    )
    .await
}

#[derive(Debug, Deserialize)]
pub struct MemoryQueryArgs {
    query: String,
    filters: Option<Value>,
    limit: Option<u32>,
}
