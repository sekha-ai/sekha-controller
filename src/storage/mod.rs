pub mod chroma_client;
pub mod db;
pub mod entities;
pub mod repository;

pub use chroma_client::{ChromaClient, ChromaError};
pub use db::init_db;
pub use entities::{conversations, messages};
pub use repository::{ConversationRepository, SeaOrmConversationRepository};

// Re-export MockConversationRepository for tests and test-utils feature
#[cfg(any(test, feature = "test-utils"))]
pub use repository::MockConversationRepository;

#[cfg(test)]
mod repository_tests;
