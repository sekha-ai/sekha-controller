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
                    .col(
                        ColumnDef::new(Conversations::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Conversations::Label).string().not_null())
                    .col(ColumnDef::new(Conversations::Folder).string().not_null())
                    .col(
                        ColumnDef::new(Conversations::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::Status)
                            .string_len(20)
                            .not_null()
                            .default("active"),
                    )
                    .col(ColumnDef::new(Conversations::ImportanceScore).integer().not_null().default(5))
                    .col(ColumnDef::new(Conversations::WordCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Conversations::SessionCount).integer().not_null().default(1))
                    .index(
                        Index::create()
                            .name("idx_conversations_label_status")
                            .col(Conversations::Label)
                            .col(Conversations::Status),
                    )
                    .index(
                        Index::create()
                            .name("idx_conversations_folder_updated")
                            .col(Conversations::Folder)
                            .col(Conversations::UpdatedAt),
                    )
                    .to_owned(),
            )
            .await
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
    Label,           # ADDED
    Folder,          # ADDED
    CreatedAt,       # ADDED
    UpdatedAt,       # ADDED
    Status,          # ADDED
    ImportanceScore, # ADDED
    WordCount,       # ADDED
    SessionCount,    # ADDED
}
