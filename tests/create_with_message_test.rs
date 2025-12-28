use sea_orm::{Database, ConnectionTrait, EntityTrait, QueryFilter, ColumnTrait};
use uuid::Uuid;
use sekha_controller::storage::repository::{ConversationRepository, SeaOrmConversationRepository};
use sekha_controller::storage::entities::{conversations, messages};
use sekha_controller::{ChromaClient, EmbeddingService};
use std::sync::Arc;
use serde_json;

#[tokio::test]
async fn test_create_with_messages_directly() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    
    // Create both tables with exact schema from migrations
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
        "#
    ).await.unwrap();

    // Mock services (won't be called due to error handling)
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));

    let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);

    let conv_id = Uuid::new_v4();
    let new_conv = sekha_controller::models::internal::NewConversation {
        id: Some(conv_id),
        label: "test_label".to_string(),
        folder: "test_folder".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 28,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![
            sekha_controller::models::internal::NewMessage {
                content: "Test message 1".to_string(),
                role: "user".to_string(),
                metadata: serde_json::json!({}),
                timestamp: chrono::Utc::now().naive_utc(),
            },
            sekha_controller::models::internal::NewMessage {
                content: "Test message 2".to_string(),
                role: "assistant".to_string(),
                metadata: serde_json::json!({}),
                timestamp: chrono::Utc::now().naive_utc(),
            },
        ],
    };

    let result = repo.create_with_messages(new_conv).await;

    match result {
        Ok(id) => {
            eprintln!("SUCCESS: create_with_messages worked, conv_id = {}", id);
            
            // Verify conversation exists
            let conv = conversations::Entity::find_by_id(id.to_string())
                .one(repo.get_db())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(conv.label, "test_label");
            
            // Verify messages exist
            let msgs = messages::Entity::find()
                .filter(messages::Column::ConversationId.eq(id.to_string()))
                .all(repo.get_db())
                .await
                .unwrap();
            assert_eq!(msgs.len(), 2);
            
            eprintln!("Verified: conversation and 2 messages created");
        },
        Err(e) => {
            panic!("create_with_messages failed: {:?}", e);
        }
    }
}