use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// Provider types supported by Sekha
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Ollama,
    LiteLlm,
    OpenRouter,
    OpenAi,
    Anthropic,
}

/// Task categories for model routing
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelTask {
    Embedding,
    ChatSmall,
    ChatLarge,
    ChatSmart,
    Vision,
    Audio,
}

/// Model capability metadata
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelCapability {
    pub model_id: String,
    pub task: ModelTask,
    pub context_window: usize,

    #[serde(default)]
    pub supports_vision: bool,

    #[serde(default)]
    pub supports_audio: bool,

    /// Embedding dimension (for embedding models only)
    pub dimension: Option<usize>,
}

/// LLM Provider configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LlmProviderConfig {
    /// Unique identifier (e.g., "ollama_local", "openai_cloud")
    pub id: String,

    /// Provider type
    #[serde(rename = "type")]
    pub provider_type: ProviderType,

    /// Base URL for provider API
    pub base_url: String,

    /// Optional API key
    pub api_key: Option<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,

    /// Provider priority (1 = highest, try first)
    pub priority: u8,

    /// Models available from this provider
    #[serde(default)]
    pub models: Vec<ModelCapability>,
}

/// Default model selections for common tasks
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DefaultModels {
    pub embedding: String,
    pub chat_fast: String,
    pub chat_smart: String,
    pub chat_vision: Option<String>,
}

/// Routing and fallback configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RoutingConfig {
    #[serde(default = "default_auto_fallback")]
    pub auto_fallback: bool,

    #[serde(default = "default_require_vision")]
    pub require_vision_for_images: bool,

    pub max_cost_per_request: Option<f64>,

    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
}

/// Circuit breaker configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u32,

    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct Config {
    // Server configuration
    pub server_host: String,

    #[validate(range(min = 1024, max = 65535))]
    pub server_port: u16,

    #[validate(length(min = 32))]
    pub mcp_api_key: String,

    pub database_url: String,
    pub chroma_url: String,
    pub llm_bridge_url: String,

    #[validate(range(min = 1, max = 100))]
    pub max_connections: u32,

    pub log_level: String,
    pub summarization_enabled: bool,
    pub pruning_enabled: bool,

    // REST API Configuration
    pub rest_api_key: Option<String>,

    #[serde(default)]
    pub additional_api_keys: Vec<String>,

    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,

    #[serde(default = "default_cors_enabled")]
    pub cors_enabled: bool,

    // ==== V2.0 CONFIGURATION ====
    /// Configuration version ("2.0")
    #[serde(default)]
    pub config_version: Option<String>,

    /// Provider registry (v2.0)
    #[serde(default)]
    pub llm_providers: Vec<LlmProviderConfig>,

    /// Default model selections (v2.0)
    pub default_models: Option<DefaultModels>,

    /// Routing configuration (v2.0)
    #[serde(default)]
    pub routing: RoutingConfig,

    // ==== DEPRECATED (v1.x) - Keep for backward compatibility ====
    /// @deprecated Use llm_providers instead
    pub ollama_url: Option<String>,

    /// @deprecated Use default_models.embedding instead
    pub embedding_model: Option<String>,

    /// @deprecated Use default_models.chat_fast instead
    pub summarization_model: Option<String>,
}

// Default functions
fn default_rate_limit() -> u32 {
    1000
}

fn default_cors_enabled() -> bool {
    true
}

fn default_timeout() -> u32 {
    120
}

fn default_auto_fallback() -> bool {
    true
}

