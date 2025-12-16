use crate::api::routes::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{api::dto::*, auth::McpAuth, models::internal::Conversation};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

// Tool: memory_store
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

// Tool: memory_query
#[derive(Debug, Deserialize)]
pub struct MemoryQueryArgs {
    query: String,
    filters: Option<Value>,
    limit: Option<u32>,
}

pub async fn memory_query(
    _auth: McpAuth,
    _state: State<AppState>,
    Json(args): Json<MemoryQueryArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    // Mock results until Module 5
    let results = vec![serde_json::json!({
        "conversation_id": Uuid::new_v4(),
        "relevance_score": 0.85,
        "summary": "Mock result for: ".to_string() + &args.query,
    })];

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({ "results": results })),
        error: None,
    }))
}

// Tool: memory_get_context
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

pub fn create_mcp_router(state: AppState) -> Router {
    Router::new()
        .route("/mcp/tools/memory_store", post(memory_store))
        .route("/mcp/tools/memory_query", post(memory_query))
        .route("/mcp/tools/memory_get_context", post(memory_get_context))
        .with_state(state)
}
