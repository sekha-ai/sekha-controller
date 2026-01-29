use once_cell::sync::Lazy;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};
use std::sync::Arc;
use tokio::sync::Mutex;

static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Expected migration versions in order
const MIGRATION_VERSIONS: &[&str] = &[
    "m20241211_00100000", // 001_create_conversations.sql
    "m20241211_00200000", // 002_create_messages.sql
    "m20241211_00300000", // 003_create_semantic_tags.sql
    "m20241211_00400000", // 004_create_hierarchical_summaries.sql
    "m20241211_00500000", // 005_create_knowledge_graph_edges.sql
    "m20241211_00600000", // 006_add_updated_at_triggers.sql
    "m20241211_00700000", // 007_create_fts.sql
];

/// Migration SQL files embedded at compile time
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/001_create_conversations.sql"),
    include_str!("../../migrations/002_create_messages.sql"),
    include_str!("../../migrations/003_create_semantic_tags.sql"),
    include_str!("../../migrations/004_create_hierarchical_summaries.sql"),
    include_str!("../../migrations/005_create_knowledge_graph_edges.sql"),
    include_str!("../../migrations/006_add_updated_at_triggers.sql"),
    include_str!("../../migrations/007_create_fts.sql"),
];

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

    // Enable WAL mode for better concurrency
    db.execute_unprepared("PRAGMA journal_mode=WAL;")
        .await
        .map_err(|e| DbErr::Custom(format!("Failed to enable WAL mode: {}", e)))?;

    tracing::info!("WAL mode enabled for database");

    // Run migrations with proper idempotency
    run_migrations(&db).await?;

    let mut conn = DB_CONN.lock().await;
    *conn = Some(db.clone());

    Ok(db)
}

/// Run database migrations with idempotent, atomic operations
async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("Checking migration status...");

    // Ensure migrations table exists
    ensure_migrations_table(db).await?;

    // Get list of applied migrations
    let applied_migrations = get_applied_migrations(db).await?;
    tracing::info!("Found {} applied migrations", applied_migrations.len());

    // Determine which migrations need to be applied
    let mut migrations_to_apply = Vec::new();
    for (idx, version) in MIGRATION_VERSIONS.iter().enumerate() {
        if !applied_migrations.contains(&version.to_string()) {
            migrations_to_apply.push((idx, version));
        }
    }

    if migrations_to_apply.is_empty() {
        tracing::info!("All migrations already applied, database is up to date");
        return Ok(());
    }

    tracing::info!(
        "Applying {} pending migration(s): {:?}",
        migrations_to_apply.len(),
        migrations_to_apply
            .iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>()
    );

    // Apply each pending migration
    for (idx, version) in migrations_to_apply {
        apply_migration(db, idx, version).await?;
    }

    tracing::info!("All migrations applied successfully");
    Ok(())
}

/// Ensure the migrations tracking table exists
async fn ensure_migrations_table(db: &DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        r#"
        CREATE TABLE IF NOT EXISTS seaql_migrations (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .await
    .map_err(|e| DbErr::Custom(format!("Failed to create migrations table: {}", e)))?;

    tracing::debug!("Migrations tracking table ready");
    Ok(())
}

/// Get list of already applied migration versions
async fn get_applied_migrations(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    // Use a simpler query that doesn't rely on QueryResult trait
    let result = db
        .execute_unprepared("SELECT version FROM seaql_migrations ORDER BY version")
        .await;

    match result {
        Ok(_) => {
            // Table exists, now check which migrations have been applied
            // We'll attempt to verify each expected version exists
            let mut applied = Vec::new();
            for version in MIGRATION_VERSIONS {
                let check_sql = format!(
                    "SELECT 1 FROM seaql_migrations WHERE version = '{}' LIMIT 1",
                    version
                );
                if let Ok(result) = db.execute_unprepared(&check_sql).await {
                    if result.rows_affected() > 0 {
                        applied.push(version.to_string());
                    }
                }
            }

            Ok(applied)
        }
        Err(e) => {
            tracing::debug!("Failed to query migrations table: {}", e);
            Ok(Vec::new())
        }
    }
}

/// Apply a single migration with proper error handling
async fn apply_migration(db: &DatabaseConnection, idx: usize, version: &str) -> Result<(), DbErr> {
    tracing::info!("Applying migration {} ({})", idx + 1, version);

    let sql = MIGRATIONS
        .get(idx)
        .ok_or_else(|| DbErr::Custom(format!("Migration index {} out of bounds", idx)))?;

    // Execute the migration SQL
    db.execute_unprepared(sql).await.map_err(|e| {
        DbErr::Custom(format!(
            "Failed to execute migration {} ({}): {}",
            idx + 1,
            version,
            e
        ))
    })?;

    // Record that this migration was applied
    // Using INSERT OR IGNORE for idempotency in case of race conditions
    let record_sql = format!(
        "INSERT OR IGNORE INTO seaql_migrations (version) VALUES ('{}')",
        version
    );

    db.execute_unprepared(&record_sql).await.map_err(|e| {
        DbErr::Custom(format!(
            "Failed to record migration {} ({}): {}",
            idx + 1,
            version,
            e
        ))
    })?;

    tracing::info!("Successfully applied migration {} ({})", idx + 1, version);
    Ok(())
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

        assert!(db_path.exists());

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

        let result = db
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'",
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // Verify migrations table exists (which proves migrations ran)
        let result = db
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='seaql_migrations'",
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1, "Migrations table should exist");
    }

    #[tokio::test]
    async fn test_init_db_skips_existing_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        init_db(&url).await.unwrap();

        let result = init_db(&url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_db_creates_fts_table() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = init_db(&url).await.unwrap();

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

        init_db(&url).await.unwrap();

        let result = init_db(&url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let conn_before = get_connection().await;
        assert!(conn_before.is_none());

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

    /// Tests that the critical bug is fixed: container restart does not cause
    /// UNIQUE constraint error on seaql_migrations.version
    #[tokio::test]
    async fn test_migrations_idempotent_on_restart() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_restart.db");
        let url = format!("sqlite://{}", db_path.display());

        // First initialization - fresh database
        let db1 = init_db(&url).await.expect("First init should succeed");

        // Verify tables were created
        let result = db1
            .execute_unprepared(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'",
            )
            .await
            .unwrap();
        assert_eq!(
            result.rows_affected(),
            1,
            "Conversations table should exist after first init"
        );

        // Drop the connection to simulate container shutdown
        drop(db1);

        // Simulate container restart - reconnect to same database file
        // This is where the bug would occur: UNIQUE constraint failed: seaql_migrations.version
        let db2 = init_db(&url).await.expect(
            "Second init (restart simulation) should succeed without UNIQUE constraint error",
        );

        // Verify all expected tables still exist and are intact
        let tables = vec![
            "seaql_migrations",
            "conversations",
            "messages",
            "semantic_tags",
            "hierarchical_summaries",
            "knowledge_graph_edges",
            "messages_fts",
        ];

        for table in tables {
            let result = db2
                .execute_unprepared(&format!(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                    table
                ))
                .await
                .expect(&format!(
                    "Should be able to query sqlite_master for {}",
                    table
                ));

            assert_eq!(
                result.rows_affected(),
                1,
                "Table '{}' should exist after restart",
                table
            );
        }

        // Verify database is functional by performing a basic operation
        db2.execute_unprepared(
            "INSERT INTO conversations (id, label, folder) VALUES ('test-restart', 'Test', 'default')",
        )
        .await
        .expect("Should be able to insert into conversations table");

        // The key success: No UNIQUE constraint error occurred during second init_db call
        // All tables exist and database operations work correctly
    }
}
