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

use crate::{api::dto::*, config::Config, storage::repository::ConversationRepository};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub repo: Arc<dyn ConversationRepository + Send + Sync>,
    pub orchestrator: Arc<MemoryOrchestrator>,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    page: Option<u32>,
    page_size: Option<u32>,
}

pub async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<ConversationResponse>), (StatusCode, Json<ErrorResponse>)> {
    use crate::models::internal::{NewConversation, NewMessage};
    use chrono::Utc;

    let id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    // Convert MessageDto to NewMessage
    let new_messages: Vec<NewMessage> = req
        .messages
        .into_iter()
        .map(|m| NewMessage {
            role: m.role,
            content: m.content,
            metadata: serde_json::json!({}),
            timestamp: now,
        })
        .collect();

    let message_count = new_messages.len();
    let word_count: i32 = new_messages.iter().map(|m| m.content.len() as i32).sum();

    let new_conv = NewConversation {
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

    // Use the correct repository method
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
        Some(c) => Ok(Json(ConversationResponse {
            id: c.id,
            label: c.label,
            folder: c.folder,
            status: c.status,
            message_count: 0, // TODO: Add message count query
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

pub async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    Query(label): Query<Option<String>>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let limit = page_size as u64;
    let offset = ((page - 1) * page_size) as u64;

    let conversations = if let Some(label_filter) = label {
        state
            .repo
            .find_by_label(&label_filter, limit, offset)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to fetch conversations: {}", e),
                        code: 500,
                    }),
                )
            })?
    } else {
        // Empty list for now - need repository.find_all()
        Vec::new()
    };

    let results: Vec<SearchResultDto> = conversations
        .into_iter()
        .map(|c| SearchResultDto {
            conversation_id: c.id,
            message_id: Uuid::nil(),
            score: 1.0,
            content: c.label.clone(),
            metadata: serde_json::json!({
                "folder": c.folder,
                "status": c.status,
            }),
            label: c.label,
            folder: c.folder,
            timestamp: c.created_at.to_string(),
        })
        .collect();

    let total = results.len() as u32;

    Ok(Json(QueryResponse {
        results,
        total,
        page,
        page_size,
    }))
}

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

pub async fn update_conversation_folder(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(folder): Query<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Get current conversation to preserve label
    let conv = state.repo.find_by_id(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    if let Some(conversation) = conv {
        state
            .repo
            .update_label(id, &conversation.label, &folder)
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
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Conversation not found".to_string(),
                code: 404,
            }),
        ))
    }
}

pub async fn pin_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(pinned): Query<bool>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Just verify conversation exists for now
    let _conv = state
        .repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 500,
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Conversation not found".to_string(),
                    code: 404,
                }),
            )
        })?;

    // TODO: Implement actual pin logic
    Ok(StatusCode::OK)
}

pub async fn archive_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(archived): Query<bool>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Verify conversation exists
    let _conv = state
        .repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 500,
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Conversation not found".to_string(),
                    code: 404,
                }),
            )
        })?;

    // TODO: Add archive field to database
    // For now, just return success
    Ok(StatusCode::OK)
}

pub async fn delete_conversation(
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

pub async fn count_conversations(
    State(state): State<AppState>,
    Query(label): Query<String>,
) -> Json<serde_json::Value> {
    let count = state.repo.count_by_label(&label).await.unwrap_or(0);
    Json(serde_json::json!({ "count": count, "label": label }))
}

pub async fn semantic_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = req.limit.unwrap_or(10) as usize;

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
        .into_iter()
        .map(|r| SearchResultDto {
            conversation_id: r.conversation_id,
            message_id: r.message_id,
            score: r.score,
            content: r.content,
            metadata: r.metadata,
            label: r.label,
            folder: r.folder,
            timestamp: r.timestamp.to_string(),
        })
        .collect();

    let total = api_results.len() as u32;
    let page = 1;
    let page_size = limit as u32;

    Ok(Json(QueryResponse {
        results: api_results,
        total,
        page,
        page_size,
    }))
}

pub async fn rebuild_conversation_embeddings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Verify conversation exists
    let _conv = state
        .repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 500,
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Conversation not found".to_string(),
                    code: 404,
                }),
            )
        })?;

    // TODO: Add embedding queue mechanism
    tracing::info!(
        "Would enqueue embedding rebuild job for conversation {}",
        id
    );

    Ok(StatusCode::ACCEPTED)
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
        .route(
            "/api/v1/conversations/{id}/rebuild-embeddings",
            post(rebuild_conversation_embeddings),
        )
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}

pub async fn health() -> &'static str {
    "OK"
}

pub async fn metrics(State(state): State<AppState>) -> String {
    let count = state.repo.count_by_label("").await.unwrap_or(0);

    format!(
        "# HELP sekha_conversations_total Total number of conversations\n\
         # TYPE sekha_conversations_total gauge\n\
         sekha_conversations_total {}\n\
         # HELP sekha_up Whether the service is up\n\
         # TYPE sekha_up gauge\n\
         sekha_up 1\n",
        count
    )
}
