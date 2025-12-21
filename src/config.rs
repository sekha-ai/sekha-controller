use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct Config {
    #[validate(range(min = 1024, max = 65535))]
    pub server_port: u16,

    #[validate(length(min = 32))]
    pub mcp_api_key: String,

    pub database_url: String,
    pub ollama_url: String,
    pub chroma_url: String,
    pub embedding_model: String,

    #[validate(range(min = 1, max = 100))]
    pub max_connections: u32,

    pub log_level: String,
    pub summarization_enabled: bool,
    pub summarization_model: String,
    pub pruning_enabled: bool,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
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
            // .add_source(config::File::with_name("config").required(false))
            // FIX: Load from ~/.sekha/config.toml
            .add_source(
                config::File::with_name(&format!(
                    "{}/.sekha/config",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                ))
                .required(false),
            )
            .add_source(config::Environment::with_prefix("SEKHA").separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}

// Hot-reloadable subset
#[derive(Debug, Clone)]
pub struct ReloadableConfig {
    pub summarization_enabled: bool,
    pub pruning_enabled: bool,
    pub log_level: String,
}

// REST API Fallback 
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server_port: u16,
    pub mcp_api_key: String,
    // ...
    pub rest_api_key: Option<String>, // new; optional, falls back to mcp_api_key
}

