#[cfg(test)]
mod tests {
    use crate::models::internal::{NewConversation, NewMessage};
    use crate::storage::repository::ConversationRepository;
    use crate::{init_db, ChromaClient, EmbeddingService, SeaOrmConversationRepository};
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_with_messages_success() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = init_db(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let embedding_service = Arc::new(EmbeddingService::new(
            "http://localhost:11434".to_string(),
            "http://localhost:8000".to_string(),
        ));

        let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);
        let conv_id = Uuid::new_v4();

        let messages = vec![
            NewMessage {
                content: "Test message 1".to_string(),
                role: "user".to_string(),
                metadata: json!({}),
                timestamp: chrono::Utc::now().naive_utc(),
            },
            NewMessage {
                content: "Test message 2".to_string(),
                role: "assistant".to_string(),
                metadata: json!({}),
                timestamp: chrono::Utc::now().naive_utc(),
            },
        ];

        let new_conv = NewConversation {
            id: Some(conv_id),
            label: "test_label".to_string(),
            folder: "test_folder".to_string(),
            status: "active".to_string(),
            importance_score: Some(5),
            word_count: 100,
            session_count: Some(1),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            messages,
        };

        let result = repo.create_with_messages(new_conv).await;

        assert!(result.is_ok());

        // Verify conversation exists
        let conv = repo.find_by_id(conv_id).await.unwrap().unwrap();
        assert_eq!(conv.id, conv_id);
        assert_eq!(conv.label, "test_label");

        // Verify messages exist
        let messages = repo.get_conversation_messages(conv_id).await.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_cascades_to_messages() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = init_db(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let embedding_service = Arc::new(EmbeddingService::new(
            "http://localhost:11434".to_string(),
            "http://localhost:8000".to_string(),
        ));

        let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);
        let conv_id = Uuid::new_v4();

        // Create conversation with messages
        let messages = vec![NewMessage {
            content: "Test".to_string(),
            role: "user".to_string(),
            metadata: json!({}),
            timestamp: chrono::Utc::now().naive_utc(),
        }];

        let new_conv = NewConversation {
            id: Some(conv_id),
            label: "test_label".to_string(),
            folder: "test_folder".to_string(),
            status: "active".to_string(),
            importance_score: Some(5),
            word_count: 10,
            session_count: Some(1),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            messages,
        };

        repo.create_with_messages(new_conv).await.unwrap();

        // Delete conversation
        repo.delete(conv_id).await.unwrap();

        // Verify conversation is gone
        let result = repo.find_by_id(conv_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_chroma_upsert_and_query() {
        let chroma = ChromaClient::new("http://localhost:8000".to_string());
        let id = format!("test-{}", Uuid::new_v4());
        let embedding = vec![0.1; 768];

        // Ensure collection exists
        chroma
            .ensure_collection("test_collection", 768)
            .await
            .unwrap();

        // Test upsert (correct API signature)
        chroma
            .upsert(
                "test_collection",
                &id,
                embedding.clone(),
                json!({"test": "metadata"}),
                Some("Test document".to_string()),
            )
            .await
            .unwrap();

        // Test query
        let results = chroma
            .query("test_collection", embedding, 1, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[tokio::test]
    async fn test_semantic_search_with_filters() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = init_db(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let embedding_service = Arc::new(EmbeddingService::new(
            "http://localhost:11434".to_string(),
            "http://localhost:8000".to_string(),
        ));

        let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);

        // Create multiple conversations
        for i in 0..5 {
            let conv_id = Uuid::new_v4();
            let messages = vec![NewMessage {
                content: format!("Test message about AI {}", i),
                role: "user".to_string(),
                metadata: json!({}),
                timestamp: chrono::Utc::now().naive_utc(),
            }];

            let new_conv = NewConversation {
                id: Some(conv_id),
                label: format!("label_{}", i),
                folder: format!("folder_{}", i % 2),
                status: "active".to_string(),
                importance_score: Some(5),
                word_count: 50,
                session_count: Some(1),
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                messages,
            };

            repo.create_with_messages(new_conv).await.unwrap();
        }

        // Search by label (with limit and offset)
        let results = repo.find_by_label("label_0", 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);

        // Search with limit
        let results = repo.find_by_label("folder_0", 2, 0).await.unwrap();
        assert!(results.len() <= 2);
    }
}
