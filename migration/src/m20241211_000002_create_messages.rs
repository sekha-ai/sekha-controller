use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Messages::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Messages::ConversationId)
                            .string_len(36)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Messages::Role).string_len(20).not_null())
                    .col(ColumnDef::new(Messages::Content).text().not_null())
                    .col(
                        ColumnDef::new(Messages::Timestamp)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Messages::EmbeddingId).string_len(36).null())
                    .col(ColumnDef::new(Messages::Metadata).json().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_message_conversation")
                            .from(Messages::Table, Messages::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_messages_conversation_id")
                            .col(Messages::ConversationId),
                    )
                    .index(
                        Index::create()
                            .name("idx_messages_timestamp")
                            .col(Messages::Timestamp),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    ConversationId,
    Role,
    Content,
    Timestamp,
    EmbeddingId,
    Metadata,
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
}
