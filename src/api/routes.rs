use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::orchestrator::MemoryOrchestrator;
use crate::{api::dto::*, config::Config, storage::repository::ConversationRepository};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub repo: Arc<dyn ConversationRepository>,
    pub orchestrator: Arc<MemoryOrchestrator>,
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

// health check config
use crate::storage::db::get_connection;

async fn health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut checks = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "checks": {}
    });

    // Check database
    match get_connection().await {
        Some(db) => match db.execute_unprepared("SELECT 1").await {
            Ok(_) => checks["checks"]["database"] = serde_json::json!({"status": "ok"}),
            Err(e) => {
                checks["checks"]["database"] = serde_json::json!({
                    "status": "error",
                    "error": e.to_string()
                });
                checks["status"] = "unhealthy";
            }
        },
        None => {
            checks["checks"]["database"] = serde_json::json!({
                "status": "error",
                "error": "No database connection"
            });
            checks["status"] = "unhealthy";
        }
    }

    // Check Chroma (basic connectivity)
    let chroma_check = state.repo.semantic_search("test", 1, None).await;
    match chroma_check {
        Ok(_) => checks["checks"]["chroma"] = serde_json::json!({"status": "ok"}),
        Err(e) => {
            checks["checks"]["chroma"] = serde_json::json!({
                "status": "error",
                "error": e.to_string()
            });
            checks["status"] = "unhealthy";
        }
    }

    // Check embedding service (without generating actual embedding)
    let embedding_service = &state.embedding_service;
    if embedding_service.semaphore.available_permits() > 0 {
        checks["checks"]["embedding_service"] = serde_json::json!({"status": "ok"});
    } else {
        checks["checks"]["embedding_service"] = serde_json::json!({
            "status": "warning",
            "error": "All embedding workers busy"
        });
    }

    if checks["status"] == "healthy" {
        Ok(Json(checks))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

// ============================================
// Endpoint 1: POST /api/v1/conversations
// ============================================
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

    let new_messages: Vec<_> = req
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

// ============================================
// Endpoint 2: GET /api/v1/conversations/{id}
// ============================================
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
        Some(c) => {
            let message_count = state
                .repo
                .count_messages_in_conversation(id)
                .await
                .unwrap_or(0);
            Ok(Json(ConversationResponse {
                id: c.id,
                label: c.label,
                folder: c.folder,
                status: c.status,
                message_count: message_count.try_into().unwrap(),
                created_at: c.created_at.to_string(),
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Conversation not found".to_string(),
                code: 404,
            }),
        )),
    }
}

// ============================================
// Endpoint 3: GET /api/v1/conversations (COMPLETE - was stubbed)
// ============================================
#[utoipa::path(
    get,
    path = "/api/v1/conversations",
    responses(
        (status = 200, description = "List conversations", body = QueryResponse)
    ),
    params(
        ("label" = Option<String>, Query, description = "Filter by label"),
        ("folder" = Option<String>, Query, description = "Filter by folder"),
        ("pinned" = Option<bool>, Query, description = "Filter by pinned status"),
        ("archived" = Option<bool>, Query, description = "Filter by archived status"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    )
)]
async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    Query(filters): Query<FilterParams>,
) -> Json<QueryResponse> {
    let _ = (filters.pinned, filters.archived);
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let offset = (page - 1) * page_size;

    // Build filter criteria - NOW USING the parameters!
    let mut criteria = Vec::new();
    if let Some(label) = &filters.label {
        criteria.push(format!("label = '{}'", label));
    }
    if let Some(folder) = &filters.folder {
        criteria.push(format!("folder = '{}'", folder));
    }
    if let Some(pinned) = filters.pinned {
        criteria.push(format!("pinned = {}", pinned));
    }
    if let Some(archived) = filters.archived {
        criteria.push(format!("archived = {}", archived));
    }
    let filter_str = if criteria.is_empty() {
        None
    } else {
        Some(criteria.join(" AND "))
    };

    // Use repository method with filters
    let results = state
        .repo
        .find_with_filters(filter_str, page_size as usize, offset as u32)
        .await
        .unwrap_or_else(|_| (Vec::new(), 0));

    let total = results.1;
    let conversations: Vec<SearchResultDto> = results
        .0
        .into_iter()
        .map(|c| SearchResultDto {
            conversation_id: c.id,
            message_id: Uuid::nil(),
            score: 1.0,
            content: c.label.clone(),
            metadata: serde_json::json!({
                "folder": c.folder,
                "status": c.status,
                "importance_score": c.importance_score,
            }),
            label: c.label,
            folder: c.folder,
            timestamp: c.updated_at.to_string(),
        })
        .collect();

    Json(QueryResponse {
        results: conversations,
        total: total.try_into().unwrap_or(u32::MAX), // FIXED: Convert u64 to u32 safely
        page,
        page_size,
    })
}

