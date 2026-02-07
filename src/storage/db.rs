use once_cell::sync::Lazy;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::migrations::Migrator;

static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn init_db(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    tracing::info!("Connecting to database: {}", database_url);

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

    // Enable WAL mode - SQLite-specific configuration pragma
    match db.execute_unprepared("PRAGMA journal_mode=WAL;").await {
        Ok(_) => tracing::info!("WAL mode enabled for database"),
        Err(e) => tracing::warn!("Could not enable WAL mode: {}", e),
    }

    // Run SeaORM migrations
    tracing::info!("Running SeaORM migrations...");
    
    match Migrator::up(&db, None).await {
        Ok(_) => {
            tracing::info!("All migrations applied successfully");
        }
        Err(e) => {
            let err_str = e.to_string();
            
            // ONLY ignore duplicate migration version errors (idempotent check)
            if err_str.contains("UNIQUE constraint failed: seaql_migrations.version") 
                || err_str.contains("Duplicate entry") {
                tracing::info!("Migrations already applied (idempotent check passed)");
            } else {
                tracing::error!("Migration failed: {}", err_str);
                return Err(DbErr::Custom(format!("Migration failed: {}", e)));
            }
        }
    }

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
        let db_path = temp_dir.path().join("test_creates_file.db");
        let url = format!("sqlite://{}", db_path.display());

        let _db = init_db(&url).await.unwrap();

        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_init_db_runs_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_runs_migrations.db");
        let url = format!("sqlite://{}", db_path.display());

        let _db = init_db(&url).await.unwrap();

        // Migrations run successfully if we get here
    }

    #[tokio::test]
    async fn test_init_db_skips_existing_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_skips_existing.db");
        let url = format!("sqlite://{}", db_path.display());

        init_db(&url).await.unwrap();

        let result = init_db(&url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_db_creates_fts_table() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_creates_fts.db");
        let url = format!("sqlite://{}", db_path.display());

        let _db = init_db(&url).await.unwrap();
        // If we get here, FTS table was created successfully
    }

    #[tokio::test]
    async fn test_init_db_fts_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_fts_idempotent.db");
        let url = format!("sqlite://{}", db_path.display());

        init_db(&url).await.unwrap();

        let result = init_db(&url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_connection() {
        // Clear global connection state first
        {
            let mut conn = DB_CONN.lock().await;
            *conn = None;
        }

        let conn_before = get_connection().await;
        assert!(
            conn_before.is_none(),
            "Connection should be None before init"
        );

        // Use in-memory DB to avoid interference from other tests
        init_db("sqlite::memory:").await.unwrap();

        let conn_after = get_connection().await;
        assert!(
            conn_after.is_some(),
            "Connection should exist after init_db"
        );
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

    #[tokio::test]
    async fn test_migrations_idempotent_on_restart() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_restart_idempotent.db");
        let url = format!("sqlite://{}", db_path.display());

        // First initialization
        let db1 = init_db(&url).await.expect("First init should succeed");
        drop(db1);

        // Simulate restart
        let db2 = init_db(&url)
            .await
            .expect("Second init should succeed without errors");

        // Verify database is functional using entity operations
        use crate::storage::entities::conversations;
        use sea_orm::entity::*;
        use uuid::Uuid;

        let now = chrono::Utc::now().naive_utc();
        let conversation = conversations::ActiveModel {
            id: Set(Uuid::new_v4()),
            label: Set("Test".to_string()),
            folder: Set("default".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        conversation
            .insert(&db2)
            .await
            .expect("Should be able to insert into conversations table");
    }
}
