-- Add updated_at triggers
CREATE TRIGGER IF NOT EXISTS update_conversations_updated_at
AFTER UPDATE ON conversations
BEGIN
    UPDATE conversations SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS update_messages_updated_at
AFTER UPDATE ON messages
BEGIN
    UPDATE messages SET timestamp = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
END;
