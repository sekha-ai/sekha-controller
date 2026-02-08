use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create FTS virtual table
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            r#"CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content,
                tokenize = 'porter'
            )"#.to_string(),
        ))
        .await?;

        // Create insert trigger
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            r#"CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
            BEGIN
                INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
            END"#.to_string(),
        ))
        .await?;

        // Create delete trigger
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            r#"CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
            BEGIN
                DELETE FROM messages_fts WHERE rowid = OLD.rowid;
            END"#.to_string(),
        ))
        .await?;

        // Create update trigger
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            r#"CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages
            BEGIN
                DELETE FROM messages_fts WHERE rowid = OLD.rowid;
                INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
            END"#.to_string(),
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Drop triggers first
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "DROP TRIGGER IF EXISTS messages_au".to_string(),
        ))
        .await?;

        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "DROP TRIGGER IF EXISTS messages_ad".to_string(),
        ))
        .await?;

        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "DROP TRIGGER IF EXISTS messages_ai".to_string(),
        ))
        .await?;

        // Drop FTS table
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "DROP TABLE IF EXISTS messages_fts".to_string(),
        ))
        .await?;

        Ok(())
    }
}
