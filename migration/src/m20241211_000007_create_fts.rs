use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create FTS virtual table
        db.execute_unprepared(
            r#"CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content,
                tokenize = 'porter'
            )"#,
        )
        .await?;

        // Create insert trigger
        db.execute_unprepared(
            r#"CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
            BEGIN
                INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
            END"#,
        )
        .await?;

        // Create delete trigger
        db.execute_unprepared(
            r#"CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
            BEGIN
                DELETE FROM messages_fts WHERE rowid = OLD.rowid;
            END"#,
        )
        .await?;

        // Create update trigger
        db.execute_unprepared(
            r#"CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages
            BEGIN
                DELETE FROM messages_fts WHERE rowid = OLD.rowid;
                INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
            END"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Drop triggers first
        db.execute_unprepared("DROP TRIGGER IF EXISTS messages_au")
            .await?;

        db.execute_unprepared("DROP TRIGGER IF EXISTS messages_ad")
            .await?;

        db.execute_unprepared("DROP TRIGGER IF EXISTS messages_ai")
            .await?;

        // Drop FTS table
        db.execute_unprepared("DROP TABLE IF EXISTS messages_fts")
            .await?;

        Ok(())
    }
}
