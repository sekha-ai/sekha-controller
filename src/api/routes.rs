use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Router, Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde_json::Value;

use crate::{
    api::dto::*,
    config::Config,
    models::internal::Conversation,
    storage::ConversationRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub repo: Arc<dyn ConversationRepository + Send + Sync>,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    page: Option<u32>,
    page_size: Option<u32>,
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations",
    request_body = CreateConversationRequest,
    responses(
        (status = 201, description = "Conversation created", body = ConversationResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    )
)]
async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<ConversationResponse>), (StatusCode, Json<ErrorResponse>)> {
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

    state.repo.create(conv).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(ConversationResponse {
            id,
            label: req.label,
            folder: req.folder,
            status: "active".to_string(),
            message_count: req.messages.len(),
            created_at: now.to_string(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/conversations/{id}",
    responses(
        (status = 200, description = "Conversation found", body = ConversationResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    params(
        ("id" = String, Path, description = "Conversation UUID")
    )
)]
async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConversationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let conv = state.repo.find_by_id(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    match conv {
        Some(c) => Ok(Json(ConversationResponse {
            id: c.id,
            label: c.label,
            folder: c.folder,
            status: c.status,
            message_count: 0, // TODO: Join with messages table
            created_at: c.created_at.to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Conversation not found".to_string(),
                code: 404,
            })),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/conversations",
    responses(
        (status = 200, description = "List conversations", body = QueryResponse)
    ),
    params(
        ("label" = Option<String>, Query, description = "Filter by label"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    )
)]
async fn list_conversations(
    State(_state): State<AppState>,
    Query(params): Query<PaginationParams>,
    Query(_label): Query<Option<String>>,
) -> Json<QueryResponse> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let _offset = (page - 1) * page_size;

    // TODO: Implement label filtering
    let results = vec![]; // Placeholder

    Json(QueryResponse {
        results,
        total: 0,
        page,
        page_size,
    })
}

#[utoipa::path(
    put,
    path = "/api/v1/conversations/{id}/label",
    request_body = UpdateLabelRequest,
    responses(
        (status = 200, description = "Label updated"),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
async fn update_conversation_label(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateLabelRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.repo.update_label(id, &req.label, &req.folder).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/api/v1/conversations/{id}",
    responses(
        (status = 200, description = "Conversation deleted"),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    params(
        ("id" = String, Path, description = "Conversation UUID")
    )
)]
async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.repo.delete(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/v1/conversations/count",
    responses(
        (status = 200, description = "Count by label", body = serde_json::Value)
    ),
    params(
        ("label" = String, Query, description = "Label to count")
    )
)]
async fn count_conversations(
    State(state): State<AppState>,
    Query(label): Query<String>,
) -> Json<serde_json::Value> {
    let count = state.repo.count_by_label(&label).await.unwrap_or(0);
    Json(serde_json::json!({ "count": count, "label": label }))
}

#[utoipa::path(
    post,
    path = "/api/v1/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Semantic search results", body = Value)
    )
)]
async fn semantic_query(
    State(_state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Json<Value> {
    let mock_results = vec![
        serde_json::json!({
            "conversation_id": Uuid::new_v4(),
            "message_id": Uuid::new_v4(),
            "score": 0.85,
            "content": format!("Mock result for: {}", req.query),
            "metadata": {
                "label": "Project:AI-Memory",
                "timestamp": "2025-12-11T21:00:00Z"
            }
        })
    ];

    Json(serde_json::json!({
        "query": req.query,
        "results": mock_results,
        "total": 1,
        "limit": req.limit,
        "filters": req.filters
    }))
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/conversations", post(create_conversation))
        .route("/api/v1/conversations/:id", get(get_conversation))
        .route("/api/v1/conversations", get(list_conversations))
        .route("/api/v1/conversations/:id/label", put(update_conversation_label))
        .route("/api/v1/conversations/:id", delete(delete_conversation))
        .route("/api/v1/conversations/count", get(count_conversations))
        .route("/api/v1/query", post(semantic_query))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn metrics() -> &'static str {
    "# HELP sekha_conversations_total Total number of conversations\n# TYPE sekha_conversations_total gauge\nsekha_conversations_total 0\n"
}
