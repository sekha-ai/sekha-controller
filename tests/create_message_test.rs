use sea_orm::{ColumnTrait, ConnectionTrait, Database, EntityTrait, QueryFilter};
use sekha_controller::storage::entities::{conversations, messages};
use sekha_controller::storage::repository::{ConversationRepository, SeaOrmConversationRepository};
use sekha_controller::{ChromaClient, EmbeddingService};
use serde_json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_create_message_directly() {
    // Create in-memory database
    let db = Database::connect("sqlite::memory:").await.unwrap();

    // Create both tables with exact schema
    db.execute_unprepared(
        r#"
        CREATE TABLE conversations (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            folder TEXT NOT NULL,
            status TEXT NOT NULL,
            importance_score INTEGER NOT NULL,
            word_count INTEGER NOT NULL,
            session_count INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        
        CREATE TABLE messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            embedding_id TEXT,
            metadata TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations (id)
        );
        "#,
    )
    .await
    .unwrap();

    // Mock services (they won't be called in this test)
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));

    let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);

    // First create a conversation (required for FK constraint)
    let conv_id = Uuid::new_v4();
    let conv = sekha_controller::models::internal::NewConversation {
        id: Some(conv_id),
        label: "test_label".to_string(),
        folder: "test_folder".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![], // No messages initially
    };

    repo.create_with_messages(conv).await.unwrap();

    // Now test create_message directly
    let new_msg = sekha_controller::models::internal::NewMessage {
        content: "Test message content".to_string(),
        role: "user".to_string(),
        metadata: serde_json::json!({"test": "metadata"}),
        timestamp: chrono::Utc::now().naive_utc(),
    };

    // This will call the create_message method in the repository
    let result = repo.create_message(conv_id, new_msg).await;

    match result {
        Ok(msg_id) => {
            eprintln!("SUCCESS: create_message worked, msg_id = {}", msg_id);

            // Verify message exists in database
            let found = messages::Entity::find_by_id(msg_id.to_string())
                .one(repo.get_db())
                .await
                .unwrap()
                .unwrap();

            assert_eq!(found.content, "Test message content");
            eprintln!(
                "Verified: message {} exists with content '{}'",
                msg_id, found.content
            );
        }
        Err(e) => {
            panic!("create_message failed: {:?}", e);
        }
    }
}
