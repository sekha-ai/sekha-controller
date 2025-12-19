//! Sekha Controller - AI Memory System

pub mod api;
pub mod auth;
pub mod config;
pub mod models;
pub mod orchestrator;
pub mod services;
pub mod storage;

// Re-export for convenience
pub use services::embedding_service::EmbeddingService;
pub use services::llm_bridge_client::LlmBridgeClient;

// MCP tool support
pub use api::mcp::{create_mcp_router, McpToolResponse};

// Re-export main types for convenience
pub use crate::api::dto::*;
pub use crate::api::routes::{create_router, AppState};
pub use crate::config::Config;
pub use crate::models::internal::{Conversation, Message, NewConversation, NewMessage};
pub use crate::storage::chroma_client::ChromaClient;
pub use crate::storage::db::init_db;
pub use crate::storage::repository::{ConversationRepository, SeaOrmConversationRepository};

#[cfg(test)]
mod tests {
    // Unit tests can go here if needed
}
