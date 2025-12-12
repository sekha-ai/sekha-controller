use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateConversationRequest {
    pub label: String,
    pub folder: String,
    pub messages: Vec<MessageDto>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct MessageDto {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub label: String,
    pub folder: String,
    pub status: String,
    pub message_count: usize,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLabelRequest {
    pub label: String,
    pub folder: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    pub query: String,
    pub filters: Option<serde_json::Value>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueryResponse {
    pub results: Vec<ConversationResponse>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u32,
}
