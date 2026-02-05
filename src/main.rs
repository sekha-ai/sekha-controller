use anyhow::Result;
use sekha_controller::{
    api::routes::{create_router, AppState},
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{
        chroma_client::ChromaClient, db::get_connection, repository::PostgresConversationRepository,
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

    // Initialize database connection
    let db = get_connection()
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to connect to database"))?;
    tracing::info!("✅ Database connected");

    // Get Ollama URL from config
    let ollama_url = config
        .ollama_url
        .clone()
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    // Initialize Chroma client
    let chroma_url =
        std::env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let chroma_client = Arc::new(ChromaClient::new(chroma_url.clone()));
    tracing::info!("✅ Chroma client initialized: {}", chroma_url);

    // Initialize embedding service
    let embedding_service = Arc::new(EmbeddingService::new(ollama_url.clone(), chroma_url));
    tracing::info!("✅ Embedding service initialized");

    // Initialize LLM bridge client
    let llm_bridge = Arc::new(LlmBridgeClient::new(&config)?);
    tracing::info!("✅ LLM Bridge client initialized");

    // Health check LLM bridge
    match llm_bridge.health_check().await {
        Ok(_) => tracing::info!("✅ LLM Bridge health check passed"),
        Err(e) => {
            tracing::warn!("⚠️ LLM Bridge health check failed: {}", e);
            if let Ok(models) = llm_bridge.list_models().await {
                tracing::info!("📊 LLM Bridge models: {:?}", models);
            }
        }
    }

    // Create repository
    let repository = Arc::new(
        PostgresConversationRepository::new(db.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create repository: {}", e))?,
    );
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
