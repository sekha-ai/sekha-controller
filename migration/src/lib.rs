pub use sea_orm_migration::prelude::*;

mod m20241211_000001_create_conversations;
mod m20241211_000002_create_messages;
mod m20241211_000003_create_semantic_tags;
mod m20241211_000004_create_hierarchical_summaries;
mod m20241211_000005_create_knowledge_graph_edges;
mod m20241211_000006_add_updated_at_triggers;
mod m20241211_000007_create_fts;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241211_000001_create_conversations::Migration),
            Box::new(m20241211_000002_create_messages::Migration),
            Box::new(m20241211_000003_create_semantic_tags::Migration),
            Box::new(m20241211_000004_create_hierarchical_summaries::Migration),
            Box::new(m20241211_000005_create_knowledge_graph_edges::Migration),
            Box::new(m20241211_000006_add_updated_at_triggers::Migration),
            Box::new(m20241211_000007_create_fts::Migration),
        ]
    }
}
