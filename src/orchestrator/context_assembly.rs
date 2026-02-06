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
        excluded_folders: Vec<String>,
    ) -> Result<Vec<Message>, RepositoryError> {
        // Phase 1: Recall - Get candidate messages
        let candidates = self
            .recall_candidates(query, &preferred_labels, &excluded_folders)
            .await?;

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
        excluded_folders: &[String],
    ) -> Result<Vec<CandidateMessage>, RepositoryError> {
        let mut candidates = Vec::new();

        // 1. Semantic search from Chroma (top 200)
        let semantic_results = self.repo.semantic_search(query, 200, None).await?;
        for result in semantic_results {
            if excluded_folders
                .iter()
                .any(|folder| result.folder.starts_with(folder))
            {
                continue; // Skip excluded conversations
            }
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
            if let Some(message) = self.fetch_message(candidate.message_id).await? {
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
                // Parse existing metadata Value to Value (no conversion needed)
                let mut meta: serde_json::Value = message
                    .metadata
                    .as_ref()
                    .cloned() // CHANGED: Clone the Value directly
                    .unwrap_or_else(|| serde_json::json!({}));

                // Insert citation
                meta["citation"] = serde_json::json!({
                    "label": conversation.label,
                    "folder": conversation.folder,
                    "timestamp": message.timestamp.to_string(),
                });

                // Keep as Value (no string conversion)
                message.metadata = Some(meta); // CHANGED: Direct assignment
            }
        }

        Ok(context)
    }

    fn calculate_recency_score(&self, timestamp: &NaiveDateTime) -> f32 {
        let days_old = (chrono::Utc::now().naive_utc() - *timestamp).num_days();
        let half_life = 7.0; // 7 day half-life
        (2.0_f32).powf(-(days_old as f32) / half_life).max(0.1) // Minimum 0.1 score
    }

    /// Helper: Get pinned messages (always included)
    pub async fn get_pinned_messages(&self) -> Result<Vec<CandidateMessage>, RepositoryError> {
        use crate::storage::entities::{conversations, messages};
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        // Find conversations with importance_score >= 10 (pinned)
        let pinned_convs = conversations::Entity::find()
            .filter(conversations::Column::ImportanceScore.gte(10))
            .filter(conversations::Column::Status.eq("active"))
            .all(self.repo.get_db())
            .await?;

        let mut candidates = Vec::new();

        for conv in pinned_convs {
            let conv_id = conv.id; // CHANGED: Remove parse_str, already Uuid

            // Get recent messages from pinned conversation
            let messages = messages::Entity::find()
                .filter(messages::Column::ConversationId.eq(conv.id)) // CHANGED: Remove .clone()
                .all(self.repo.get_db())
                .await?;

            for msg in messages {
                candidates.push(CandidateMessage {
                    message_id: msg.id, // CHANGED: Remove parse_str, already Uuid
                    conversation_id: conv_id,
                    score: 10.0,
                    timestamp: msg.timestamp, // CHANGED: Direct use, already NaiveDateTime
                    label: conv.label.clone(),
                    is_pinned: true,
                    importance: 10.0,
                });
            }
        }

        Ok(candidates)
    }

    /// Helper: Get recent messages from preferred labels
    pub async fn get_recent_labeled_messages(
        &self,
        labels: &[String],
        days: i64,
    ) -> Result<Vec<CandidateMessage>, RepositoryError> {
        use crate::storage::entities::{conversations, messages};
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        if labels.is_empty() {
            return Ok(Vec::new());
        }

        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(days); // CHANGED: Keep as NaiveDateTime

        let mut candidates = Vec::new();

        for label in labels {
            let convs = conversations::Entity::find()
                .filter(conversations::Column::Label.eq(label))
                .filter(conversations::Column::Status.eq("active"))
                .all(self.repo.get_db())
                .await?;

            for conv in convs {
                let conv_id = conv.id; // CHANGED: Remove parse_str

                let messages = messages::Entity::find()
                    .filter(messages::Column::ConversationId.eq(conv.id)) // CHANGED: Remove .clone()
                    .filter(messages::Column::Timestamp.gte(cutoff)) // CHANGED: Direct comparison, no to_string()
                    .all(self.repo.get_db())
                    .await?;

                for msg in messages {
                    candidates.push(CandidateMessage {
                        message_id: msg.id, // CHANGED: Remove parse_str
                        conversation_id: conv_id,
                        score: 5.0,
                        timestamp: msg.timestamp, // CHANGED: Direct use
                        label: conv.label.clone(),
                        is_pinned: false,
                        importance: conv.importance_score as f32,
                    });
                }
            }
        }

        Ok(candidates)
    }

    pub async fn fetch_message(&self, id: Uuid) -> Result<Option<Message>, RepositoryError> {
        use crate::storage::entities::messages as message_entity;

        let model = message_entity::Entity::find_by_id(id) // CHANGED: Remove .to_string()
            .one(self.repo.get_db())
            .await
            .map_err(RepositoryError::DbError)?;

        Ok(model.map(|m| Message {
            id: m.id,                           // CHANGED: Remove parse_str
            conversation_id: m.conversation_id, // CHANGED: Remove parse_str
            role: m.role,
            content: m.content,
            timestamp: m.timestamp, // CHANGED: Direct use
            embedding_id: None,     // TODO: populate from model.embedding_id if needed
            metadata: m.metadata,   // CHANGED: Direct use, already Option<Value>
        }))
    }
}

