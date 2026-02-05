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

/// Content part for multi-modal messages (text + images)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ImageUrl {
    /// URL to the image (http/https) or base64 data URI
    pub url: String,
    /// Optional detail level for vision models (low, high, auto)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Message in a conversation
/// Supports both simple text and multi-modal content (text + images)
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct MessageDto {
    pub role: String,
    /// Content can be either a simple string or an array of content parts
    /// For text-only: content = "Hello"
    /// For vision: content = [{"type": "text", "text": "What's in this image?"}, {"type": "image_url", "image_url": {"url": "https://..."}}]
    pub content: MessageContent,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content (backward compatible)
    Text(String),
    /// Multi-modal content with text and images
    Parts(Vec<ContentPart>),
}

impl MessageDto {
    /// Extract text content from the message
    pub fn get_text(&self) -> String {
        match &self.content {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// Check if message contains images
    pub fn has_images(&self) -> bool {
        match &self.content {
            MessageContent::Text(_) => false,
            MessageContent::Parts(parts) => parts
                .iter()
                .any(|part| matches!(part, ContentPart::ImageUrl { .. })),
        }
    }

    /// Extract image URLs from the message
    pub fn get_image_urls(&self) -> Vec<String> {
        match &self.content {
            MessageContent::Text(_) => vec![],
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::ImageUrl { image_url } => Some(image_url.url.clone()),
                    _ => None,
                })
                .collect(),
        }
    }
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
    pub created_at: NaiveDateTime,
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
    pub timestamp: NaiveDateTime,
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
    #[serde(default)]
    pub excluded_folders: Vec<String>,
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
    pub generated_at: NaiveDateTime,
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
    pub last_accessed: NaiveDateTime,
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
