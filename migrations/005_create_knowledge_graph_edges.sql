-- knowledge_graph_edges table (optional v2)
CREATE TABLE IF NOT EXISTS knowledge_graph_edges (
    subject_id TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    extracted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (subject_id, predicate, object_id, conversation_id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

CREATE INDEX idx_edges_subject ON knowledge_graph_edges(subject_id);
CREATE INDEX idx_edges_object ON knowledge_graph_edges(object_id);
