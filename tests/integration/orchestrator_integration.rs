use chrono::Local;
use sea_orm::ConnectionTrait;
use sekha_controller::{
    config::Config,
    init_db,
    models::internal::{Conversation, NewConversation, NewMessage},
    orchestrator::label_intelligence::LabelIntelligence,
    storage::repository::{ConversationRepository, SeaOrmConversationRepository},
};
use std::sync::Arc;
use uuid::Uuid;

async fn setup() -> (
    Arc<SeaOrmConversationRepository>,
    sea_orm::DatabaseConnection,
) {
    use sekha_controller::services::embedding_service::EmbeddingService;
    use sekha_controller::storage::chroma_client::ChromaClient;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = init_db(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();

    // Create mock services (use invalid URLs so they fail gracefully in tests)
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:1".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:1".to_string(),
        "http://localhost:1".to_string(),
    ));

    let repo = Arc::new(SeaOrmConversationRepository::new(
        db.clone(),
        chroma_client,
        embedding_service,
    ));
    (repo, db)
}

async fn execute_schema(
    db: &sea_orm::DatabaseConnection,
    schema: &str,
) -> Result<(), sea_orm::DbErr> {
    // Split on semicolons and execute each statement
    for statement in schema.split(';') {
        let stmt = statement.trim();
        if !stmt.is_empty() {
            db.execute_unprepared(stmt).await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_create_and_find_conversation() {
    let (repo, _db) = setup().await;

    let conv_id = Uuid::new_v4();
    let conv = NewConversation {
        id: Some(conv_id),
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    let created_id = repo.create_with_messages(conv).await.unwrap();
    assert_eq!(created_id, conv_id);

    let found = repo.find_by_id(conv_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, conv_id);
}

#[tokio::test]
async fn test_create_conversation_with_messages() {
    let (repo, _db) = setup().await;

    let conv_id = Uuid::new_v4();
    let messages = vec![
        NewMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            metadata: serde_json::json!({"test": true}),
            timestamp: Local::now().naive_local(),
        },
        NewMessage {
            role: "assistant".to_string(),
            content: "Hi there!".to_string(),
            metadata: serde_json::json!({"test": true}),
            timestamp: Local::now().naive_local(),
        },
    ];

    let conv = NewConversation {
        id: Some(conv_id),
        label: "Test Conversation".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages,
    };

    let created_id = repo.create_with_messages(conv).await.unwrap();
    assert_eq!(created_id, conv_id);

    let msgs = repo.get_conversation_messages(conv_id).await.unwrap();
    assert_eq!(msgs.len(), 2);
}

#[tokio::test]
async fn test_update_label() {
    let (repo, _db) = setup().await;

    let conv_id = Uuid::new_v4();
    let conv = NewConversation {
        id: Some(conv_id),
        label: "Old Label".to_string(),
        folder: "/old".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv).await.unwrap();
    repo.update_label(conv_id, "New Label", "/new")
        .await
        .unwrap();

    let found = repo.find_by_id(conv_id).await.unwrap().unwrap();
    assert_eq!(found.label, "New Label");
    assert_eq!(found.folder, "/new");
}

#[tokio::test]
async fn test_count_operations() {
    let (repo, _db) = setup().await;

    let conv1 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "TestLabel".to_string(),
        folder: "/folder1".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    let conv2 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "TestLabel".to_string(),
        folder: "/folder2".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv1).await.unwrap();
    repo.create_with_messages(conv2).await.unwrap();

    let label_count = repo.count_by_label("TestLabel").await.unwrap();
    assert_eq!(label_count, 2);

    let folder_count = repo.count_by_folder("/folder1").await.unwrap();
    assert_eq!(folder_count, 1);

    let total = repo.count_all().await.unwrap();
    assert_eq!(total, 2);
}

#[tokio::test]
async fn test_label_intelligence_creation() {
    use sekha_controller::services::llm_bridge_client::LlmBridgeClient;

    let config = Config::default();
    let (repo, _db) = setup().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config).unwrap());

    // Create label intelligence with correct Arc<LlmBridgeClient>
    let intelligence = LabelIntelligence::new(repo, llm_bridge);
    // Just verify it compiles and creates successfully
    assert!(true);
}

#[tokio::test]
async fn test_delete_conversation() {
    let (repo, _db) = setup().await;

    let conv_id = Uuid::new_v4();
    let conv = NewConversation {
        id: Some(conv_id),
        label: "To Delete".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv).await.unwrap();
    repo.delete(conv_id).await.unwrap();

    let found = repo.find_by_id(conv_id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_find_by_label() {
    let (repo, _db) = setup().await;

    let conv1 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "UniqueLabel".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    let conv2 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "UniqueLabel".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv1).await.unwrap();
    repo.create_with_messages(conv2).await.unwrap();

    let found = repo.find_by_label("UniqueLabel", 10, 0).await.unwrap();
    assert_eq!(found.len(), 2);
}

#[tokio::test]
async fn test_find_by_folder() {
    let (repo, _db) = setup().await;

    let conv1 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Test1".to_string(),
        folder: "/unique_folder".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    let conv2 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Test2".to_string(),
        folder: "/unique_folder".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv1).await.unwrap();
    repo.create_with_messages(conv2).await.unwrap();

    let found = repo.find_by_folder("/unique_folder", 10, 0).await.unwrap();
    assert_eq!(found.len(), 2);
}

#[tokio::test]
async fn test_update_status() {
    let (repo, _db) = setup().await;

    let conv_id = Uuid::new_v4();
    let conv = NewConversation {
        id: Some(conv_id),
        label: "Test".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv).await.unwrap();
    repo.update_status(conv_id, "archived").await.unwrap();

    let found = repo.find_by_id(conv_id).await.unwrap().unwrap();
    assert_eq!(found.status, "archived");
}

#[tokio::test]
async fn test_update_importance() {
    let (repo, _db) = setup().await;

    let conv_id = Uuid::new_v4();
    let conv = NewConversation {
        id: Some(conv_id),
        label: "Test".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 100,
        session_count: Some(1),
        created_at: Local::now().naive_local(),
        updated_at: Local::now().naive_local(),
        messages: vec![],
    };

    repo.create_with_messages(conv).await.unwrap();
    repo.update_importance(conv_id, 9).await.unwrap();

    let found = repo.find_by_id(conv_id).await.unwrap().unwrap();
    assert_eq!(found.importance_score, 9);
}
