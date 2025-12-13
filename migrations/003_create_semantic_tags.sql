-- semantic_tags table
CREATE TABLE IF NOT EXISTS semantic_tags (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    confidence REAL NOT NULL,
    extracted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tags_conversation ON semantic_tags(conversation_id);
CREATE INDEX IF NOT EXISTS idx_tags_tag ON semantic_tags(tag);
