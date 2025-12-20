pub mod embedding_queue;
pub mod embedding_service;
pub mod file_watcher;
pub mod llm_bridge_client;

// Re-export for convenience
pub use embedding_queue::EmbeddingJob;
pub use embedding_service::EmbeddingService;
pub use llm_bridge_client::LlmBridgeClient;

// Orchestrator services (needed for MCP)
pub use crate::orchestrator::context_assembly::ContextAssembler;
pub use crate::orchestrator::pruning_engine::PruningEngine;
