use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create trigger for conversations
        manager
            .execute_unprepared(
                r#"
                CREATE TRIGGER IF NOT EXISTS update_conversations_updated_at
                AFTER UPDATE ON conversations
                BEGIN
                    UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
                END;
                "#,
            )
            .await?;

        // Create trigger for messages
        manager
            .execute_unprepared(
                r#"
                CREATE TRIGGER IF NOT EXISTS update_messages_updated_at
                AFTER UPDATE ON messages
                BEGIN
                    UPDATE messages SET timestamp = CURRENT_TIMESTAMP WHERE id = OLD.id;
                END;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .execute_unprepared("DROP TRIGGER IF EXISTS update_conversations_updated_at")
            .await?;

        manager
            .execute_unprepared("DROP TRIGGER IF EXISTS update_messages_updated_at")
            .await?;

        Ok(())
    }
}