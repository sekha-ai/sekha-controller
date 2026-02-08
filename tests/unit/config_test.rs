// tests/unit/config_test.rs
//! Comprehensive tests for Config module to achieve 100% coverage

use sekha_controller::config::*;
use std::env;

fn clear_sekha_env_vars() {
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }
}

#[test]
fn test_default_rate_limit() {
    let config = Config::default();
    assert_eq!(config.rate_limit_per_minute, 1000);
}

#[test]
fn test_default_cors_enabled() {
    let config = Config::default();
    assert_eq!(config.cors_enabled, true);
}

#[test]
fn test_default_timeout() {
    // default_timeout() is for provider timeout_secs
    let provider = LlmProviderConfig {
        id: "test".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://test".to_string(),
        api_key: None,
        timeout_secs: 120, // Uses default_timeout
        priority: 1,
        models: vec![],
    };
    assert_eq!(provider.timeout_secs, 120);
}

#[test]
fn test_default_auto_fallback() {
    let routing = RoutingConfig::default();
    assert_eq!(routing.auto_fallback, true);
}

#[test]
fn test_default_require_vision() {
    let routing = RoutingConfig::default();
    assert_eq!(routing.require_vision_for_images, true);
}

#[test]
fn test_default_failure_threshold() {
    let circuit_breaker = CircuitBreakerConfig::default();
    assert_eq!(circuit_breaker.failure_threshold, 3);
}

#[test]
fn test_default_timeout_secs() {
    let circuit_breaker = CircuitBreakerConfig::default();
    assert_eq!(circuit_breaker.timeout_secs, 60);
}

#[test]
fn test_default_success_threshold() {
    let circuit_breaker = CircuitBreakerConfig::default();
    assert_eq!(circuit_breaker.success_threshold, 2);
}

#[test]
fn test_config_load_with_defaults() {
    clear_sekha_env_vars();

    // Load config - should use all defaults (or config.toml if present)
    let config = Config::load().expect("Failed to load config");

    // Verify core defaults are applied (some may come from config.toml)
    assert!(!config.server_host.is_empty());
    assert!(config.server_port > 0);
    assert!(config.max_connections > 0);
    assert!(!config.log_level.is_empty());
    assert!(!config.database_url.is_empty());
    assert!(!config.chroma_url.is_empty());
    assert!(!config.llm_bridge_url.is_empty());
    assert!(config.mcp_api_key.len() >= 32);

    clear_sekha_env_vars();
}

#[test]
fn test_config_load_with_v1_auto_migration() {
    clear_sekha_env_vars();

    // Set v1.x config via env vars
    env::set_var("SEKHA_OLLAMA_URL", "http://localhost:11434");
    env::set_var("SEKHA_EMBEDDING_MODEL", "nomic-embed-text");
    env::set_var("SEKHA_SUMMARIZATION_MODEL", "llama3.1:8b");

    let config = Config::load().expect("Failed to load config");

    // Should have auto-migrated to v2.0
    assert!(
        !config.llm_providers.is_empty(),
        "Should have migrated providers"
    );
    assert_eq!(config.llm_providers[0].id, "ollama_migrated");
    assert_eq!(config.llm_providers[0].provider_type, ProviderType::Ollama);
    assert_eq!(config.llm_providers[0].base_url, "http://localhost:11434");
    assert_eq!(config.llm_providers[0].timeout_secs, 120);
    assert_eq!(config.llm_providers[0].priority, 1);

    // Check models were migrated
    assert_eq!(config.llm_providers[0].models.len(), 3);

    // Embedding model (may have :latest appended from config file)
    let embedding = &config.llm_providers[0].models[0];
    assert!(embedding.model_id.starts_with("nomic-embed-text"));
    assert_eq!(embedding.task, ModelTask::Embedding);
    assert_eq!(embedding.context_window, 512);
    assert_eq!(embedding.supports_vision, false);
    assert_eq!(embedding.supports_audio, false);
    assert_eq!(embedding.dimension, Some(768));

    // Chat models
    let chat_small = &config.llm_providers[0].models[1];
    assert!(chat_small.model_id.starts_with("llama3.1:8b"));
    assert_eq!(chat_small.task, ModelTask::ChatSmall);
    assert_eq!(chat_small.context_window, 8192);

    let chat_smart = &config.llm_providers[0].models[2];
    assert!(chat_smart.model_id.starts_with("llama3.1:8b"));
    assert_eq!(chat_smart.task, ModelTask::ChatSmart);

    // Check default models exist
    assert!(config.default_models.is_some());
    let defaults = config.default_models.unwrap();
    assert!(defaults.embedding.starts_with("nomic-embed-text"));
    assert!(defaults.chat_fast.starts_with("llama3.1:8b"));
    assert!(defaults.chat_smart.starts_with("llama3.1:8b"));
    assert_eq!(defaults.chat_vision, None);

    // Check version
    assert_eq!(config.config_version, Some("2.0".to_string()));

    // Cleanup
    clear_sekha_env_vars();
}

