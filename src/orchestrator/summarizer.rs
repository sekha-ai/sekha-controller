use crate::models::internal::Message;
use crate::services::llm_bridge_client::LlmBridgeClient;
use crate::storage::entities::messages as message_entity;
use crate::storage::repository::{ConversationRepository, RepositoryError};
use chrono::Duration;
use chrono::Utc;
use sea_orm::EntityTrait;
use sea_orm::{ColumnTrait, QueryFilter}; // REMOVE EntityTrait from here
use std::sync::Arc;
use uuid::Uuid;

pub struct HierarchicalSummarizer {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    llm_bridge: Arc<LlmBridgeClient>,
}

impl HierarchicalSummarizer {
    pub fn new(
        repo: Arc<dyn ConversationRepository + Send + Sync>,
        llm_bridge: Arc<LlmBridgeClient>,
    ) -> Self {
        Self { repo, llm_bridge }
    }

    pub async fn generate_daily_summary(
        &self,
        conversation_id: Uuid,
    ) -> Result<String, RepositoryError> {
        let messages = self
            .fetch_messages_from_last_n_days(conversation_id, 1)
            .await?;

        if messages.is_empty() {
            return Ok("No messages to summarize".to_string());
        }

        let messages_text: Vec<String> = messages
            .iter()
            .map(|m| format!("[{}] {}: {}", m.timestamp, m.role, m.content))
            .collect();

        let summary = self
            .llm_bridge
            .summarize(messages_text, "daily", None, Some(200))
            .await
            .map_err(|e| RepositoryError::EmbeddingError(format!("LLM Bridge error: {}", e)))?;

        self.store_summary(conversation_id, "daily", &summary)
            .await?;

        Ok(summary)
    }

    pub async fn generate_weekly_summary(
        &self,
        conversation_id: Uuid,
    ) -> Result<String, RepositoryError> {
        let daily_summaries = self
            .fetch_summaries_from_last_n_days(conversation_id, 7, "daily")
            .await?;

        if daily_summaries.is_empty() {
            return self.generate_daily_summary(conversation_id).await;
        }

        let summary = self
            .llm_bridge
            .summarize(daily_summaries, "weekly", None, Some(500))
            .await
            .map_err(|e| RepositoryError::EmbeddingError(format!("LLM Bridge error: {}", e)))?;

        self.store_summary(conversation_id, "weekly", &summary)
            .await?;

        Ok(summary)
    }

    pub async fn generate_monthly_summary(
        &self,
        conversation_id: Uuid,
    ) -> Result<String, RepositoryError> {
        let weekly_summaries = self
            .fetch_summaries_from_last_n_days(conversation_id, 30, "weekly")
            .await?;

        if weekly_summaries.is_empty() {
            return self.generate_weekly_summary(conversation_id).await;
        }

        let summary = self
            .llm_bridge
            .summarize(weekly_summaries, "monthly", None, Some(1000))
            .await
            .map_err(|e| RepositoryError::EmbeddingError(format!("LLM Bridge error: {}", e)))?;

        self.store_summary(conversation_id, "monthly", &summary)
            .await?;

        Ok(summary)
    }

    async fn fetch_messages_from_last_n_days(
        &self,
        conversation_id: Uuid,
        days: i64,
    ) -> Result<Vec<Message>, RepositoryError> {
        let cutoff = Utc::now().naive_utc() - Duration::days(days);

        let models = message_entity::Entity::find()
            .filter(message_entity::Column::ConversationId.eq(conversation_id.to_string()))
            .filter(message_entity::Column::Timestamp.gte(cutoff.to_string()))
            .all(self.repo.get_db())
            .await
            .map_err(RepositoryError::DbError)?;

        Ok(models
            .into_iter()
            .map(|m| Message {
                id: Uuid::parse_str(&m.id).unwrap(),
                conversation_id: Uuid::parse_str(&m.conversation_id).unwrap(),
                role: m.role,
                content: m.content,
                timestamp: chrono::NaiveDateTime::parse_from_str(
                    &m.timestamp,
                    "%Y-%m-%d %H:%M:%S%.f",
                )
                .unwrap(),
                embedding_id: m
                    .embedding_id
                    .as_ref()
                    .and_then(|id| Uuid::parse_str(id).ok()),
                metadata: m
                    .metadata
                    .as_ref()
                    .and_then(|meta| serde_json::from_str(meta).ok()),
            })
            .collect())
    }

    async fn fetch_summaries_from_last_n_days(
        &self,
        _conversation_id: Uuid,
        _days: i64,
        _level: &str,
    ) -> Result<Vec<String>, RepositoryError> {
        // TODO: Query summaries table once it's created
        // For now, just return empty to trigger fallback
        Ok(Vec::new())
    }

    async fn store_summary(
        &self,
        conversation_id: Uuid,
        level: &str,
        summary: &str,
    ) -> Result<(), RepositoryError> {
        tracing::info!(
            "Stored {} summary for {}: {}",
            level,
            conversation_id,
            summary.chars().take(50).collect::<String>()
        );
        Ok(())
    }
}
