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
        Database::connect(database_url)
            .await
            .map_err(|e| DbErr::Custom(format!("Connection failed: {}", e)))?
    } else if let Some(path_str) = database_url.strip_prefix("sqlite://") {
        let path_str = path_str.split('?').next().unwrap_or(path_str);
        let path = std::path::Path::new(path_str);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| DbErr::Custom(format!("Failed to create DB directory: {}", e)))?;
                tracing::info!("Created database directory: {}", parent.display());
            }
        }

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

    db.execute_unprepared("PRAGMA journal_mode=WAL;")
        .await
        .map_err(|e| DbErr::Custom(format!("Failed to enable WAL mode: {}", e)))?;

    tracing::info!("WAL mode enabled for database");

    // Apply migrations if needed
    tracing::info!("Applying migrations...");

    // Check if migrations table exists by trying to query it
    let migrations_need_setup = match db
        .execute_unprepared("SELECT COUNT(*) FROM seaql_migrations")
        .await
    {
        Ok(_) => {
            tracing::info!("Migrations table exists, skipping setup");
            false
        }
        Err(_) => {
            tracing::info!("Migrations table does not exist, will create");
            true
        }
    };

    if migrations_need_setup {
        tracing::info!("First run: executing all migration SQL files");

        // FIX: Removed migration 007 from this list - it's now handled separately below
        let migrations = [
            include_str!("../../migrations/001_create_conversations.sql"),
            include_str!("../../migrations/002_create_messages.sql"),
            include_str!("../../migrations/003_create_semantic_tags.sql"),
            include_str!("../../migrations/004_create_hierarchical_summaries.sql"),
            include_str!("../../migrations/005_create_knowledge_graph_edges.sql"),
            include_str!("../../migrations/006_add_updated_at_triggers.sql"),
            include_str!("../../migrations/007_create_fts.sql"),
        ];

        for (i, sql) in migrations.iter().enumerate() {
            db.execute_unprepared(sql).await?;
            tracing::info!("Applied migration {}", i + 1);
        }

        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS seaql_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .await?;

        for i in 1..=migrations.len() {
            db.execute_unprepared(&format!(
                "INSERT OR IGNORE INTO seaql_migrations (version) VALUES ('{}')",
                format!("m20241211_{:08}", i * 100000)
            ))
            .await?;
        }
    } else {
        tracing::info!("Migrations already applied, skipping");
    }

    // FIX: Create FTS table unconditionally and separately from migrations
    // This avoids SeaORM's migration runner bugs with virtual tables
    db.execute_unprepared(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content,
            tokenize='porter'
        );
        "#,
    )
    .await?;

    // Store connection
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
    use sea_orm::{DatabaseBackend, Statement};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_db_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = init_db(&url).await.unwrap();

        // Verify file exists
        assert!(db_path.exists());

        // Verify migrations table was created (proves migrations ran)
        let result = db
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='seaql_migrations'",
            )
            .await
            .unwrap();

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

        // Verify migrations tracking table exists and has entries
        let result = db
            .execute_unprepared("SELECT COUNT(*) FROM seaql_migrations")
            .await
            .unwrap();

        assert!(result.rows_affected() > 0);
    }

    #[tokio::test]
    async fn test_init_db_skips_existing_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        // First init - creates everything
        let db1 = init_db(&url).await.unwrap();

        // Verify conversations table exists after first init
        let result1 = db1
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'",
            )
            .await
            .unwrap();
        assert_eq!(result1.rows_affected(), 1);

        // Second init - should skip migrations and not error
        let db2 = init_db(&url).await.unwrap();

        // Verify conversations table still exists after second init
        let result2 = db2
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'",
            )
            .await
            .unwrap();
        assert_eq!(result2.rows_affected(), 1);
    }

    #[tokio::test]
    async fn test_init_db_creates_fts_table() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = init_db(&url).await.unwrap();

        // Verify FTS table exists
        let result = db
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='messages_fts'",
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);
    }

    #[tokio::test]
    async fn test_init_db_fts_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        // First init
        init_db(&url).await.unwrap();

        // Second init - FTS creation should not fail
        let result = init_db(&url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        // Before init, connection should be None
        let conn_before = get_connection().await;
        assert!(conn_before.is_none());

        // After init, connection should be available
        init_db(&url).await.unwrap();
        let conn_after = get_connection().await;
        assert!(conn_after.is_some());
    }

    #[tokio::test]
    async fn test_init_db_invalid_url() {
        let result = init_db("invalid://path").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid SQLite URL format"));
    }
}
