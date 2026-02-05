//! Bridge client with v2.0 routing support.
//!
//! This module provides a client for interacting with the LLM Bridge service,
//! including support for v2.0 multi-provider routing.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::config::Config;

/// Bridge routing request
#[derive(Debug, Serialize)]
pub struct RoutingRequest {
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_vision: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost: Option<f64>,
}

/// Bridge routing response
#[derive(Debug, Deserialize, Clone)]
pub struct RoutingResponse {
    pub provider_id: String,
    pub model_id: String,
    pub estimated_cost: f64,
    pub reason: String,
    pub provider_type: String,
}

/// Model information from bridge
#[derive(Debug, Deserialize, Clone)]
pub struct ModelInfo {
    pub model_id: String,
    pub provider_id: String,
    pub task: String,
    pub context_window: i32,
    pub dimension: Option<i32>,
    pub supports_vision: bool,
    pub supports_audio: bool,
}

/// Chat message
#[derive(Debug, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
}

/// Chat completion response
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: TokenUsage,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub index: i32,
    pub message: ChatMessageResponse,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageResponse {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

/// Embedding request
#[derive(Debug, Serialize)]
pub struct EmbedRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Embedding response
#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: i32,
    pub tokens_used: i32,
}

/// Bridge client for LLM operations
#[derive(Clone)]
pub struct BridgeClient {
    client: Client,
    base_url: String,
    use_v2_routing: bool,
}

impl BridgeClient {
    /// Create a new bridge client
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;

        // Determine if we should use v2 routing
        let use_v2_routing = !config.llm_providers.is_empty();

        if use_v2_routing {
            info!("Bridge client initialized with v2.0 routing support");
        } else {
            warn!("Bridge client using legacy mode (no v2 providers configured)");
        }

        Ok(Self {
            client,
            base_url: config.llm_bridge_url.clone(),
            use_v2_routing,
        })
    }

    /// Route a request to get optimal provider and model
    pub async fn route_request(
        &self,
        task: &str,
        preferred_model: Option<String>,
        max_cost: Option<f64>,
    ) -> anyhow::Result<RoutingResponse> {
        let url = format!("{}/api/v1/route", self.base_url);

        let request = RoutingRequest {
            task: task.to_string(),
            preferred_model,
            require_vision: None,
            max_cost,
        };

        debug!(
            "Routing request: task={}, preferred_model={:?}",
            task, request.preferred_model
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let routing: RoutingResponse = response.json().await?;

        info!(
            "Routed to provider={}, model={}, cost=${:.4}",
            routing.provider_id, routing.model_id, routing.estimated_cost
        );

        Ok(routing)
    }

    /// List all available models
    pub async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let url = format!("{}/api/v1/models", self.base_url);

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let models: Vec<ModelInfo> = response.json().await?;

        debug!("Retrieved {} models from bridge", models.len());

        Ok(models)
    }

    /// Generate chat completion
    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: Option<String>,
        temperature: Option<f32>,
    ) -> anyhow::Result<ChatCompletionResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatCompletionRequest {
            messages,
            model,
            temperature,
            max_tokens: None,
        };

        debug!("Chat completion request: model={:?}", request.model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let completion: ChatCompletionResponse = response.json().await?;

        debug!(
            "Chat completion: tokens={}, model={}",
            completion.usage.total_tokens, completion.model
        );

        Ok(completion)
    }

    /// Generate chat completion with automatic routing
    pub async fn chat_completion_routed(
        &self,
        messages: Vec<ChatMessage>,
        task: &str,
        preferred_model: Option<String>,
        temperature: Option<f32>,
        max_cost: Option<f64>,
    ) -> anyhow::Result<(ChatCompletionResponse, RoutingResponse)> {
        // Route the request if v2 is available
        let routing = if self.use_v2_routing {
            self.route_request(task, preferred_model.clone(), max_cost)
                .await?
        } else {
            // Fallback: create a fake routing response for legacy mode
            RoutingResponse {
                provider_id: "legacy".to_string(),
                model_id: preferred_model
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                estimated_cost: 0.0,
                reason: "Legacy mode".to_string(),
                provider_type: "ollama".to_string(),
            }
        };

        // Make the chat completion request with routed model
        let completion = self
            .chat_completion(messages, Some(routing.model_id.clone()), temperature)
            .await?;

        Ok((completion, routing))
    }

    /// Generate embedding
    pub async fn generate_embedding(
        &self,
        text: String,
        model: Option<String>,
    ) -> anyhow::Result<EmbedResponse> {
        let url = format!("{}/api/v1/embed", self.base_url);

        let request = EmbedRequest { text, model };

        debug!("Embedding request: model={:?}", request.model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let embed: EmbedResponse = response.json().await?;

        debug!(
            "Embedding generated: dimension={}, tokens={}",
            embed.dimension, embed.tokens_used
        );

        Ok(embed)
    }

    /// Generate embedding with automatic routing
    pub async fn generate_embedding_routed(
        &self,
        text: String,
        preferred_model: Option<String>,
        max_cost: Option<f64>,
    ) -> anyhow::Result<(EmbedResponse, RoutingResponse)> {
        // Route the request if v2 is available
        let routing = if self.use_v2_routing {
            self.route_request("embedding", preferred_model.clone(), max_cost)
                .await?
        } else {
            RoutingResponse {
                provider_id: "legacy".to_string(),
                model_id: preferred_model
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                estimated_cost: 0.0,
                reason: "Legacy mode".to_string(),
                provider_type: "ollama".to_string(),
            }
        };

        // Generate embedding with routed model
        let embed = self
            .generate_embedding(text, Some(routing.model_id.clone()))
            .await?;

        Ok((embed, routing))
    }

    /// Check bridge health
    pub async fn health_check(&self) -> anyhow::Result<bool> {
        let url = format!("{}/health", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                error!("Bridge health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_request_serialization() {
        let request = RoutingRequest {
            task: "embedding".to_string(),
            preferred_model: Some("nomic-embed-text".to_string()),
            require_vision: None,
            max_cost: Some(0.01),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("embedding"));
        assert!(json.contains("nomic-embed-text"));
    }
}
