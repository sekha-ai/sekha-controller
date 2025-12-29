use chrono::NaiveDateTime;
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct FtsSearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FtsSearchResponse {
    pub results: Vec<crate::models::internal::Message>,
    pub total: usize,
}

// ==================== RESPONSE DTOs ====================

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub label: String,
    pub folder: String,
    pub status: String,
    pub message_count: usize,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: NaiveDateTime, // CHANGED: String → NaiveDateTime
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
    #[schema(value_type = String, format = DateTime)]
    pub timestamp: NaiveDateTime, // CHANGED: String → NaiveDateTime
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

// Module 5 DTOs
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ContextAssembleRequest {
    pub query: String,
    pub preferred_labels: Vec<String>,
    pub context_budget: usize,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SummarizeRequest {
    pub conversation_id: Uuid,
    pub level: String, // "daily", "weekly", "monthly"
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SummaryResponse {
    pub conversation_id: Uuid,
    pub level: String,
    pub summary: String,
    #[schema(value_type = String, format = DateTime)]
    pub generated_at: NaiveDateTime, // CHANGED: String → NaiveDateTime
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PruneRequest {
    pub threshold_days: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PruneResponse {
    pub suggestions: Vec<PruningSuggestionDto>,
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PruningSuggestionDto {
    pub conversation_id: Uuid,
    pub conversation_label: String,
    #[schema(value_type = String, format = DateTime)]
    pub last_accessed: NaiveDateTime, // CHANGED: String → NaiveDateTime
    pub message_count: u64,
    pub token_estimate: u32,
    pub importance_score: f32,
    pub preview: String,
    pub recommendation: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecutePruneRequest {
    pub conversation_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct LabelSuggestRequest {
    pub conversation_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LabelSuggestResponse {
    pub conversation_id: Uuid,
    pub suggestions: Vec<LabelSuggestionDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LabelSuggestionDto {
    pub label: String,
    pub confidence: f32,
    pub is_existing: bool,
    pub reason: String,
}