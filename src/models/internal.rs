use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    #[schema(value_type = String, format = DateTime)]
    pub timestamp: NaiveDateTime,
    pub embedding_id: Option<String>, // CHANGED: Uuid → String (Chroma ID)
    pub metadata: Option<serde_json::Value>,
}

// NEW: For creating conversations with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConversation {
    pub id: Option<Uuid>,
    pub label: String,
    pub folder: String,
    pub status: String,
    pub importance_score: Option<i32>,
    pub word_count: i32,
    pub session_count: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub messages: Vec<NewMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMessage {
    pub role: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub timestamp: NaiveDateTime,
}
