use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // This migration fixes the seaql_migrations table schema from v0.1.x to v0.2.0
        // v0.1.x had applied_at as TEXT, v0.2.0 needs INTEGER
        // Since this is a metadata table, the cleanest approach is drop and recreate
        
        manager
            .drop_table(
                Table::drop()
                    .table(SeaqlMigrations::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SeaqlMigrations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SeaqlMigrations::Version)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SeaqlMigrations::AppliedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No down migration - this fixes a schema issue
        Ok(())
    }
}

#[derive(DeriveIden)]
enum SeaqlMigrations {
    Table,
    Version,
    AppliedAt,
}
