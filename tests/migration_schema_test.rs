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

    // Check for TIMESTAMP as a column type (not as part of CURRENT_TIMESTAMP function)
    let lines: Vec<&str> = schema.lines().collect();
    let mut has_timestamp_type = false;
    
    for line in &lines {
        let trimmed = line.trim();
        // Skip trigger definitions and function calls
        if trimmed.contains("CURRENT_TIMESTAMP") || trimmed.starts_with("--") {
            continue;
        }
        // Check for TIMESTAMP as a type declaration (with space or comma after)
        if trimmed.contains(" TIMESTAMP ") || trimmed.contains(" TIMESTAMP,") || trimmed.ends_with(" TIMESTAMP") {
            has_timestamp_type = true;
            eprintln!("❌ Found TIMESTAMP type in line: {}", trimmed);
        }
    }

    if has_timestamp_type {
        panic!(
            "❌ Migration created TIMESTAMP column type(s)!\n\nExpected TEXT for all datetime columns.\n\n{}",
            schema
        );
    }

    // Verify required tables exist
    if !schema.contains("CREATE TABLE conversations") {
        panic!("❌ No conversations table found in schema");
    }

    if !schema.contains("CREATE TABLE messages") {
        panic!("❌ No messages table found in schema");
    }

    // Verify FTS table exists
    if !schema.contains("CREATE VIRTUAL TABLE messages_fts") {
        panic!("❌ No FTS table found in schema");
    }

    // Verify triggers exist
    if !schema.contains("CREATE TRIGGER update_conversations_updated_at") {
        panic!("❌ Missing update_conversations_updated_at trigger");
    }

    if !schema.contains("CREATE TRIGGER messages_ai") {
        panic!("❌ Missing FTS insert trigger");
    }

    eprintln!("✅ All schema validations passed!");
}
