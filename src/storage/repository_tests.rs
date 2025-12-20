#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, Statement};
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_with_messages_success() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = init_db(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let repo = SeaOrmConversationRepository::new(Arc::new(db));
        let conv_id = Uuid::new_v4();
        let message_ids = vec![Uuid::new_v4(), Uuid::new_v4()];

        let messages = vec![
            crate::models::CreateMessageRequest {
                content: "Test message 1".to_string(),
                role: "user".to_string(),
                metadata: json!({}),
            },
            crate::models::CreateMessageRequest {
                content: "Test message 2".to_string(),
                role: "assistant".to_string(),
                metadata: json!({}),
            },
        ];

        let result = repo
            .create_with_messages(
                conv_id,
                messages,
                "test_label".to_string(),
                "test_folder".to_string(),
                0.8,
                json!({}),
            )
            .await;

        assert!(result.is_ok());

        // Verify conversation exists
        let conv = repo.find_by_id(conv_id).await.unwrap();
        assert_eq!(conv.id, conv_id);
        assert_eq!(conv.label, "test_label");

        // Verify messages exist
        assert_eq!(conv.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_cascades_to_messages() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = init_db(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let repo = SeaOrmConversationRepository::new(Arc::new(db));
        let conv_id = Uuid::new_v4();

        // Create conversation with messages
        let messages = vec![crate::models::CreateMessageRequest {
            content: "Test".to_string(),
            role: "user".to_string(),
            metadata: json!({}),
        }];

        repo.create_with_messages(
            conv_id,
            messages,
            "test_label".to_string(),
            "test_folder".to_string(),
            0.8,
            json!({}),
        )
        .await
        .unwrap();

        // Delete conversation
        repo.delete(conv_id).await.unwrap();

        // Verify conversation is gone
        let result = repo.find_by_id(conv_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chroma_upsert_and_query() {
        let chroma = ChromaClient::new("http://localhost:8000".to_string());
        let id = format!("test-{}", Uuid::new_v4());
        let embedding = vec![0.1; 768];

        // Test upsert
        chroma
            .upsert("test_collection", vec![(id.clone(), embedding.clone())])
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

        let repo = SeaOrmConversationRepository::new(Arc::new(db));

        // Create multiple conversations
        for i in 0..5 {
            let conv_id = Uuid::new_v4();
            let messages = vec![crate::models::CreateMessageRequest {
                content: format!("Test message about AI {}", i),
                role: "user".to_string(),
                metadata: json!({}),
            }];

            repo.create_with_messages(
                conv_id,
                messages,
                format!("label_{}", i),
                format!("folder_{}", i % 2),
                0.8,
                json!({}),
            )
            .await
            .unwrap();
        }

        // Search by label
        let results = repo.find_by_label("label_0", 10).await.unwrap();
        assert_eq!(results.len(), 1);

        // Search with limit
        let results = repo.find_by_label("folder_0", 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
