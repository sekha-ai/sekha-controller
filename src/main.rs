use anyhow::Result;
use sekha_controller::{
    api::routes::{create_router, AppState},
    config::Config,
    llm::bridge_client::BridgeClient,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{
        chroma_client::ChromaClient, db::init_db, repository::SeaOrmConversationRepository,
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("🚀 Starting Sekha Controller...");

    // Load configuration
    let config = Config::load()?;
    tracing::info!("✅ Configuration loaded");

    // Initialize database connection with init_db() which creates DB and runs migrations
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| config.database_url.clone());
    let db = init_db(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
    tracing::info!("✅ Database initialized and connected");

    // Initialize Chroma client
    let chroma_url =
        std::env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let chroma_client = Arc::new(ChromaClient::new(chroma_url.clone()));
    tracing::info!("✅ Chroma client initialized: {}", chroma_url);

    // Initialize LLM bridge client (must be created before EmbeddingService)
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config)?);
    tracing::info!("✅ LLM Bridge client initialized");

    // Health check LLM bridge
    match llm_bridge.health_check().await {
        Ok(_) => tracing::info!("✅ LLM Bridge health check passed"),
        Err(e) => {
            tracing::warn!("⚠️ LLM Bridge health check failed: {}", e);
            if let Ok(models) = llm_bridge.list_models().await {
                tracing::info!("📊 LLM Bridge models available: {}", models.len());
            }
        }
    }

    // Initialize embedding service with BridgeClient from llm_bridge
    // Note: We need to create a new BridgeClient since Arc<LlmBridgeClient> doesn't expose bridge
    let bridge_client = BridgeClient::new(&config)?;
    let embedding_service = Arc::new(EmbeddingService::new(bridge_client, chroma_url));
    tracing::info!("✅ Embedding service initialized");

    // Create repository
    let repository = Arc::new(SeaOrmConversationRepository::new(
        db.clone(),
        chroma_client.clone(),
        embedding_service.clone(),
    ));
    tracing::info!("✅ Repository initialized");

    // Create orchestrator
    let orchestrator = Arc::new(MemoryOrchestrator::new(
        repository.clone(),
        llm_bridge.clone(),
    ));
    tracing::info!("✅ Orchestrator initialized");

    // Wrap config in Arc<RwLock>
    let config = Arc::new(RwLock::new(config));

    // Create application state
    let state = AppState {
        config,
        repo: repository,
        orchestrator,
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    };

    // Create router
    let app = create_router(state).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );

    // Start server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("🎯 Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
