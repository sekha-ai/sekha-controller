use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Conversations::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Conversations::Id).string_len(36).not_null().primary_key())
                    .col(ColumnDef::new(Conversations::Label).string().not_null())
                    .col(ColumnDef::new(Conversations::Folder).string().not_null())
                    .col(ColumnDef::new(Conversations::Status).string().not_null().extra("DEFAULT 'active'".to_owned()))
                    .col(ColumnDef::new(Conversations::ImportanceScore).integer().not_null().extra("DEFAULT 5".to_owned()))
                    .col(ColumnDef::new(Conversations::WordCount).integer().not_null().extra("DEFAULT 0".to_owned()))
                    .col(ColumnDef::new(Conversations::SessionCount).integer().not_null().extra("DEFAULT 1".to_owned()))
                    .col(ColumnDef::new(Conversations::CreatedAt).string().not_null())
                    .col(ColumnDef::new(Conversations::UpdatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_conversations_label_status")
                    .table(Conversations::Table)
                    .col(Conversations::Label)
                    .col(Conversations::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_conversations_folder_updated")
                    .table(Conversations::Table)
                    .col(Conversations::Folder)
                    .col(Conversations::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        // Enable WAL mode
        manager
            .execute_unprepared("PRAGMA journal_mode=WAL;")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Conversations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
    Label,
    Folder,
    CreatedAt,
    UpdatedAt,
    Status,
    ImportanceScore,
    WordCount,
    SessionCount,
}
