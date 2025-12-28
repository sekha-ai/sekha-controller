use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SemanticTags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SemanticTags::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SemanticTags::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SemanticTags::Tag)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SemanticTags::Confidence)
                            .float()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SemanticTags::ExtractedAt)
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-semantic_tags-conversation_id")
                            .from(SemanticTags::Table, SemanticTags::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_tags_conversation")
                    .table(SemanticTags::Table)
                    .col(SemanticTags::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tags_tag")
                    .table(SemanticTags::Table)
                    .col(SemanticTags::Tag)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SemanticTags::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SemanticTags {
    Table,
    Id,
    ConversationId,
    Tag,
    Confidence,
    ExtractedAt,
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
}