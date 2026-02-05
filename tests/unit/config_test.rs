use sekha_controller::config::*;
use validator::Validate;

#[test]
fn test_config_validation() {
    let config = Config::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_has_required_fields() {
    let config = Config::default();
    assert!(!config.mcp_api_key.is_empty());
    assert!(!config.database_url.is_empty());
    assert!(!config.chroma_url.is_empty());
}

#[test]
fn test_provider_type_serialization() {
    let provider = ProviderType::Ollama;
    let json = serde_json::to_string(&provider).unwrap();
    assert_eq!(json, "\"ollama\"");

    let provider = ProviderType::OpenAi;
    let json = serde_json::to_string(&provider).unwrap();
    assert_eq!(json, "\"openai\"");
}

#[test]
fn test_model_task_serialization() {
    let task = ModelTask::Embedding;
    let json = serde_json::to_string(&task).unwrap();
    assert_eq!(json, "\"embedding\"");

    let task = ModelTask::ChatSmall;
    let json = serde_json::to_string(&task).unwrap();
    assert_eq!(json, "\"chat_small\"");
}

#[test]
fn test_circuit_breaker_config_defaults() {
    let cb = CircuitBreakerConfig::default();
    assert_eq!(cb.failure_threshold, 3);
    assert_eq!(cb.timeout_secs, 60);
    assert_eq!(cb.success_threshold, 2);
}

#[test]
fn test_routing_config_defaults() {
    let routing = RoutingConfig::default();
    assert!(routing.auto_fallback);
    assert!(routing.require_vision_for_images);
    assert!(routing.max_cost_per_request.is_none());
}

#[test]
fn test_config_default_values() {
    let config = Config::default();
    assert_eq!(config.server_host, "0.0.0.0");
    assert_eq!(config.server_port, 8080);
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.log_level, "info");
    assert!(config.summarization_enabled);
    assert!(config.pruning_enabled);
    assert_eq!(config.rate_limit_per_minute, 1000);
    assert!(config.cors_enabled);
}

#[test]
fn test_provider_config_creation() {
    let provider = LlmProviderConfig {
        id: "test_provider".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: Some("test_key".to_string()),
        timeout_secs: 120,
        priority: 1,
        models: vec![],
    };

    assert_eq!(provider.id, "test_provider");
    assert_eq!(provider.priority, 1);
    assert_eq!(provider.timeout_secs, 120);
}

#[test]
fn test_model_capability_with_embedding() {
    let model = ModelCapability {
        model_id: "nomic-embed-text".to_string(),
        task: ModelTask::Embedding,
        context_window: 512,
        supports_vision: false,
        supports_audio: false,
        dimension: Some(768),
    };

    assert_eq!(model.task, ModelTask::Embedding);
    assert_eq!(model.dimension, Some(768));
    assert!(!model.supports_vision);
}

#[test]
fn test_default_models_configuration() {
    let defaults = DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:70b".to_string(),
        chat_vision: Some("llava".to_string()),
    };

    assert_eq!(defaults.embedding, "nomic-embed-text");
    assert_eq!(defaults.chat_fast, "llama3.1:8b");
    assert!(defaults.chat_vision.is_some());
}

#[test]
fn test_config_validate_providers_success() {
    let mut config = Config::default();

    // Add provider with models
    config.llm_providers = vec![LlmProviderConfig {
        id: "ollama".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![
            ModelCapability {
                model_id: "nomic-embed-text".to_string(),
                task: ModelTask::Embedding,
                context_window: 512,
                supports_vision: false,
                supports_audio: false,
                dimension: Some(768),
            },
            ModelCapability {
                model_id: "llama3.1:8b".to_string(),
                task: ModelTask::ChatSmall,
                context_window: 8192,
                supports_vision: false,
                supports_audio: false,
                dimension: None,
            },
            ModelCapability {
                model_id: "llama3.1:70b".to_string(),
                task: ModelTask::ChatSmart,
                context_window: 8192,
                supports_vision: false,
                supports_audio: false,
                dimension: None,
            },
        ],
    }];

    config.default_models = Some(DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:70b".to_string(),
        chat_vision: None,
    });

    assert!(config.validate_providers().is_ok());
}

#[test]
fn test_config_validate_providers_missing_defaults() {
    let mut config = Config::default();

    config.llm_providers = vec![LlmProviderConfig {
        id: "ollama".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![],
    }];

    // Missing default_models
    let result = config.validate_providers();
    assert!(result.is_err());
}

#[test]
fn test_config_validate_providers_duplicate_ids() {
    let mut config = Config::default();

    config.llm_providers = vec![
        LlmProviderConfig {
            id: "ollama".to_string(),
            provider_type: ProviderType::Ollama,
            base_url: "http://localhost:11434".to_string(),
            api_key: None,
            timeout_secs: 120,
            priority: 1,
            models: vec![],
        },
        LlmProviderConfig {
            id: "ollama".to_string(), // Duplicate!
            provider_type: ProviderType::Ollama,
            base_url: "http://localhost:11435".to_string(),
            api_key: None,
            timeout_secs: 120,
            priority: 2,
            models: vec![],
        },
    ];

    config.default_models = Some(DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:70b".to_string(),
        chat_vision: None,
    });

    let result = config.validate_providers();
    assert!(result.is_err());
}

