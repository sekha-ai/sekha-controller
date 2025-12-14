use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use dotenv;
use axum::Router;  // Add this missing import

// Import our modules
use sekha_controller::{
    config::Config,
    api::{routes, mcp},
    storage::{self, chroma_client::ChromaClient, repository::SeaOrmConversationRepository},
    services::embedding_service::EmbeddingService,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sekha_controller=info".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = Arc::new(RwLock::new(Config::load()?));
    
    // Initialize database
    let db_url = config.read().await.database_url.clone();
    let db_conn = storage::init_db(&db_url).await?;
    
    // Create Chroma client for vector storage
    let chroma_url = config.read().await.chroma_url.clone();
    // Use value if not empty, otherwise default
    let chroma_url = if chroma_url.is_empty() { 
        "http://localhost:8000".to_string() 
    } else { 
        chroma_url 
    };
    let chroma_client = Arc::new(ChromaClient::new(chroma_url.clone()));

    // Create embedding service (Ollama + Chroma)
    let ollama_url = config.read().await.ollama_url.clone();
    let ollama_url = if ollama_url.is_empty() { 
        "http://localhost:11434".to_string() 
    } else { 
        ollama_url 
    };
    let embedding_service = Arc::new(EmbeddingService::new(
        ollama_url.clone(),
        chroma_url.clone(),
    ));

    // Create repository with both SQLite and Chroma integration
    let repository = Arc::new(SeaOrmConversationRepository::new(
        db_conn,
        chroma_client,
        embedding_service,
    ));

    // Create application state
    let state = routes::AppState {
        config: config.clone(),
        repo: repository.clone(),
    };

    // Build router with both REST and MCP endpoints
    let app = Router::new()
        .merge(routes::create_router(state.clone()))
        .merge(mcp::create_mcp_router(state.clone()));

    // Start server
    let addr_str = format!("127.0.0.1:{}", config.read().await.server_port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid address");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🚀 Server listening on {}", addr);
    tracing::info!("📊 Chroma URL: {}", chroma_url);
    tracing::info!("🤖 Ollama URL: {}", ollama_url);

    axum::serve(listener, app).await?;
    
    Ok(())
}