/// Internal candidate message with scoring metadata
#[derive(Debug, Clone)]
pub struct CandidateMessage {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub score: f32,
    pub timestamp: chrono::NaiveDateTime,
    pub label: String,
    pub is_pinned: bool,
    pub importance: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::internal::{Conversation, SemanticSearchResult};
    use crate::storage::repository::MockConversationRepository;
    use chrono::Utc;
    use mockall::predicate::*;
    use sea_orm::DatabaseConnection;

    fn create_test_message(id: Uuid, conv_id: Uuid, content: &str) -> Message {
        Message {
            id,
            conversation_id: conv_id,
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: Utc::now().naive_utc(),
            embedding_id: None,
            metadata: Some(serde_json::json!({"test": true})),
        }
    }

    fn create_test_conversation(id: Uuid, label: &str, folder: &str) -> Conversation {
        Conversation {
            id,
            label: label.to_string(),
            folder: folder.to_string(),
            status: "active".to_string(),
            importance_score: 5,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            summary: None,
            has_images: false,
        }
    }

    #[test]
    fn test_new_context_assembler() {
        let repo = Arc::new(MockConversationRepository::new());
        let assembler = ContextAssembler::new(repo);
        assert!(std::mem::size_of_val(&assembler) > 0);
    }

    #[tokio::test]
    async fn test_assemble_full_pipeline_empty_results() {
        let mut mock_repo = MockConversationRepository::new();

        // Mock semantic_search to return empty results
        mock_repo
            .expect_semantic_search()
            .returning(|_, _, _| Ok(vec![]));

        // Mock get_db to return a valid connection (needed for pinned/recent)
        mock_repo
            .expect_get_db()
            .returning(|| panic!("get_db should not be called in this test"));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let result = assembler.assemble("test query", vec![], 4000, vec![]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_recall_candidates_with_semantic_results() {
        let mut mock_repo = MockConversationRepository::new();

        let msg_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();

        mock_repo
            .expect_semantic_search()
            .returning(move |_, _, _| {
                Ok(vec![SemanticSearchResult {
                    message_id: msg_id,
                    conversation_id: conv_id,
                    score: 0.95,
                    timestamp: Utc::now().naive_utc(),
                    label: "test".to_string(),
                    folder: "/test".to_string(),
                }])
            });

        mock_repo
            .expect_get_db()
            .returning(|| panic!("get_db should not be called"));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let candidates = assembler
            .recall_candidates("test query", &[], &[])
            .await
            .unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].message_id, msg_id);
        assert_eq!(candidates[0].score, 0.95);
    }

