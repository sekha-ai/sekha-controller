// tests/unit/embedding_service_test.rs
//! Unit tests for embedding service - pure logic only
//! Tests that require Chroma moved to integration tests

use sekha_controller::services::{
    EmbeddingService,
    EmbeddingProvider,
    MockProvider,
};
use sekha_controller::services::embedding_provider::ProviderError;
use sekha_controller::services::embedding_service::EmbeddingError;
use std::sync::Arc;

// ============================================
// Test: Successful embedding generation
// ============================================

#[tokio::test]
async fn test_generate_embedding_success() {
    let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
    let provider_clone = provider.clone();
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let result = service.generate_embedding("test content").await;
    
    assert!(result.is_ok());
    let embedding = result.unwrap();
    assert_eq!(embedding.len(), 768);
    assert_eq!(*provider_clone.call_count.lock().unwrap(), 1);
}

// ============================================
// Test: Error propagation
// ============================================

#[tokio::test]
async fn test_generate_embedding_error() {
    let provider = Arc::new(MockProvider::new_error(
        ProviderError::NoEmbeddings
    ));
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let result = service.generate_embedding("test").await;
    assert!(result.is_err());
    
    // Check it's ProviderError variant (mapped from ProviderError)
    let error_str = result.unwrap_err().to_string();
    assert!(error_str.contains("No embeddings returned"));
}

// ============================================
// Test: Retry logic success
// ============================================

#[tokio::test]
async fn test_generate_embedding_with_retry_success() {
    let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let result = service.generate_embedding_with_retry("test content", 3).await;
    
    assert!(result.is_ok());
    let embedding = result.unwrap();
    assert_eq!(embedding.len(), 768);
}

// ============================================
// Test: Retry exhaustion
// ============================================

#[tokio::test]
async fn test_generate_embedding_with_retry_exhaustion() {
    let provider = Arc::new(MockProvider::new_error(
        ProviderError::Http("Connection failed".to_string())
    ));
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let result = service.generate_embedding_with_retry("test", 2).await;
    assert!(result.is_err());
    
    let error_str = result.unwrap_err().to_string();
    assert!(error_str.contains("Max retries exceeded"));
}

// ============================================
// Test: Don't retry on NoEmbeddings
// ============================================

#[tokio::test]
async fn test_generate_embedding_with_retry_no_embeddings_no_retry() {
    let provider = Arc::new(MockProvider::new_error(
        ProviderError::NoEmbeddings
    ));
    let provider_clone = provider.clone();
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let result = service.generate_embedding_with_retry("test", 3).await;
    assert!(result.is_err());
    
    // Should fail immediately without retries
    assert_eq!(*provider_clone.call_count.lock().unwrap(), 1);
}

// ============================================
// Test: Batch processing
// ============================================

#[tokio::test]
async fn test_generate_embeddings_batch() {
    let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
    let provider_clone = provider.clone();
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let texts = vec!["text1".to_string(), "text2".to_string(), "text3".to_string()];
    let result = service.generate_embeddings_batch(texts, 2).await;
    
    assert!(result.is_ok());
    let embeddings = result.unwrap();
    assert_eq!(embeddings.len(), 3);
    assert_eq!(*provider_clone.call_count.lock().unwrap(), 3);
}

// ============================================
// Test: Empty batch
// ============================================

#[tokio::test]
async fn test_generate_embeddings_batch_empty() {
    let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
    let service = EmbeddingService::with_provider(provider, "http://localhost:8000".to_string());
    
    let result = service.generate_embeddings_batch(vec![], 10).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

// ============================================
// Test: Rate limiting (semaphore)
// ============================================

#[tokio::test]
async fn test_semaphore_rate_limiting() {
    let provider = Arc::new(MockProvider::new_success(vec![0.1; 768]));
    let provider_clone = provider.clone();
    let service = Arc::new(EmbeddingService::with_provider(provider, "http://localhost:8000".to_string()));
    
    // Spawn 10 concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let service = service.clone();
        let handle = tokio::spawn(async move {
            service.generate_embedding(&format!("text{}", i)).await
        });
        handles.push(handle);
    }
    
    // Wait for all
    let results = futures::future::join_all(handles).await;
    assert!(results.iter().all(|r| r.is_ok()));
    
    // All 10 requests should have been processed (rate limited at 5 concurrent)
    assert_eq!(*provider_clone.call_count.lock().unwrap(), 10);
}