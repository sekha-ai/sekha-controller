// src/api/routes.rs - FINAL VERSION WITH FIXED DB CALLS

use crate::orchestrator::MemoryOrchestrator;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    api::dto::*, config::Config, storage::entities::conversations,
    storage::repository::ConversationRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub repo: Arc<dyn ConversationRepository + Send + Sync>,
    pub orchestrator: Arc<MemoryOrchestrator>,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub label: Option<String>,
    pub folder: Option<String>,
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

    let word_count: i32 = req.messages.iter().map(|m| m.content.len() as i32).sum();

    // Map MessageDto to NewMessage
    let new_messages: Vec<crate::models::internal::NewMessage> = req
        .messages
        .into_iter()
        .map(|m| crate::models::internal::NewMessage {
            role: m.role,
            content: m.content,
            metadata: serde_json::json!({}),
            timestamp: now,
        })
        .collect();

    let message_count = new_messages.len();

    let new_conv = crate::models::internal::NewConversation {
        id: Some(id),
        label: req.label.clone(),
        folder: req.folder.clone(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count,
        session_count: Some(1),
        created_at: now,
        updated_at: now,
        messages: new_messages,
    };

    state
        .repo
        .create_with_messages(new_conv)
        .await
        .map_err(|e| {
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
            message_count,
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
            }),
        )),
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
        ("folder" = Option<String>, Query, description = "Filter by folder"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    )
)]
async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Use sensible defaults
    let page_size = params.page_size.unwrap_or(50).max(1).min(100); // Clamp between 1-100
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * page_size;

    // Build query based on filters
    let mut query = conversations::Entity::find();

    if let Some(ref label) = params.label {
        query = query.filter(conversations::Column::Label.eq(label));
    }

    if let Some(ref folder) = params.folder {
        query = query.filter(conversations::Column::Folder.eq(folder));
    }

    // Get total count - FIX: Pass connection directly, not &&connection
    let total = query
        .clone()
        .count(state.repo.get_db())
        .await
        .map_err(|e| internal_server_error(e))?;

    // Get paginated results - FIX: Pass connection directly, not &&connection
    let models = query
        .order_by_desc(conversations::Column::UpdatedAt)
        .limit(page_size as u64)
        .offset(offset as u64)
        .all(state.repo.get_db())
        .await
        .map_err(|e| internal_server_error(e))?;

    let results: Vec<SearchResultDto> = models
        .into_iter()
        .map(|c| SearchResultDto {
            conversation_id: Uuid::parse_str(&c.id).unwrap(),
            message_id: Uuid::nil(), // No specific message
            score: 0.0,
            content: "".to_string(),
            metadata: json!({}),
            label: c.label,
            folder: c.folder,
            timestamp: c.created_at.clone(),
        })
        .collect();

    Ok(Json(QueryResponse {
        results,
        total: total as u32,
        page,
        page_size,
    }))
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
    state
        .repo
        .update_label(id, &req.label, &req.folder)
        .await
        .map_err(|e| {
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
    put,
    path = "/api/v1/conversations/{id}/status",
    request_body = UpdateStatusRequest,
    responses(
        (status = 200, description = "Status updated"),
        (status = 400, description = "Invalid status", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    params(
        ("id" = String, Path, description = "Conversation UUID")
    )
)]
async fn update_conversation_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Validate status
    if !["active", "archived", "pinned"].contains(&req.status.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid status. Must be 'active', 'archived', or 'pinned'".to_string(),
                code: 400,
            }),
        ));
    }

    // Update status in database
    state
        .repo
        .update_status(id, &req.status)
        .await
        .map_err(|e| {
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
    path = "/api/v1/export",
    responses(
        (status = 200, description = "Export successful", body = ExportResponse)
    ),
    params(
        ("label" = Option<String>, Query, description = "Filter by label"),
        ("format" = Option<String>, Query, description = "Export format: markdown or json")
    )
)]
async fn export_conversations(
    State(state): State<AppState>,
    Query(label): Query<Option<String>>,
    Query(format): Query<Option<String>>,
) -> Result<Json<ExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let format = format.unwrap_or_else(|| "markdown".to_string());

    // Validate format
    if !["markdown", "json"].contains(&format.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid format. Must be 'markdown' or 'json'".to_string(),
                code: 400,
            }),
        ));
    }

    // Fetch conversations
    let conversations = state.repo.export_conversations(label).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    let conversation_count = conversations.len();

    // Format content based on format parameter
    let content = if format == "json" {
        serde_json::to_string_pretty(&conversations).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to serialize to JSON: {}", e),
                    code: 500,
                }),
            )
        })?
    } else {
        // Markdown format
        let mut md = String::new();
        md.push_str("# Sekha Conversations Export\n\n");
        for conv in &conversations {
            md.push_str(&format!("## {}\n", conv.label));
            md.push_str(&format!("- **ID:** {}\n", conv.id));
            md.push_str(&format!("- **Folder:** {}\n", conv.folder));
            md.push_str(&format!("- **Status:** {}\n", conv.status));
            md.push_str(&format!("- **Created:** {}\n\n", conv.created_at));
        }
        md
    };

    Ok(Json(ExportResponse {
        content,
        format: format.clone(),
        conversation_count,
    }))
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
        (status = 200, description = "Semantic search results", body = QueryResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Search error", body = ErrorResponse)
    )
)]
async fn semantic_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Semantic query: {}", req.query);

    let limit = req.limit.unwrap_or(10) as usize;
    let offset = req.offset.unwrap_or(0);

    // Calculate page number
    let page = if limit > 0 {
        (offset as f64 / limit as f64).ceil() as u32
    } else {
        1
    };

    // Use repository's semantic search (now powered by Chroma)
    let results = state
        .repo
        .semantic_search(&req.query, limit, req.filters)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Semantic search failed: {}", e),
                    code: 500,
                }),
            )
        })?;

    let api_results: Vec<SearchResultDto> = results
        .iter()
        .map(|r| SearchResultDto {
            conversation_id: r.conversation_id,
            message_id: r.message_id,
            score: r.score,
            content: r.content.clone(),
            metadata: r.metadata.clone(),
            label: r.label.clone(),
            folder: r.folder.clone(),
            timestamp: r.timestamp.to_string(),
        })
        .collect();

    Ok(Json(QueryResponse {
        results: api_results,
        total: results.len() as u32,
        page,
        page_size: limit as u32,
    }))
}