    #[tokio::test]
    async fn test_recall_candidates_excluded_folders() {
        let mut mock_repo = MockConversationRepository::new();

        let msg_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();

        mock_repo
            .expect_semantic_search()
            .returning(move |_, _, _| {
                Ok(vec![SemanticSearchResult {
                    message_id: msg_id,
                    conversation_id: conv_id,
                    score: 0.95,
                    timestamp: Utc::now().naive_utc(),
                    label: "test".to_string(),
                    folder: "/excluded/subfolder".to_string(),
                }])
            });

        mock_repo
            .expect_get_db()
            .returning(|| panic!("get_db should not be called"));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let candidates = assembler
            .recall_candidates("test query", &[], &["/excluded".to_string()])
            .await
            .unwrap();

        assert_eq!(candidates.len(), 0);
    }

    #[tokio::test]
    async fn test_rank_candidates_with_preferred_labels() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let now = Utc::now().naive_utc();
        let candidates = vec![
            CandidateMessage {
                message_id: Uuid::new_v4(),
                conversation_id: Uuid::new_v4(),
                score: 0.5,
                timestamp: now,
                label: "preferred".to_string(),
                is_pinned: false,
                importance: 5.0,
            },
            CandidateMessage {
                message_id: Uuid::new_v4(),
                conversation_id: Uuid::new_v4(),
                score: 0.5,
                timestamp: now,
                label: "other".to_string(),
                is_pinned: false,
                importance: 5.0,
            },
        ];

        let ranked = assembler
            .rank_candidates(candidates, "test", &["preferred".to_string()])
            .await
            .unwrap();

