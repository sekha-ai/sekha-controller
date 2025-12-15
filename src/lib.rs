//! Sekha Controller - AI Memory System

pub mod api;
pub mod auth;
pub mod config;
pub mod models;
pub mod services;
pub mod storage;
pub mod orchestrator;

// Re-export for convenience
pub use services::llm_bridge_client::LlmBridgeClient;

// Re-export main types for convenience
pub use crate::storage::db::init_db;
pub use crate::storage::repository::{ConversationRepository, SeaOrmConversationRepository};
pub use crate::storage::chroma_client::ChromaClient;
pub use crate::models::internal::{Conversation, Message, NewConversation, NewMessage};
pub use crate::config::Config;
pub use crate::api::routes::{create_router, AppState};
pub use crate::api::dto::*;

#[cfg(test)]
mod tests {
    // Unit tests can go here if needed
}
