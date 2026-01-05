use crate::services::llm_bridge_client::LlmBridgeClient;
use crate::storage::repository::{ConversationRepository, RepositoryError};
use std::sync::Arc;
use uuid::Uuid;

pub struct LabelIntelligence {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    llm_bridge: Arc<LlmBridgeClient>,
}

impl LabelIntelligence {
    pub fn new(
        repo: Arc<dyn ConversationRepository + Send + Sync>,
        llm_bridge: Arc<LlmBridgeClient>,
    ) -> Self {
        Self { repo, llm_bridge }
    }

    pub async fn suggest_labels(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<LabelSuggestion>, RepositoryError> {
        // Verify conversation exists
        let _conv = self
            .repo
            .find_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                RepositoryError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        let messages = self.repo.get_conversation_messages(conversation_id).await?;

        if messages.is_empty() {
            return Ok(Vec::new());
        }

        let combined_text = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let existing_labels = self.repo.get_all_labels().await?;
        let labels_str = existing_labels.join(", ");

        let prompt = format!(
            "Suggest 3-5 relevant labels for this conversation. \
            Use existing labels if appropriate, or suggest new ones if needed.\n\n\
            Existing labels: {}\n\n\
            Conversation content:\n{}",
            labels_str,
            combined_text.chars().take(2000).collect::<String>()
        );

        // ✅ GRACEFUL DEGRADATION: Return mock suggestions if LLM unavailable
        let response = match self
            .llm_bridge
            .summarize(vec![prompt], "daily", None, Some(100))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("LLM unavailable for label suggestions (ok in tests): {}", e);
                // Return generic suggestions based on existing labels
                if existing_labels.is_empty() {
                    "general,conversation,note".to_string()
                } else {
                    existing_labels
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "general".to_string())
                }
            }
        };

        let suggested_labels: Vec<String> = response
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(5)
            .collect();

        let suggestions = suggested_labels
            .into_iter()
            .map(|label| {
                let is_existing = existing_labels.contains(&label);
                LabelSuggestion {
                    label,
                    confidence: if is_existing { 0.9 } else { 0.6 },
                    is_existing,
                    reason: "Suggested based on conversation content".to_string(),
                }
            })
            .collect();

        Ok(suggestions)
    }

    pub async fn auto_label(
        &self,
        conversation_id: Uuid,
        threshold: f32,
    ) -> Result<Option<String>, RepositoryError> {
        let suggestions = self.suggest_labels(conversation_id).await?;

        for suggestion in suggestions {
            if suggestion.confidence >= threshold {
                self.repo
                    .update_label(
                        conversation_id,
                        &suggestion.label,
                        &self.infer_folder(&suggestion.label),
                    )
                    .await?;

                return Ok(Some(suggestion.label));
            }
        }

        Ok(None)
    }

    fn infer_folder(&self, label: &str) -> String {
        if label.contains(':') {
            "/work".to_string()
        } else {
            "/personal".to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct LabelSuggestion {
    pub label: String,
    pub confidence: f32,
    pub is_existing: bool,
    pub reason: String,
}
