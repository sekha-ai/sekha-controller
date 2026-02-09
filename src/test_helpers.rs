//! Test helper utilities for creating test instances

use crate::config::{Config, LlmProviderConfig, ModelCapability, ModelTask, ProviderType};
use crate::llm::bridge_client::BridgeClient;

/// Create a test BridgeClient with minimal configuration
pub fn create_test_bridge_client() -> BridgeClient {
    let config = create_test_config("http://localhost:11434");
    BridgeClient::new(&config).expect("Failed to create test BridgeClient")
}

/// Create a test configuration with a given bridge URL
pub fn create_test_config(bridge_url: &str) -> Config {
    let mut config = Config::default();
    config.llm_bridge_url = bridge_url.to_string();
    config.llm_providers.push(LlmProviderConfig {
        id: "test-provider".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![ModelCapability {
            model_id: "nomic-embed-text".to_string(),
            task: ModelTask::Embedding,
            context_window: 8192,
            supports_vision: false,
            supports_audio: false,
            dimension: Some(768),
        }],
    });
    config
}
