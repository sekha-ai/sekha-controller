use crate::api::dto::*;
use crate::models::internal::Message;
use crate::services::embedding_service::EmbeddingService;
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

// #[derive(Deserialize)]
// pub struct CountQuery {
//     label: String,
// }

#[derive(Deserialize)]
pub struct CountParams {
    label: Option<String>,
    folder: Option<String>,
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
pub async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // ✅ Changed return type
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
        Json(serde_json::json!({
            "id": id,
            "conversation_id": id,  // ✅ Both fields for compatibility
            "label": req.label,
            "folder": req.folder,
            "status": "active",
            "message_count": message_count,
            "created_at": now,
        })),
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
pub async fn get_conversation(
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
                created_at: c.created_at, // CHANGED: Remove .to_string()
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
pub async fn list_conversations(
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
            timestamp: c.updated_at, // CHANGED: Remove .to_string()
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
pub async fn update_conversation_label(
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
pub async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Check if conversation exists first
    let exists = state.repo.find_by_id(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    if exists.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Conversation not found".to_string(),
                code: 404,
            }),
        ));
    }

    state.repo.delete(id).await.map_err(|e| {
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
// Endpoint 6: GET /api/v1/conversations/count
// ============================================
#[utoipa::path(
    get,
    path = "/api/v1/conversations/count",
    responses(
        (status = 200, description = "Count conversations by label or folder", body = serde_json::Value)
    ),
    params(
        ("label" = Option<String>, Query, description = "Label to filter by"),
        ("folder" = Option<String>, Query, description = "Folder to filter by")
    )
)]
pub async fn count_conversations(
    State(state): State<AppState>,
    Query(params): Query<CountParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clone values before they are moved
    let label_for_response = params.label.clone();
    let folder_for_response = params.folder.clone();

    let count = match (&params.label, &params.folder) {
        (Some(label), None) => state.repo.count_by_label(label).await,
        (None, Some(folder)) => state.repo.count_by_folder(folder).await,  // ✅ CHANGED
        (None, None) => state.repo.count_all().await,  // ✅ CHANGED
        (Some(_), Some(_)) => {
            return Ok(Json(serde_json::json!({
                "count": 0,
                "error": "Cannot specify both label and folder"
            })));
        }
    }
    .map_err(|e| {
        tracing::error!("Count failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "count": count,
        "label": label_for_response,
        "folder": folder_for_response
    })))
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

pub async fn semantic_query(
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
            timestamp: r.timestamp, // CHANGED: Remove .to_string()
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
pub async fn health(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut checks = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "checks": {}
    });

    // Check database
    match get_connection().await {
        Some(db) => match db.execute_unprepared("SELECT 1").await {
            Ok(_) => checks["checks"]["database"] = json!({"status": "ok"}),
            Err(e) => {
                checks["checks"]["database"] = json!({"status": "error", "error": e.to_string()});
                checks["status"] = "unhealthy".into();
            }
        },
        None => {
            checks["checks"]["database"] = json!({"status": "error", "error": "No connection"});
            checks["status"] = "unhealthy".into();
        }
    }

    // Check Chroma
    match state.chroma_client.ping().await {
        Ok(_) => checks["checks"]["chroma"] = json!({"status": "ok"}),
        Err(e) => {
            checks["checks"]["chroma"] = json!({"status": "error", "error": e.to_string()});
            checks["status"] = "unhealthy".into();
        }
    }

    if checks["status"] == "healthy" {
        Ok(Json(checks))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

// ============================================
// Endpoint 9: GET /metrics
// ============================================
pub async fn metrics() -> &'static str {
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
        (status = 200, description = "Full-text search results", body = FtsSearchResponse)
    )
)]
async fn full_text_search(
    State(state): State<AppState>,
    Json(req): Json<FtsSearchRequest>,
) -> Result<Json<FtsSearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let messages = state
        .repo
        .full_text_search(&req.query, req.limit)
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

    let total = messages.len();

    Ok(Json(FtsSearchResponse {
        results: messages,
        total,
    }))
}

// ============================================
// MODULE 5 ORCHESTRATION ENDPOINTS
// ============================================

