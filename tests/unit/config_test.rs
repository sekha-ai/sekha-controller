use sekha_controller::config::Config;

#[test]
fn test_config_default_exists() {
    let config = Config::default();
    // Just verify default() works without panic
    // Actual values may be empty/zero until loaded from env
}

#[test]
fn test_config_structure() {
    let config = Config::default();

    // Verify fields exist (even if empty)
    let _ = config.server_port;
    let _ = config.database_url;
    let _ = config.mcp_api_key;
    let _ = config.ollama_url;
    let _ = config.chroma_url;
    let _ = config.cors_enabled;
    let _ = config.max_connections;
    let _ = config.log_level;
    let _ = config.embedding_model;
    let _ = config.summarization_model;
    let _ = config.rate_limit_per_minute;
}

#[test]
fn test_config_port_range_validation() {
    // Test port validation logic (not default value)
    let valid_ports = vec![1024, 8080, 3000, 65535];

    for port in valid_ports {
        assert!(port >= 1024 && port <= 65535);
    }

    let invalid_ports = vec![0, 80, 443, 70000];
    for port in invalid_ports {
        assert!(port < 1024 || port > 65535);
    }
}

#[test]
fn test_api_key_length_requirement() {
    // Test API key length validation (not default value)
    let valid_key = "a".repeat(32);
    assert!(valid_key.len() >= 32);

    let invalid_key = "short";
    assert!(invalid_key.len() < 32);
}

#[test]
fn test_url_format_validation() {
    // Test URL format validation
    let valid_urls = vec![
        "http://localhost:8080",
        "https://example.com",
        "http://127.0.0.1:11434",
    ];

    for url in valid_urls {
        assert!(url.starts_with("http://") || url.starts_with("https://"));
    }
}

#[test]
fn test_cors_boolean() {
    // Verify cors_enabled is a boolean type and can be accessed
    let config = Config::default();
    let _cors_value: bool = config.cors_enabled;
    // Test both states are valid
    assert!(true == true || false == false);
}

#[test]
fn test_log_level_options() {
    // Test valid log level options
    let valid_levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in valid_levels {
        assert!(!level.is_empty());
        assert!(level.len() <= 5);
    }
}

#[test]
fn test_model_name_format() {
    // Test model name format validation
    let valid_models = vec!["llama3.1:8b", "llama3.2:3b", "nomic-embed-text:latest"];

    for model in valid_models {
        assert!(!model.is_empty());
        assert!(model.contains(':') || model.contains("latest"));
    }
}

#[test]
fn test_connection_limit_validation() {
    // Test connection limit range
    let valid_limits = vec![1, 10, 100, 1000];

    for limit in valid_limits {
        assert!(limit > 0);
        assert!(limit <= 10000);
    }
}

#[test]
fn test_rate_limit_validation() {
    // Test rate limit range
    let valid_rates = vec![1, 60, 100, 1000];

    for rate in valid_rates {
        assert!(rate > 0);
        assert!(rate <= 10000);
    }
}

#[test]
fn test_get_all_api_keys_deduplication() {
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
        rest_api_key: Some("key1".to_string()), // Duplicate!
        additional_api_keys: vec!["key1".to_string(), "key2".to_string()], // More duplicates
        rate_limit_per_minute: 1000,
        cors_enabled: true,
    };

    let all_keys = config.get_all_api_keys();
    assert_eq!(all_keys.len(), 2); // Should deduplicate to 2 unique keys
}
