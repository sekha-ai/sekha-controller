use async_trait::async_trait;
use sea_orm::{prelude::*, QueryOrder, QuerySelect, Set};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

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

// ============================================
// TRAIT DEFINITION - with Send + Sync bounds
// ============================================
#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn create(&self, conv: Conversation) -> Result<Uuid, RepositoryError>;
    async fn create_with_messages(&self, conv: NewConversation) -> Result<Uuid, RepositoryError>;
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    async fn count_by_label(&self, label: &str) -> Result<u64, RepositoryError>;
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
        filters: Option<Value>,
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

// ============================================
// TRAIT IMPLEMENTATION
// ============================================
#[async_trait]
impl ConversationRepository for SeaOrmConversationRepository {
    fn get_db(&self) -> &DatabaseConnection {
        &self.db
    }

    async fn create(&self, conv: Conversation) -> Result<Uuid, RepositoryError> {
        let active_model = conversations::ActiveModel {
            id: Set(conv.id.to_string()),
            label: Set(conv.label),
            folder: Set(conv.folder),
            status: Set(conv.status),
            importance_score: Set(conv.importance_score as i64),
            word_count: Set(conv.word_count as i64),
            session_count: Set(conv.session_count as i64),
            created_at: Set(conv.created_at.to_string()),
            updated_at: Set(conv.updated_at.to_string()),
        };

        let result = active_model.insert(&self.db).await?;
        Ok(Uuid::parse_str(&result.id).unwrap())
    }

