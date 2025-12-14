pub mod embedding_queue;
pub mod embedding_service;

// Re-export for convenience
pub use embedding_service::EmbeddingService;
pub use embedding_queue::EmbeddingJob;