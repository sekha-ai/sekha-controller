use crate::models::internal::Message;
use crate::storage::repository::{ConversationRepository, RepositoryError};
use chrono::NaiveDateTime;
use sea_orm::EntityTrait;
use std::sync::Arc;
use uuid::Uuid;

pub struct ContextAssembler {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
}

impl ContextAssembler {
    pub fn new(repo: Arc<dyn ConversationRepository + Send + Sync>) -> Self {
        Self { repo }
    }

    /// 4-phase context assembly algorithm
    pub async fn assemble(
        &self,
        query: &str,
        preferred_labels: Vec<String>,
        context_budget: usize,
    ) -> Result<Vec<Message>, RepositoryError> {
        // Phase 1: Recall - Get candidate messages
        let candidates = self.recall_candidates(query, &preferred_labels).await?;

        // Phase 2: Ranking - Score each candidate
        let mut ranked = self
            .rank_candidates(candidates, query, &preferred_labels)
            .await?;

        // Phase 3: Assembly - Build context window within budget
        let context = self.assemble_context(&mut ranked, context_budget).await?;

        // Phase 4: Enhancement - Add citations and summaries
        let enhanced_context = self.enhance_context(context).await?;

        Ok(enhanced_context)
    }

    /// Phase 1: Recall - Semantic search + pinned + recent
    async fn recall_candidates(
        &self,
        query: &str,
        preferred_labels: &[String],
    ) -> Result<Vec<CandidateMessage>, RepositoryError> {
        let mut candidates = Vec::new();

        // 1. Semantic search from Chroma (top 200)
        let semantic_results = self.repo.semantic_search(query, 200, None).await?;
        for result in semantic_results {
            candidates.push(CandidateMessage {
                message_id: result.message_id,
                conversation_id: result.conversation_id,
                score: result.score,
                timestamp: result.timestamp,
                label: result.label,
                is_pinned: false,
                importance: 5.0, // Default, will be refined
            });
        }

        // 2. Add pinned conversations (always included)
        let pinned = self.get_pinned_messages().await?;
        candidates.extend(pinned);

        // 3. Add recent messages from preferred labels (last 7 days)
        let recent = self
            .get_recent_labeled_messages(preferred_labels, 7)
            .await?;
        candidates.extend(recent);

        Ok(candidates)
    }

    /// Phase 2: Ranking - Composite scoring
    async fn rank_candidates(
        &self,
        mut candidates: Vec<CandidateMessage>,
        _query: &str, // TODO: Use for query similarity boost
        preferred_labels: &[String],
    ) -> Result<Vec<CandidateMessage>, RepositoryError> {
        for candidate in &mut candidates {
            // Calculate recency score (exponential decay, 7-day half-life)
            let recency_score = self.calculate_recency_score(&candidate.timestamp);

            // Calculate label match score
            let label_score = if preferred_labels.contains(&candidate.label) {
                5.0
            } else {
                0.0
            };

            // Composite score: 50% importance, 30% recency, 20% label match
            candidate.score =
                (candidate.importance * 0.5) + (recency_score * 0.3) + (label_score * 0.2);
        }

        // Sort by composite score (highest first)
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(candidates)
    }

    /// Phase 3: Assembly - Build context within token budget
    async fn assemble_context(
        &self,
        candidates: &mut [CandidateMessage],
        context_budget: usize,
    ) -> Result<Vec<Message>, RepositoryError> {
        let mut context = Vec::new();
        let mut token_count = 0;
        let target_tokens = (context_budget as f32 * 0.85) as usize; // Reserve 15% for system prompt

        // Estimate: 1 token ≈ 4 characters
        for candidate in candidates {
            if token_count >= target_tokens {
                break;
            }

            // Fetch full message from SQLite
            if let Some(message) = self.repo.find_message_by_id(candidate.message_id).await? {
                let msg_tokens = message.content.len() / 4;

                if token_count + msg_tokens <= target_tokens {
                    context.push(message);
                    token_count += msg_tokens;
                }
            }
        }

        Ok(context)
    }

    /// Phase 4: Enhancement - Add citations and summaries
    async fn enhance_context(
        &self,
        mut context: Vec<Message>,
    ) -> Result<Vec<Message>, RepositoryError> {
        for message in &mut context {
            // Fetch conversation metadata for citation
            if let Some(conversation) = self.repo.find_by_id(message.conversation_id).await? {
                // Parse existing metadata string to Value
                let mut meta: serde_json::Value = message
                    .metadata
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_else(|| serde_json::json!({}));

                // Insert citation
                meta["citation"] = serde_json::json!({
                    "label": conversation.label,
                    "folder": conversation.folder,
                    "timestamp": message.timestamp.to_string(),
                });

                // Convert back to string
                message.metadata = Some(meta.to_string());
            }
        }

        Ok(context)
    }

    fn calculate_recency_score(&self, timestamp: &NaiveDateTime) -> f32 {
        let days_old = (chrono::Utc::now().naive_utc() - *timestamp).num_days();
        let half_life = 7.0; // 7 day half-life
        (2.0_f32).powf(-(days_old as f32) / half_life).max(0.1) // Minimum 0.1 score
    }

    /// Helper: Fetch a single message by ID
    async fn fetch_message(&self, id: Uuid) -> Result<Option<Message>, RepositoryError> {
        use crate::storage::entities::messages as message_entity;

        let model = message_entity::Entity::find_by_id(id.to_string())
            .one(self.repo.get_db()) // Need to access db directly
            .await
            .map_err(RepositoryError::DbError)?;

        Ok(model.map(|m| Message {
            id: Uuid::parse_str(&m.id).unwrap(),
            conversation_id: Uuid::parse_str(&m.conversation_id).unwrap(),
            role: m.role,
            content: m.content,
            timestamp: chrono::NaiveDateTime::parse_from_str(&m.timestamp, "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
            embedding_id: m.embedding_id.and_then(|id| Uuid::parse_str(&id).ok()),
            metadata: m.metadata.and_then(|meta| serde_json::from_str(&meta).ok()),
        }))
    }

    /// Helper: Get pinned messages (always included)
    async fn get_pinned_messages(&self) -> Result<Vec<CandidateMessage>, RepositoryError> {
        // TODO: Implement once 'pinned' status is in schema (Module 5 enhancement)
        Ok(Vec::new())
    }

    /// Helper: Get recent messages from preferred labels
    async fn get_recent_labeled_messages(
        &self,
        _labels: &[String],
        _days: i64,
    ) -> Result<Vec<CandidateMessage>, RepositoryError> {
        // TODO: Implement FTS5 search for recent messages (Module 5 enhancement)
        Ok(Vec::new())
    }
}

/// Internal candidate message with scoring metadata
#[derive(Debug, Clone)]
struct CandidateMessage {
    message_id: Uuid,
    conversation_id: Uuid,
    score: f32,
    timestamp: chrono::NaiveDateTime,
    label: String,
    is_pinned: bool,
    importance: f32,
}
