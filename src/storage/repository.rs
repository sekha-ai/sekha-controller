// At the very top of the file, BEFORE any imports
#[cfg(test)]
use mockall::automock;

use async_trait::async_trait;
use sea_orm::{
    prelude::*, DatabaseBackend, FromQueryResult, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, Statement, TransactionTrait, Value,
};
use serde_json::json;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

use crate::init_db;
use crate::models::internal::{Conversation, Message, NewConversation, NewMessage};
use crate::services::embedding_service::EmbeddingService;
use crate::storage::chroma_client::ChromaClient;
use crate::storage::entities::{conversations, messages};

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DbError(#[from] sea_orm::DbErr),
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Chroma error: {0}")]
    ChromaError(String),
    #[error("Embedding error: {0}")]
    EmbeddingError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

#[derive(Debug, serde::Serialize)]
pub struct Stats {
    pub total_conversations: usize,
    pub average_importance: f32,
    pub group_type: String,
    pub groups: Vec<String>,
}

// ============================================
// TRAIT DEFINITION - with automock
// ============================================
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn create(&self, conv: Conversation) -> Result<Uuid, RepositoryError>;
    async fn create_with_messages(&self, conv: NewConversation) -> Result<Uuid, RepositoryError>;
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    async fn count_by_label(&self, label: &str) -> Result<u64, RepositoryError>;
    async fn count_by_folder(&self, folder: &str) -> Result<u64, RepositoryError>;
    async fn count_all(&self) -> Result<u64, RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Conversation>, RepositoryError>;
    async fn find_by_label(
        &self,
        label: &str,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Conversation>, RepositoryError>;
    async fn get_conversation_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<Message>, RepositoryError>;
    async fn find_message_by_id(&self, id: Uuid) -> Result<Option<Message>, RepositoryError>;
    async fn find_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Message>, RepositoryError>;
    async fn find_with_filters(
        &self,
        filter: Option<String>,
        limit: usize,
        offset: u32,
    ) -> Result<(Vec<Conversation>, u64), RepositoryError>;
    async fn update_label(
        &self,
        id: Uuid,
        new_label: &str,
        new_folder: &str,
    ) -> Result<(), RepositoryError>;
    async fn get_message_list(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>;
    async fn get_stats(&self, folder: Option<String>) -> Result<Stats, Box<dyn std::error::Error>>;
    async fn get_stats_by_folder(
        &self,
        folder: Option<String>,
    ) -> Result<Stats, Box<dyn std::error::Error>>;
    async fn get_stats_by_label(
        &self,
        label: Option<String>,
    ) -> Result<Stats, Box<dyn std::error::Error>>;
    async fn get_all_folders(&self) -> Result<Vec<String>, RepositoryError>;
    async fn find_by_folder(
        &self,
        folder: &str,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Conversation>, RepositoryError>;
    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), RepositoryError>;
    async fn update_importance(&self, id: Uuid, score: i32) -> Result<(), RepositoryError>;
    async fn count_messages_in_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<u64, RepositoryError>;
    async fn full_text_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Message>, RepositoryError>;
    async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<JsonValue>,
    ) -> Result<Vec<SearchResult>, RepositoryError>;
    async fn get_all_labels(&self) -> Result<Vec<String>, RepositoryError>;
    fn get_db(&self) -> &DatabaseConnection;
}

// ============================================
// IMPLEMENTATION STRUCT
// ============================================
pub struct SeaOrmConversationRepository {
    db: DatabaseConnection,
    chroma: Arc<ChromaClient>,
    embedding_service: Arc<EmbeddingService>,
}

impl SeaOrmConversationRepository {
    pub fn new(
        db: DatabaseConnection,
        chroma: Arc<ChromaClient>,
        embedding_service: Arc<EmbeddingService>,
    ) -> Self {
        Self {
            db,
            chroma,
            embedding_service,
        }
    }
}

// ... rest of the implementation stays the same ...
// (I'm truncating to save space - the implementation code from lines 200-900+ remains identical)
