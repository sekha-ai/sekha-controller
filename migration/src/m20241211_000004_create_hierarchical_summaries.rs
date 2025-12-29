use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HierarchicalSummaries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HierarchicalSummaries::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HierarchicalSummaries::ConversationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HierarchicalSummaries::Level)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HierarchicalSummaries::SummaryText)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HierarchicalSummaries::TimestampRange)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HierarchicalSummaries::GeneratedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HierarchicalSummaries::ModelUsed).string().null())
                    .col(ColumnDef::new(HierarchicalSummaries::TokenCount).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-hierarchical_summaries-conversation_id")
                            .from(
                                HierarchicalSummaries::Table,
                                HierarchicalSummaries::ConversationId,
                            )
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
                    .name("idx_summaries_conversation")
                    .table(HierarchicalSummaries::Table)
                    .col(HierarchicalSummaries::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_summaries_level")
                    .table(HierarchicalSummaries::Table)
                    .col(HierarchicalSummaries::Level)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(HierarchicalSummaries::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum HierarchicalSummaries {
    Table,
    Id,
    ConversationId,
    Level,
    SummaryText,
    TimestampRange,
    GeneratedAt,
    ModelUsed,
    TokenCount,
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
}