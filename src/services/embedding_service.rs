// src/services/embedding_service.rs
//! Embedding service with provider abstraction

use crate::services::embedding_provider::{EmbeddingProvider, OllamaProvider, ProviderError};
use crate::storage::chroma_client::ChromaClient;
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
    OllamaError(String),
    #[error("Chroma error: {0}")]
    ChromaError(#[from] crate::storage::chroma_client::ChromaError),
    #[error("No embeddings returned")]
    NoEmbeddings,
    #[error("Semaphore error: {0}")]
    SemaphoreError(String),
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
    #[error("Provider error: {0}")]
    ProviderError(String),
}

impl From<AcquireError> for EmbeddingError {
    fn from(err: AcquireError) -> Self {
        EmbeddingError::SemaphoreError(err.to_string())
    }
}

#[derive(Clone)]
pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    chroma: Arc<ChromaClient>,
    semaphore: Arc<Semaphore>,
    max_retries: u32,
}

impl EmbeddingService {
    /// Production constructor with Ollama provider
    pub fn new(ollama_url: String, chroma_url: String) -> Self {
        let provider = Arc::new(OllamaProvider::new(
            ollama_url,
            "nomic-embed-text:latest".to_string(),
        ));

        let chroma = Arc::new(ChromaClient::new(chroma_url));
        let semaphore = Arc::new(Semaphore::new(5));
        let max_retries = 3;

        Self {
            provider,
            chroma,
            semaphore,
            max_retries,
        }
    }

    /// Test constructor with custom provider
    pub fn with_provider(provider: Arc<dyn EmbeddingProvider>, chroma_url: String) -> Self {
        let chroma = Arc::new(ChromaClient::new(chroma_url));
        let semaphore = Arc::new(Semaphore::new(5));
        let max_retries = 3;

        Self {
            provider,
            chroma,
            semaphore,
            max_retries,
        }
    }

    /// Generate embedding for a message and store in Chroma with retry logic
    #[cfg(not(tarpaulin_include))]
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

        // Generate embedding via provider
        let embedding = self.generate_embedding(content).await?;

        // Flatten metadata for Chroma (Chroma only accepts flat key-value pairs with simple types)
        let mut chroma_metadata = json!({
            "conversation_id": conversation_id.to_string(),
            "message_id": message_id.to_string(),
            "content_preview": &content[..content.len().min(100)],
        });

        // Extract and flatten nested metadata fields
        if let Some(meta_obj) = metadata.as_object() {
            for (key, value) in meta_obj {
                // Only include simple types that Chroma accepts
                match value {
                    Value::String(s) => {
                        chroma_metadata[key] = Value::String(s.clone());
                    }
                    Value::Number(n) => {
                        chroma_metadata[key] = Value::Number(n.clone());
                    }
                    Value::Bool(b) => {
                        chroma_metadata[key] = Value::Bool(*b);
                    }
                    // Convert other types to strings
                    _ => {
                        chroma_metadata[key] = Value::String(value.to_string());
                    }
                }
            }
        }

        // Store in Chroma
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

    /// Generate embedding using configured provider
    pub async fn generate_embedding(&self, content: &str) -> Result<Vec<f32>, EmbeddingError> {
        let _permit = self.semaphore.acquire().await?;

        self.provider
            .generate_embedding(content)
            .await
            .map_err(|e| EmbeddingError::ProviderError(e.to_string()))
    }

    /// Generate embedding with retry logic
    pub async fn generate_embedding_with_retry(
        &self,
        content: &str,
        max_retries: u32,
    ) -> Result<Vec<f32>, EmbeddingError> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match self.provider.generate_embedding(content).await {
                Ok(embedding) => return Ok(embedding),
                Err(ProviderError::NoEmbeddings) => {
                    // Don't retry - immediately return NoEmbeddings
                    return Err(EmbeddingError::NoEmbeddings);
                }
                Err(e) => {
                    last_error = Some(EmbeddingError::ProviderError(e.to_string()));

                    // Exponential backoff (except on last attempt)
                    if attempt < max_retries - 1 {
                        sleep(Duration::from_millis(100 * (2_u64.pow(attempt)))).await;
                    }
                }
            }
        }

        Err(EmbeddingError::MaxRetriesExceeded)
    }

    /// Generate embeddings for multiple texts in batches
    pub async fn generate_embeddings_batch(
        &self,
        texts: Vec<String>,
        batch_size: usize,
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut all_embeddings = Vec::new();

        for chunk in texts.chunks(batch_size) {
            // Process batch concurrently
            let mut batch_futures = Vec::new();

            for text in chunk {
                batch_futures.push(self.generate_embedding(text));
            }

            let batch_results = futures::future::join_all(batch_futures).await;

            // Collect results, failing if any individual embedding fails
            for result in batch_results {
                all_embeddings.push(result?);
            }
        }

        Ok(all_embeddings)
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
    use crate::services::embedding_provider::MockProvider;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_generate_embedding_with_retry_success() {
        let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
        let service =
            EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());

        let result = service
            .generate_embedding_with_retry("test content", 3)
            .await;

        assert!(result.is_ok());
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 768);
    }

    #[tokio::test]
    async fn test_generate_embedding_error() {
        let provider = Arc::new(MockProvider::new_error(ProviderError::NoEmbeddings));
        let service =
            EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());

        let result = service.generate_embedding("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_embedding_with_retry_exhaustion() {
        let provider = Arc::new(MockProvider::new_error(ProviderError::Http(
            "Connection failed".to_string(),
        )));
        let service =
            EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());

        let result = service.generate_embedding_with_retry("test", 2).await;
        assert!(result.is_err());
    }
}
