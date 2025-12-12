-- hierarchical_summaries table
CREATE TABLE IF NOT EXISTS hierarchical_summaries (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    level TEXT NOT NULL CHECK (level IN ('daily', 'weekly', 'monthly')),
    summary_text TEXT NOT NULL,
    timestamp_range TEXT NOT NULL,
    generated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    model_used TEXT,
    token_count INTEGER,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

CREATE INDEX idx_summaries_conversation ON hierarchical_summaries(conversation_id);
CREATE INDEX idx_summaries_level ON hierarchical_summaries(level);
