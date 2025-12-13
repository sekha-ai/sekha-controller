//! Sekha Controller - AI Memory System

pub mod api;
pub mod auth;
pub mod config;
pub mod models;
pub mod services;
pub mod storage;

// Re-export main types for convenience
pub use storage::{init_db, SeaOrmConversationRepository, ConversationRepository};
pub use models::internal::{Conversation, Message};
pub use config::Config;
pub use api::routes::{create_router, AppState};
pub use api::dto::*;

#[cfg(test)]
mod tests {
    // Unit tests can go here if needed
}