/// Enhanced health check with dependency verification
async fn health(State(state): State<AppState>) -> Json<Value> {
    let mut checks = json!({});
    let mut all_healthy = true;

    // Check SQLite
    let db_healthy = state.repo.ping().await.is_ok();
    checks["database"] = json!({
        "status": if db_healthy { "healthy" } else { "unhealthy" }
    });
    all_healthy &= db_healthy;

    // Check external dependencies through repository trait method
    match state.repo.check_dependencies().await {
        Ok(deps) => {
            if let Some(chroma_check) = deps.get("chroma") {
                checks["chroma"] = chroma_check.clone();
                if let Some(status) = chroma_check.get("status").and_then(|s| s.as_str()) {
                    all_healthy &= status == "healthy";
                }
            }
        }
        Err(e) => {
            checks["chroma"] = json!({
                "status": "unhealthy",
                "error": e.to_string()
            });
            all_healthy = false;
        }
    }

    // Check Ollama (optional, soft failure)
    let ollama_url = state.config.read().await.ollama_url.clone();
    let ollama_healthy = check_ollama(&ollama_url).await;
    checks["ollama"] = json!({
        "status": if ollama_healthy { "healthy" } else { "unavailable" },
        "optional": true
    });

    let status = if all_healthy { "healthy" } else { "unhealthy" };

    Json(json!({
        "status": status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "checks": checks
    }))
}

/// Helper to check Ollama health (soft check, doesn't fail health endpoint)
async fn check_ollama(url: &str) -> bool {
    let client = reqwest::Client::new();
    client
        .get(format!("{}/api/tags", url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/conversations", post(create_conversation))
        .route("/api/v1/conversations/{id}", get(get_conversation))
        .route("/api/v1/conversations", get(list_conversations))
        .route(
            "/api/v1/conversations/{id}/label",
            put(update_conversation_label),
        )
        .route(
            "/api/v1/conversations/{id}/status",
            put(update_conversation_status),
        )
        .route("/api/v1/conversations/{id}", delete(delete_conversation))
        .route("/api/v1/conversations/count", get(count_conversations))
        .route("/api/v1/query", post(semantic_query))
        .route("/api/v1/export", get(export_conversations))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn metrics() -> &'static str {
    "# HELP sekha_conversations_total Total number of conversations\n# TYPE sekha_conversations_total gauge\nsekha_conversations_total 0\n"
}

pub async fn update_status() -> &'static str {
    "Status updated"
}

fn internal_server_error<E: std::fmt::Display>(e: E) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: e.to_string(),
            code: 500,
        }),
    )
}
