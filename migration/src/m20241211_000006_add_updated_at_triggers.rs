use sea_orm_migration::prelude::*;
use sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create trigger for conversations.updated_at
        let db = manager.get_connection();
        db.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            r#"
                CREATE TRIGGER IF NOT EXISTS update_conversations_updated_at
                AFTER UPDATE ON conversations
                BEGIN
                    UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
                END;
            "#.to_owned(),
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "DROP TRIGGER IF EXISTS update_conversations_updated_at".to_owned(),
        ))
        .await?;

        Ok(())
    }
}