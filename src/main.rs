use axum::{middleware, Router};
use dotenvy;
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

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sekha-controller")]
#[command(about = "Sekha AI Memory Controller", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the controller server
    Start {
        /// Run as background daemon
        #[arg(short, long)]
        daemon: bool,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },

    /// Stop the running daemon
    Stop,

    /// Check controller health
    Health,

    /// Show current status
    Status,

    /// Initialize configuration
    Setup,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Start { daemon, port }) => {
            if *daemon {
                start_daemon(*port).await?;
            } else {
                start_server(*port).await?;
            }
        }
        Some(Commands::Stop) => {
            stop_daemon().await?;
        }
        Some(Commands::Health) => {
            check_health().await?;
        }
        Some(Commands::Status) => {
            show_status().await?;
        }
        Some(Commands::Setup) => {
            run_setup().await?;
        }
        None => {
            // Default: start server
            start_server(8080).await?;
        }
    }

    Ok(())
}

async fn start_daemon(port: u16) -> anyhow::Result<()> {
    use daemonize::Daemonize;

    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let pid_file = home_dir.join(".sekha/sekha.pid");
    let log_dir = home_dir.join(".sekha/logs");

    std::fs::create_dir_all(&log_dir)?;

    let daemonize = Daemonize::new()
        .pid_file(pid_file)
        .working_directory(home_dir)
        .umask(0o027);

    match daemonize.start() {
        Ok(_) => {
            tracing::info!("🔧 Sekha Controller started as daemon on port {}", port);
            start_server(port).await
        }
        Err(e) => {
            eprintln!("❌ Failed to start daemon: {}", e);
            std::process::exit(1);
        }
    }
}

async fn stop_daemon() -> anyhow::Result<()> {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let pid_file = home_dir.join(".sekha/sekha.pid");

    if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
            std::fs::remove_file(&pid_file)?;
            println!("✅ Sekha Controller stopped");
        }
    } else {
        println!("❌ No running daemon found");
    }

    Ok(())
}

async fn check_health() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    match client.get("http://localhost:8080/health").send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("✅ Sekha Controller is healthy");
            } else {
                println!(
                    "⚠️  Sekha Controller responded with status: {}",
                    resp.status()
                );
            }
        }
        Err(_) => {
            println!("❌ Sekha Controller is not running");
        }
    }
    Ok(())
}

async fn show_status() -> anyhow::Result<()> {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let pid_file = home_dir.join(".sekha/sekha.pid");

    if pid_file.exists() {
        println!("✅ Daemon running (PID file exists)");
        check_health().await?;
    } else {
        println!("❌ Daemon not running");
    }

    Ok(())
}

async fn run_setup() -> anyhow::Result<()> {
    println!("🔧 Setting up Sekha Controller...");

    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let config_dir = home_dir.join(".sekha");

    std::fs::create_dir_all(&config_dir)?;
    std::fs::create_dir_all(config_dir.join("data"))?;
    std::fs::create_dir_all(config_dir.join("logs"))?;
    std::fs::create_dir_all(config_dir.join("import"))?;
    std::fs::create_dir_all(config_dir.join("imported"))?;

    println!("✅ Directories created");
    println!("✅ Setup complete!");
    println!("\nNext steps:");
    println!("  1. Edit config: ~/.sekha/config.toml");
    println!("  2. Start server: sekha-controller start --daemon");

    Ok(())
}

async fn start_server(port: u16) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

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
        tracing::info!(
            "🚦 Rate limit: {} requests/minute",
            cfg.rate_limit_per_minute
        );
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
        chroma_client.clone(),
        embedding_service.clone(),
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
        embedding_service: embedding_service.clone(),
        chroma_client: chroma_client.clone(),
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

    // Get host and port from config
    let server_host = config.read().await.server_host.clone();
    let server_port = config.read().await.server_port;
    
    // Use CLI port if provided, otherwise use config port
    let actual_port = if port != 8080 { port } else { server_port };
    
    let addr_str = format!("{}:{}", server_host, actual_port);
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
