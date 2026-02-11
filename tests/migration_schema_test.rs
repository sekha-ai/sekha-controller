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
        if trimmed.contains(" TIMESTAMP ")
            || trimmed.contains(" TIMESTAMP,")
            || trimmed.ends_with(" TIMESTAMP")
        {
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

    // Verify required tables exist (with quotes as SQLite outputs them)
    if !schema.contains("conversations") {
        panic!("❌ No conversations table found in schema");
    }

    if !schema.contains("messages") {
        panic!("❌ No messages table found in schema");
    }

    // Verify FTS table exists
    if !schema.contains("messages_fts") {
        panic!("❌ No FTS table found in schema");
    }

    // Verify triggers exist
    if !schema.contains("update_conversations_updated_at") {
        panic!("❌ Missing update_conversations_updated_at trigger");
    }

    if !schema.contains("messages_ai") {
        panic!("❌ Missing FTS insert trigger");
    }

    // Verify seaql_migrations table has correct schema
    if !schema.contains("seaql_migrations") {
        panic!("❌ No seaql_migrations table found in schema");
    }

    // Verify seaql_migrations.applied_at is INTEGER (not TEXT from v0.1.x)
    // The schema should look like: applied_at INTEGER NOT NULL
    let mut found_seaql_migrations_block = false;
    let mut found_applied_at_integer = false;
    
    for line in &lines {
        let trimmed = line.trim();
        
        // Check if we're in seaql_migrations table definition
        if trimmed.contains("CREATE TABLE") && trimmed.contains("seaql_migrations") {
            found_seaql_migrations_block = true;
        }
        
        // If in the block, check for applied_at with INTEGER
        if found_seaql_migrations_block {
            if trimmed.contains("applied_at") && trimmed.contains("INTEGER") {
                found_applied_at_integer = true;
                eprintln!("✅ Found correct seaql_migrations.applied_at type: {}", trimmed);
                break;
            }
            // Exit block when we hit the closing parenthesis
            if trimmed == ");" {
                found_seaql_migrations_block = false;
            }
        }
    }

    if !found_applied_at_integer {
        panic!(
            "❌ seaql_migrations.applied_at is not INTEGER type!\n\nThis means the migration fix didn't work.\nExpected: applied_at INTEGER NOT NULL\n\n{}",
            schema
        );
    }

    eprintln!("✅ All schema validations passed!");
}
