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

CREATE INDEX IF NOT EXISTS idx_conversations_label_status ON conversations(label, status);
CREATE INDEX IF NOT EXISTS idx_conversations_folder_updated ON conversations(folder, updated_at);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    tokenize = 'porter'
);

CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages
BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.id, NEW.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages
BEGIN
    DELETE FROM messages_fts WHERE rowid = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages
BEGIN
    DELETE FROM messages_fts WHERE rowid = OLD.id;
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.id, NEW.content);
END;

PRAGMA journal_mode=WAL;
