use axum::{
    extract::State,
    http::StatusCode,
    Json,
    Router,
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    api::dto::*,
    auth::McpAuth,
    config::Config,
    models::internal::Conversation,
    storage::ConversationRepository,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct McpToolRequest {
    pub tool: String,
    pub arguments: JsonValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolResponse {
    pub success: bool,
    pub data: Option<JsonValue>,
    pub error: Option<String>,
}

// Tool: memory_store tool
#[utoipa::path(
    post,
    path = "/mcp/tools/memory_store",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Stored successfully", body = McpToolResponse)
    )
)]
pub async fn memory_store(
    _auth: McpAuth,  // NEW: Authentication enforced
    State(state): State<crate::api::routes::AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();
    
    let conv = Conversation {
        id,
        label: req.label,
        folder: req.folder,
        status: "active".to_string(),
        importance_score: 5,
        word_count: req.messages.iter().map(|m| m.content.len() as i32).sum(),
        session_count: 1,
        created_at: now,
        updated_at: now,
    };
    
    match state.repo.create(conv).await {
        Ok(id) => Ok(Json(McpToolResponse {
            success: true,
            data: Some(serde_json::json!({ "conversation_id": id })),
            error: None,
        })),
        Err(e) => Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

// MARK: memory_query tool
#[derive(Debug, Deserialize)]
pub struct MemoryQueryArgs {
    query: String,
    filters: Option<JsonValue>,
    limit: Option<u32>,
}

pub async fn memory_query(
    _auth: McpAuth,
    State(state): State<crate::api::routes::AppState>,
    Json(args): Json<MemoryQueryArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    // TODO: Integrate with Chroma vector search
    // For now, return mock results
    
    let results = vec![
        serde_json::json!({
            "conversation_id": Uuid::new_v4(),
            "relevance_score": 0.85,
            "summary": "Mock result for: ".to_string() + &args.query,
        })
    ];
    
    Ok(Json(McpToolResponse {
        success: true,
        data: Some(serde_json::json!({ "results": results })),
        error: None,
    }))
}

// MARK: memory_get_context tool
#[derive(Debug, Deserialize)]
pub struct MemoryGetContextArgs {
    conversation_id: Uuid,
}

pub async fn memory_get_context(
    _auth: McpAuth,
    State(state): State<crate::api::routes::AppState>,
    Json(args): Json<MemoryGetContextArgs>,
) -> Result<Json<McpToolResponse>, StatusCode> {
    // TODO: Implement hierarchical context retrieval
    // For now, return conversation metadata
    
    match state.repo.find_by_id(args.conversation_id).await {
        Ok(Some(conv)) => Ok(Json(McpToolResponse {
            success: true,
            data: Some(serde_json::json!({
                "conversation_id": conv.id,
                "label": conv.label,
                "message_count": 0,
                "status": conv.status,
            })),
            error: None,
        })),
        Ok(None) => Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some("Conversation not found".to_string()),
        })),
        Err(e) => Ok(Json(McpToolResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

pub fn create_mcp_router(state: crate::api::routes::AppState) -> axum::Router {
    axum::Router::new()
        .route("/mcp/tools/memory_store", axum::routing::post(memory_store))
        .route("/mcp/tools/memory_query", axum::routing::post(memory_query))
        .route("/mcp/tools/memory_get_context", axum::routing::post(memory_get_context))
        .with_state(state)
}
