CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY,
    label TEXT NOT NULL,
    folder TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL DEFAULT 'active',
    importance_score INTEGER NOT NULL DEFAULT 5,
    word_count INTEGER NOT NULL DEFAULT 0,
    session_count INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_conversations_label_status ON conversations(label, status);
CREATE INDEX IF NOT EXISTS idx_conversations_folder_updated ON conversations(folder, updated_at);