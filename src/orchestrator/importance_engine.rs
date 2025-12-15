pub struct ImportanceEngine {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
    // llm_bridge: Arc<dyn LlmBridge>, // Add in Module 6
}

impl ImportanceEngine {
    pub async fn calculate_score(&self, message_id: Uuid) -> Result<f32, RepositoryError> {
        // For now: heuristic only
        // TODO: Call LLM Bridge in Module 6 for LLM scoring
        self.heuristic_score(message_id).await
    }
}