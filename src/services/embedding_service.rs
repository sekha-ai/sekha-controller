// src/services/embedding_service.rs
//! Embedding service using LLM Bridge with v2.0 routing and dimension-aware collections

use crate::llm::bridge_client::BridgeClient;
use crate::storage::chroma_client::ChromaClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{AcquireError, Semaphore, TryAcquireError};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Bridge error: {0}")]
    BridgeError(String),
    #[error("Chroma error: {0}")]
    ChromaError(#[from] crate::storage::chroma_client::ChromaError),
    #[error("No embeddings returned")]
    NoEmbeddings,
    #[error("Semaphore error: {0}")]
    SemaphoreError(String),
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("No embedding models available")]
    NoModelsAvailable,
}

impl From<AcquireError> for EmbeddingError {
    fn from(err: AcquireError) -> Self {
        EmbeddingError::SemaphoreError(err.to_string())
    }
}

impl From<TryAcquireError> for EmbeddingError {
    fn from(err: TryAcquireError) -> Self {
        EmbeddingError::SemaphoreError(err.to_string())
    }
}

impl From<anyhow::Error> for EmbeddingError {
    fn from(err: anyhow::Error) -> Self {
        EmbeddingError::BridgeError(err.to_string())
    }
}

#[derive(Clone)]
pub struct EmbeddingService {
    bridge: Arc<BridgeClient>,
    chroma: Arc<ChromaClient>,
    semaphore: Arc<Semaphore>,
    max_retries: u32,
    // Cache for model dimensions to avoid repeated API calls
    dimension_cache: Arc<tokio::sync::RwLock<HashMap<String, i32>>>,
}

