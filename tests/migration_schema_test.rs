use std::process::Command;

#[tokio::test]
async fn test_migration_schema() {
    // Create temporary database file
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}", db_path.display());

    // Initialize database (runs migrations)
    let _db = sekha_controller::init_db(&db_url).await.unwrap();
    drop(_db); // Close connection so sqlite3 can read it

    // Inspect schema directly
    let output = Command::new("sqlite3")
        .arg(&db_path)
        .arg(".schema")
        .output()
        .expect("sqlite3 command failed");

    let schema = String::from_utf8_lossy(&output.stdout);
    eprintln!("=== ACTUAL SCHEMA FROM MIGRATIONS ===");
    eprintln!("{}", schema);

    // Check for wrong column types
    if schema.contains("TIMESTAMP") {
        panic!(
            "❌ Migration created TIMESTAMP column(s)!\n\nExpected TEXT for all columns.\n\n{}",
            schema
        );
    }

    if !schema.contains("CREATE TABLE conversations") {
        panic!("❌ No conversations table found in schema");
    }
}
