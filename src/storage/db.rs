use sea_orm::{Database, DatabaseConnection, DbErr, ConnectionTrait};
use tracing::{info, error};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn init_db(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    // Extract path from database URL
    let path = if database_url.starts_with("sqlite:") {
        &database_url[7..]  // Remove "sqlite:" prefix
    } else {
        database_url
    };
    
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DbErr::Custom(format!("Failed to create database directory: {}", e))
            })?;
        }
    }
    
    // Connect to database
    let db = Database::connect(database_url).await?;
    
    // Check if migrations need to be applied
    let result = db.execute_unprepared(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'"
    ).await?;
    
    if result.rows_affected() == 0 {
        tracing::info!("Database empty, applying migrations...");
        
        let migrations = vec![
            include_str!("../../migrations/001_create_conversations.sql"),
            include_str!("../../migrations/002_create_messages.sql"),
            include_str!("../../migrations/003_create_semantic_tags.sql"),
            include_str!("../../migrations/004_create_hierarchical_summaries.sql"),
            include_str!("../../migrations/005_create_knowledge_graph_edges.sql"),
            include_str!("../../migrations/006_add_updated_at_triggers.sql"),
        ];
        
        for (i, sql) in migrations.iter().enumerate() {
            db.execute_unprepared(sql).await?;
            tracing::info!("Applied migration {}", i + 1);
        }
    } else {
        tracing::info!("Database already migrated, skipping");
    }
    
    // Store connection in static
    static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> = 
        Lazy::new(|| Arc::new(Mutex::new(None)));
    let mut conn = DB_CONN.lock().await;
    *conn = Some(db.clone());
    
    Ok(db)
}

pub async fn get_connection() -> Option<DatabaseConnection> {
    DB_CONN.lock().await.clone()
}
