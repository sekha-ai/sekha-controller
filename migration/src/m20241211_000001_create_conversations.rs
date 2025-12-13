use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Use raw SQL for FTS5 (SeaORM doesn't support it natively)
        let sql = r#"
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    folder TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL DEFAULT 'active',
    importance_score INTEGER NOT NULL DEFAULT 5,
    word_count INTEGER NOT NULL DEFAULT 0,
    session_count INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_conversations_label_status ON conversations(label, status);
CREATE INDEX idx_conversations_folder_updated ON conversations(folder, updated_at);

-- FTS5 virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(content, tokenize = 'porter');

CREATE TRIGGER messages_ai AFTER INSERT ON messages
BEGIN INSERT INTO messages_fts(rowid, content) VALUES (NEW.id, NEW.content); END;

CREATE TRIGGER messages_ad AFTER DELETE ON messages
BEGIN DELETE FROM messages_fts WHERE rowid = OLD.id; END;

CREATE TRIGGER messages_au AFTER UPDATE ON messages
BEGIN DELETE FROM messages_fts WHERE rowid = OLD.id; INSERT INTO messages_fts(rowid, content) VALUES (NEW.id, NEW.content); END;

PRAGMA journal_mode=WAL;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP TABLE IF EXISTS conversations; DROP TABLE IF EXISTS messages_fts;").await?;
        Ok(())
    }
}
