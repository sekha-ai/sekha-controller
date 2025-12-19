use crate::storage::chroma_client::ChromaClient;
use ollama_rs::generation::embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest};
use ollama_rs::{error::OllamaError, Ollama};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::AcquireError;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Ollama error: {0}")]
    OllamaError(#[from] OllamaError),
    #[error("Chroma error: {0}")]
    ChromaError(#[from] crate::storage::chroma_client::ChromaError),
    #[error("No embeddings returned")]
    NoEmbeddings,
    #[error("Semaphore error: {0}")]
    SemaphoreError(String),
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}

impl From<AcquireError> for EmbeddingError {
    fn from(err: AcquireError) -> Self {
        EmbeddingError::SemaphoreError(err.to_string())
    }
}

#[derive(Clone)]
pub struct EmbeddingService {
    ollama: Ollama,
    chroma: Arc<ChromaClient>,
    semaphore: Arc<Semaphore>,
    max_retries: u32,
}

impl EmbeddingService {
    pub fn new(ollama_url: String, chroma_url: String) -> Self {
        let ollama = Ollama::new(ollama_url, 11434);
        let chroma = Arc::new(ChromaClient::new(chroma_url));

        // Rate limiting: max 4 concurrent embedding requests (per Module 4 spec)
        let semaphore = Arc::new(Semaphore::new(4));

        // Module 4 spec: retry with exponential backoff
        let max_retries = 3;

        Self {
            ollama,
            chroma,
            semaphore,
            max_retries,
        }
    }

    /// Generate embedding for a message and store in Chroma with retry logic
    pub async fn process_message_with_retry(
        &self,
        message_id: Uuid,
        content: &str,
        conversation_id: Uuid,
        metadata: Value,
    ) -> Result<String, EmbeddingError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                warn!(
                    "Embedding attempt {} failed, retrying in {:?}: {}",
                    attempt,
                    delay,
                    last_error.as_ref().unwrap()
                );
                sleep(delay).await;
            }

            match self
                .process_message(message_id, content, conversation_id, metadata.clone())
                .await
            {
                Ok(result) => {
                    if attempt > 0 {
                        info!(
                            "Retry succeeded for message {} on attempt {}",
                            message_id,
                            attempt + 1
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    debug!(
                        "Embedding attempt {} failed for message {}: {}",
                        attempt + 1,
                        message_id,
                        last_error.as_ref().unwrap()
                    );
                }
            }
        }

        error!(
            "Max retries exceeded for message {}: {}",
            message_id,
            last_error.as_ref().unwrap()
        );
        Err(EmbeddingError::MaxRetriesExceeded)
    }

    /// Generate embedding for a message and store in Chroma (no retry)
    pub async fn process_message(
        &self,
        message_id: Uuid,
        content: &str,
        conversation_id: Uuid,
        metadata: Value,
    ) -> Result<String, EmbeddingError> {
        let _permit = self.semaphore.acquire().await?;

        debug!("Generating embedding for message: {}", message_id);

        // Generate embedding via Ollama
        let embedding = self.generate_embedding(content).await?;

        // Prepare Chroma metadata
        let chroma_metadata = json!({
            "conversation_id": conversation_id.to_string(),
            "message_id": message_id.to_string(),
            "content_preview": &content[..content.len().min(100)],
            "metadata": metadata,
        });

        // Store in Chroma - FIXED: Collection name to "conversations" per spec
        let embedding_id = message_id.to_string();
        self.chroma
            .ensure_collection("conversations", embedding.len() as i32)
            .await?;

        self.chroma
            .upsert(
                "conversations",
                &embedding_id,
                embedding.clone(),
                chroma_metadata,
                Some(content.to_string()),
            )
            .await?;

        info!("Successfully stored embedding for message: {}", message_id);

        Ok(embedding_id)
    }

    /// Generate embedding using Ollama
    #[allow(clippy::map_clone)] // False positive - type conversion is necessary
    async fn generate_embedding(&self, content: &str) -> Result<Vec<f32>, EmbeddingError> {
        let input = EmbeddingsInput::Single(content.to_string());
        let request = GenerateEmbeddingsRequest::new("nomic-embed-text:latest".to_string(), input);

        let response = self.ollama.generate_embeddings(request).await?;

        if response.embeddings.is_empty() {
            return Err(EmbeddingError::NoEmbeddings);
        }

        // Convert f64 to f32 (necessary for Chroma compatibility)
        let embedding: Vec<f32> = match response.embeddings.len() {
            0 => return Err(EmbeddingError::NoEmbeddings),
            1 => response.embeddings[0].iter().map(|&v| v as f32).collect(),
            _ => response
                .embeddings
                .into_iter()
                .next()
                .unwrap()
                .into_iter()
                .map(|v| v as f32)
                .collect(),
        };

        Ok(embedding)
    }

    /// Semantic search across messages
    pub async fn search_messages(
        &self,
        query: &str,
        limit: usize,
        filters: Option<Value>,
    ) -> Result<Vec<crate::storage::chroma_client::ScoredResult>, EmbeddingError> {
        // Generate query embedding
        let query_embedding = self.generate_embedding(query).await?;

        // Search in Chroma
        let results = self
            .chroma
            .query("conversations", query_embedding, limit as u32, filters)
            .await?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running Ollama
    async fn test_embedding_generation() {
        let service = EmbeddingService::new(
            "http://localhost".to_string(),
            "http://localhost:8000".to_string(),
        );

        let embedding = service.generate_embedding("test content").await;
        assert!(embedding.is_ok());
        assert_eq!(embedding.unwrap().len(), 768); // nomic-embed-text dimension
    }

    #[tokio::test]
    async fn test_retry_logic_eventually_succeeds() {
        // This test would require mocking Ollama/Chroma to simulate failures
        // For now, it's a placeholder for the retry logic
        // assert!(true); // Placeholder
    }
}

/// Retry logic for Ollama failures
use tokio_retry::{strategy::ExponentialBackoff, Retry};

async fn generate_embedding_with_retry(&self, content: &str) -> Result<Vec<f32>, EmbeddingError> {
    let retry_strategy = ExponentialBackoff::from_millis(100).map(|d| d * 2).take(3); // 100ms, 200ms, 400ms

    let result = Retry::spawn(retry_strategy.clone(), || async {
        self.generate_embedding(content).await
    })
    .await?;

    Ok(result)
}
