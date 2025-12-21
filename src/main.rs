use axum::{middleware, Router};
use dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import our modules
use sekha_controller::{
    api::{mcp, rate_limiter::RateLimiter, routes},
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{self, chroma_client::ChromaClient, repository::SeaOrmConversationRepository},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sekha_controller=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = Arc::new(RwLock::new(Config::load()?));

    // Log API configuration
    {
        let cfg = config.read().await;
        tracing::info!("🔒 API authentication enabled");
        tracing::info!("🚦 Rate limit: {} requests/minute", cfg.rate_limit_per_minute);
        tracing::info!("🌐 CORS enabled: {}", cfg.cors_enabled);
        tracing::info!("🔑 Configured API keys: {}", cfg.get_all_api_keys().len());
    }

    // Initialize database
    let db_url = config.read().await.database_url.clone();
    let db_conn = storage::init_db(&db_url).await?;

    // Create Chroma client for vector storage
    let chroma_url = config.read().await.chroma_url.clone();
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

    // Initialize LLM Bridge client (MODULE 6 integration)
    let llm_bridge = Arc::new(LlmBridgeClient::new("http://localhost:5001".to_string()));

    // Verify LLM Bridge health on startup
    match llm_bridge.health_check().await {
        Ok(true) => {
            tracing::info!("✅ LLM Bridge connected successfully");

            // List available models
            if let Ok(models) = llm_bridge.list_models().await {
                tracing::info!("📊 LLM Bridge models: {}", models.join(", "));
            }
        }
        Ok(false) => tracing::warn!("⚠️ LLM Bridge health check returned false"),
        Err(e) => tracing::warn!(
            "⚠️ LLM Bridge not available: {}. Intelligence features will be limited.",
            e
        ),
    }

    // Create Memory Orchestrator with LLM Bridge (MODULE 5 + 6 integration)
    let orchestrator = Arc::new(MemoryOrchestrator::new(repository.clone(), llm_bridge));

    // Create rate limiter (Module 6.3)
    let rate_limit_per_minute = config.read().await.rate_limit_per_minute;
    let rate_limiter = RateLimiter::new(rate_limit_per_minute);

    // Create application state
    let state = routes::AppState {
        config: config.clone(),
        repo: repository.clone(),
        orchestrator,
    };

    // Start file watcher in background
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let watch_path = home_dir.join(".sekha").join("import");

    let watcher_repo = repository.clone();
    tokio::spawn(async move {
        let watcher =
            sekha_controller::services::file_watcher::ImportWatcher::new(watch_path, watcher_repo);

        if let Err(e) = watcher.watch().await {
            tracing::error!("❌ File watcher error: {}", e);
        }
    });

    tracing::info!("👀 File watcher started for ~/.sekha/import/");

    // Build CORS layer
    let cors = if config.read().await.cors_enabled {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::permissive()
    };

    // Build router with REST, MCP endpoints, rate limiting, and CORS
    let app = Router::new()
        .merge(routes::create_router(state.clone()))
        .merge(mcp::create_mcp_router(state.clone()))
        // Apply rate limiting middleware
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            |state, req, next| async move {
                sekha_controller::api::rate_limiter::rate_limit_middleware(state, req, next).await
            },
        ))
        // Apply CORS
        .layer(cors);

    // Start server
    let addr_str = format!("127.0.0.1:{}", config.read().await.server_port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid address");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 Server listening on {}", addr);
    tracing::info!("📊 Chroma URL: {}", chroma_url);
    tracing::info!("🤖 Ollama URL: {}", ollama_url);
    tracing::info!("🧠 LLM Bridge URL: http://localhost:5001");
    tracing::info!("🤖 Smart Query: POST /api/v1/query/smart");
    tracing::info!("📖 API Docs: http://{}/docs (if enabled)", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
