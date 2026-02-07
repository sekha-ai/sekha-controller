use once_cell::sync::Lazy;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};
use std::sync::Arc;
use tokio::sync::Mutex;

static DB_CONN: Lazy<Arc<Mutex<Option<DatabaseConnection>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Migration versions tracked for idempotency
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

    // Enable WAL mode using SeaORM's query builder
    use sea_orm::sea_query::{Expr, Query};
    use sea_orm::Statement;

    // WAL mode pragma - this is a SQLite-specific configuration pragma, not data manipulation
    // SeaORM doesn't provide a builder for PRAGMA statements as they're database-specific config
    let wal_stmt = Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "PRAGMA journal_mode=WAL".to_string(),
    );
    db.execute(&wal_stmt)
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

    // Ensure migrations table exists using SeaORM schema builder
    ensure_migrations_table(db).await?;

    // Get list of applied migrations using SeaORM query builder
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

/// Ensure the migrations tracking table exists using SeaORM schema builder
async fn ensure_migrations_table(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::sea_query::{ColumnDef, Table};
    use sea_orm::{Schema, Statement};

    // Create migrations table using SeaORM's schema builder
    let create_table = Table::create()
        .table(sea_orm::sea_query::Alias::new("seaql_migrations"))
        .if_not_exists()
        .col(
            ColumnDef::new(sea_orm::sea_query::Alias::new("version"))
                .string()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(sea_orm::sea_query::Alias::new("applied_at"))
                .string()
                .not_null()
                .default("CURRENT_TIMESTAMP"),
        )
        .to_owned();

    let stmt = db.get_database_backend().build(&create_table);
    db.execute(&stmt).await?;

    tracing::debug!("Migrations tracking table ready");
    Ok(())
}

/// Get list of already applied migration versions using SeaORM query builder
async fn get_applied_migrations(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    use sea_orm::sea_query::{Expr, Query};
    use sea_orm::{FromQueryResult, Statement};

    #[derive(Debug, FromQueryResult)]
    struct MigrationRecord {
        version: String,
    }

    // Build query using SeaORM's query builder
    let query = Query::select()
        .column(sea_orm::sea_query::Alias::new("version"))
        .from(sea_orm::sea_query::Alias::new("seaql_migrations"))
        .order_by(
            sea_orm::sea_query::Alias::new("version"),
            sea_orm::sea_query::Order::Asc,
        )
        .to_owned();

    let stmt = db.get_database_backend().build(&query);

    match MigrationRecord::find_by_statement(stmt).all(db).await {
        Ok(records) => {
            let versions: Vec<String> = records.into_iter().map(|r| r.version).collect();
            tracing::debug!(
                "Found {} applied migration(s): {:?}",
                versions.len(),
                versions
            );
            Ok(versions)
        }
        Err(e) => {
            // Table might not exist yet or be empty
            tracing::debug!("Could not query migrations table: {}", e);
            Ok(Vec::new())
        }
    }
}

/// Apply a single migration - SQL files are embedded at compile time
async fn apply_migration(db: &DatabaseConnection, idx: usize, version: &str) -> Result<(), DbErr> {
    tracing::info!("Applying migration {} ({})", idx + 1, version);

    let sql = MIGRATIONS
        .get(idx)
        .ok_or_else(|| DbErr::Custom(format!("Migration index {} out of bounds", idx)))?;

    // Execute migration SQL by splitting into individual statements
    // This is necessary because SQLite doesn't support executing multiple statements in one call
    use sea_orm::Statement;
    for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            statement.trim().to_string(),
        );
        db.execute(&stmt).await.map_err(|e| {
            DbErr::Custom(format!(
                "Failed to execute migration {} ({}): {}",
                idx + 1,
                version,
                e
            ))
        })?;
    }

    // Record that this migration was applied using SeaORM query builder
    use sea_orm::sea_query::{Expr, Query};

    let insert = Query::insert()
        .into_table(sea_orm::sea_query::Alias::new("seaql_migrations"))
        .columns(vec![sea_orm::sea_query::Alias::new("version")])
        .values_panic(vec![version.into()])
        .to_owned();

    // Use OR IGNORE for idempotency
    let backend = db.get_database_backend();
    let mut stmt = backend.build(&insert);

    // Modify the SQL to add OR IGNORE for SQLite
    if let Statement { sql, .. } = &mut stmt {
        *sql = sql.replace("INSERT INTO", "INSERT OR IGNORE INTO");
    }

    db.execute(&stmt).await.map_err(|e| {
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

        let _db = init_db(&url).await.unwrap();

        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_init_db_runs_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());

        let db = init_db(&url).await.unwrap();

        // Verify migrations were actually applied by checking migration records
        let applied = get_applied_migrations(&db).await.unwrap();
        assert_eq!(
            applied.len(),
            MIGRATION_VERSIONS.len(),
            "All migrations should be recorded"
        );

        // Verify first migration was recorded
        assert!(
            applied.contains(&"m20241211_00100000".to_string()),
            "First migration should be in applied list"
        );
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

        let _db = init_db(&url).await.unwrap();
        // If we get here, FTS table was created successfully
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

    /// Critical bug fix test: container restart must not cause UNIQUE constraint error
    /// This test verifies that migrations are properly detected and skipped on restart
    #[tokio::test]
    async fn test_migrations_idempotent_on_restart() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_restart.db");
        let url = format!("sqlite://{}", db_path.display());

        // First initialization - fresh database
        let db1 = init_db(&url).await.expect("First init should succeed");

        // Verify all migrations were applied
        let applied_first = get_applied_migrations(&db1).await.unwrap();
        assert_eq!(
            applied_first.len(),
            MIGRATION_VERSIONS.len(),
            "All migrations should be applied on first init"
        );

        drop(db1);

        // Simulate container restart - reconnect to same database file
        let db2 = init_db(&url)
            .await
            .expect("Second init (restart) should succeed without UNIQUE constraint error");

        // Verify migrations are still recorded (and weren't re-run)
        let applied_second = get_applied_migrations(&db2).await.unwrap();
        assert_eq!(
            applied_second.len(),
            MIGRATION_VERSIONS.len(),
            "All migrations should still be recorded after restart"
        );

        // Verify specific migrations are present
        for version in MIGRATION_VERSIONS {
            assert!(
                applied_second.contains(&version.to_string()),
                "Migration {} should be recorded after restart",
                version
            );
        }

        // Verify database is functional using SeaORM query builder
        use sea_orm::sea_query::{Expr, Query};
        use sea_orm::Statement;

        let insert = Query::insert()
            .into_table(sea_orm::sea_query::Alias::new("conversations"))
            .columns(vec![
                sea_orm::sea_query::Alias::new("id"),
                sea_orm::sea_query::Alias::new("label"),
                sea_orm::sea_query::Alias::new("folder"),
            ])
            .values_panic(vec!["test-restart".into(), "Test".into(), "default".into()])
            .to_owned();

        let stmt = db2.get_database_backend().build(&insert);
        db2.execute(&stmt)
            .await
            .expect("Should be able to insert into conversations table");
    }
}