    async fn create_with_messages(&self, conv: NewConversation) -> Result<Uuid, RepositoryError> {
        let conv_id = conv.id.unwrap_or_else(Uuid::new_v4);
        let now = chrono::Utc::now().naive_utc();

        let word_count: i64 = conv.messages.iter().map(|m| m.content.len() as i64).sum();

        // Store conversation in SQLite
        let conversation = conversations::ActiveModel {
            id: Set(conv_id.to_string()),
            label: Set(conv.label.clone()),
            folder: Set(conv.folder.clone()),
            status: Set("active".to_string()),
            importance_score: Set(5i64),
            word_count: Set(word_count),
            session_count: Set(conv.session_count.unwrap_or(1) as i64),
            created_at: Set(now.to_string()),
            updated_at: Set(now.to_string()),
        };

        conversation.insert(&self.db).await?;
        tracing::info!("Created conversation: {}", conv_id);

        // Store messages with embeddings
        for msg in conv.messages {
            self.create_message(conv_id, msg).await?;
        }

        Ok(conv_id)
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        // Also delete from Chroma
        if let Ok(Some(_conv)) = self.find_by_id(id).await {
            // Find and delete all message embeddings
            let messages = messages::Entity::find()
                .filter(messages::Column::ConversationId.eq(id.to_string()))
                .all(&self.db)
                .await?;

            let embedding_ids: Vec<String> = messages
                .into_iter()
                .filter_map(|m| m.embedding_id)
                .collect();

            if !embedding_ids.is_empty() {
                self.chroma.delete("messages", embedding_ids).await?;
            }
        }

        conversations::Entity::delete_by_id(id.to_string())
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn count_by_label(&self, label: &str) -> Result<u64, RepositoryError> {
        let count = conversations::Entity::find()
            .filter(conversations::Column::Label.eq(label))
            .count(&self.db)
            .await?;
        Ok(count)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Conversation>, RepositoryError> {
        let model = conversations::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await?;

        Ok(model.map(Conversation::from))
    }

    async fn find_message_by_id(&self, id: Uuid) -> Result<Option<Message>, RepositoryError> {
        let model = messages::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await?;

        Ok(model.map(Message::from))
    }

    async fn find_by_label(
        &self,
        label: &str,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Conversation>, RepositoryError> {
        let models = conversations::Entity::find()
            .filter(conversations::Column::Label.eq(label))
            .order_by_desc(conversations::Column::UpdatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;

        Ok(models.into_iter().map(Conversation::from).collect())
    }

    // NEW: Find with filters (implementation)
    async fn find_with_filters(
        &self,
        filter: Option<String>,
        limit: usize,
        offset: u32,
    ) -> Result<(Vec<Conversation>, u64), RepositoryError> {
        let mut query = conversations::Entity::find();

        // Apply filter if provided
        if let Some(filter_sql) = filter {
            // Use filter condition
            query = query.filter(conversations::Column::Label.contains(filter_sql.as_str()));
        }

        // Get total count
        let total = query.clone().count(&self.db).await?;

        // Apply pagination
        let results = query
            .order_by_desc(conversations::Column::UpdatedAt)
            .limit(limit as u64)
            .offset(offset as u64)
            .all(&self.db)
            .await?;

        Ok((results.into_iter().map(Conversation::from).collect(), total))
    }

    async fn update_label(
        &self,
        id: Uuid,
        new_label: &str,
        new_folder: &str,
    ) -> Result<(), RepositoryError> {
        let model = conversations::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| RepositoryError::NotFound("Conversation not found".to_string()))?;

        let mut active_model: conversations::ActiveModel = model.into();
        active_model.label = Set(new_label.to_string());
        active_model.folder = Set(new_folder.to_string());
        active_model.updated_at = Set(chrono::Utc::now().naive_utc().to_string());

        active_model.update(&self.db).await?;
        Ok(())
    }

    async fn get_all_labels(&self) -> Result<Vec<String>, RepositoryError> {
        use sea_orm::QuerySelect;

        let labels = conversations::Entity::find()
            .select_only()
            .column(conversations::Column::Label)
            .distinct()
            .order_by_asc(conversations::Column::Label)
            .into_tuple::<String>()
            .all(&self.db)
            .await?;

        Ok(labels)
    }

    async fn get_conversation_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<Message>, RepositoryError> {
        let msg_models = messages::Entity::find()
            .filter(messages::Column::ConversationId.eq(conversation_id.to_string()))
            .order_by_asc(messages::Column::Timestamp)
            .all(&self.db)
            .await?;

        Ok(msg_models.into_iter().map(Message::from).collect())
    }

    async fn find_recent_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Message>, RepositoryError> {
        use crate::storage::entities::messages;
        use sea_orm::{QueryOrder, QuerySelect};

        let models = messages::Entity::find()
            .filter(messages::Column::ConversationId.eq(conversation_id.to_string()))
            .order_by_desc(messages::Column::Timestamp)
            .limit(limit as u64)
            .all(&self.db)
            .await
            .map_err(RepositoryError::DbError)?;

        Ok(models.into_iter().map(Message::from).collect())
    }

    // NEW: Update status
    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), RepositoryError> {
        let model = conversations::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| RepositoryError::NotFound("Conversation not found".to_string()))?;

        let mut active_model: conversations::ActiveModel = model.into();
        active_model.status = Set(status.to_string());
        active_model.updated_at = Set(chrono::Utc::now().naive_utc().to_string());

        active_model.update(&self.db).await?;
        Ok(())
    }

    // NEW: Update importance
    async fn update_importance(&self, id: Uuid, score: i32) -> Result<(), RepositoryError> {
        let model = conversations::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| RepositoryError::NotFound("Conversation not found".to_string()))?;

        let mut active_model: conversations::ActiveModel = model.into();
        active_model.importance_score = Set(score as i64);
        active_model.updated_at = Set(chrono::Utc::now().naive_utc().to_string());

        active_model.update(&self.db).await?;
        Ok(())
    }

    // NEW: Count messages in conversation
    async fn count_messages_in_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<u64, RepositoryError> {
        let count = messages::Entity::find()
            .filter(messages::Column::ConversationId.eq(conversation_id.to_string()))
            .count(&self.db)
            .await?;
        Ok(count)
    }

