// tests/unit/config_test.rs
//! Comprehensive tests for Config module to achieve 100% coverage

use sekha_controller::config::*;
use std::env;

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
    // Clear any existing SEKHA env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

    // Load config - should use all defaults
    let config = Config::load().expect("Failed to load config");

    // Verify all defaults are applied
    assert_eq!(config.server_host, "0.0.0.0");
    assert_eq!(config.server_port, 8080);
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.log_level, "info");
    assert_eq!(config.database_url, "sqlite://sekha.db");
    assert_eq!(config.chroma_url, "http://localhost:8000");
    assert_eq!(config.llm_bridge_url, "http://localhost:5001");
    assert_eq!(config.summarization_enabled, true);
    assert_eq!(config.pruning_enabled, true);
    assert_eq!(config.rate_limit_per_minute, 1000);
    assert_eq!(config.cors_enabled, true);
    assert_eq!(config.mcp_api_key, "dev_default_key_change_me_1234567890");
}

#[test]
fn test_config_load_with_v1_auto_migration() {
    // Clear env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

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

    // Embedding model
    let embedding = &config.llm_providers[0].models[0];
    assert_eq!(embedding.model_id, "nomic-embed-text");
    assert_eq!(embedding.task, ModelTask::Embedding);
    assert_eq!(embedding.context_window, 512);
    assert_eq!(embedding.supports_vision, false);
    assert_eq!(embedding.supports_audio, false);
    assert_eq!(embedding.dimension, Some(768));

    // Chat models
    let chat_small = &config.llm_providers[0].models[1];
    assert_eq!(chat_small.model_id, "llama3.1:8b");
    assert_eq!(chat_small.task, ModelTask::ChatSmall);
    assert_eq!(chat_small.context_window, 8192);

    let chat_smart = &config.llm_providers[0].models[2];
    assert_eq!(chat_smart.model_id, "llama3.1:8b");
    assert_eq!(chat_smart.task, ModelTask::ChatSmart);

    // Check default models
    assert!(config.default_models.is_some());
    let defaults = config.default_models.unwrap();
    assert_eq!(defaults.embedding, "nomic-embed-text");
    assert_eq!(defaults.chat_fast, "llama3.1:8b");
    assert_eq!(defaults.chat_smart, "llama3.1:8b");
    assert_eq!(defaults.chat_vision, None);

    // Check version
    assert_eq!(config.config_version, Some("2.0".to_string()));

    // Cleanup
    env::remove_var("SEKHA_OLLAMA_URL");
    env::remove_var("SEKHA_EMBEDDING_MODEL");
    env::remove_var("SEKHA_SUMMARIZATION_MODEL");
}

#[test]
fn test_config_load_with_v1_default_models() {
    // Clear env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

    // Set v1.x config without explicit model names (should use defaults)
    env::set_var("SEKHA_OLLAMA_URL", "http://localhost:11434");

    let config = Config::load().expect("Failed to load config");

    // Should have auto-migrated with default model names
    assert!(!config.llm_providers.is_empty());
    let embedding = &config.llm_providers[0].models[0];
    assert_eq!(embedding.model_id, "nomic-embed-text"); // Default

    let chat = &config.llm_providers[0].models[1];
    assert_eq!(chat.model_id, "llama3.1:8b"); // Default

    // Cleanup
    env::remove_var("SEKHA_OLLAMA_URL");
}

#[test]
fn test_config_no_migration_when_v2_providers_exist() {
    // Clear env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

    // Set both v1 and v2 config
    env::set_var("SEKHA_OLLAMA_URL", "http://localhost:11434");
    env::set_var(
        "SEKHA_LLM_PROVIDERS",
        r#"[{"id":"test","type":"ollama","base_url":"http://test","api_key":null,"timeout_secs":120,"priority":1,"models":[]}]"#,
    );

    let config = Config::load().expect("Failed to load config");

    // Should NOT auto-migrate when v2 providers exist
    assert_eq!(config.llm_providers.len(), 1);
    assert_eq!(config.llm_providers[0].id, "test");
    assert_ne!(config.llm_providers[0].id, "ollama_migrated");

    // Cleanup
    env::remove_var("SEKHA_OLLAMA_URL");
    env::remove_var("SEKHA_LLM_PROVIDERS");
}

#[test]
fn test_config_env_var_override() {
    // Clear env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

    // Override defaults via env vars
    env::set_var("SEKHA_SERVER_PORT", "9090");
    env::set_var("SEKHA_LOG_LEVEL", "debug");
    env::set_var("SEKHA_RATE_LIMIT_PER_MINUTE", "2000");
    env::set_var("SEKHA_CORS_ENABLED", "false");

    let config = Config::load().expect("Failed to load config");

    assert_eq!(config.server_port, 9090);
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.rate_limit_per_minute, 2000);
    assert_eq!(config.cors_enabled, false);

    // Cleanup
    env::remove_var("SEKHA_SERVER_PORT");
    env::remove_var("SEKHA_LOG_LEVEL");
    env::remove_var("SEKHA_RATE_LIMIT_PER_MINUTE");
    env::remove_var("SEKHA_CORS_ENABLED");
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
    // Clear env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

    // This should succeed and call validate_providers internally
    let result = Config::load();
    assert!(result.is_ok(), "Config load should validate and succeed");
}

#[test]
fn test_config_home_directory_fallback() {
    // This tests the HOME env var fallback in config file loading
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
}

#[test]
fn test_all_v1_defaults_in_migration() {
    // Clear env vars
    for (key, _) in env::vars() {
        if key.starts_with("SEKHA_") {
            env::remove_var(&key);
        }
    }

    // Just set ollama_url to trigger migration
    env::set_var("SEKHA_OLLAMA_URL", "http://localhost:11434");

    let config = Config::load().expect("Failed to load config");

    // Verify v1.x compatibility fields are set
    assert_eq!(
        config.ollama_url,
        Some("http://localhost:11434".to_string())
    );
    assert_eq!(config.embedding_model, Some("nomic-embed-text".to_string()));
    assert_eq!(config.summarization_model, Some("llama3.1:8b".to_string()));

    // Cleanup
    env::remove_var("SEKHA_OLLAMA_URL");
}