fn default_require_vision() -> bool {
    true
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_timeout_secs() -> u32 {
    60
}

fn default_success_threshold() -> u32 {
    2
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            auto_fallback: true,
            require_vision_for_images: true,
            max_cost_per_request: None,
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            timeout_secs: 60,
            success_threshold: 2,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let mut settings = config::Config::builder()
            .set_default("server_host", "0.0.0.0")?
            .set_default("server_port", 8080)?
            .set_default("max_connections", 10)?
            .set_default("log_level", "info")?
            .set_default("database_url", "sqlite://sekha.db")?
            .set_default("chroma_url", "http://localhost:8000")?
            .set_default("llm_bridge_url", "http://localhost:5001")?
            .set_default("summarization_enabled", true)?
            .set_default("pruning_enabled", true)?
            .set_default("rate_limit_per_minute", 1000)?
            .set_default("cors_enabled", true)?
            .set_default("mcp_api_key", "dev_default_key_change_me_1234567890")?
            // V1.x defaults (for backward compatibility)
            .set_default("ollama_url", "http://localhost:11434")?
            .set_default("embedding_model", "nomic-embed-text")?
            .set_default("summarization_model", "llama3.1:8b")?
            // Load from config files
            .add_source(config::File::with_name("config").required(false))
            .add_source(
                config::File::with_name(&format!(
                    "{}/.sekha/config",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                ))
                .required(false),
            )
            // Environment variables with JSON parsing support
            .add_source(
                config::Environment::with_prefix("SEKHA")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let mut config: Config = settings.try_deserialize()?;

        // ==== AUTO-MIGRATION LOGIC ====
        // If no v2.0 providers configured but v1.x config exists, auto-migrate
        if config.llm_providers.is_empty() {
            if let Some(ollama_url) = &config.ollama_url {
                tracing::warn!("⚠️  Detected v1.x configuration. Auto-migrating to v2.0 format...");

                let embedding_model = config
                    .embedding_model
                    .clone()
                    .unwrap_or_else(|| "nomic-embed-text".to_string());
                let summarization_model = config
                    .summarization_model
                    .clone()
                    .unwrap_or_else(|| "llama3.1:8b".to_string());

                // Create default Ollama provider from v1.x config
                let migrated_provider = LlmProviderConfig {
                    id: "ollama_migrated".to_string(),
                    provider_type: ProviderType::Ollama,
                    base_url: ollama_url.clone(),
                    api_key: None,
                    timeout_secs: 120,
                    priority: 1,
                    models: vec![
                        ModelCapability {
                            model_id: embedding_model.clone(),
                            task: ModelTask::Embedding,
                            context_window: 512,
                            supports_vision: false,
                            supports_audio: false,
                            dimension: Some(768), // Assume nomic-embed-text default
                        },
                        ModelCapability {
                            model_id: summarization_model.clone(),
                            task: ModelTask::ChatSmall,
                            context_window: 8192,
                            supports_vision: false,
                            supports_audio: false,
                            dimension: None,
                        },
                        ModelCapability {
                            model_id: summarization_model.clone(),
                            task: ModelTask::ChatSmart,
                            context_window: 8192,
                            supports_vision: false,
                            supports_audio: false,
                            dimension: None,
                        },
                    ],
                };

                config.llm_providers.push(migrated_provider);
                config.default_models = Some(DefaultModels {
                    embedding: embedding_model,
                    chat_fast: summarization_model.clone(),
                    chat_smart: summarization_model,
                    chat_vision: None,
                });
                config.config_version = Some("2.0".to_string());

                tracing::info!(
                    "✅ Auto-migration complete. Please update config file to v2.0 format."
                );
            }
        }

        // Validate the configuration
        config.validate_providers()?;

        Ok(config)
    }

    /// Validate provider configuration
    pub fn validate_providers(&self) -> Result<(), config::ConfigError> {
        // If v2.0 config is present, validate it
        if !self.llm_providers.is_empty() {
            // Ensure default models are specified
            if self.default_models.is_none() {
                return Err(config::ConfigError::Message(
                    "default_models must be specified when using llm_providers".to_string(),
                ));
            }

            // Collect all model IDs from all providers
            let available_models: Vec<String> = self
                .llm_providers
                .iter()
                .flat_map(|p| p.models.iter().map(|m| m.model_id.clone()))
                .collect();

            // Validate that default models exist in some provider
            if let Some(defaults) = &self.default_models {
                if !available_models.contains(&defaults.embedding) {
                    return Err(config::ConfigError::Message(format!(
                        "Default embedding model '{}' not found in any provider",
                        defaults.embedding
                    )));
                }
                if !available_models.contains(&defaults.chat_fast) {
                    return Err(config::ConfigError::Message(format!(
                        "Default chat_fast model '{}' not found in any provider",
                        defaults.chat_fast
                    )));
                }
                if !available_models.contains(&defaults.chat_smart) {
                    return Err(config::ConfigError::Message(format!(
                        "Default chat_smart model '{}' not found in any provider",
                        defaults.chat_smart
                    )));
                }
            }

            // Check for duplicate provider IDs
            let mut seen_ids = std::collections::HashSet::new();
            for provider in &self.llm_providers {
                if !seen_ids.insert(&provider.id) {
                    return Err(config::ConfigError::Message(format!(
                        "Duplicate provider ID: {}",
                        provider.id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get provider for a specific task (helper for orchestrator)
    pub fn get_provider_for_task(&self, task: &ModelTask) -> Option<&LlmProviderConfig> {
        // Find providers that have models for this task, sorted by priority
        let mut candidates: Vec<&LlmProviderConfig> = self
            .llm_providers
            .iter()
            .filter(|p| p.models.iter().any(|m| &m.task == task))
            .collect();

        candidates.sort_by_key(|p| p.priority);
        candidates.first().copied()
    }

    /// Get the effective REST API key (rest_api_key or fallback to mcp_api_key)
    pub fn get_rest_api_key(&self) -> String {
        self.rest_api_key
            .clone()
            .unwrap_or_else(|| self.mcp_api_key.clone())
    }

    /// Get all valid API keys (primary + additional)
    pub fn get_all_api_keys(&self) -> Vec<String> {
        let mut keys = vec![self.mcp_api_key.clone(), self.get_rest_api_key()];
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
    fn test_provider_type_deserialization() {
        let json = r#""ollama""#;
        let provider_type: ProviderType = serde_json::from_str(json).unwrap();
        assert_eq!(provider_type, ProviderType::Ollama);
    }

    #[test]
    fn test_model_task_deserialization() {
        let json = r#""chat_small""#;
        let task: ModelTask = serde_json::from_str(json).unwrap();
        assert_eq!(task, ModelTask::ChatSmall);
    }

    #[test]
    fn test_circuit_breaker_defaults() {
        let cb = CircuitBreakerConfig::default();
        assert_eq!(cb.failure_threshold, 3);
        assert_eq!(cb.timeout_secs, 60);
        assert_eq!(cb.success_threshold, 2);
    }

    #[test]
    fn test_routing_config_defaults() {
        let routing = RoutingConfig::default();
        assert_eq!(routing.auto_fallback, true);
        assert_eq!(routing.require_vision_for_images, true);
        assert!(routing.max_cost_per_request.is_none());
    }
}
