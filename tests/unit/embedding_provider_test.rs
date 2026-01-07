#[tokio::test]
async fn test_ollama_no_embeddings_error() {
    // Mock response with empty embeddings array
    let provider = MockProvider::new_error(ProviderError::NoEmbeddings);
    let result = provider.generate_embedding("test").await;
    assert!(matches!(result, Err(ProviderError::NoEmbeddings)));
}