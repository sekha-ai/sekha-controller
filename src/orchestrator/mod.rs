pub mod context_assembly;
// pub mod importance_engine;
// pub mod summarizer;
// pub mod pruning_engine;
// pub mod label_intelligence;

use std::sync::Arc;
use uuid::Uuid; // Keep this import
use crate::storage::repository::ConversationRepository;
use crate::models::internal::Message;

pub struct MemoryOrchestrator {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    pub context_assembler: context_assembly::ContextAssembler,
    // pub importance_engine: importance_engine::ImportanceEngine,
    // pub summarizer: summarizer::HierarchicalSummarizer,
    // pub pruning_engine: pruning_engine::PruningEngine,
    // pub label_intelligence: label_intelligence::LabelIntelligence,
}

impl MemoryOrchestrator {
    pub fn new(repo: Arc<dyn ConversationRepository + Send + Sync>) -> Self {
        Self {
            repo: repo.clone(),
            context_assembler: context_assembly::ContextAssembler::new(repo.clone()),
            // importance_engine: importance_engine::ImportanceEngine::new(repo.clone()),
            // summarizer: summarizer::HierarchicalSummarizer::new(repo.clone()),
            // pruning_engine: pruning_engine::PruningEngine::new(repo.clone()),
            // label_intelligence: label_intelligence::LabelIntelligence::new(repo.clone()),
        }
    }

    /// High-level API: Assemble context for a query
    pub async fn assemble_context(
        &self,
        query: &str,
        preferred_labels: Vec<String>,
        context_budget: usize,
    ) -> Result<Vec<Message>, crate::storage::repository::RepositoryError> {
        self.context_assembler.assemble(query, preferred_labels, context_budget).await
    }

    // Commented until implemented:
    // pub async fn score_message_importance(
    //     &self,
    //     message_id: Uuid,
    // ) -> Result<f32, crate::storage::repository::RepositoryError> {
    //     self.importance_engine.calculate_score(message_id).await
    // }

    // pub async fn suggest_pruning(
    //     &self,
    //     threshold_days: i64,
    // ) -> Result<Vec<pruning_engine::PruningSuggestion>, crate::storage::repository::RepositoryError> {
    //     self.pruning_engine.generate_suggestions(threshold_days).await
    // }
}