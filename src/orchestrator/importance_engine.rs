use crate::models::internal::Message;
use crate::services::llm_bridge_client::LlmBridgeClient;
use crate::storage::repository::{ConversationRepository, RepositoryError};
use std::sync::Arc;
use uuid::Uuid;

pub struct ImportanceEngine {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    llm_bridge: Arc<LlmBridgeClient>,
}

impl ImportanceEngine {
    pub fn new(
        repo: Arc<dyn ConversationRepository + Send + Sync>,
        llm_bridge: Arc<LlmBridgeClient>,
    ) -> Self {
        Self { repo, llm_bridge }
    }

    pub async fn calculate_score(&self, message_id: Uuid) -> Result<f32, RepositoryError> {
        // Fetch message
        let message = self
            .repo
            .find_message_by_id(message_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound("Message not found".to_string()))?;

        // Heuristic score
        let heuristic_score = self.heuristic_score(&message);

        // LLM score
        let llm_score = self
            .llm_bridge
            .score_importance(&message.content, None, None)
            .await
            .map_err(|e| RepositoryError::EmbeddingError(format!("LLM Bridge error: {}", e)))?;

        // Weighted average
        Ok((heuristic_score * 0.3) + (llm_score * 0.7))
    }

    fn heuristic_score(&self, message: &Message) -> f32 {
        let mut score: f32 = 5.0;

        // Length bonus
        if message.content.len() > 100 {
            score += 1.0;
        }

        // Code blocks
        if message.content.contains("```") {
            score += 2.0;
        }

        // Questions
        if message.content.ends_with("?") {
            score += 0.5;
        }

        // Keywords
        let important_words = ["critical", "important", "urgent", "decision"];
        for word in important_words {
            if message.content.to_lowercase().contains(word) {
                score += 1.0;
            }
        }

        score.min(10.0).max(1.0)
    }
}
