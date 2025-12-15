use sea_orm::EntityTrait;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use chrono::Duration;
use crate::storage::repository::{ConversationRepository, RepositoryError};
use crate::models::internal::Conversation;
use crate::services::llm_bridge_client::LlmBridgeClient;

pub struct PruningEngine {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    llm_bridge: Arc<LlmBridgeClient>,
}

impl PruningEngine {
    pub fn new(
        repo: Arc<dyn ConversationRepository + Send + Sync>,
        llm_bridge: Arc<LlmBridgeClient>,
    ) -> Self {
        Self { repo, llm_bridge }
    }

    pub async fn generate_suggestions(
        &self,
        threshold_days: i64,
        importance_threshold: f32,
    ) -> Result<Vec<PruningSuggestion>, RepositoryError> {
        let cutoff = Utc::now().naive_utc() - Duration::days(threshold_days);
        
        let candidates = self.find_pruning_candidates(cutoff, importance_threshold).await?;
        
        let mut suggestions = Vec::new();
        
        for conv in candidates {
            let suggestion = self.generate_suggestion_for_conversation(&conv).await?;
            suggestions.push(suggestion);
        }
        
        Ok(suggestions)
    }

    async fn find_pruning_candidates(
        &self,
        cutoff: chrono::NaiveDateTime,
        _importance_threshold: f32,
    ) -> Result<Vec<Conversation>, RepositoryError> {
        use crate::storage::entities::conversations;
        use sea_orm::{QueryFilter, ColumnTrait};
        
        let models = conversations::Entity::find()
            .filter(conversations::Column::UpdatedAt.lt(cutoff.to_string()))
            .filter(conversations::Column::Status.eq("active"))
            .all(self.repo.get_db())
            .await
            .map_err(RepositoryError::DbError)?;
        
        Ok(models.into_iter().map(Conversation::from).collect())
    }

    async fn generate_suggestion_for_conversation(
        &self,
        conv: &Conversation,
    ) -> Result<PruningSuggestion, RepositoryError> {
        let message_count = self.repo.count_messages_in_conversation(conv.id).await?;
        let token_estimate = message_count * 200;
        
        let preview = self.generate_preview(conv).await?;
        
        let suggestion = PruningSuggestion {
            conversation_id: conv.id,
            conversation_label: conv.label.clone(),
            last_accessed: conv.updated_at,
            message_count,
            token_estimate: token_estimate as u32,
            importance_score: conv.importance_score as f32,
            preview,
            recommendation: if token_estimate > 5000 && conv.importance_score < 5 {
                "archive".to_string()
            } else {
                "keep".to_string()
            },
        };
        
        Ok(suggestion)
    }

    async fn generate_preview(&self, conv: &Conversation) -> Result<String, RepositoryError> {
        let recent_messages = self.repo.find_recent_messages(conv.id, 5).await?;
        
        let messages_text: Vec<String> = recent_messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content.chars().take(100).collect::<String>()))
            .collect();
        
        let prompt = format!(
            "Summarize what would be lost if this conversation were archived. \
            Focus on unique information, decisions, or context that might be needed later.\n\n\
            Conversation: {}\n\
            Recent messages:\n{}",
            conv.label,
            messages_text.join("\n")
        );
        
        let preview = self.llm_bridge.summarize(
            vec![prompt],
            "daily",
            None,
            Some(50),
        ).await.map_err(|e| {
            RepositoryError::EmbeddingError(format!("LLM Bridge error: {}", e))
        })?;
        
        Ok(preview)
    }
}

#[derive(Debug, Clone)]
pub struct PruningSuggestion {
    pub conversation_id: Uuid,
    pub conversation_label: String,
    pub last_accessed: chrono::NaiveDateTime,
    pub message_count: u64,
    pub token_estimate: u32,
    pub importance_score: f32,
    pub preview: String,
    pub recommendation: String,
}