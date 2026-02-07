pub mod api;
pub mod auth;  // ✅ Added auth module declaration
pub mod config;
pub mod llm;
pub mod models;
pub mod orchestrator;
pub mod services;
pub mod storage;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_helpers;

use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::OnceLock;
use tokio::sync::Mutex;

static DB: OnceLock<Mutex<Option<DatabaseConnection>>> = OnceLock::new();

pub async fn init_db(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
}

pub fn set_db(db: DatabaseConnection) {
    let _ = DB.set(Mutex::new(Some(db)));
}

pub async fn get_db() -> Option<DatabaseConnection> {
    DB.get()?.lock().await.clone()
}
