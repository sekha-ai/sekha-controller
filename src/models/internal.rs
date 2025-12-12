use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    pub label: String,
    pub folder: String,
    pub status: String,
    pub importance_score: i32,
    pub word_count: i32,
    pub session_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: NaiveDateTime,
    pub embedding_id: Option<Uuid>,
    pub metadata: Option<String>,
}
