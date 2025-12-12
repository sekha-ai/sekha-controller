pub use sea_orm_migration::prelude::*;

mod m20241211_000001_create_conversations;
mod m20241211_000002_create_messages;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241211_000001_create_conversations::Migration),
            Box::new(m20241211_000002_create_messages::Migration),
        ]
    }
}
