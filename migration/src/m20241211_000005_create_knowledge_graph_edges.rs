use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeGraphEdges::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KnowledgeGraphEdges::SubjectId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeGraphEdges::Predicate)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeGraphEdges::ObjectId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeGraphEdges::ConversationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeGraphEdges::ExtractedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk-knowledge_graph_edges")
                            .col(KnowledgeGraphEdges::SubjectId)
                            .col(KnowledgeGraphEdges::Predicate)
                            .col(KnowledgeGraphEdges::ObjectId)
                            .col(KnowledgeGraphEdges::ConversationId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-knowledge_graph_edges-conversation_id")
                            .from(
                                KnowledgeGraphEdges::Table,
                                KnowledgeGraphEdges::ConversationId,
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
                    .name("idx_edges_subject")
                    .table(KnowledgeGraphEdges::Table)
                    .col(KnowledgeGraphEdges::SubjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_edges_object")
                    .table(KnowledgeGraphEdges::Table)
                    .col(KnowledgeGraphEdges::ObjectId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(KnowledgeGraphEdges::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum KnowledgeGraphEdges {
    Table,
    SubjectId,
    Predicate,
    ObjectId,
    ConversationId,
    ExtractedAt,
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
}