pub mod context_assembly;
pub mod importance_engine;
pub mod label_intelligence;
pub mod pruning_engine;
pub mod summarizer;

use crate::models::internal::Message;
use crate::services::llm_bridge_client::LlmBridgeClient;
use crate::storage::repository::{ConversationRepository, RepositoryError};
use std::sync::Arc;
use uuid::Uuid;

pub struct MemoryOrchestrator {
    #[allow(dead_code)] // Used in future methods
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    pub context_assembler: context_assembly::ContextAssembler,
    pub importance_engine: importance_engine::ImportanceEngine,
    pub summarizer: summarizer::HierarchicalSummarizer,
    pub pruning_engine: pruning_engine::PruningEngine,
    pub label_intelligence: label_intelligence::LabelIntelligence,
}

impl MemoryOrchestrator {
    pub fn new(
        repo: Arc<dyn ConversationRepository + Send + Sync>,
        llm_bridge: Arc<LlmBridgeClient>,
    ) -> Self {
        Self {
            repo: repo.clone(),
            context_assembler: context_assembly::ContextAssembler::new(repo.clone()),
            importance_engine: importance_engine::ImportanceEngine::new(
                repo.clone(),
                llm_bridge.clone(),
            ),
            summarizer: summarizer::HierarchicalSummarizer::new(repo.clone(), llm_bridge.clone()),
            pruning_engine: pruning_engine::PruningEngine::new(repo.clone(), llm_bridge.clone()),
            label_intelligence: label_intelligence::LabelIntelligence::new(
                repo.clone(),
                llm_bridge.clone(),
            ),
        }
    }

    pub async fn assemble_context(
        &self,
        query: &str,
        preferred_labels: Vec<String>,
        context_budget: usize,
        excluded_folders: Vec<String>,
    ) -> Result<Vec<Message>, RepositoryError> {
        self.context_assembler
            .assemble(query, preferred_labels, context_budget, excluded_folders)
            .await
    }

    pub async fn score_message_importance(&self, message_id: Uuid) -> Result<f32, RepositoryError> {
        self.importance_engine.calculate_score(message_id).await
    }

    pub async fn generate_daily_summary(
        &self,
        conversation_id: Uuid,
    ) -> Result<String, RepositoryError> {
        self.summarizer
            .generate_daily_summary(conversation_id)
            .await
    }

    pub async fn suggest_pruning(
        &self,
        threshold_days: i64,
    ) -> Result<Vec<pruning_engine::PruningSuggestion>, RepositoryError> {
        self.pruning_engine
            .generate_suggestions(threshold_days, 3.0)
            .await
    }

    pub async fn suggest_labels(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<label_intelligence::LabelSuggestion>, RepositoryError> {
        self.label_intelligence
            .suggest_labels(conversation_id)
            .await
    }
}
