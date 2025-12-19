use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ==================== REQUEST DTOs ====================

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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLabelRequest {
    pub label: String,
    pub folder: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFolderRequest {
    pub folder: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    pub query: String,
    pub filters: Option<serde_json::Value>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RebuildEmbeddingsRequest {}

// ==================== RESPONSE DTOs ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub label: String,
    pub folder: String,
    pub status: String,
    pub message_count: usize,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueryResponse {
    pub results: Vec<SearchResultDto>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchResultDto {
    pub conversation_id: Uuid,
    pub message_id: Uuid,
    pub score: f32,
    pub content: String,
    pub metadata: serde_json::Value,
    pub label: String,
    pub folder: String,
    pub timestamp: String,
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

#[derive(Debug, Serialize, ToSchema)]
pub struct RebuildEmbeddingsResponse {
    pub success: bool,
    pub message: String,
    pub estimated_completion_seconds: u32,
}

// ==================== MCP DTOs ====================

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemoryStoreRequest {
    pub label: String,
    pub folder: String,
    pub messages: Vec<MessageDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryStoreResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemoryQueryRequest {
    pub query: String,
    pub filters: Option<serde_json::Value>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryQueryResponse {
    pub success: bool,
    pub data: QueryResponse,
    pub error: Option<String>,
}