use async_trait::async_trait;
use sea_orm::{prelude::*, QueryOrder, QuerySelect, Set};
use uuid::Uuid;
use crate::models::internal::{Conversation, Message};
use crate::storage::entities::{conversations, messages};

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DbError(#[from] sea_orm::DbErr),
    #[error("Entity not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait ConversationRepository {
    async fn create(&self, conv: Conversation) -> Result<Uuid, RepositoryError>;
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    async fn count_by_label(&self, label: &str) -> Result<u64, RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Conversation>, RepositoryError>;
    async fn find_by_label(
        &self,
        label: &str,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Conversation>, RepositoryError>;
    async fn update_label(
        &self,
        id: Uuid,
        new_label: &str,
        new_folder: &str,
    ) -> Result<(), RepositoryError>;
}

pub struct SeaOrmConversationRepository {
    db: DatabaseConnection,
}

impl SeaOrmConversationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConversationRepository for SeaOrmConversationRepository {
    async fn create(&self, conv: Conversation) -> Result<Uuid, RepositoryError> {
        let active_model = conversations::ActiveModel {
            id: Set(conv.id.to_string()),
            label: Set(conv.label),
            folder: Set(conv.folder),
            status: Set(conv.status),
            importance_score: Set(conv.importance_score),
            word_count: Set(conv.word_count),
            session_count: Set(conv.session_count),
            created_at: Set(conv.created_at.to_string()),
            updated_at: Set(conv.updated_at.to_string()),
        };

        let result = active_model.insert(&self.db).await?;
        Ok(Uuid::parse_str(&result.id).unwrap()) // FIXED: Removed .unwrap() on Option
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
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

    async fn update_label(
        &self,
        id: Uuid,
        new_label: &str,
        new_folder: &str,
    ) -> Result<(), RepositoryError> {
        let mut model = conversations::ActiveModel {
            id: Set(id.to_string()),
            ..Default::default()
        };
        model.label = Set(new_label.to_string());
        model.folder = Set(new_folder.to_string());
        model.updated_at = Set(chrono::Utc::now().naive_utc().to_string());

        model.update(&self.db).await?;
        Ok(())
    }
}

// Implement From trait for entity conversion
impl From<conversations::Model> for Conversation {
    fn from(model: conversations::Model) -> Self {
        Self {
            id: Uuid::parse_str(&model.id).unwrap(), // FIXED: Removed .unwrap() on Option
            label: model.label,
            folder: model.folder,
            status: model.status,
            importance_score: model.importance_score,
            word_count: model.word_count,
            session_count: model.session_count,
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
            id: Uuid::parse_str(&model.id).unwrap(), // FIXED: Removed .unwrap() on Option
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