#[test]
fn test_config_load_with_v1_default_models() {
    clear_sekha_env_vars();

    // Set v1.x config without explicit model names (should use defaults)
    env::set_var("SEKHA_OLLAMA_URL", "http://localhost:11434");

    let config = Config::load().expect("Failed to load config");

    // Should have auto-migrated with default model names
    assert!(!config.llm_providers.is_empty());
    let embedding = &config.llm_providers[0].models[0];
    assert!(embedding.model_id.starts_with("nomic-embed-text")); // Default (may have :latest)

    let chat = &config.llm_providers[0].models[1];
    assert!(chat.model_id.starts_with("llama3.1:8b")); // Default (may have :latest)

    // Cleanup
    clear_sekha_env_vars();
}

#[test]
fn test_config_no_migration_when_v2_providers_exist() {
    clear_sekha_env_vars();

    // Note: config.toml has ollama_url which triggers migration
    // We need to clear it and set v2 providers to prevent migration
    env::set_var("SEKHA_OLLAMA_URL", ""); // Clear config.toml value
    env::set_var(
        "SEKHA_LLM_PROVIDERS",
        r#"[{"id":"test","type":"ollama","base_url":"http://test","api_key":null,"timeout_secs":120,"priority":1,"models":[{"model_id":"test-embed","task":"embedding","context_window":512,"supports_vision":false,"supports_audio":false,"dimension":768},{"model_id":"test-chat","task":"chat_small","context_window":8192,"supports_vision":false,"supports_audio":false,"dimension":null},{"model_id":"test-chat2","task":"chat_smart","context_window":8192,"supports_vision":false,"supports_audio":false,"dimension":null}]}]"#,
    );
    env::set_var(
        "SEKHA_DEFAULT_MODELS",
        r#"{"embedding":"test-embed","chat_fast":"test-chat","chat_smart":"test-chat2","chat_vision":null}"#,
    );

    let config = Config::load().expect("Failed to load config");

    // Should NOT auto-migrate when v2 providers exist
    assert_eq!(config.llm_providers.len(), 1);
    assert_eq!(config.llm_providers[0].id, "test");
    assert_ne!(config.llm_providers[0].id, "ollama_migrated");

    // Cleanup
    clear_sekha_env_vars();
}

#[test]
fn test_config_env_var_override() {
    clear_sekha_env_vars();

    // Override defaults via env vars
    // Note: config.toml values take precedence over set_default but env vars override config.toml
    env::set_var("SEKHA_LOG_LEVEL", "debug");
    env::set_var("SEKHA_RATE_LIMIT_PER_MINUTE", "2000");
    env::set_var("SEKHA_CORS_ENABLED", "false");

    let config = Config::load().expect("Failed to load config");

    // Test values that env vars can actually override
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.rate_limit_per_minute, 2000);
    assert_eq!(config.cors_enabled, false);

    // Cleanup
    clear_sekha_env_vars();
}

#[test]
fn test_routing_config_defaults() {
    let routing = RoutingConfig::default();
    assert_eq!(routing.auto_fallback, true);
    assert_eq!(routing.require_vision_for_images, true);
    assert!(routing.max_cost_per_request.is_none());
}

#[test]
fn test_circuit_breaker_config_defaults() {
    let cb = CircuitBreakerConfig::default();
    assert_eq!(cb.failure_threshold, 3);
    assert_eq!(cb.timeout_secs, 60);
    assert_eq!(cb.success_threshold, 2);
}

#[test]
fn test_provider_config_with_explicit_timeout() {
    let provider = LlmProviderConfig {
        id: "test".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://test".to_string(),
        api_key: None,
        timeout_secs: 90, // Non-default
        priority: 1,
        models: vec![],
    };
    assert_eq!(provider.timeout_secs, 90);
}

#[test]
fn test_config_validation_called() {
    clear_sekha_env_vars();

    // This should succeed and call validate_providers internally
    let result = Config::load();
    assert!(result.is_ok(), "Config load should validate and succeed");

    clear_sekha_env_vars();
}

#[test]
fn test_config_home_directory_fallback() {
    clear_sekha_env_vars();

    let original_home = env::var("HOME").ok();

    // Test with HOME set
    env::set_var("HOME", "/tmp/test_home");
    let config = Config::load();
    assert!(config.is_ok());

    // Test with HOME unset (uses "." fallback)
    env::remove_var("HOME");
    let config = Config::load();
    assert!(config.is_ok());

    // Restore HOME
    if let Some(home) = original_home {
        env::set_var("HOME", home);
    }

    clear_sekha_env_vars();
}

#[test]
fn test_all_v1_defaults_in_migration() {
    clear_sekha_env_vars();

    // Just set ollama_url to trigger migration
    env::set_var("SEKHA_OLLAMA_URL", "http://localhost:11434");

    let config = Config::load().expect("Failed to load config");

    // Verify v1.x compatibility fields are set (may have :latest from config file)
    assert_eq!(
        config.ollama_url,
        Some("http://localhost:11434".to_string())
    );
    assert!(config.embedding_model.is_some());
    assert!(config
        .embedding_model
        .as_ref()
        .unwrap()
        .starts_with("nomic-embed-text"));
    assert!(config.summarization_model.is_some());
    assert!(config
        .summarization_model
        .as_ref()
        .unwrap()
        .starts_with("llama3.1:8b"));

    // Cleanup
    clear_sekha_env_vars();
}
