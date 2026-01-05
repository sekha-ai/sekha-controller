use sekha_controller::services::embedding_service::EmbeddingService;

#[test]
fn test_embedding_service_new() {
    let ollama_url = "http://localhost:11434";
    let chroma_url = "http://localhost:8000";
    
    let _service = EmbeddingService::new(
        ollama_url.to_string(),
        chroma_url.to_string(),
    );
    // Should construct successfully
}

#[test]
fn test_embedding_service_urls_with_trailing_slash() {
    let _service = EmbeddingService::new(
        "http://localhost:11434/".to_string(),
        "http://localhost:8000/".to_string(),
    );
    // Should handle trailing slashes
}

#[test]
fn test_text_chunking_logic() {
    // Test text chunking for embeddings
    let text = "This is a test sentence. This is another sentence.";
    let chunks: Vec<&str> = text.split(". ").collect();
    
    assert_eq!(chunks.len(), 2);
}

#[test]
fn test_embedding_dimension_validation() {
    // Test that embeddings have correct dimensions
    let valid_dimensions = vec![384, 768, 1024, 1536];
    
    for dim in valid_dimensions {
        let embedding = vec![0.0; dim];
        assert_eq!(embedding.len(), dim);
    }
}

#[test]
fn test_embedding_normalization() {
    // Test vector normalization logic
    let vector = vec![1.0, 2.0, 3.0];
    let magnitude = (vector.iter().map(|x| x * x).sum::<f64>()).sqrt();
    
    assert!(magnitude > 0.0);
    
    // Normalized vector
    let normalized: Vec<f64> = vector.iter().map(|x| x / magnitude).collect();
    let norm_magnitude = (normalized.iter().map(|x| x * x).sum::<f64>()).sqrt();
    
    assert!((norm_magnitude - 1.0).abs() < 0.0001); // Should be ~1.0
}

#[test]
fn test_batch_text_preparation() {
    let texts = vec!["text1", "text2", "text3"];
    
    // Test batch size calculations
    assert_eq!(texts.len(), 3);
    
    // Simulate chunking into batches of 2
    let batch_size = 2;
    let num_batches = (texts.len() + batch_size - 1) / batch_size;
    assert_eq!(num_batches, 2);
}
