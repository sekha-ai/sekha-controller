// use async_trait::async_trait;
use serde::{Deserialize, Serialize};
// use serde_json::Value;
use std::sync::Arc;
// use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LlmBridgeError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Clone)]
pub struct LlmBridgeClient {
    client: reqwest::Client,
    base_url: String,
}

impl LlmBridgeClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn embed_text(
        &self,
        text: &str,
        model: Option<&str>,
    ) -> Result<Vec<f32>, LlmBridgeError> {
        let request = EmbedRequest {
            text: text.to_string(),
            model: model.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(format!("{}/embed", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LlmBridgeError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let embed_response: EmbedResponse = response.json().await?;
        Ok(embed_response.embedding)
    }

    pub async fn summarize(
        &self,
        messages: Vec<String>,
        level: &str,
        model: Option<&str>,
        max_words: Option<u32>,
    ) -> Result<String, LlmBridgeError> {
        let request = SummarizeRequest {
            messages,
            level: level.to_string(),
            model: model.map(|s| s.to_string()),
            max_words: max_words.unwrap_or(200),
        };

        let response = self
            .client
            .post(format!("{}/summarize", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LlmBridgeError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let summary_response: SummarizeResponse = response.json().await?;
        Ok(summary_response.summary)
    }

    pub async fn score_importance(
        &self,
        message: &str,
        context: Option<&str>,
        model: Option<&str>,
    ) -> Result<f32, LlmBridgeError> {
        let request = ScoreImportanceRequest {
            message: message.to_string(),
            context: context.map(|s| s.to_string()),
            model: model.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(format!("{}/score_importance", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LlmBridgeError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let score_response: ScoreImportanceResponse = response.json().await?;
        Ok(score_response.score)
    }

    pub async fn list_models(&self) -> Result<Vec<String>, LlmBridgeError> {
        let response = self
            .client
            .get(format!(
                "{}/api/tags",
                self.base_url.replace("5001", "11434")
            ))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(vec![]); // Return empty if Ollama not available
        }

        #[derive(Deserialize)]
        struct TagsResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let tags: TagsResponse = response.json().await?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    pub async fn health_check(&self) -> Result<bool, LlmBridgeError> {
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

// Request/Response Models
#[derive(Serialize)]
struct EmbedRequest {
    text: String,
    model: Option<String>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
    model: String,
    tokens_used: u32,
}

#[derive(Serialize)]
struct SummarizeRequest {
    messages: Vec<String>,
    level: String,
    model: Option<String>,
    max_words: u32,
}

#[derive(Deserialize)]
struct SummarizeResponse {
    summary: String,
    level: String,
    model: String,
    tokens_used: u32,
}

#[derive(Serialize)]
struct ScoreImportanceRequest {
    message: String,
    context: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct ScoreImportanceResponse {
    score: f32,
    reasoning: Option<String>,
    model: String,
}
