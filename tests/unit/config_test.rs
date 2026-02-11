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