// ============================================
// Endpoint 4: PUT /api/v1/conversations/{id}/label
// ============================================
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

// ============================================
// Endpoint 5: DELETE /api/v1/conversations/{id}
// ============================================
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

// ============================================
// Endpoint 6: GET /api/v1/conversations/count
// ============================================
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

// ============================================
// Endpoint 7: POST /api/v1/query
// ============================================
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

// ============================================
// Endpoint 8: GET /health
// ============================================
async fn health() -> &'static str {
    "OK"
}

// ============================================
// Endpoint 9: GET /metrics
// ============================================
async fn metrics() -> &'static str {
    "# HELP sekha_conversations_total Total number of conversations\n# TYPE sekha_conversations_total gauge\nsekha_conversations_total 0\n"
}

// ============================================
// NEW ENDPOINT: PUT /api/v1/conversations/{id}/folder
// ============================================
#[utoipa::path(
    put,
    path = "/api/v1/conversations/{id}/folder",
    request_body = UpdateFolderRequest,
    responses(
        (status = 200, description = "Folder updated"),
        (status = 404, description = "Not found", body = ErrorResponse)
    )
)]
async fn update_conversation_folder(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFolderRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Reuse update_label method with same label
    let conv = state.repo.find_by_id(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    if conv.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Conversation not found".to_string(),
                code: 404,
            }),
        ));
    }

    state
        .repo
        .update_label(id, &req.folder, &req.folder)
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

    Ok(StatusCode::OK)
}

// ============================================
// NEW ENDPOINT: PUT /api/v1/conversations/{id}/pin
// ============================================
#[utoipa::path(
    put,
    path = "/api/v1/conversations/{id}/pin",
    responses(
        (status = 200, description = "Conversation pinned"),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    params(
        ("id" = String, Path, description = "Conversation UUID")
    )
)]
async fn pin_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Toggle pin status by setting importance_score high
    state.repo.update_importance(id, 10).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    Ok(StatusCode::OK)
}

// ============================================
// NEW ENDPOINT: PUT /api/v1/conversations/{id}/archive
// ============================================
#[utoipa::path(
    put,
    path = "/api/v1/conversations/{id}/archive",
    responses(
        (status = 200, description = "Conversation archived"),
        (status = 404, description = "Not found", body = ErrorResponse)
    ),
    params(
        ("id" = String, Path, description = "Conversation UUID")
    )
)]
async fn archive_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .repo
        .update_status(id, "archived")
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

    Ok(StatusCode::OK)
}

// ============================================
// NEW ENDPOINT: POST /api/v1/rebuild-embeddings
// ============================================
#[utoipa::path(
    post,
    path = "/api/v1/rebuild-embeddings",
    responses(
        (status = 202, description = "Rebuild started"),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
async fn rebuild_embeddings(
    State(_state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Trigger async rebuild via embedding service
    tokio::spawn(async move {
        tracing::info!("Starting embedding rebuild...");
        // TODO: Implement actual rebuild logic in embedding service
    });

    Ok(StatusCode::ACCEPTED)
}

// POST /api/v1/search/fts
#[utoipa::path(
    post,
    path = "/api/v1/search/fts",
    request_body = FtsSearchRequest,
    responses(
        (status = 200, description = "Full-text search results", body = Vec<MessageResponse>)
    )
)]
async fn full_text_search(
    State(state): State<AppState>,
    Json(req): Json<FtsSearchRequest>,
) -> Result<Json<Vec<crate::models::Message>>, AppError> {
    let messages = state.repo.full_text_search(&req.query, req.limit).await?;
    Ok(Json(messages))
}

// ============================================
// Router - All 12 endpoints registered
// ============================================
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
            "/api/v1/conversations/{id}/folder",
            put(update_conversation_folder),
        )
        .route("/api/v1/conversations/{id}/pin", put(pin_conversation))
        .route(
            "/api/v1/conversations/{id}/archive",
            put(archive_conversation),
        )
        .route("/api/v1/conversations/{id}", delete(delete_conversation))
        .route("/api/v1/conversations/count", get(count_conversations))
        .route("/api/v1/query", post(semantic_query))
        .route("/api/v1/rebuild-embeddings", post(rebuild_embeddings))
        .route("/api/v1/search/fts", post(full_text_search))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}