impl EmbeddingService {
    /// Create new embedding service with bridge client
    pub fn new(bridge: BridgeClient, chroma_url: String) -> Self {
        let chroma = Arc::new(ChromaClient::new(chroma_url));
        let semaphore = Arc::new(Semaphore::new(5));
        let max_retries = 3;
        let dimension_cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));

        Self {
            bridge: Arc::new(bridge),
            chroma,
            semaphore,
            max_retries,
            dimension_cache,
        }
    }

    /// Get the dimension of a model from bridge
    pub async fn get_model_dimension(&self, model: Option<&str>) -> Result<i32, EmbeddingError> {
        // Check cache first
        if let Some(model_id) = model {
            let cache = self.dimension_cache.read().await;
            if let Some(&dim) = cache.get(model_id) {
                debug!("Cache hit for model dimension: {} -> {}", model_id, dim);
                return Ok(dim);
            }
        }

        // Query bridge for model list
        let models = self.bridge.list_models().await?;

        // Filter for embedding models
        let embedding_models: Vec<_> = models
            .iter()
            .filter(|m| m.task == "embedding" && m.dimension.is_some())
            .collect();

        if embedding_models.is_empty() {
            return Err(EmbeddingError::NoModelsAvailable);
        }

        // If model specified, find it
        if let Some(model_id) = model {
            if let Some(model_info) = embedding_models.iter().find(|m| m.model_id == model_id) {
                let dim = model_info.dimension.unwrap();
                // Update cache
                let mut cache = self.dimension_cache.write().await;
                cache.insert(model_id.to_string(), dim);
                return Ok(dim);
            } else {
                return Err(EmbeddingError::ModelNotFound(model_id.to_string()));
            }
        }

        // No model specified, use first available
        let default_model = embedding_models[0];
        let dim = default_model.dimension.unwrap();
        let model_id = &default_model.model_id;

        // Update cache
        let mut cache = self.dimension_cache.write().await;
        cache.insert(model_id.clone(), dim);

        debug!("Using default embedding model: {} (dim={})", model_id, dim);
        Ok(dim)
    }

    /// Generate embedding with automatic collection routing based on dimension
    pub async fn embed_with_collection_routing(
        &self,
        content: &str,
        preferred_model: Option<String>,
    ) -> Result<(Vec<f32>, i32, String), EmbeddingError> {
        let _permit = self.semaphore.acquire().await?;

        // Generate embedding through bridge with routing
        let (embed_response, routing) = self
            .bridge
            .generate_embedding_routed(content.to_string(), preferred_model, None)
            .await?;

        // Validate embedding is not empty
        if embed_response.embedding.is_empty() {
            return Err(EmbeddingError::NoEmbeddings);
        }

        let dimension = embed_response.dimension;
        let model_used = routing.model_id;

        // Update dimension cache
        {
            let mut cache = self.dimension_cache.write().await;
            cache.insert(model_used.clone(), dimension);
        }

        debug!(
            "Generated embedding: model={}, dimension={}, provider={}",
            model_used, dimension, routing.provider_id
        );

        Ok((embed_response.embedding, dimension, model_used))
    }

    /// Search across all dimension-specific collections and merge results
    pub async fn search_all_dimensions(
        &self,
        query: &str,
        limit: usize,
        filters: Option<Value>,
        preferred_model: Option<String>,
    ) -> Result<Vec<crate::storage::chroma_client::ScoredResult>, EmbeddingError> {
        // Generate query embedding
        let (query_embedding, query_dim, _) = self
            .embed_with_collection_routing(query, preferred_model)
            .await?;

        // Search in primary collection for this dimension
        let primary_collection = format!("conversations_{}", query_dim);
        let mut results = self
            .chroma
            .query(
                &primary_collection,
                query_embedding.clone(),
                limit as u32,
                filters.clone(),
            )
            .await
            .unwrap_or_default();

        // If we have enough results, return them
        if results.len() >= limit {
            return Ok(results);
        }

        // Otherwise, try to find other dimensions and search them too
        let remaining = limit - results.len();

        // Get all available embedding models
        if let Ok(models) = self.bridge.list_models().await {
            let other_dimensions: Vec<i32> = models
                .iter()
                .filter(|m| m.task == "embedding" && m.dimension.is_some())
                .filter_map(|m| m.dimension)
                .filter(|&d| d != query_dim)
                .collect();

            // Search other dimension collections
            for other_dim in other_dimensions {
                if results.len() >= limit {
                    break;
                }

                let other_collection = format!("conversations_{}", other_dim);

                // Generate embedding in the other dimension
                // Try to find a model with that dimension
                if let Some(other_model) = models
                    .iter()
                    .find(|m| m.dimension == Some(other_dim) && m.task == "embedding")
                {
                    match self
                        .embed_with_collection_routing(query, Some(other_model.model_id.clone()))
                        .await
                    {
                        Ok((other_embedding, _, _)) => {
                            if let Ok(other_results) = self
                                .chroma
                                .query(
                                    &other_collection,
                                    other_embedding,
                                    remaining as u32,
                                    filters.clone(),
                                )
                                .await
                            {
                                results.extend(other_results);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to search dimension {}: {}", other_dim, e);
                        }
                    }
                }
            }
        }

        // Sort by score and limit
        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    /// Generate embedding for a message and store in dimension-specific Chroma collection
    pub async fn process_message(
        &self,
        message_id: Uuid,
        content: &str,
        conversation_id: Uuid,
        metadata: Value,
        preferred_model: Option<String>,
    ) -> Result<String, EmbeddingError> {
        let _permit = self.semaphore.acquire().await?;

        debug!("Generating embedding for message: {}", message_id);

        // Generate embedding with routing
        let (embedding, dimension, model_used) = self
            .embed_with_collection_routing(content, preferred_model)
            .await?;

        // Use dimension-specific collection
        let collection_name = format!("conversations_{}", dimension);

        // Flatten metadata for Chroma
        let mut chroma_metadata = json!({
            "conversation_id": conversation_id.to_string(),
            "message_id": message_id.to_string(),
            "content_preview": &content[..content.len().min(100)],
            "model": model_used,
            "dimension": dimension,
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

        // Ensure collection exists with correct dimension
        self.chroma
            .ensure_collection(&collection_name, dimension)
            .await?;

        // Store in Chroma - this WILL fail if Chroma is unavailable
        let embedding_id = message_id.to_string();
        self.chroma
            .upsert(
                &collection_name,
                &embedding_id,
                embedding,
                chroma_metadata,
                Some(content.to_string()),
            )
            .await?;

        info!(
            "Stored embedding for message {} in collection {} (model={}, dim={})",
            message_id, collection_name, model_used, dimension
        );

        Ok(embedding_id)
    }

    /// Generate embedding for a message and store in Chroma with retry logic
    pub async fn process_message_with_retry(
        &self,
        message_id: Uuid,
        content: &str,
        conversation_id: Uuid,
        metadata: Value,
        preferred_model: Option<String>,
    ) -> Result<String, EmbeddingError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                warn!(
                    "Embedding attempt {} failed, retrying in {:?}",
                    attempt, delay
                );
                sleep(delay).await;
            }

            match self
                .process_message(
                    message_id,
                    content,
                    conversation_id,
                    metadata.clone(),
                    preferred_model.clone(),
                )
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
                        "Embedding attempt {} failed for message {}",
                        attempt + 1,
                        message_id
                    );
                }
            }
        }

        error!("Max retries exceeded for message {}", message_id);
        Err(EmbeddingError::MaxRetriesExceeded)
    }

    /// Generate embedding using bridge (simplified interface)
    pub async fn generate_embedding(
        &self,
        content: &str,
        preferred_model: Option<String>,
    ) -> Result<Vec<f32>, EmbeddingError> {
        let (embedding, _, _) = self
            .embed_with_collection_routing(content, preferred_model)
            .await?;
        Ok(embedding)
    }

    /// Generate embedding with retry logic
    pub async fn generate_embedding_with_retry(
        &self,
        content: &str,
        max_retries: u32,
        preferred_model: Option<String>,
    ) -> Result<Vec<f32>, EmbeddingError> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match self
                .generate_embedding(content, preferred_model.clone())
                .await
            {
                Ok(embedding) => {
                    // Validate embedding is not empty
                    if embedding.is_empty() {
                        return Err(EmbeddingError::NoEmbeddings);
                    }
                    return Ok(embedding);
                }
                Err(EmbeddingError::NoEmbeddings) => {
                    // Don't retry on NoEmbeddings - immediate fail
                    return Err(EmbeddingError::NoEmbeddings);
                }
                Err(e) => {
                    last_error = Some(e);
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
        preferred_model: Option<String>,
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut all_embeddings = Vec::new();

        for chunk in texts.chunks(batch_size) {
            let mut batch_futures = Vec::new();

            for text in chunk {
                batch_futures.push(self.generate_embedding(text, preferred_model.clone()));
            }

            let batch_results = futures::future::join_all(batch_futures).await;

            for result in batch_results {
                all_embeddings.push(result?);
            }
        }

        Ok(all_embeddings)
    }

    /// Semantic search in dimension-specific collection
    pub async fn search_messages(
        &self,
        query: &str,
        limit: usize,
        filters: Option<Value>,
        preferred_model: Option<String>,
    ) -> Result<Vec<crate::storage::chroma_client::ScoredResult>, EmbeddingError> {
        // Use cross-dimensional search for best results
        self.search_all_dimensions(query, limit, filters, preferred_model)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LlmProviderConfig, ModelCapability, ModelTask, ProviderType};
    use crate::storage::chroma_client::ScoredResult;
    use httptest::{matchers::*, responders::*, Expectation, Server};

    fn create_test_config(bridge_url: &str) -> Config {
        let mut config = Config::default();
        config.llm_bridge_url = bridge_url.to_string();
        config.llm_providers.push(LlmProviderConfig {
            id: "test-provider".to_string(),
            provider_type: ProviderType::Ollama,
            base_url: "http://test".to_string(),
            api_key: None,
            timeout_secs: 120,
            priority: 1,
            models: vec![ModelCapability {
                model_id: "nomic-embed-text".to_string(),
                task: ModelTask::Embedding,
                context_window: 8192,
                supports_vision: false,
                supports_audio: false,
                dimension: Some(768),
            }],
        });
        config
    }

    #[tokio::test]
    async fn test_get_model_dimension_success() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
                json_encoded(serde_json::json!([
                    {
                        "model_id": "nomic-embed-text",
                        "provider_id": "ollama",
                        "task": "embedding",
                        "context_window": 8192,
                        "dimension": 768,
                        "supports_vision": false,
                        "supports_audio": false
                    }
                ])),
            ),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.get_model_dimension(Some("nomic-embed-text")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 768);
    }

    #[tokio::test]
    async fn test_get_model_dimension_not_found() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
                json_encoded(serde_json::json!([
                    {
                        "model_id": "other-model",
                        "provider_id": "ollama",
                        "task": "embedding",
                        "context_window": 8192,
                        "dimension": 1536,
                        "supports_vision": false,
                        "supports_audio": false
                    }
                ])),
            ),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.get_model_dimension(Some("nonexistent-model")).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EmbeddingError::ModelNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_get_model_dimension_no_models() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("GET", "/api/v1/models"))
                .respond_with(json_encoded(serde_json::json!([]))),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.get_model_dimension(None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EmbeddingError::NoModelsAvailable
        ));
    }

    #[tokio::test]
    async fn test_get_model_dimension_cache() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("GET", "/api/v1/models"))
                .times(1)
                .respond_with(json_encoded(serde_json::json!([
                    {
                        "model_id": "nomic-embed-text",
                        "provider_id": "ollama",
                        "task": "embedding",
                        "context_window": 8192,
                        "dimension": 768,
                        "supports_vision": false,
                        "supports_audio": false
                    }
                ]))),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        // First call - hits API
        let result1 = service.get_model_dimension(Some("nomic-embed-text")).await;
        assert_eq!(result1.unwrap(), 768);

        // Second call - hits cache (server expectation of .times(1) ensures this)
        let result2 = service.get_model_dimension(Some("nomic-embed-text")).await;
        assert_eq!(result2.unwrap(), 768);
    }

    #[tokio::test]
    async fn test_embed_with_collection_routing_success() {
        let server = Server::run();

        // Mock /api/v1/route endpoint
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
                json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                })),
            ),
        );

        // Mock /api/v1/embed endpoint
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service
            .embed_with_collection_routing("test content", None)
            .await;
        assert!(result.is_ok());
        let (embedding, dimension, model) = result.unwrap();
        assert_eq!(embedding.len(), 768);
        assert_eq!(dimension, 768);
        assert_eq!(model, "nomic-embed-text");
    }

    #[tokio::test]
    async fn test_generate_embedding_success() {
        let server = Server::run();

        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
                json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                })),
            ),
        );

        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.generate_embedding("test", None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 768);
    }

    #[tokio::test]
    async fn test_generate_embedding_with_retry_success() {
        let server = Server::run();

        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
                json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                })),
            ),
        );

        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.generate_embedding_with_retry("test", 3, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 768);
    }

    #[tokio::test]
    async fn test_generate_embedding_with_retry_exhaustion() {
        let server = Server::run();

        // All attempts fail with 500 - with max_retries=2, we make 2 attempts (0..2 = 0,1)
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route"))
                .times(2)
                .respond_with(status_code(500)),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.generate_embedding_with_retry("test", 2, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EmbeddingError::MaxRetriesExceeded
        ));
    }

    #[tokio::test]
    async fn test_generate_embeddings_batch() {
        let server = Server::run();

        // Mock route and embed for batch
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route"))
                .times(3)
                .respond_with(json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                }))),
        );

        server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed"))
                .times(3)
                .respond_with(json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                }))),
        );

        let config = create_test_config(&server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let texts = vec![
            "text1".to_string(),
            "text2".to_string(),
            "text3".to_string(),
        ];
        let result = service.generate_embeddings_batch(texts, 2, None).await;

        assert!(result.is_ok());
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 768);
    }

    #[tokio::test]
    async fn test_generate_embeddings_batch_empty() {
        let config = create_test_config("http://localhost:5001");
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service.generate_embeddings_batch(vec![], 10, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_error_conversion_anyhow() {
        let anyhow_err = anyhow::anyhow!("test error");
        let err: EmbeddingError = anyhow_err.into();
        assert!(matches!(err, EmbeddingError::BridgeError(_)));
    }

    // NEW TESTS FOR UNCOVERED LINES

    #[tokio::test]
    async fn test_search_all_dimensions_enough_results_in_primary() {
        let chroma_server = Server::run();
        let bridge_server = Server::run();

        // Mock embedding generation
        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
                json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                })),
            ),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        // Mock Chroma query with enough results
        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768/query",
            ))
            .respond_with(json_encoded(serde_json::json!({
                "ids": [["id1", "id2", "id3"]],
                "distances": [[0.1, 0.2, 0.3]],
                "documents": [["doc1", "doc2", "doc3"]],
                "metadatas": [[{"key": "value1"}, {"key": "value2"}, {"key": "value3"}]]
            }))),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let mut service = EmbeddingService::new(
            bridge,
            chroma_server.url_str("").trim_end_matches('/').to_string(),
        );

        let result = service.search_all_dimensions("query", 3, None, None).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_all_dimensions_cross_dimensional_search() {
        let chroma_server = Server::run();
        let bridge_server = Server::run();

        // Mock embedding generation for primary dimension
        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route"))
                .times(2)
                .respond_with(json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                }))),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed"))
                .times(2)
                .respond_with(json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                }))),
        );

        // Mock list_models to provide other dimensions
        bridge_server.expect(
            Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
                json_encoded(serde_json::json!([
                    {
                        "model_id": "nomic-embed-text",
                        "provider_id": "ollama",
                        "task": "embedding",
                        "context_window": 8192,
                        "dimension": 768,
                        "supports_vision": false,
                        "supports_audio": false
                    },
                    {
                        "model_id": "text-embedding-3-small",
                        "provider_id": "openai",
                        "task": "embedding",
                        "context_window": 8192,
                        "dimension": 1536,
                        "supports_vision": false,
                        "supports_audio": false
                    }
                ])),
            ),
        );

        // Mock Chroma query for primary dimension (returns 1 result)
        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768/query",
            ))
            .respond_with(json_encoded(serde_json::json!({
                "ids": [["id1"]],
                "distances": [[0.1]],
                "documents": [["doc1"]],
                "metadatas": [[{"key": "value1"}]]
            }))),
        );

        // Mock Chroma query for other dimension
        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_1536/query",
            ))
            .respond_with(json_encoded(serde_json::json!({
                "ids": [["id2"]],
                "distances": [[0.2]],
                "documents": [["doc2"]],
                "metadatas": [[{"key": "value2"}]]
            }))),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(
            bridge,
            chroma_server.url_str("").trim_end_matches('/').to_string(),
        );

        let result = service.search_all_dimensions("query", 5, None, None).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert!(results.len() >= 1); // At least primary result
    }

    #[tokio::test]
    async fn test_search_all_dimensions_with_failed_secondary() {
        let chroma_server = Server::run();
        let bridge_server = Server::run();

        // Mock primary embedding
        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route"))
                .times(2)
                .respond_with(json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                }))),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed"))
                .times(1) // Primary succeeds
                .respond_with(json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })))
                .then()
                .times(1) // Secondary fails
                .respond_with(status_code(500)),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
                json_encoded(serde_json::json!([
                    {
                        "model_id": "nomic-embed-text",
                        "provider_id": "ollama",
                        "task": "embedding",
                        "dimension": 768
                    },
                    {
                        "model_id": "other-model",
                        "provider_id": "openai",
                        "task": "embedding",
                        "dimension": 1536
                    }
                ])),
            ),
        );

        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768/query",
            ))
            .respond_with(json_encoded(serde_json::json!({
                "ids": [["id1"]],
                "distances": [[0.1]],
                "documents": [["doc1"]],
                "metadatas": [[{"key": "value1"}]]
            }))),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(
            bridge,
            chroma_server.url_str("").trim_end_matches('/').to_string(),
        );

        // Should succeed with primary results even if secondary fails
        let result = service.search_all_dimensions("query", 5, None, None).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1); // Only primary result
    }

    #[tokio::test]
    async fn test_process_message_with_metadata_types() {
        let chroma_server = Server::run();
        let bridge_server = Server::run();

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
                json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                })),
            ),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768",
            ))
            .respond_with(status_code(200)),
        );

        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768/upsert",
            ))
            .respond_with(status_code(200)),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(
            bridge,
            chroma_server.url_str("").trim_end_matches('/').to_string(),
        );

        // Test with various metadata types
        let metadata = json!({
            "string_field": "value",
            "number_field": 42,
            "bool_field": true,
            "array_field": [1, 2, 3], // Will be converted to string
            "nested_object": {"inner": "value"} // Will be converted to string
        });

        let result = service
            .process_message(
                Uuid::new_v4(),
                "test content",
                Uuid::new_v4(),
                metadata,
                None,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_message_with_retry_success_after_failure() {
        let chroma_server = Server::run();
        let bridge_server = Server::run();

        // First attempt fails, second succeeds
        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route"))
                .times(1)
                .respond_with(status_code(500))
                .then()
                .times(1)
                .respond_with(json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                }))),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768",
            ))
            .respond_with(status_code(200)),
        );

        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768/upsert",
            ))
            .respond_with(status_code(200)),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(
            bridge,
            chroma_server.url_str("").trim_end_matches('/').to_string(),
        );

        let result = service
            .process_message_with_retry(
                Uuid::new_v4(),
                "test content",
                Uuid::new_v4(),
                json!({}),
                None,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_message_with_retry_max_retries_exceeded() {
        let bridge_server = Server::run();

        // All attempts fail (max_retries=3 means 4 total attempts: 0,1,2,3)
        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route"))
                .times(4)
                .respond_with(status_code(500)),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

        let result = service
            .process_message_with_retry(
                Uuid::new_v4(),
                "test content",
                Uuid::new_v4(),
                json!({}),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EmbeddingError::MaxRetriesExceeded
        ));
    }

    #[tokio::test]
    async fn test_search_messages_delegates_to_search_all_dimensions() {
        let chroma_server = Server::run();
        let bridge_server = Server::run();

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
                json_encoded(serde_json::json!({
                    "provider_id": "ollama",
                    "model_id": "nomic-embed-text",
                    "estimated_cost": 0.0,
                    "reason": "Best match",
                    "provider_type": "ollama"
                })),
            ),
        );

        bridge_server.expect(
            Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": vec![0.1; 768],
                    "model": "nomic-embed-text",
                    "dimension": 768,
                    "tokens_used": 5
                })),
            ),
        );

        chroma_server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/api/v1/collections/conversations_768/query",
            ))
            .respond_with(json_encoded(serde_json::json!({
                "ids": [["id1"]],
                "distances": [[0.1]],
                "documents": [["doc1"]],
                "metadatas": [[{"key": "value1"}]]
            }))),
        );

        let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
        let bridge = BridgeClient::new(&config).unwrap();
        let service = EmbeddingService::new(
            bridge,
            chroma_server.url_str("").trim_end_matches('/').to_string(),
        );

        let result = service.search_messages("query", 5, None, None).await;
        assert!(result.is_ok());
    }
}
