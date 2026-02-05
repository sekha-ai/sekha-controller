// src/services/embedding_service.rs
//! Embedding service with provider abstraction and dimension-aware collections

use crate::services::embedding_provider::{EmbeddingProvider, OllamaProvider, ProviderError};
use crate::services::llm_bridge_client::LlmBridgeClient;
use crate::storage::chroma_client::ChromaClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::{AcquireError, RwLock};
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
    #[error("Bridge error: {0}")]
    BridgeError(String),
}

impl From<AcquireError> for EmbeddingError {
    fn from(err: AcquireError) -> Self {
        EmbeddingError::SemaphoreError(err.to_string())
    }
}

#[derive(Clone)]
pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    bridge_client: Option<Arc<LlmBridgeClient>>,
    chroma: Arc<ChromaClient>,
    semaphore: Arc<Semaphore>,
    max_retries: u32,
    /// Cache of model_id -> dimension for fast lookups
    dimension_cache: Arc<RwLock<HashMap<String, usize>>>,
}

impl EmbeddingService {
    /// Production constructor with Ollama provider (legacy mode)
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
            bridge_client: None,
            chroma,
            semaphore,
            max_retries,
            dimension_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// V2.0 constructor with bridge client for routing
    pub fn with_bridge(
        bridge_client: Arc<LlmBridgeClient>,
        chroma_url: String,
        provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        let chroma = Arc::new(ChromaClient::new(chroma_url));
        let semaphore = Arc::new(Semaphore::new(5));
        let max_retries = 3;

        info!("EmbeddingService initialized with v2.0 bridge routing");

        Self {
            provider,
            bridge_client: Some(bridge_client),
            chroma,
            semaphore,
            max_retries,
            dimension_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Test constructor with custom provider
    pub fn with_provider(provider: Arc<dyn EmbeddingProvider>, chroma_url: String) -> Self {
        let chroma = Arc::new(ChromaClient::new(chroma_url));
        let semaphore = Arc::new(Semaphore::new(5));
        let max_retries = 3;

        Self {
            provider,
            bridge_client: None,
            chroma,
            semaphore,
            max_retries,
            dimension_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get dimension for a specific model (with caching)
    pub async fn get_model_dimension(&self, model_id: &str) -> Result<usize, EmbeddingError> {
        // Check cache first
        {
            let cache = self.dimension_cache.read().await;
            if let Some(&dimension) = cache.get(model_id) {
                debug!("Cache hit for model {} dimension: {}", model_id, dimension);
                return Ok(dimension);
            }
        }

        // Not in cache - query bridge if available
        if let Some(ref bridge) = self.bridge_client {
            match bridge.list_models().await {
                Ok(models) => {
                    // Find the model and extract dimension
                    for model in models {
                        if model == model_id {
                            // For now, generate test embedding to detect dimension
                            // TODO: Bridge should expose dimension in model info
                            let test_embedding = self.generate_embedding("test").await?;
                            let dimension = test_embedding.len();

                            // Cache the result
                            let mut cache = self.dimension_cache.write().await;
                            cache.insert(model_id.to_string(), dimension);

                            info!(
                                "Detected dimension {} for model {} (cached)",
                                dimension, model_id
                            );
                            return Ok(dimension);
                        }
                    }
                }
                Err(e) => {
                    warn!("Could not query bridge for model info: {}", e);
                }
            }
        }

        // Fallback: Generate test embedding to detect dimension
        let test_embedding = self.generate_embedding("test").await?;
        let dimension = test_embedding.len();

        // Cache the result
        let mut cache = self.dimension_cache.write().await;
        cache.insert(model_id.to_string(), dimension);

        info!(
            "Detected dimension {} for model {} via test embedding (cached)",
            dimension, model_id
        );
        Ok(dimension)
    }

    /// Get collection name for a specific dimension
    pub fn get_collection_name(&self, dimension: usize) -> String {
        format!("conversations_{}", dimension)
    }

    /// Ensure collection exists for a specific dimension
    pub async fn ensure_collection_for_dimension(
        &self,
        dimension: usize,
    ) -> Result<String, EmbeddingError> {
        let collection_name = self.get_collection_name(dimension);
        self.chroma
            .ensure_collection(&collection_name, dimension as i32)
            .await?;
        Ok(collection_name)
    }

    /// Generate embedding with automatic collection routing (v2.0)
    pub async fn embed_with_collection_routing(
        &self,
        text: &str,
        model_id: Option<&str>,
    ) -> Result<(Vec<f32>, String), EmbeddingError> {
        // Generate embedding via bridge routing if available
        let (embedding, actual_model) = if let Some(ref bridge) = self.bridge_client {
            match bridge.embed_text_routed(text, model_id, None).await {
                Ok(routed_result) => {
                    let model_used = routed_result
                        .routing
                        .as_ref()
                        .map(|r| r.model_id.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    (routed_result.result, model_used)
                }
                Err(e) => {
                    return Err(EmbeddingError::BridgeError(e.to_string()));
                }
            }
        } else {
            // Fallback to legacy provider
            let embedding = self.generate_embedding(text).await?;
            let model_used = model_id.unwrap_or("legacy").to_string();
            (embedding, model_used)
        };

        // Get dimension and ensure collection exists
        let dimension = embedding.len();
        let collection_name = self.ensure_collection_for_dimension(dimension).await?;

        debug!(
            "Embedding routed to collection {} (dimension={}, model={})",
            collection_name, dimension, actual_model
        );

        Ok((embedding, collection_name))
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

    /// Generate embedding for a message and store in dimension-aware collection
    pub async fn process_message(
        &self,
        message_id: Uuid,
        content: &str,
        conversation_id: Uuid,
        metadata: Value,
    ) -> Result<String, EmbeddingError> {
        let _permit = self.semaphore.acquire().await?;

        debug!("Generating embedding for message: {}", message_id);

        // Use v2.0 routing if bridge is available
        let (embedding, collection_name) = if self.bridge_client.is_some() {
            self.embed_with_collection_routing(content, None).await?
        } else {
            // Legacy mode: use default provider
            let embedding = self.generate_embedding(content).await?;
            let dimension = embedding.len();
            let collection_name = self.ensure_collection_for_dimension(dimension).await?;
            (embedding, collection_name)
        };

        // Flatten metadata for Chroma
        let mut chroma_metadata = json!({
            "conversation_id": conversation_id.to_string(),
            "message_id": message_id.to_string(),
            "content_preview": &content[..content.len().min(100)],
            "dimension": embedding.len(),
        });

        // Extract and flatten nested metadata fields
        if let Some(meta_obj) = metadata.as_object() {
            for (key, value) in meta_obj {
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
                    _ => {
                        chroma_metadata[key] = Value::String(value.to_string());
                    }
                }
            }
        }

        // Store in dimension-specific collection
        let embedding_id = message_id.to_string();
        self.chroma
            .upsert(
                &collection_name,
                &embedding_id,
                embedding.clone(),
                chroma_metadata,
                Some(content.to_string()),
            )
            .await?;

        info!(
            "Successfully stored embedding for message {} in collection {}",
            message_id, collection_name
        );

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
                    return Err(EmbeddingError::NoEmbeddings);
                }
                Err(e) => {
                    last_error = Some(EmbeddingError::ProviderError(e.to_string()));

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
            let mut batch_futures = Vec::new();

            for text in chunk {
                batch_futures.push(self.generate_embedding(text));
            }

            let batch_results = futures::future::join_all(batch_futures).await;

            for result in batch_results {
                all_embeddings.push(result?);
            }
        }

        Ok(all_embeddings)
    }

    /// Semantic search across all dimension collections
    pub async fn search_all_dimensions(
        &self,
        query: &str,
        limit: usize,
        filters: Option<Value>,
    ) -> Result<Vec<crate::storage::chroma_client::ScoredResult>, EmbeddingError> {
        // Generate query embedding
        let (query_embedding, primary_collection) =
            self.embed_with_collection_routing(query, None).await?;

        // Search in primary collection
        let mut all_results = self
            .chroma
            .query(
                &primary_collection,
                query_embedding,
                limit as u32,
                filters.clone(),
            )
            .await?;

        // If using v2.0, also search other known dimensions
        if self.bridge_client.is_some() {
            let cache = self.dimension_cache.read().await;
            let known_dimensions: Vec<usize> = cache.values().copied().collect();

            for dimension in known_dimensions {
                let collection_name = self.get_collection_name(dimension);
                if collection_name != primary_collection {
                    // Try to search this collection (may not exist)
                    if let Ok(results) = self
                        .chroma
                        .query(
                            &collection_name,
                            query_embedding.clone(),
                            limit as u32,
                            filters.clone(),
                        )
                        .await
                    {
                        all_results.extend(results);
                    }
                }
            }
        }

        // Sort by score and limit
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Semantic search in specific collection (backward compatible)
    pub async fn search_messages(
        &self,
        query: &str,
        limit: usize,
        filters: Option<Value>,
    ) -> Result<Vec<crate::storage::chroma_client::ScoredResult>, EmbeddingError> {
        // Use multi-dimension search in v2.0, fall back to legacy collection
        if self.bridge_client.is_some() {
            self.search_all_dimensions(query, limit, filters).await
        } else {
            // Legacy: search default collection
            let query_embedding = self.generate_embedding(query).await?;
            let results = self
                .chroma
                .query("conversations", query_embedding, limit as u32, filters)
                .await?;
            Ok(results)
        }
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
    async fn test_get_collection_name() {
        let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
        let service =
            EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());

        assert_eq!(service.get_collection_name(768), "conversations_768");
        assert_eq!(service.get_collection_name(3072), "conversations_3072");
        assert_eq!(service.get_collection_name(1536), "conversations_1536");
    }

    #[tokio::test]
    async fn test_dimension_detection() {
        let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
        let service =
            EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());

        let dimension = service.get_model_dimension("test-model").await.unwrap();
        assert_eq!(dimension, 768);

        // Verify caching
        let cached_dimension = service.get_model_dimension("test-model").await.unwrap();
        assert_eq!(cached_dimension, 768);
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
