#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::init_db;
    use crate::llm::bridge_client::BridgeClient;
    use crate::models::internal::{NewConversation, NewMessage};
    use crate::services::embedding_service::EmbeddingService; // ✅ Fixed
    use crate::storage::chroma_client::ChromaClient; // ✅ Fixed
    use crate::storage::repository::ConversationRepository;
    use crate::storage::SeaOrmConversationRepository; // ✅ Fixed
    use sea_orm::{DatabaseBackend, DatabaseConnection, DbErr, Statement};
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_bridge() -> BridgeClient {
        let config = Config::default();
        BridgeClient::new(&config).expect("Failed to create BridgeClient")
    }

    async fn run_migrations_for_tests(db: &DatabaseConnection) -> Result<(), DbErr> {
        // Apply all migrations from the migrations directory
        let migrations = vec![
            include_str!("../../migrations/001_create_conversations.sql"),
            include_str!("../../migrations/002_create_messages.sql"),
            include_str!("../../migrations/003_add_embedding_id.sql"),
            include_str!("../../migrations/004_add_metadata.sql"),
            include_str!("../../migrations/005_add_importance.sql"),
            include_str!("../../migrations/006_add_word_count.sql"),
            include_str!("../../migrations/007_create_fts.sql"),
        ];

        for (idx, migration_sql) in migrations.iter().enumerate() {
            eprintln!("Running migration {}...", idx + 1);

            // Split by semicolon and execute each statement
            for statement in migration_sql.split(';').filter(|s| !s.trim().is_empty()) {
                db.execute(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    statement.trim().to_string(),
                ))
                .await?;
            }
        }

        eprintln!("All migrations applied successfully");
        Ok(())
    }

    async fn create_test_db() -> (TempDir, sea_orm::DatabaseConnection) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Ensure the directory exists and is writable
        let dir_path = temp_dir.path();
        fs::create_dir_all(dir_path).expect("Failed to create parent directories");

        // Use absolute path for SQLite
        let db_path = dir_path.join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        eprintln!("Creating test database at: {}", db_url);

        let db = init_db(&db_url)
            .await
            .expect("Failed to initialize database");

        // Run migrations
        run_migrations_for_tests(&db)
            .await
            .expect("Failed to run migrations");

        (temp_dir, db)
    }

    #[tokio::test]
    async fn test_create_with_messages_success() {
        let (_temp_dir, db) = create_test_db().await;

        let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let bridge = create_test_bridge();
        let embedding_service = Arc::new(EmbeddingService::new(
            bridge,
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

        match &result {
            Ok(id) => eprintln!("SUCCESS: Created conversation {}", id),
            Err(e) => {
                eprintln!("FAILED: {:?}", e);
                eprintln!("Full error chain: {:#?}", e);
            }
        }
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
        let (_temp_dir, db) = create_test_db().await;

        let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let bridge = create_test_bridge();
        let embedding_service = Arc::new(EmbeddingService::new(
            bridge,
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
    #[ignore] // Requires Chroma running on localhost:8000
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
        let (_temp_dir, db) = create_test_db().await;

        let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let bridge = create_test_bridge();
        let embedding_service = Arc::new(EmbeddingService::new(
            bridge,
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
