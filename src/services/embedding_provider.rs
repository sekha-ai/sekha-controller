// src/services/embedding_provider.rs

use async_trait::async_trait;
use thiserror::Error;

/// Provider-specific errors
#[derive(Debug, Error, Clone)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("No embeddings returned")]
    NoEmbeddings,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Trait for embedding providers (Ollama, OpenAI, etc.)
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for the given text content
    async fn generate_embedding(&self, content: &str) -> Result<Vec<f32>, ProviderError>;
}

/// Ollama provider implementation
pub struct OllamaProvider {
    ollama: ollama_rs::Ollama,
    model: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            ollama: ollama_rs::Ollama::new(base_url, 11434),
            model,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn generate_embedding(&self, content: &str) -> Result<Vec<f32>, ProviderError> {
        use ollama_rs::generation::embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest};
        
        let input = EmbeddingsInput::Single(content.to_string());
        let request = GenerateEmbeddingsRequest::new(self.model.clone(), input);
        
        let response = self.ollama.generate_embeddings(request).await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        
        if response.embeddings.is_empty() {
            return Err(ProviderError::NoEmbeddings);
        }
        
        // Convert f64 to f32 for Chroma compatibility
        let embedding: Vec<f32> = match response.embeddings.len() {
            0 => return Err(ProviderError::NoEmbeddings),
            1 => response.embeddings[0].iter().map(|&v| v as f32).collect(),
            _ => response.embeddings.into_iter().next().unwrap().into_iter().map(|v| v as f32).collect(),
        };
        
        Ok(embedding)
    }
}

/// Mock provider for testing
pub struct MockProvider {
    pub response: Result<Vec<f32>, ProviderError>,
    pub call_count: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl MockProvider {
    /// Create a mock provider that returns a successful embedding
    pub fn new_success(embedding: Vec<f32>) -> Self {
        Self {
            response: Ok(embedding),
            call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }
    
    /// Create a mock provider that returns an error
    pub fn new_error(error: ProviderError) -> Self {
        Self {
            response: Err(error),
            call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for MockProvider {
    async fn generate_embedding(&self, _content: &str) -> Result<Vec<f32>, ProviderError> {
        *self.call_count.lock().unwrap() += 1;
        // Clone the result to allow multiple calls
        match &self.response {
            Ok(vec) => Ok(vec.clone()),
            Err(err) => Err(err.clone()),
        }
    }
}