use sea_orm::{Database, ConnectionTrait, EntityTrait};
use uuid::Uuid;
use sekha_controller::storage::repository::{ConversationRepository, SeaOrmConversationRepository};
use sekha_controller::storage::entities::conversations;
use sekha_controller::{ChromaClient, EmbeddingService};
use std::sync::Arc;
use sekha_controller::models::internal::Conversation;

#[tokio::test]
async fn test_repository_create_directly() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    
    // Create schema
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
        )
        "#
    ).await.unwrap();

    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));

    let repo = SeaOrmConversationRepository::new(db, chroma, embedding_service);

    let conv = Conversation {
        id: Uuid::new_v4(),
        label: "test_label".to_string(),
        folder: "test_folder".to_string(),
        status: "active".to_string(),
        importance_score: 5,
        word_count: 28,
        session_count: 1,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    // Test the repository's create method directly
    let result = repo.create(conv).await;
    
    match result {
        Ok(id) => {
            eprintln!("SUCCESS: Repository::create worked, id = {}", id);
            
            // Verify it exists
            let found = conversations::Entity::find_by_id(id.to_string())
                .one(repo.get_db())
                .await
                .unwrap()
                .unwrap();
            
            assert_eq!(found.label, "test_label");
            eprintln!("Verified: conversation {} exists with label '{}'", id, found.label);
        },
        Err(e) => {
            panic!("Repository::create failed: {:?}", e);
        }
    }
}