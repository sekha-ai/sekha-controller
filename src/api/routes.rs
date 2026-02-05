use crate::api::dto::*;
use crate::models::internal::Message;
use crate::services::embedding_service::EmbeddingService;
use crate::services::llm_bridge_client::LlmBridgeClient;
use crate::storage::chroma_client::ChromaClient;
use crate::storage::db::get_connection;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::ConnectionTrait;
use serde_json::{json, Value};

use axum::http::StatusCode;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::orchestrator::MemoryOrchestrator;
use crate::{config::Config, storage::repository::ConversationRepository};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub repo: Arc<dyn ConversationRepository>,
    pub orchestrator: Arc<MemoryOrchestrator>,
    pub embedding_service: Arc<EmbeddingService>,
    pub chroma_client: Arc<ChromaClient>,
    pub llm_client: Arc<LlmBridgeClient>,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Deserialize)]
pub struct FilterParams {
    label: Option<String>,
    folder: Option<String>,
    pinned: Option<bool>,
    archived: Option<bool>,
}

// ==================== ROUTE HANDLERS ====================

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
    })
}

/// Metrics endpoint
pub async fn metrics(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "metrics": "not_implemented"
    }))
}

/// Create a new conversation
pub async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<ConversationResponse>, (StatusCode, String)> {
    match state.orchestrator.create_conversation(&req).await {
        Ok(conversation) => Ok(Json(ConversationResponse {
            id: conversation.id,
            label: conversation.label,
            folder: conversation.folder,
            status: conversation.status,
            message_count: req.messages.len(),
            created_at: conversation.created_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Get a conversation by ID
pub async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.repo.get_conversation_by_id(id).await {
        Ok(Some(conv)) => Ok(Json(json!(conv))),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Conversation not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// List conversations
pub async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    match state.repo.list_conversations(page, page_size).await {
        Ok(conversations) => Ok(Json(json!({
            "conversations": conversations,
            "page": page,
            "page_size": page_size
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Update conversation label
pub async fn update_conversation_label(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateLabelRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state
        .repo
        .update_conversation_label(id, &req.label, &req.folder)
        .await
    {
        Ok(_) => Ok(Json(json!({ "success": true }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Delete a conversation
pub async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.repo.delete_conversation(id).await {
        Ok(_) => Ok(Json(json!({ "success": true }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Count conversations
pub async fn count_conversations(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.repo.count_conversations().await {
        Ok(count) => Ok(Json(json!({ "count": count }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Semantic query endpoint (Module 5 integration)
#[utoipa::path(
    post,
    path = "/api/v1/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Semantic search results", body = serde_json::Value)
    )
)]
pub async fn semantic_query(
    State(_state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Json<Value> {
    // TODO: In Module 5, integrate with Chroma
    // For now, return mock results with correct schema

    let mock_results = vec![serde_json::json!({
        "conversation_id": Uuid::new_v4(),
        "message_id": Uuid::new_v4(),
        "score": 0.85,
        "content": "Mock result for: ".to_string() + &req.query,
        "metadata": {
            "label": "Project:AI-Memory",
            "timestamp": "2025-12-11T21:00:00Z"
        }
    })];

    Json(serde_json::json!({
        "query": req.query,
        "results": mock_results,
        "total": 1,
        "limit": req.limit,
        "filters": req.filters
    }))
}

// ==================== ROUTER CREATION ====================

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Conversation endpoints
        .route("/api/v1/conversations", post(create_conversation))
        .route("/api/v1/conversations/{id}", get(get_conversation))
        .route("/api/v1/conversations", get(list_conversations))
        .route(
            "/api/v1/conversations/{id}/label",
            put(update_conversation_label),
        )
        .route("/api/v1/conversations/{id}", delete(delete_conversation))
        .route("/api/v1/conversations/count", get(count_conversations))
        // Query endpoint
        .route("/api/v1/query", post(semantic_query))
        // Health and metrics
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}
