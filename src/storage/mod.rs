pub mod chroma_client;
pub mod db;
pub mod entities;
pub mod repository;

pub use chroma_client::{ChromaClient, ChromaError};
pub use db::init_db;
pub use entities::{conversations, messages};
pub use repository::{ConversationRepository, SeaOrmConversationRepository};

// Export mock for testing
#[cfg(test)]
pub use repository::MockConversationRepository;

#[cfg(test)]
mod repository_tests;