// Endpoint: POST /api/v1/context/assemble
#[utoipa::path(
    post,
    path = "/api/v1/context/assemble",
    request_body = ContextAssembleRequest,
    responses(
        (status = 200, description = "Context assembled", body = Vec<Message>),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
async fn assemble_context(
    State(state): State<AppState>,
    Json(req): Json<ContextAssembleRequest>,
) -> Result<Json<Vec<Message>>, (StatusCode, Json<ErrorResponse>)> {
    let results = state
        .orchestrator
        .assemble_context(&req.query, req.preferred_labels, req.context_budget)
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

    Ok(Json(results))
}

// Endpoint: POST /api/v1/summarize
#[utoipa::path(
    post,
    path = "/api/v1/summarize",
    request_body = SummarizeRequest,
    responses(
        (status = 200, description = "Summary generated", body = SummaryResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
async fn generate_summary(
    State(state): State<AppState>,
    Json(req): Json<SummarizeRequest>,
) -> Result<Json<SummaryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let summary = match req.level.as_str() {
        "daily" => {
            state
                .orchestrator
                .generate_daily_summary(req.conversation_id)
                .await
        }
        "weekly" => {
            state
                .orchestrator
                .summarizer
                .generate_weekly_summary(req.conversation_id)
                .await
        }
        "monthly" => {
            state
                .orchestrator
                .summarizer
                .generate_monthly_summary(req.conversation_id)
                .await
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid level: must be daily, weekly, or monthly".to_string(),
                    code: 400,
                }),
            ))
        }
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    Ok(Json(SummaryResponse {
        conversation_id: req.conversation_id,
        level: req.level,
        summary,
        generated_at: chrono::Utc::now().naive_utc(), // CHANGED: Remove .to_rfc3339()
    }))
}

// Endpoint: POST /api/v1/prune/dry-run
#[utoipa::path(
    post,
    path = "/api/v1/prune/dry-run",
    request_body = PruneRequest,
    responses(
        (status = 200, description = "Pruning suggestions", body = PruneResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
async fn prune_dry_run(
    State(state): State<AppState>,
    Json(req): Json<PruneRequest>,
) -> Result<Json<PruneResponse>, (StatusCode, Json<ErrorResponse>)> {
    let suggestions = state
        .orchestrator
        .suggest_pruning(req.threshold_days)
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

    let total = suggestions.len(); // Calculate before consuming

    Ok(Json(PruneResponse {
        suggestions: suggestions
            .into_iter()
            .map(|s| PruningSuggestionDto {
                conversation_id: s.conversation_id,
                conversation_label: s.conversation_label,
                last_accessed: s.last_accessed, // CHANGED: Remove .to_string()
                message_count: s.message_count,
                token_estimate: s.token_estimate,
                importance_score: s.importance_score,
                preview: s.preview,
                recommendation: s.recommendation,
            })
            .collect(),
        total,
    }))
}

// Endpoint: POST /api/v1/prune/execute
#[utoipa::path(
    post,
    path = "/api/v1/prune/execute",
    request_body = ExecutePruneRequest,
    responses(
        (status = 200, description = "Conversations archived"),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
async fn prune_execute(
    State(state): State<AppState>,
    Json(req): Json<ExecutePruneRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    for id in req.conversation_ids {
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
    }

    Ok(StatusCode::OK)
}

// Endpoint: POST /api/v1/labels/suggest
#[utoipa::path(
    post,
    path = "/api/v1/labels/suggest",
    request_body = LabelSuggestRequest,
    responses(
        (status = 200, description = "Label suggestions", body = LabelSuggestResponse),
        (status = 500, description = "Server error", body = ErrorResponse)
    )
)]
async fn suggest_labels(
    State(state): State<AppState>,
    Json(req): Json<LabelSuggestRequest>,
) -> Result<Json<LabelSuggestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let suggestions = state
        .orchestrator
        .suggest_labels(req.conversation_id)
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

    Ok(Json(LabelSuggestResponse {
        conversation_id: req.conversation_id,
        suggestions: suggestions
            .into_iter()
            .map(|s| LabelSuggestionDto {
                label: s.label,
                confidence: s.confidence,
                is_existing: s.is_existing,
                reason: s.reason,
            })
            .collect(),
    }))
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
        .route("/api/v1/context/assemble", post(assemble_context))
        .route("/api/v1/summarize", post(generate_summary))
        .route("/api/v1/prune/dry-run", post(prune_dry_run))
        .route("/api/v1/prune/execute", post(prune_execute))
        .route("/api/v1/labels/suggest", post(suggest_labels))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}
