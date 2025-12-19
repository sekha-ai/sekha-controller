use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create FTS5 virtual table
        manager
            .execute_unprepared(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                    content,
                    tokenize = 'porter'
                );
                "#,
            )
            .await?;

        // Create sync trigger for INSERT
        manager
            .execute_unprepared(
                r#"
                CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
                BEGIN
                    INSERT INTO messages_fts(rowid, content) VALUES (NEW.id, NEW.content);
                END;
                "#,
            )
            .await?;

        // Create sync trigger for DELETE
        manager
            .execute_unprepared(
                r#"
                CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
                BEGIN
                    DELETE FROM messages_fts WHERE rowid = OLD.id;
                END;
                "#,
            )
            .await?;

        // Create sync trigger for UPDATE
        manager
            .execute_unprepared(
                r#"
                CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages
                BEGIN
                    DELETE FROM messages_fts WHERE rowid = OLD.id;
                    INSERT INTO messages_fts(rowid, content) VALUES (NEW.id, NEW.content);
                END;
                "#,
            )
            .await?;

        // Enable WAL mode for better concurrency
        manager
            .execute_unprepared("PRAGMA journal_mode=WAL;")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .execute_unprepared("DROP TRIGGER IF EXISTS messages_ai")
            .await?;
        manager
            .execute_unprepared("DROP TRIGGER IF EXISTS messages_ad")
            .await?;
        manager
            .execute_unprepared("DROP TRIGGER IF EXISTS messages_au")
            .await?;
        manager
            .execute_unprepared("DROP TABLE IF EXISTS messages_fts")
            .await?;

        Ok(())
    }
}