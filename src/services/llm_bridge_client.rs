//! LLM Bridge client for controller services.
//!
//! This client provides high-level methods for common LLM operations
//! with automatic routing via Bridge v2.0.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::llm::bridge_client::{BridgeClient, ChatMessage, RoutingResponse};

#[derive(Debug, thiserror::Error)]
pub enum LlmBridgeError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Bridge error: {0}")]
    BridgeError(#[from] anyhow::Error),
}

/// Result of an LLM operation with routing information
#[derive(Debug, Clone)]
pub struct RoutedResult<T> {
    pub result: T,
    pub routing: Option<RoutingInfo>,
}

/// Routing information from bridge
#[derive(Debug, Clone)]
pub struct RoutingInfo {
    pub provider_id: String,
    pub model_id: String,
    pub estimated_cost: f64,
    pub actual_cost: Option<f64>,
}

impl From<RoutingResponse> for RoutingInfo {
    fn from(response: RoutingResponse) -> Self {
        Self {
            provider_id: response.provider_id,
            model_id: response.model_id,
            estimated_cost: response.estimated_cost,
            actual_cost: None,
        }
    }
}

#[derive(Clone)]
pub struct LlmBridgeClient {
    bridge: BridgeClient,
    base_url: String,
}

impl LlmBridgeClient {
    /// Create a new LlmBridgeClient
    pub fn new(config: &crate::config::Config) -> anyhow::Result<Self> {
        let bridge = BridgeClient::new(config)?;
        Ok(Self {
            bridge,
            base_url: config.llm_bridge_url.clone(),
        })
    }

    /// Embed text using automatic provider routing
    pub async fn embed_text(
        &self,
        text: &str,
        model: Option<&str>,
    ) -> Result<Vec<f32>, LlmBridgeError> {
        let result = self.embed_text_routed(text, model, None).await?;
        Ok(result.result)
    }

    /// Embed text with routing information
    pub async fn embed_text_routed(
        &self,
        text: &str,
        preferred_model: Option<&str>,
        max_cost: Option<f64>,
    ) -> Result<RoutedResult<Vec<f32>>, LlmBridgeError> {
        let (embed_response, routing) = self
            .bridge
            .generate_embedding_routed(
                text.to_string(),
                preferred_model.map(|s| s.to_string()),
                max_cost,
            )
            .await?;

        info!(
            "Routed to provider={}, model={}, cost=${:.4}",
            routing.provider_id, routing.model_id, routing.estimated_cost
        );

        Ok(RoutedResult {
            result: embed_response.embedding,
            routing: Some(routing.into()),
        })
    }

    /// Summarize messages
    pub async fn summarize(
        &self,
        messages: Vec<String>,
        level: &str,
        model: Option<&str>,
        max_words: Option<u32>,
    ) -> Result<String, LlmBridgeError> {
        let result = self
            .summarize_routed(messages, level, model, max_words, None)
            .await?;
        Ok(result.result)
    }

    /// Summarize with routing information
    pub async fn summarize_routed(
        &self,
        messages: Vec<String>,
        level: &str,
        preferred_model: Option<&str>,
        max_words: Option<u32>,
        max_cost: Option<f64>,
    ) -> Result<RoutedResult<String>, LlmBridgeError> {
        // Build prompt
        let messages_str = messages.join("\n");
        let max_words_val = max_words.unwrap_or(200);
        let prompt = format!(
            "Summarize these messages in approximately {} words. Level: {}\n\n{}",
            max_words_val, level, messages_str
        );

        // Use chat completion with routing
        let chat_messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let (completion, routing) = self
            .bridge
            .chat_completion_routed(
                chat_messages,
                "chat_smart", // Changed from chat_small - use smart model for summaries
                preferred_model.map(|s| s.to_string()),
                Some(0.7),
                max_cost,
            )
            .await?;

        let summary = completion.choices[0].message.content.clone();

        info!(
            "Summary generated via {}/{} - {} tokens",
            routing.provider_id, routing.model_id, completion.usage.total_tokens
        );

        Ok(RoutedResult {
            result: summary,
            routing: Some(routing.into()),
        })
    }

    /// Score message importance
    pub async fn score_importance(
        &self,
        message: &str,
        context: Option<&str>,
        model: Option<&str>,
    ) -> Result<f32, LlmBridgeError> {
        let result = self
            .score_importance_routed(message, context, model, None)
            .await?;
        Ok(result.result)
    }

    /// Score importance with routing
    pub async fn score_importance_routed(
        &self,
        message: &str,
        context: Option<&str>,
        preferred_model: Option<&str>,
        max_cost: Option<f64>,
    ) -> Result<RoutedResult<f32>, LlmBridgeError> {
        // Build prompt for scoring
        let prompt = if let Some(ctx) = context {
            format!(
                "Rate the importance of this message on a scale of 0.0 to 1.0.\n\nContext: {}\n\nMessage: {}\n\nRespond with only a number between 0.0 and 1.0.",
                ctx, message
            )
        } else {
            format!(
                "Rate the importance of this message on a scale of 0.0 to 1.0.\n\nMessage: {}\n\nRespond with only a number between 0.0 and 1.0.",
                message
            )
        };

        let chat_messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let (completion, routing) = self
            .bridge
            .chat_completion_routed(
                chat_messages,
                "chat_smart", // Changed from chat_small
                preferred_model.map(|s| s.to_string()),
                Some(0.3), // Low temperature for consistent scoring
                max_cost,
            )
            .await?;

        // Parse score from response
        let response_text = completion.choices[0].message.content.trim();
        let score: f32 = response_text
            .parse()
            .unwrap_or_else(|_| {
                // Try to extract first number from response
                response_text
                    .split_whitespace()
                    .find_map(|word| word.parse::<f32>().ok())
                    .unwrap_or(0.5) // Default to 0.5 if parsing fails
            })
            .clamp(0.0, 1.0);

        debug!(
            "Importance scored via {}/{}: {}",
            routing.provider_id, routing.model_id, score
        );

        Ok(RoutedResult {
            result: score,
            routing: Some(routing.into()),
        })
    }

    /// List available models from bridge
    pub async fn list_models(&self) -> Result<Vec<String>, LlmBridgeError> {
        let models = self.bridge.list_models().await?;
        Ok(models.into_iter().map(|m| m.model_id).collect())
    }

    /// Check bridge health
    pub async fn health_check(&self) -> Result<bool, LlmBridgeError> {
        Ok(self.bridge.health_check().await?)
    }

    /// Get routing information for a task
    pub async fn get_routing(
        &self,
        task: &str,
        preferred_model: Option<String>,
        max_cost: Option<f64>,
    ) -> Result<RoutingInfo, LlmBridgeError> {
        let routing = self
            .bridge
            .route_request(task, preferred_model, max_cost)
            .await?;

        Ok(routing.into())
    }
}

// Legacy request/response types for backward compatibility
// These are kept for any code still using the old structures

#[derive(Serialize)]
pub struct EmbedRequest {
    pub text: String,
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub tokens_used: u32,
}

#[derive(Serialize)]
pub struct SummarizeRequest {
    pub messages: Vec<String>,
    pub level: String,
    pub model: Option<String>,
    pub max_words: u32,
}

#[derive(Deserialize)]
pub struct SummarizeResponse {
    pub summary: String,
    pub level: String,
    pub model: String,
    pub tokens_used: u32,
}

#[derive(Serialize)]
pub struct ScoreImportanceRequest {
    pub message: String,
    pub context: Option<String>,
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct ScoreImportanceResponse {
    pub score: f32,
    pub reasoning: Option<String>,
    pub model: String,
}
