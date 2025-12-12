mod config;
mod auth;
mod api;
mod storage;
mod services;
mod models;

use std::sync::Arc;
use tokio::sync::RwLock;
use axum::Router;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::net::TcpListener;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sekha_controller=info".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = Arc::new(RwLock::new(config::Config::load()?));
    
    // Initialize database
    let db_url = config.read().await.database_url.clone();
    let db_conn = storage::init_db(&db_url).await?;
    
    // Create embedding queue
    let _embedding_queue = Arc::new(services::EmbeddingQueue::new());
    
    // Create Chroma client
    let _chroma_client = Arc::new(storage::ChromaClient::new(
        "http://localhost:8000".to_string()
    ));

    // Create repository
    let repo = Arc::new(storage::SeaOrmConversationRepository::new(db_conn));
    
    // Create application state
    let state = api::routes::AppState {
        config: config.clone(),
        repo: repo.clone(),
    };

    // Build router
    let app = Router::new()
        .merge(api::routes::create_router(state.clone()))
        .merge(api::mcp::create_mcp_router(state.clone()));

    // Start server
    let addr_str = format!("127.0.0.1:{}", config.read().await.server_port);
    let addr: std::net::SocketAddr = addr_str.parse().expect("Invalid address");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;
    
    Ok(())
}