#[test]
fn test_get_provider_for_task() {
    let mut config = Config::default();

    config.llm_providers = vec![
        LlmProviderConfig {
            id: "ollama_low".to_string(),
            provider_type: ProviderType::Ollama,
            base_url: "http://localhost:11434".to_string(),
            api_key: None,
            timeout_secs: 120,
            priority: 2, // Lower priority
            models: vec![ModelCapability {
                model_id: "nomic-embed-text".to_string(),
                task: ModelTask::Embedding,
                context_window: 512,
                supports_vision: false,
                supports_audio: false,
                dimension: Some(768),
            }],
        },
        LlmProviderConfig {
            id: "ollama_high".to_string(),
            provider_type: ProviderType::Ollama,
            base_url: "http://localhost:11435".to_string(),
            api_key: None,
            timeout_secs: 120,
            priority: 1, // Higher priority
            models: vec![ModelCapability {
                model_id: "another-embed".to_string(),
                task: ModelTask::Embedding,
                context_window: 512,
                supports_vision: false,
                supports_audio: false,
                dimension: Some(768),
            }],
        },
    ];

    let provider = config.get_provider_for_task(&ModelTask::Embedding);
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().id, "ollama_high"); // Should pick higher priority
}

#[test]
fn test_get_rest_api_key_with_explicit_key() {
    let mut config = Config::default();
    config.rest_api_key = Some("explicit_key".to_string());

    assert_eq!(config.get_rest_api_key(), "explicit_key");
}

#[test]
fn test_get_rest_api_key_fallback_to_mcp() {
    let config = Config::default();
    // rest_api_key is None, should fallback to mcp_api_key
    assert_eq!(config.get_rest_api_key(), config.mcp_api_key);
}

#[test]
fn test_get_all_api_keys() {
    let mut config = Config::default();
    config.rest_api_key = Some("rest_key".to_string());
    config.additional_api_keys = vec!["key1".to_string(), "key2".to_string()];

    let keys = config.get_all_api_keys();

    // Should contain mcp_api_key, rest_api_key, and additional keys
    assert!(keys.contains(&config.mcp_api_key));
    assert!(keys.contains(&"rest_key".to_string()));
    assert!(keys.contains(&"key1".to_string()));
    assert!(keys.contains(&"key2".to_string()));

    // Should be deduplicated
    let unique_count = keys.len();
    assert_eq!(unique_count, 4);
}

#[test]
fn test_is_valid_api_key() {
    let mut config = Config::default();
    config.additional_api_keys = vec!["valid_key".to_string()];

    assert!(config.is_valid_api_key(&config.mcp_api_key));
    assert!(config.is_valid_api_key("valid_key"));
    assert!(!config.is_valid_api_key("invalid_key"));
}

#[test]
fn test_config_port_validation_low() {
    let mut config = Config::default();
    config.server_port = 80; // Below 1024

    // Should fail validation
    assert!(config.validate().is_err());
}

#[test]
fn test_config_port_validation_valid() {
    let mut config = Config::default();
    config.server_port = 8080; // Valid
    assert!(config.validate().is_ok());

    config.server_port = 65535; // Max valid u16
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_api_key_length_validation() {
    let mut config = Config::default();
    config.mcp_api_key = "short".to_string(); // Too short (< 32 chars)

    assert!(config.validate().is_err());

    config.mcp_api_key = "a".repeat(32); // Minimum length
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_max_connections_validation() {
    let mut config = Config::default();
    config.max_connections = 0; // Invalid

    assert!(config.validate().is_err());

    config.max_connections = 50; // Valid
    assert!(config.validate().is_ok());

    config.max_connections = 101; // Above 100
    assert!(config.validate().is_err());
}

#[test]
fn test_model_capability_vision_and_audio() {
    let model = ModelCapability {
        model_id: "gpt-4-vision".to_string(),
        task: ModelTask::Vision,
        context_window: 4096,
        supports_vision: true,
        supports_audio: false,
        dimension: None,
    };

    assert!(model.supports_vision);
    assert!(!model.supports_audio);
    assert_eq!(model.task, ModelTask::Vision);
}

#[test]
fn test_provider_with_api_key() {
    let provider = LlmProviderConfig {
        id: "openai".to_string(),
        provider_type: ProviderType::OpenAi,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test123".to_string()),
        timeout_secs: 60,
        priority: 1,
        models: vec![],
    };

    assert!(provider.api_key.is_some());
    assert_eq!(provider.api_key.unwrap(), "sk-test123");
}

#[test]
fn test_routing_config_with_cost_limit() {
    let routing = RoutingConfig {
        auto_fallback: true,
        require_vision_for_images: false,
        max_cost_per_request: Some(0.05),
        circuit_breaker: CircuitBreakerConfig::default(),
    };

    assert!(routing.max_cost_per_request.is_some());
    assert_eq!(routing.max_cost_per_request.unwrap(), 0.05);
}

#[test]
fn test_reloadable_config() {
    let config = Config::default();

    let reloadable = ReloadableConfig {
        summarization_enabled: config.summarization_enabled,
        pruning_enabled: config.pruning_enabled,
        log_level: config.log_level.clone(),
    };

    assert!(reloadable.summarization_enabled);
    assert!(reloadable.pruning_enabled);
    assert_eq!(reloadable.log_level, "info");
}
