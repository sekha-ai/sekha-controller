use sea_orm::{Database, ConnectionTrait, Statement, DatabaseBackend, Value, EntityTrait};
use uuid::Uuid;
use sekha_controller::storage::entities::conversations;

#[tokio::test]
async fn test_repository_raw_sql_pattern() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    
    // Create the EXACT same schema as migrations
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

    // Use the EXACT same pattern as repository::create_with_messages
    let conv_id = Uuid::new_v4();
    let created_at_str = format!("{}", chrono::Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S%.3f"));
    let updated_at_str = format!("{}", chrono::Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S%.3f"));

    let sql = r#"
        INSERT INTO conversations (id, label, folder, status, importance_score, word_count, session_count, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#;
    
    let values = vec![
        Value::String(Some(conv_id.to_string())),
        Value::String(Some("test_label".to_string())),
        Value::String(Some("test_folder".to_string())),
        Value::String(Some("active".to_string())),
        Value::BigInt(Some(5)),
        Value::BigInt(Some(28)),
        Value::BigInt(Some(1)),
        Value::String(Some(created_at_str)),
        Value::String(Some(updated_at_str)),
    ];

    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        sql,
        values,
    );

    // This is the critical test - does the pattern work?
    db.execute_raw(stmt).await.unwrap();

    // Verify it worked using the entity
    let found = conversations::Entity::find_by_id(conv_id.to_string())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    
    assert_eq!(found.label, "test_label");
    eprintln!("SUCCESS: Inserted conversation {} with label {}", conv_id, found.label);
}