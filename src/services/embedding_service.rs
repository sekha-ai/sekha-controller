use crate::storage::chroma_client::ChromaClient;
use ollama_rs::{Ollama, error::OllamaError};
use ollama_rs::generation::embeddings::request::{GenerateEmbeddingsRequest, EmbeddingsInput};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::AcquireError;
use tracing::{error};
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
}

impl EmbeddingService {
    pub fn new(ollama_url: String, chroma_url: String) -> Self {
        let ollama = Ollama::new(ollama_url, 11434);
        let chroma = Arc::new(ChromaClient::new(chroma_url));
        
        // Rate limiting: max 5 concurrent embedding requests
        let semaphore = Arc::new(Semaphore::new(5));

        Self {
            ollama,
            chroma,
            semaphore,
        }
    }

    /// Generate embedding for a message and store in Chroma
    pub async fn process_message(
        &self,
        message_id: Uuid,
        content: &str,
        conversation_id: Uuid,
        metadata: Value,
    ) -> Result<String, EmbeddingError> {
        let _permit = self.semaphore.acquire().await?;
        
        tracing::debug!("Generating embedding for message: {}", message_id);
        
        // Generate embedding via Ollama
        let embedding = self.generate_embedding(content).await?;
        
        // Prepare Chroma metadata
        let chroma_metadata = json!({
            "conversation_id": conversation_id.to_string(),
            "message_id": message_id.to_string(),
            "content_preview": &content[..content.len().min(100)],
            "metadata": metadata,
        });
        
        // Store in Chroma
        let embedding_id = message_id.to_string();
        self.chroma.ensure_collection("messages", embedding.len() as i32).await?;
        
        self.chroma.upsert(
            "messages",
            &embedding_id,
            embedding.clone(),
            chroma_metadata,
            Some(content.to_string()),
        ).await?;
        
        tracing::info!("Successfully stored embedding for message: {}", message_id);
        
        Ok(embedding_id)
    }

    /// Generate embedding using Ollama
    async fn generate_embedding(&self, content: &str) -> Result<Vec<f32>, EmbeddingError> {
        // NEW API: EmbeddingsInput::Single for a single string
        let input = EmbeddingsInput::Single(content.to_string());
        let request = GenerateEmbeddingsRequest::new(
            "nomic-embed-text:latest".to_string(),
            input,
        );

        let response = self.ollama.generate_embeddings(request).await?;
        
        if response.embeddings.is_empty() {
            return Err(EmbeddingError::NoEmbeddings);
        }

        // The embeddings might be Vec<Vec<f64>> for batch, or Vec<f64> for single
        // Try to extract the first embedding if it's nested
        let embedding: Vec<f32> = match response.embeddings.len() {
            0 => return Err(EmbeddingError::NoEmbeddings),
            1 => response.embeddings[0].iter().map(|&v| v as f32).collect(),
            _ => response.embeddings.into_iter().next().unwrap().into_iter().map(|v| v as f32).collect(),
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
        let results = self.chroma
            .query("messages", query_embedding, limit as u32, filters)
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
}