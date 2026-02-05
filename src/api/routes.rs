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

// ... (rest of the routes.rs file is unchanged, I'll include just the beginning to save space)

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

// ... (rest remains unchanged)