use serde::Deserialize;
use validator::Validate;

/// Main configuration for Sekha Controller
#[derive(Debug, Deserialize, Validate, Clone)]
pub struct Config {
    /// HTTP server port
    #[validate(range(min = 1024, max = 65535))]
    pub server_port: u16,

    /// API key used by MCP clients
    #[validate(length(min = 32))]
    pub mcp_api_key: String,

    /// Optional dedicated REST API key; if `None`, REST falls back to `mcp_api_key`
    #[validate(length(min = 32))]
    pub rest_api_key: Option<String>,

    /// Database URL (SeaORM / SQLite)
    pub database_url: String,

    /// Ollama base URL
    pub ollama_url: String,

    /// Chroma base URL
    pub chroma_url: String,

    /// Embedding model name (Ollama / LLM bridge)
    pub embedding_model: String,

    /// Maximum database connections
    #[validate(range(min = 1, max = 100))]
    pub max_connections: u32,

    /// Log level (e.g., info, debug, trace)
    pub log_level: String,

    /// Whether summarization pipeline is enabled
    pub summarization_enabled: bool,

    /// Model used for summarization
    pub summarization_model: String,

    /// Whether pruning engine is enabled
    pub pruning_enabled: bool,

    /// Optional REST rate limit in requests per minute (for REST API fallback)
    /// If `None`, defaults to 1000.
    pub rest_rate_limit_per_minute: Option<u32>,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            // Core defaults
            .set_default("server_port", 8080)?
            .set_default("max_connections", 10)?
            .set_default("log_level", "info")?
            .set_default("database_url", "sqlite://sekha.db")?
            .set_default("ollama_url", "http://localhost:11434")?
            .set_default("chroma_url", "http://localhost:8000")?
            .set_default("embedding_model", "nomic-embed-text:latest")?
            .set_default("summarization_model", "llama3.1:8b")?
            .set_default("summarization_enabled", true)?
            .set_default("pruning_enabled", true)?
            // REST fallback defaults
            .set_default("rest_rate_limit_per_minute", 1000u32)?
            // Load from ~/.sekha/config.toml (if present)
            .add_source(
                config::File::with_name(&format!(
                    "{}/.sekha/config",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                ))
                .required(false),
            )
            // Environment overrides: SEKHA__SERVER_PORT, SEKHA__MCP_API_KEY, etc.
            .add_source(config::Environment::with_prefix("SEKHA").separator("__"))
            .build()?;

        let cfg: Config = settings.try_deserialize()?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Returns the effective REST API key:
    /// - `rest_api_key` if configured
    /// - otherwise falls back to `mcp_api_key`
    pub fn effective_rest_api_key(&self) -> &str {
        if let Some(ref rest_key) = self.rest_api_key {
            rest_key
        } else {
            &self.mcp_api_key
        }
    }

    /// Returns the effective REST rate limit (requests per minute).
    /// Defaults to 1000 if not explicitly set.
    pub fn effective_rest_rate_limit(&self) -> u32 {
        self.rest_rate_limit_per_minute.unwrap_or(1000)
    }
}

/// Subset of configuration that can be hot-reloaded at runtime.
#[derive(Debug, Clone)]
pub struct ReloadableConfig {
    pub summarization_enabled: bool,
    pub pruning_enabled: bool,
    pub log_level: String,
}

impl From<&Config> for ReloadableConfig {
    fn from(cfg: &Config) -> Self {
        ReloadableConfig {
            summarization_enabled: cfg.summarization_enabled,
            pruning_enabled: cfg.pruning_enabled,
            log_level: cfg.log_level.clone(),
        }
    }
}