        assert_eq!(ranked[0].label, "preferred");
        assert!(ranked[0].score > ranked[1].score);
    }

    #[tokio::test]
    async fn test_rank_candidates_empty_labels() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let now = Utc::now().naive_utc();
        let candidates = vec![CandidateMessage {
            message_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            score: 0.5,
            timestamp: now,
            label: "test".to_string(),
            is_pinned: false,
            importance: 8.0,
        }];

        let ranked = assembler
            .rank_candidates(candidates, "test", &[])
            .await
            .unwrap();

        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].score > 0.0);
    }

    #[tokio::test]
    async fn test_assemble_context_within_budget() {
        let mut mock_repo = MockConversationRepository::new();
        let msg_id1 = Uuid::new_v4();
        let msg_id2 = Uuid::new_v4();
        let conv_id = Uuid::new_v4();

        mock_repo
            .expect_get_db()
            .returning(|| panic!("Should use fetch_message mock instead"));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let mut candidates = vec![
            CandidateMessage {
                message_id: msg_id1,
                conversation_id: conv_id,
                score: 10.0,
                timestamp: Utc::now().naive_utc(),
                label: "test".to_string(),
                is_pinned: false,
                importance: 5.0,
            },
            CandidateMessage {
                message_id: msg_id2,
                conversation_id: conv_id,
                score: 8.0,
                timestamp: Utc::now().naive_utc(),
                label: "test".to_string(),
                is_pinned: false,
                importance: 5.0,
            },
        ];

        // This will fail fetching but test the budget logic
        let context = assembler
            .assemble_context(&mut candidates, 100)
            .await
            .unwrap();

        // Context is empty because fetch_message returns None
        assert_eq!(context.len(), 0);
    }

    #[tokio::test]
    async fn test_assemble_context_empty_candidates() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let mut candidates = vec![];
        let context = assembler
            .assemble_context(&mut candidates, 4000)
            .await
            .unwrap();

        assert_eq!(context.len(), 0);
    }

    #[tokio::test]
    async fn test_enhance_context_with_conversation() {
        let mut mock_repo = MockConversationRepository::new();

        let msg_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();
        let conversation = create_test_conversation(conv_id, "test-label", "/test-folder");

        mock_repo
            .expect_find_by_id()
            .with(eq(conv_id))
            .returning(move |_| Ok(Some(conversation.clone())));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let messages = vec![create_test_message(msg_id, conv_id, "test content")];

        let enhanced = assembler.enhance_context(messages).await.unwrap();

        assert_eq!(enhanced.len(), 1);
        assert!(enhanced[0].metadata.is_some());
        let meta = enhanced[0].metadata.as_ref().unwrap();
        assert!(meta["citation"].is_object());
        assert_eq!(meta["citation"]["label"], "test-label");
        assert_eq!(meta["citation"]["folder"], "/test-folder");
    }

    #[tokio::test]
    async fn test_enhance_context_no_conversation() {
        let mut mock_repo = MockConversationRepository::new();

        let msg_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();

        mock_repo
            .expect_find_by_id()
            .with(eq(conv_id))
            .returning(|_| Ok(None));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let messages = vec![create_test_message(msg_id, conv_id, "test content")];

        let enhanced = assembler.enhance_context(messages).await.unwrap();

        assert_eq!(enhanced.len(), 1);
        // Metadata should remain unchanged when conversation not found
        assert!(enhanced[0].metadata.is_some());
    }

    #[tokio::test]
    async fn test_enhance_context_empty() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let enhanced = assembler.enhance_context(vec![]).await.unwrap();
        assert_eq!(enhanced.len(), 0);
    }

    #[test]
    fn test_calculate_recency_score_recent() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let recent = Utc::now().naive_utc();
        let score = assembler.calculate_recency_score(&recent);

        assert!(score >= 0.9);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_recency_score_old() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let old = Utc::now().naive_utc() - chrono::Duration::days(30);
        let score = assembler.calculate_recency_score(&old);

        assert!(score >= 0.1);
        assert!(score < 0.3);
    }

    #[test]
    fn test_calculate_recency_score_minimum() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let very_old = Utc::now().naive_utc() - chrono::Duration::days(365);
        let score = assembler.calculate_recency_score(&very_old);

        assert_eq!(score, 0.1);
    }

    #[tokio::test]
    async fn test_get_recent_labeled_messages_empty_labels() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let result = assembler.get_recent_labeled_messages(&[], 7).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_candidate_message_structure() {
        let msg_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();
        let timestamp = Utc::now().naive_utc();

        let candidate = CandidateMessage {
            message_id: msg_id,
            conversation_id: conv_id,
            score: 5.5,
            timestamp,
            label: "test".to_string(),
            is_pinned: true,
            importance: 10.0,
        };

        assert_eq!(candidate.message_id, msg_id);
        assert_eq!(candidate.conversation_id, conv_id);
        assert_eq!(candidate.score, 5.5);
        assert_eq!(candidate.label, "test");
        assert!(candidate.is_pinned);
        assert_eq!(candidate.importance, 10.0);
    }

    #[tokio::test]
    async fn test_recall_candidates_semantic_search_error() {
        let mut mock_repo = MockConversationRepository::new();

        mock_repo
            .expect_semantic_search()
            .returning(|_, _, _| Err(RepositoryError::ConnectionError));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let result = assembler.recall_candidates("test", &[], &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enhance_context_with_no_metadata() {
        let mut mock_repo = MockConversationRepository::new();

        let msg_id = Uuid::new_v4();
        let conv_id = Uuid::new_v4();
        let conversation = create_test_conversation(conv_id, "label", "/folder");

        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(conversation.clone())));

        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let mut message = create_test_message(msg_id, conv_id, "content");
        message.metadata = None;

        let enhanced = assembler.enhance_context(vec![message]).await.unwrap();

        assert_eq!(enhanced.len(), 1);
        assert!(enhanced[0].metadata.is_some());
        assert!(enhanced[0].metadata.as_ref().unwrap()["citation"].is_object());
    }

    #[tokio::test]
    async fn test_rank_candidates_sorts_by_composite_score() {
        let mock_repo = MockConversationRepository::new();
        let assembler = ContextAssembler::new(Arc::new(mock_repo));

        let now = Utc::now().naive_utc();
        let old = now - chrono::Duration::days(10);

        let candidates = vec![
            CandidateMessage {
                message_id: Uuid::new_v4(),
                conversation_id: Uuid::new_v4(),
                score: 0.5,
                timestamp: old,
                label: "other".to_string(),
                is_pinned: false,
                importance: 3.0,
            },
            CandidateMessage {
                message_id: Uuid::new_v4(),
                conversation_id: Uuid::new_v4(),
                score: 0.5,
                timestamp: now,
                label: "preferred".to_string(),
                is_pinned: false,
                importance: 8.0,
            },
        ];

        let ranked = assembler
            .rank_candidates(candidates, "query", &["preferred".to_string()])
            .await
            .unwrap();

        // Second candidate should rank higher (newer + preferred label + higher importance)
        assert_eq!(ranked[0].label, "preferred");
        assert!(ranked[0].score > ranked[1].score);
    }
}
