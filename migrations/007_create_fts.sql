-- Full-text search virtual table for messages
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    tokenize = 'porter'
);

-- Sync triggers to keep FTS index updated
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

-- Enable WAL mode for better concurrency
PRAGMA journal_mode=WAL;