    pub async fn full_text_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Message>, RepositoryError> {
        let sql = r#"
            SELECT m.* FROM messages_fts fts
            JOIN messages m ON fts.rowid = m.id
            WHERE messages_fts MATCH ?
            ORDER BY rank
            LIMIT ?
        "#;

        let messages = Message::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            vec![query.into(), (limit as i64).into()],
        ))
        .all(&self.db)
        .await?;

        Ok(messages.into_iter().map(Message::from).collect())
    }

    async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<Value>,
    ) -> Result<Vec<SearchResult>, RepositoryError> {
        let chroma_results = self
            .embedding_service
            .search_messages(query, limit, filters)
            .await
            .map_err(|e| RepositoryError::ChromaError(e.to_string()))?;

        let mut results = Vec::new();

        for scored in chroma_results {
            if let Ok(msg_id) = Uuid::parse_str(&scored.id) {
                // Fetch message and conversation data from SQLite
                if let Some(message) = messages::Entity::find_by_id(msg_id.to_string())
                    .one(&self.db)
                    .await?
                {
                    if let Some(conversation) =
                        conversations::Entity::find_by_id(message.conversation_id.clone())
                            .one(&self.db)
                            .await?
                    {
                        results.push(SearchResult {
                            conversation_id: Uuid::parse_str(&conversation.id).unwrap(),
                            message_id: msg_id,
                            score: scored.score,
                            content: message.content,
                            metadata: scored.metadata,
                            label: conversation.label,
                            folder: conversation.folder,
                            timestamp: chrono::NaiveDateTime::parse_from_str(
                                &message.timestamp,
                                "%Y-%m-%d %H:%M:%S%.f",
                            )
                            .unwrap(),
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

// ============================================
// Helper: Create message with embedding
// ============================================
impl SeaOrmConversationRepository {
    async fn create_message(
        &self,
        conversation_id: Uuid,
        new_msg: NewMessage,
    ) -> Result<Uuid, RepositoryError> {
        let msg_id = Uuid::new_v4();
        let now = chrono::Utc::now().naive_utc();

        // Try to generate embedding, but don't fail if Ollama/Chroma unavailable
        let embedding_id = match self
            .embedding_service
            .process_message(
                msg_id,
                &new_msg.content,
                conversation_id,
                serde_json::json!({
                    "role": new_msg.role.clone(),
                    "conversation_id": conversation_id.to_string(),
                    "timestamp": now.to_string(),
                }),
            )
            .await
        {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!("Embedding generation failed (ok in tests): {}", e);
                None
            }
        };

        let has_embedding = embedding_id.is_some();

        // Store message in SQLite (with or without embedding_id)
        let message = messages::ActiveModel {
            id: Set(msg_id.to_string()),
            conversation_id: Set(conversation_id.to_string()),
            role: Set(new_msg.role),
            content: Set(new_msg.content),
            timestamp: Set(now.to_string()),
            embedding_id: Set(embedding_id),
            metadata: Set(Some(new_msg.metadata.to_string())),
        };

        message.insert(&self.db).await?;
        tracing::debug!(
            "Stored message{}: {}",
            if has_embedding { " with embedding" } else { "" },
            msg_id
        );

        Ok(msg_id)
    }
}

// ============================================
// Data structures
// ============================================

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub score: f32,
    pub content: String,
    pub metadata: Value,
    pub label: String,
    pub folder: String,
    pub timestamp: chrono::NaiveDateTime,
}

// ============================================
// Conversions
// ============================================

impl From<conversations::Model> for Conversation {
    fn from(model: conversations::Model) -> Self {
        Self {
            id: Uuid::parse_str(&model.id).unwrap(),
            label: model.label,
            folder: model.folder,
            status: model.status,
            importance_score: model.importance_score as i32,
            word_count: model.word_count as i32,
            session_count: model.session_count as i32,
            created_at: chrono::NaiveDateTime::parse_from_str(
                &model.created_at,
                "%Y-%m-%d %H:%M:%S%.f",
            )
            .unwrap(),
            updated_at: chrono::NaiveDateTime::parse_from_str(
                &model.updated_at,
                "%Y-%m-%d %H:%M:%S%.f",
            )
            .unwrap(),
        }
    }
}

impl From<messages::Model> for Message {
    fn from(model: messages::Model) -> Self {
        Self {
            id: Uuid::parse_str(&model.id).unwrap(),
            conversation_id: Uuid::parse_str(&model.conversation_id).unwrap(),
            role: model.role,
            content: model.content,
            timestamp: chrono::NaiveDateTime::parse_from_str(
                &model.timestamp,
                "%Y-%m-%d %H:%M:%S%.f",
            )
            .unwrap(),
            embedding_id: model.embedding_id.map(|id| Uuid::parse_str(&id).unwrap()),
            metadata: model.metadata,
        }
    }
}

// Error conversion
impl From<crate::storage::chroma_client::ChromaError> for RepositoryError {
    fn from(err: crate::storage::chroma_client::ChromaError) -> Self {
        RepositoryError::ChromaError(err.to_string())
    }
}
