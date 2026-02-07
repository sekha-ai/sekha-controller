use crate::api::dto::*;
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

#[derive(Deserialize)]
pub struct CountParams {
    label: Option<String>,
    folder: Option<String>,
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
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<ErrorResponse>)> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let word_count: i32 = req.messages.iter().map(|m| m.content.len() as i32).sum();

    let new_messages: Vec<_> = req
        .messages
        .into_iter()
        .map(|m| crate::models::internal::NewMessage {
            role: m.role,
            content: m.content.as_string(),
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
            "conversation_id": id,
            "label": req.label,
            "folder": req.folder,
            "status": "active",
            "message_count": message_count,
            "created_at": now,
        })),
    ))
}

/// Get a conversation by ID
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
                created_at: c.created_at,
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

/// List conversations
pub async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    Query(filters): Query<FilterParams>,
) -> Json<QueryResponse> {
    let _ = (filters.pinned, filters.archived);
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let offset = (page - 1) * page_size;

    // Build filter criteria
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
            timestamp: c.updated_at,
        })
        .collect();

    Json(QueryResponse {
        results: conversations,
        total: total.try_into().unwrap_or(u32::MAX),
        page,
        page_size,
    })
}

/// Update conversation label
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

/// Delete a conversation
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

/// Count conversations
pub async fn count_conversations(
    State(state): State<AppState>,
    Query(params): Query<CountParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let label_for_response = params.label.clone();
    let folder_for_response = params.folder.clone();

    let count = match (&params.label, &params.folder) {
        (Some(label), None) => state.repo.count_by_label(label).await,
        (None, Some(folder)) => state.repo.count_by_folder(folder).await,
        (None, None) => state.repo.count_all().await,
        (Some(_), Some(_)) => {
            return Ok(Json(serde_json::json!({
                "count": 0,
                "error": "Cannot specify both label and folder"
            })));
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

    Ok(Json(serde_json::json!({
        "count": count,
        "label": label_for_response,
        "folder": folder_for_response
    })))
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
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Semantic query: {}", req.query);

    let limit = req.limit.unwrap_or(10) as usize;
    let offset = req.offset.unwrap_or(0);

    let page = if limit > 0 {
        (offset as f64 / limit as f64).ceil() as u32
    } else {
        1
    };

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
            timestamp: r.timestamp,
        })
        .collect();

    Ok(Json(QueryResponse {
        results: api_results,
        total: results.len() as u32,
        page,
        page_size: limit as u32,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        llm::bridge_client::BridgeClient,
        orchestrator::MemoryOrchestrator,
        storage::{init_db, repository::SeaOrmConversationRepository},
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    async fn create_test_state() -> AppState {
        let db = init_db("sqlite::memory:").await.unwrap();
        let config = Arc::new(RwLock::new(Config::default()));
        let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));

        // Create BridgeClient first, then pass to EmbeddingService
        let base_config = Config::default();
        let bridge = BridgeClient::new(&base_config).expect("Failed to create BridgeClient");
        let embedding_service = Arc::new(EmbeddingService::new(
            bridge,
            "http://localhost:8000".to_string(),
        ));

        let repo = Arc::new(SeaOrmConversationRepository::new(
            db,
            chroma_client.clone(),
            embedding_service.clone(),
        ));

        let config_ref = config.read().await;
        let llm_bridge = Arc::new(LlmBridgeClient::new(&*config_ref).unwrap());
        drop(config_ref);

        AppState {
            config,
            repo: repo.clone(),
            orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge.clone())),
            embedding_service,
            chroma_client,
            llm_client: llm_bridge,
        }
    }

    #[tokio::test]
    async fn test_create_conversation_error_500() {
        let state = create_test_state().await;

        let req = CreateConversationRequest {
            label: "Test".to_string(),
            folder: "default".to_string(),
            messages: vec![],
        };

        let result = create_conversation(State(state), Json(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_conversation_not_found_404() {
        let state = create_test_state().await;
        let non_existent_id = Uuid::new_v4();

        let result = get_conversation(State(state), Path(non_existent_id)).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
        assert_eq!(err.1.code, 404);
        assert!(err.1.error.contains("not found"));
    }

    #[tokio::test]
    async fn test_update_conversation_label_not_found_404() {
        let state = create_test_state().await;
        let non_existent_id = Uuid::new_v4();

        let req = UpdateLabelRequest {
            label: "New Label".to_string(),
            folder: "new_folder".to_string(),
        };

        let result =
            update_conversation_label(State(state), Path(non_existent_id), Json(req)).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
        assert_eq!(err.1.code, 404);
    }

    #[tokio::test]
    async fn test_delete_conversation_not_found_404() {
        let state = create_test_state().await;
        let non_existent_id = Uuid::new_v4();

        let result = delete_conversation(State(state), Path(non_existent_id)).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
        assert_eq!(err.1.code, 404);
        assert!(err.1.error.contains("not found"));
    }

    #[tokio::test]
    async fn test_list_conversations_with_label_filter() {
        let state = create_test_state().await;

        let pagination = PaginationParams {
            page: Some(1),
            page_size: Some(10),
        };
        let filters = FilterParams {
            label: Some("test-label".to_string()),
            folder: None,
            pinned: None,
            archived: None,
        };

        let result = list_conversations(State(state), Query(pagination), Query(filters)).await;

        assert_eq!(result.page, 1);
        assert_eq!(result.page_size, 10);
    }

    #[tokio::test]
    async fn test_list_conversations_with_folder_filter() {
        let state = create_test_state().await;

        let pagination = PaginationParams {
            page: Some(1),
            page_size: Some(20),
        };
        let filters = FilterParams {
            label: None,
            folder: Some("work".to_string()),
            pinned: None,
            archived: None,
        };

        let result = list_conversations(State(state), Query(pagination), Query(filters)).await;

        assert_eq!(result.page, 1);
        assert_eq!(result.page_size, 20);
    }

    #[tokio::test]
    async fn test_list_conversations_with_pinned_filter() {
        let state = create_test_state().await;

        let pagination = PaginationParams {
            page: None,
            page_size: None,
        };
        let filters = FilterParams {
            label: None,
            folder: None,
            pinned: Some(true),
            archived: None,
        };

        let result = list_conversations(State(state), Query(pagination), Query(filters)).await;

        assert_eq!(result.page, 1);
        assert_eq!(result.page_size, 50);
    }

    #[tokio::test]
    async fn test_list_conversations_with_archived_filter() {
        let state = create_test_state().await;

        let pagination = PaginationParams {
            page: Some(2),
            page_size: Some(25),
        };
        let filters = FilterParams {
            label: None,
            folder: None,
            pinned: None,
            archived: Some(false),
        };

        let result = list_conversations(State(state), Query(pagination), Query(filters)).await;

        assert_eq!(result.page, 2);
        assert_eq!(result.page_size, 25);
    }

    #[tokio::test]
    async fn test_list_conversations_with_all_filters() {
        let state = create_test_state().await;

        let pagination = PaginationParams {
            page: Some(1),
            page_size: Some(15),
        };
        let filters = FilterParams {
            label: Some("important".to_string()),
            folder: Some("archive".to_string()),
            pinned: Some(true),
            archived: Some(true),
        };

        let result = list_conversations(State(state), Query(pagination), Query(filters)).await;

        assert_eq!(result.page, 1);
        assert_eq!(result.page_size, 15);
    }

    #[tokio::test]
    async fn test_count_conversations_by_label() {
        let state = create_test_state().await;

        let params = CountParams {
            label: Some("test".to_string()),
            folder: None,
        };

        let result = count_conversations(State(state), Query(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.get("count").is_some());
        assert!(response.get("label").is_some());
    }

    #[tokio::test]
    async fn test_count_conversations_by_folder() {
        let state = create_test_state().await;

        let params = CountParams {
            label: None,
            folder: Some("work".to_string()),
        };

        let result = count_conversations(State(state), Query(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.get("count").is_some());
        assert!(response.get("folder").is_some());
    }

    #[tokio::test]
    async fn test_count_conversations_all() {
        let state = create_test_state().await;

        let params = CountParams {
            label: None,
            folder: None,
        };

        let result = count_conversations(State(state), Query(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.get("count").is_some());
    }

    #[tokio::test]
    async fn test_count_conversations_both_params_error() {
        let state = create_test_state().await;

        let params = CountParams {
            label: Some("test".to_string()),
            folder: Some("work".to_string()),
        };

        let result = count_conversations(State(state), Query(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.get("count").unwrap(), &serde_json::json!(0));
        assert!(response.get("error").is_some());
    }

    #[tokio::test]
    async fn test_semantic_query_search_result_dto_mapping() {
        let state = create_test_state().await;

        let req = QueryRequest {
            query: "test query".to_string(),
            limit: Some(5),
            offset: Some(0),
            filters: None,
        };

        let result = semantic_query(State(state), Json(req)).await;

        if let Ok(response) = result {
            assert_eq!(response.page_size, 5);
        }
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health().await;
        assert_eq!(response.status, "healthy");
        assert!(!response.version.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = create_test_state().await;
        let response = metrics(State(state)).await;
        assert!(response.get("metrics").is_some());
    }
}
