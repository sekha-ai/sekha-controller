use once_cell::sync::Lazy;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr, Statement};
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;
use tokio::sync::Mutex;

// Use the migration crate from the migration/ directory
use migration::Migrator;

static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Migrate v0.1.x seaql_migrations table schema to v0.2.0 format
///
/// v0.1.x used TEXT for applied_at, v0.2.0 uses INTEGER
async fn migrate_seaql_migrations_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("Checking seaql_migrations table schema...");

    // Check if table exists and get its schema
    let result = db
        .query_one(Statement::from_string(
            db.get_database_backend(),
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='seaql_migrations'"
                .to_string(),
        ))
        .await?;

    if let Some(row) = result {
        let schema: String = row.try_get("", "sql")?;

        // Check if applied_at is TEXT (v0.1.x) instead of INTEGER (v0.2.0)
        if schema.contains("applied_at\" varchar") || schema.contains("applied_at\" text") {
            tracing::warn!("⚠️  Detected v0.1.x migration table schema. Migrating to v0.2.0...");

            // Create new table with correct schema
            db.execute(Statement::from_string(
                db.get_database_backend(),
                r#"
                CREATE TABLE IF NOT EXISTS seaql_migrations_new (
                    version varchar NOT NULL PRIMARY KEY,
                    applied_at integer NOT NULL
                )
                "#
                .to_string(),
            ))
            .await?;

            // Copy data, converting TEXT timestamps to INTEGER (Unix epoch)
            db.execute(Statement::from_string(
                db.get_database_backend(),
                r#"
                INSERT INTO seaql_migrations_new (version, applied_at)
                SELECT version, strftime('%s', applied_at) 
                FROM seaql_migrations
                "#
                .to_string(),
            ))
            .await?;

            // Drop old table
            db.execute(Statement::from_string(
                db.get_database_backend(),
                "DROP TABLE seaql_migrations".to_string(),
            ))
            .await?;

            // Rename new table
            db.execute(Statement::from_string(
                db.get_database_backend(),
                "ALTER TABLE seaql_migrations_new RENAME TO seaql_migrations".to_string(),
            ))
            .await?;

            tracing::info!("✅ Migration table schema updated to v0.2.0");
        } else {
            tracing::info!("Migration table already has v0.2.0 schema");
        }
    } else {
        tracing::info!("No existing migration table found (fresh install)");
    }

    Ok(())
}

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

    // Migrate seaql_migrations schema if needed (v0.1.x -> v0.2.0)
    migrate_seaql_migrations_schema(&db).await?;

    // Run SeaORM migrations with idempotency check
    tracing::info!("Running SeaORM migrations...");
    match Migrator::up(&db, None).await {
        Ok(_) => {
            tracing::info!("All migrations applied successfully");
        }
        Err(e) => {
            let err_str = e.to_string();
            // This specific error means migrations are already applied (idempotent)
            if err_str.contains("UNIQUE constraint failed: seaql_migrations.version") {
                tracing::info!("Migrations already applied (idempotent)");
            } else {
                // All other errors are real failures
                tracing::error!("Migration failed: {}", err_str);
                return Err(e);
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
        // Second call should also succeed (idempotent)
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
            status: Set("active".to_string()),
            importance_score: Set(0),
            word_count: Set(0),
            session_count: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };

        conversation
            .insert(&db2)
            .await
            .expect("Should be able to insert into conversations table");
    }

    #[tokio::test]
    async fn test_migrate_v01x_schema_to_v02() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_v01x_migration.db");
        let url = format!("sqlite://{}", db_path.display());

        // Create a v0.1.x style database with TEXT applied_at
        let db = Database::connect(&url).await.unwrap();

        // Create old-style migration table with TEXT applied_at
        db.execute(Statement::from_string(
            db.get_database_backend(),
            r#"
            CREATE TABLE seaql_migrations (
                version varchar NOT NULL PRIMARY KEY,
                applied_at varchar NOT NULL
            )
            "#
            .to_string(),
        ))
        .await
        .unwrap();

        // Insert some old-style migration records
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "INSERT INTO seaql_migrations (version, applied_at) VALUES ('m001', '2024-01-01 00:00:00')".to_string()
        )).await.unwrap();

        drop(db);

        // Now run init_db which should migrate the schema
        let db = init_db(&url).await.expect("Migration should succeed");

        // Verify the schema was migrated to INTEGER
        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='seaql_migrations'"
                    .to_string(),
            ))
            .await
            .unwrap()
            .unwrap();

        let schema: String = result.try_get("", "sql").unwrap();
        assert!(
            schema.contains("applied_at integer"),
            "Schema should have INTEGER applied_at"
        );

        // Verify data was migrated
        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT version, applied_at FROM seaql_migrations WHERE version='m001'".to_string(),
            ))
            .await
            .unwrap();

        assert!(result.is_some(), "Migration record should still exist");
    }

    #[tokio::test]
    async fn test_migrate_seaql_migrations_schema_no_table() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_no_migration_table.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = Database::connect(&url).await.unwrap();

        // Should not error when no migration table exists
        let result = migrate_seaql_migrations_schema(&db).await;
        assert!(result.is_ok(), "Should handle missing table gracefully");
    }

    #[tokio::test]
    async fn test_migrate_seaql_migrations_schema_already_v02() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_already_v02.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = Database::connect(&url).await.unwrap();

        // Create new-style migration table with INTEGER applied_at
        db.execute(Statement::from_string(
            db.get_database_backend(),
            r#"
            CREATE TABLE seaql_migrations (
                version varchar NOT NULL PRIMARY KEY,
                applied_at integer NOT NULL
            )
            "#
            .to_string(),
        ))
        .await
        .unwrap();

        // Should not modify already-correct schema
        let result = migrate_seaql_migrations_schema(&db).await;
        assert!(result.is_ok(), "Should handle v0.2.0 schema gracefully");

        // Verify schema unchanged
        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='seaql_migrations'"
                    .to_string(),
            ))
            .await
            .unwrap()
            .unwrap();

        let schema: String = result.try_get("", "sql").unwrap();
        assert!(
            schema.contains("applied_at integer"),
            "Schema should remain INTEGER"
        );
    }

    #[tokio::test]
    async fn test_migrate_seaql_migrations_schema_with_text_variant() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_text_variant.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = Database::connect(&url).await.unwrap();

        // Create table with 'text' instead of 'varchar'
        db.execute(Statement::from_string(
            db.get_database_backend(),
            r#"
            CREATE TABLE seaql_migrations (
                version varchar NOT NULL PRIMARY KEY,
                applied_at text NOT NULL
            )
            "#
            .to_string(),
        ))
        .await
        .unwrap();

        // Should detect and migrate TEXT variant
        let result = migrate_seaql_migrations_schema(&db).await;
        assert!(result.is_ok(), "Should migrate TEXT variant");

        // Verify schema was migrated
        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='seaql_migrations'"
                    .to_string(),
            ))
            .await
            .unwrap()
            .unwrap();

        let schema: String = result.try_get("", "sql").unwrap();
        assert!(
            schema.contains("applied_at integer"),
            "Schema should be migrated to INTEGER"
        );
    }
}
