use once_cell::sync::Lazy;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};
use std::sync::Arc;
use tokio::sync::Mutex;

static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn init_db(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    tracing::info!("Connecting to database: {}", database_url);

    // Handle special SQLite URL formats
    let db = if database_url == "sqlite::memory:" {
        // In-memory database - no file operations needed
        Database::connect(database_url)
            .await
            .map_err(|e| DbErr::Custom(format!("Connection failed: {}", e)))?
    } else if let Some(path_str) = database_url.strip_prefix("sqlite://") {
        // File-based database
        let path_str = path_str.split('?').next().unwrap_or(path_str);
        let path = std::path::Path::new(path_str);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| DbErr::Custom(format!("Failed to create DB directory: {}", e)))?;
                tracing::info!("Created database directory: {}", parent.display());
            }
        }

        // Create file if it doesn't exist
        if !path.exists() {
            std::fs::File::create(path)
                .map_err(|e| DbErr::Custom(format!("Failed to create DB file: {}", e)))?;
            tracing::info!("Created database file: {}", path.display());
        }

        Database::connect(database_url)
            .await
            .map_err(|e| DbErr::Custom(format!("Connection failed: {}", e)))?
    } else {
        return Err(DbErr::Custom("Invalid SQLite URL format".to_string()));
    };

    // Check and apply migrations if needed
    let needs_migration = match db
        .execute_unprepared("SELECT 1 FROM conversations LIMIT 1")
        .await
    {
        Ok(_) => false, // Table exists
        Err(_) => true, // Table doesn't exist
    };

    if needs_migration {
        tracing::info!("Database empty, applying migrations...");

        let migrations = [
            include_str!("../../migrations/001_create_conversations.sql"),
            include_str!("../../migrations/002_create_messages.sql"),
            include_str!("../../migrations/003_create_semantic_tags.sql"),
            include_str!("../../migrations/004_create_hierarchical_summaries.sql"),
            include_str!("../../migrations/005_create_knowledge_graph_edges.sql"),
            // REVIEW: FTS and triggers disabled for MVP - re-enable in Module 9-12
            // include_str!("../../migrations/006_add_updated_at_triggers.sql"),
            // include_str!("../../migrations/007_create_fts.sql"),
        ];

        for (i, sql) in migrations.iter().enumerate() {
            db.execute_unprepared(sql).await?;
            tracing::info!("Applied migration {}", i + 1);
        }
    } else {
        tracing::info!("Database already migrated, skipping");
    }

    // Store connection in static
    let mut conn = DB_CONN.lock().await;
    *conn = Some(db.clone());

    Ok(db)
}

pub async fn get_connection() -> Option<DatabaseConnection> {
    DB_CONN.lock().await.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_db_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = init_db(&url).await.unwrap();

        // Verify file exists
        assert!(db_path.exists());

        // Verify we can query
        let result = db.execute_unprepared("SELECT 1").await.unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    #[tokio::test]
    async fn test_init_db_runs_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = init_db(&url).await.unwrap();

        // Verify conversations table exists
        let result = db
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'",
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);
    }
}
