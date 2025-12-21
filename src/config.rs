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

    // REST API Configuration (Module 6.3)
    /// Optional REST API key (falls back to mcp_api_key if not provided)
    pub rest_api_key: Option<String>,
    
    /// Additional API keys for multi-user access
    #[serde(default)]
    pub additional_api_keys: Vec<String>,
    
    /// Rate limit: requests per minute
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    
    /// Enable CORS
    #[serde(default = "default_cors_enabled")]
    pub cors_enabled: bool,
}

fn default_rate_limit() -> u32 {
    1000
}

fn default_cors_enabled() -> bool {
    true
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
            .set_default("rate_limit_per_minute", 1000)?
            .set_default("cors_enabled", true)?
            // Load from ~/.sekha/config.toml
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

    /// Get the effective REST API key (rest_api_key or fallback to mcp_api_key)
    pub fn get_rest_api_key(&self) -> String {
        self.rest_api_key
            .clone()
            .unwrap_or_else(|| self.mcp_api_key.clone())
    }

    /// Get all valid API keys (primary + additional)
    pub fn get_all_api_keys(&self) -> Vec<String> {
        let mut keys = vec![
            self.mcp_api_key.clone(),
            self.get_rest_api_key(),
        ];
        keys.extend(self.additional_api_keys.clone());
        
        // Deduplicate
        keys.sort();
        keys.dedup();
        keys
    }

    /// Check if a given API key is valid
    pub fn is_valid_api_key(&self, key: &str) -> bool {
        self.get_all_api_keys().contains(&key.to_string())
    }
}

// Hot-reloadable subset
#[derive(Debug, Clone)]
pub struct ReloadableConfig {
    pub summarization_enabled: bool,
    pub pruning_enabled: bool,
    pub log_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rate_limit() {
        assert_eq!(default_rate_limit(), 1000);
    }

    #[test]
    fn test_default_cors_enabled() {
        assert_eq!(default_cors_enabled(), true);
    }

    #[test]
    fn test_get_rest_api_key_fallback() {
        let config = Config {
            server_port: 8080,
            mcp_api_key: "mcp_key_12345678901234567890123456789012".to_string(),
            database_url: "sqlite://test.db".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            chroma_url: "http://localhost:8000".to_string(),
            embedding_model: "test-model".to_string(),
            max_connections: 10,
            log_level: "info".to_string(),
            summarization_enabled: true,
            summarization_model: "test-model".to_string(),
            pruning_enabled: true,
            rest_api_key: None,
            additional_api_keys: vec![],
            rate_limit_per_minute: 1000,
            cors_enabled: true,
        };

        // Should fall back to mcp_api_key
        assert_eq!(
            config.get_rest_api_key(),
            "mcp_key_12345678901234567890123456789012"
        );
    }

    #[test]
    fn test_get_rest_api_key_explicit() {
        let config = Config {
            server_port: 8080,
            mcp_api_key: "mcp_key_12345678901234567890123456789012".to_string(),
            database_url: "sqlite://test.db".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            chroma_url: "http://localhost:8000".to_string(),
            embedding_model: "test-model".to_string(),
            max_connections: 10,
            log_level: "info".to_string(),
            summarization_enabled: true,
            summarization_model: "test-model".to_string(),
            pruning_enabled: true,
            rest_api_key: Some("rest_key_12345678901234567890123456789012".to_string()),
            additional_api_keys: vec![],
            rate_limit_per_minute: 1000,
            cors_enabled: true,
        };

        // Should use explicit rest_api_key
        assert_eq!(
            config.get_rest_api_key(),
            "rest_key_12345678901234567890123456789012"
        );
    }

    #[test]
    fn test_get_all_api_keys() {
        let config = Config {
            server_port: 8080,
            mcp_api_key: "key1".to_string(),
            database_url: "sqlite://test.db".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            chroma_url: "http://localhost:8000".to_string(),
            embedding_model: "test-model".to_string(),
            max_connections: 10,
            log_level: "info".to_string(),
            summarization_enabled: true,
            summarization_model: "test-model".to_string(),
            pruning_enabled: true,
            rest_api_key: Some("key2".to_string()),
            additional_api_keys: vec!["key3".to_string(), "key4".to_string()],
            rate_limit_per_minute: 1000,
            cors_enabled: true,
        };

        let all_keys = config.get_all_api_keys();
        assert_eq!(all_keys.len(), 4);
        assert!(all_keys.contains(&"key1".to_string()));
        assert!(all_keys.contains(&"key2".to_string()));
        assert!(all_keys.contains(&"key3".to_string()));
        assert!(all_keys.contains(&"key4".to_string()));
    }

    #[test]
    fn test_is_valid_api_key() {
        let config = Config {
            server_port: 8080,
            mcp_api_key: "valid_key".to_string(),
            database_url: "sqlite://test.db".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            chroma_url: "http://localhost:8000".to_string(),
            embedding_model: "test-model".to_string(),
            max_connections: 10,
            log_level: "info".to_string(),
            summarization_enabled: true,
            summarization_model: "test-model".to_string(),
            pruning_enabled: true,
            rest_api_key: None,
            additional_api_keys: vec!["extra_key".to_string()],
            rate_limit_per_minute: 1000,
            cors_enabled: true,
        };

        assert!(config.is_valid_api_key("valid_key"));
        assert!(config.is_valid_api_key("extra_key"));
        assert!(!config.is_valid_api_key("invalid_key"));
    }
}
