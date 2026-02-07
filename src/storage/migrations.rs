use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241211_001_create_conversations::Migration),
            Box::new(m20241211_002_create_messages::Migration),
            Box::new(m20241211_003_create_semantic_tags::Migration),
            Box::new(m20241211_004_create_hierarchical_summaries::Migration),
            Box::new(m20241211_005_create_knowledge_graph_edges::Migration),
            Box::new(m20241211_006_add_updated_at_triggers::Migration),
            Box::new(m20241211_007_create_fts::Migration),
        ]
    }
}

mod m20241211_001_create_conversations {
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
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Conversations::Label).string().not_null())
                        .col(ColumnDef::new(Conversations::Folder).string().not_null())
                        .col(ColumnDef::new(Conversations::CreatedAt).string().not_null())
                        .col(ColumnDef::new(Conversations::UpdatedAt).string().not_null())
                        .col(
                            ColumnDef::new(Conversations::Status)
                                .string()
                                .not_null()
                                .default("active"),
                        )
                        .col(
                            ColumnDef::new(Conversations::ImportanceScore)
                                .integer()
                                .not_null()
                                .default(5),
                        )
                        .col(
                            ColumnDef::new(Conversations::WordCount)
                                .integer()
                                .not_null()
                                .default(0),
                        )
                        .col(
                            ColumnDef::new(Conversations::SessionCount)
                                .integer()
                                .not_null()
                                .default(1),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
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
                        .if_not_exists()
                        .name("idx_conversations_folder_updated")
                        .table(Conversations::Table)
                        .col(Conversations::Folder)
                        .col(Conversations::UpdatedAt)
                        .to_owned(),
                )
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
}

mod m20241211_002_create_messages {
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
                        .col(ColumnDef::new(Messages::Id).string().not_null().primary_key())
                        .col(ColumnDef::new(Messages::ConversationId).string().not_null())
                        .col(ColumnDef::new(Messages::Role).string().not_null())
                        .col(ColumnDef::new(Messages::Content).string().not_null())
                        .col(ColumnDef::new(Messages::Timestamp).string().not_null())
                        .col(ColumnDef::new(Messages::EmbeddingId).string())
                        .col(ColumnDef::new(Messages::Metadata).string())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_messages_conversation")
                                .from(Messages::Table, Messages::ConversationId)
                                .to(Conversations::Table, Conversations::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_messages_conversation_id")
                        .table(Messages::Table)
                        .col(Messages::ConversationId)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_messages_timestamp")
                        .table(Messages::Table)
                        .col(Messages::Timestamp)
                        .to_owned(),
                )
                .await?;

            Ok(())
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
}

mod m20241211_003_create_semantic_tags {
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
                        .col(ColumnDef::new(SemanticTags::Id).string().not_null().primary_key())
                        .col(ColumnDef::new(SemanticTags::ConversationId).string().not_null())
                        .col(ColumnDef::new(SemanticTags::Tag).string().not_null())
                        .col(ColumnDef::new(SemanticTags::Confidence).float().not_null())
                        .col(ColumnDef::new(SemanticTags::ExtractedAt).string().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_semantic_tags_conversation")
                                .from(SemanticTags::Table, SemanticTags::ConversationId)
                                .to(Conversations::Table, Conversations::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_tags_conversation")
                        .table(SemanticTags::Table)
                        .col(SemanticTags::ConversationId)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
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
}

mod m20241211_004_create_hierarchical_summaries {
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
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(HierarchicalSummaries::ConversationId)
                                .string()
                                .not_null(),
                        )
                        .col(ColumnDef::new(HierarchicalSummaries::Level).string().not_null())
                        .col(
                            ColumnDef::new(HierarchicalSummaries::SummaryText)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(HierarchicalSummaries::TimestampRange)
                                .string()
                                .not_null(),
                        )
                        .col(ColumnDef::new(HierarchicalSummaries::GeneratedAt).string().not_null())
                        .col(ColumnDef::new(HierarchicalSummaries::ModelUsed).string())
                        .col(ColumnDef::new(HierarchicalSummaries::TokenCount).integer())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_hierarchical_summaries_conversation")
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

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_summaries_conversation")
                        .table(HierarchicalSummaries::Table)
                        .col(HierarchicalSummaries::ConversationId)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
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
                .drop_table(Table::drop().table(HierarchicalSummaries::Table).to_owned())
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
}

mod m20241211_005_create_knowledge_graph_edges {
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
                        .col(ColumnDef::new(KnowledgeGraphEdges::SubjectId).string().not_null())
                        .col(ColumnDef::new(KnowledgeGraphEdges::Predicate).string().not_null())
                        .col(ColumnDef::new(KnowledgeGraphEdges::ObjectId).string().not_null())
                        .col(
                            ColumnDef::new(KnowledgeGraphEdges::ConversationId)
                                .string()
                                .not_null(),
                        )
                        .col(ColumnDef::new(KnowledgeGraphEdges::ExtractedAt).string().not_null())
                        .primary_key(
                            Index::create()
                                .col(KnowledgeGraphEdges::SubjectId)
                                .col(KnowledgeGraphEdges::Predicate)
                                .col(KnowledgeGraphEdges::ObjectId)
                                .col(KnowledgeGraphEdges::ConversationId),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_knowledge_graph_edges_conversation")
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

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_edges_subject")
                        .table(KnowledgeGraphEdges::Table)
                        .col(KnowledgeGraphEdges::SubjectId)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
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
                .drop_table(Table::drop().table(KnowledgeGraphEdges::Table).to_owned())
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
}

mod m20241211_006_add_updated_at_triggers {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            // SQLite triggers - no SeaORM builder support, must use raw SQL
            let sql = r#"
                CREATE TRIGGER IF NOT EXISTS update_conversations_updated_at
                AFTER UPDATE ON conversations
                BEGIN
                    UPDATE conversations SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
                END;

                CREATE TRIGGER IF NOT EXISTS update_messages_updated_at
                AFTER UPDATE ON messages
                BEGIN
                    UPDATE messages SET timestamp = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
                END;
            "#;

            manager.get_connection().execute_unprepared(sql).await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            let sql = r#"
                DROP TRIGGER IF EXISTS update_conversations_updated_at;
                DROP TRIGGER IF EXISTS update_messages_updated_at;
            "#;

            manager.get_connection().execute_unprepared(sql).await?;
            Ok(())
        }
    }
}

mod m20241211_007_create_fts {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            // SQLite FTS5 and triggers - no SeaORM builder support
            let sql = r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                    content,
                    tokenize = 'porter'
                );

                CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
                BEGIN
                    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
                END;

                CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
                BEGIN
                    DELETE FROM messages_fts WHERE rowid = OLD.rowid;
                END;

                CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages
                BEGIN
                    DELETE FROM messages_fts WHERE rowid = OLD.rowid;
                    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
                END;
            "#;

            manager.get_connection().execute_unprepared(sql).await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            let sql = r#"
                DROP TRIGGER IF EXISTS messages_au;
                DROP TRIGGER IF EXISTS messages_ad;
                DROP TRIGGER IF EXISTS messages_ai;
                DROP TABLE IF EXISTS messages_fts;
            "#;

            manager.get_connection().execute_unprepared(sql).await?;
            Ok(())
        }
    }
}
