use sekha_controller::config::*;
use std::collections::HashSet;

#[test]
fn test_provider_type_all_variants() {
    // Test all ProviderType variants
    let ollama_json = r#""ollama""#;
    let ollama: ProviderType = serde_json::from_str(ollama_json).unwrap();
    assert_eq!(ollama, ProviderType::Ollama);

    let litellm_json = r#""litellm""#;
    let litellm: ProviderType = serde_json::from_str(litellm_json).unwrap();
    assert_eq!(litellm, ProviderType::LiteLlm);

    let openrouter_json = r#""openrouter""#;
    let openrouter: ProviderType = serde_json::from_str(openrouter_json).unwrap();
    assert_eq!(openrouter, ProviderType::OpenRouter);

    let openai_json = r#""openai""#;
    let openai: ProviderType = serde_json::from_str(openai_json).unwrap();
    assert_eq!(openai, ProviderType::OpenAi);

    let anthropic_json = r#""anthropic""#;
    let anthropic: ProviderType = serde_json::from_str(anthropic_json).unwrap();
    assert_eq!(anthropic, ProviderType::Anthropic);
}

#[test]
fn test_model_task_all_variants() {
    // Test all ModelTask variants
    let tasks = vec![
        (r#""embedding""#, ModelTask::Embedding),
        (r#""chat_small""#, ModelTask::ChatSmall),
        (r#""chat_large""#, ModelTask::ChatLarge),
        (r#""chat_smart""#, ModelTask::ChatSmart),
        (r#""vision""#, ModelTask::Vision),
        (r#""audio""#, ModelTask::Audio),
    ];

    for (json, expected) in tasks {
        let task: ModelTask = serde_json::from_str(json).unwrap();
        assert_eq!(task, expected);
    }
}

#[test]
fn test_model_capability_with_vision() {
    let cap = ModelCapability {
        model_id: "gpt-4-vision".to_string(),
        task: ModelTask::Vision,
        context_window: 128000,
        supports_vision: true,
        supports_audio: false,
        dimension: None,
    };

    assert!(cap.supports_vision);
    assert!(!cap.supports_audio);
    assert_eq!(cap.context_window, 128000);
}

#[test]
fn test_model_capability_with_audio() {
    let cap = ModelCapability {
        model_id: "whisper-1".to_string(),
        task: ModelTask::Audio,
        context_window: 25000,
        supports_vision: false,
        supports_audio: true,
        dimension: None,
    };

    assert!(cap.supports_audio);
    assert!(!cap.supports_vision);
}

#[test]
fn test_model_capability_embedding_with_dimension() {
    let cap = ModelCapability {
        model_id: "nomic-embed-text".to_string(),
        task: ModelTask::Embedding,
        context_window: 8192,
        supports_vision: false,
        supports_audio: false,
        dimension: Some(768),
    };

    assert_eq!(cap.dimension, Some(768));
    assert_eq!(cap.task, ModelTask::Embedding);
}

#[test]
fn test_llm_provider_config_with_api_key() {
    let provider = LlmProviderConfig {
        id: "openai_cloud".to_string(),
        provider_type: ProviderType::OpenAi,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test123".to_string()),
        timeout_secs: 60,
        priority: 1,
        models: vec![],
    };

    assert!(provider.api_key.is_some());
    assert_eq!(provider.priority, 1);
}

#[test]
fn test_llm_provider_config_without_api_key() {
    let provider = LlmProviderConfig {
        id: "ollama_local".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 2,
        models: vec![],
    };

    assert!(provider.api_key.is_none());
    assert_eq!(provider.timeout_secs, 120);
}

#[test]
fn test_default_models_all_fields() {
    let defaults = DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:70b".to_string(),
        chat_vision: Some("gpt-4-vision".to_string()),
    };

    assert!(defaults.chat_vision.is_some());
    assert_eq!(defaults.embedding, "nomic-embed-text");
}

#[test]
fn test_default_models_no_vision() {
    let defaults = DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:70b".to_string(),
        chat_vision: None,
    };

    assert!(defaults.chat_vision.is_none());
}

#[test]
fn test_routing_config_default() {
    let routing = RoutingConfig::default();

    assert_eq!(routing.auto_fallback, true);
    assert_eq!(routing.require_vision_for_images, true);
    assert!(routing.max_cost_per_request.is_none());
    assert_eq!(routing.circuit_breaker.failure_threshold, 3);
}

#[test]
fn test_routing_config_with_cost_limit() {
    let routing = RoutingConfig {
        auto_fallback: true,
        require_vision_for_images: false,
        max_cost_per_request: Some(0.50),
        circuit_breaker: CircuitBreakerConfig::default(),
    };

    assert_eq!(routing.max_cost_per_request, Some(0.50));
    assert_eq!(routing.require_vision_for_images, false);
}

#[test]
fn test_circuit_breaker_default() {
    let cb = CircuitBreakerConfig::default();

    assert_eq!(cb.failure_threshold, 3);
    assert_eq!(cb.timeout_secs, 60);
    assert_eq!(cb.success_threshold, 2);
}

#[test]
fn test_circuit_breaker_custom_values() {
    let cb = CircuitBreakerConfig {
        failure_threshold: 5,
        timeout_secs: 120,
        success_threshold: 3,
    };

    assert_eq!(cb.failure_threshold, 5);
    assert_eq!(cb.timeout_secs, 120);
    assert_eq!(cb.success_threshold, 3);
}

#[test]
fn test_config_default() {
    let config = Config::default();

    assert_eq!(config.server_host, "0.0.0.0");
    assert_eq!(config.server_port, 8080);
    assert!(config.mcp_api_key.len() >= 32);
    assert_eq!(config.database_url, "sqlite://sekha.db");
    assert_eq!(config.chroma_url, "http://localhost:8000");
    assert_eq!(config.llm_bridge_url, "http://localhost:5001");
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.log_level, "info");
    assert_eq!(config.summarization_enabled, true);
    assert_eq!(config.pruning_enabled, true);
    assert!(config.rest_api_key.is_none());
    assert_eq!(config.additional_api_keys.len(), 0);
    assert_eq!(config.rate_limit_per_minute, 1000);
    assert_eq!(config.cors_enabled, true);
    assert_eq!(config.config_version, Some("2.0".to_string()));
    assert_eq!(config.llm_providers.len(), 0);
    assert!(config.default_models.is_none());
    assert_eq!(config.ollama_url, Some("http://localhost:11434".to_string()));
}

#[test]
fn test_config_validate_providers_empty_providers() {
    let config = Config::default();
    // Empty providers is valid (falls back to v1.x config)
    assert!(config.validate_providers().is_ok());
}

#[test]
fn test_config_validate_providers_missing_default_models() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "test".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![],
    });
    config.default_models = None;

    let result = config.validate_providers();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("default_models must be specified"));
}

#[test]
fn test_config_validate_providers_embedding_model_not_found() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "test".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![ModelCapability {
            model_id: "llama3.1:8b".to_string(),
            task: ModelTask::ChatSmall,
            context_window: 8192,
            supports_vision: false,
            supports_audio: false,
            dimension: None,
        }],
    });
    config.default_models = Some(DefaultModels {
        embedding: "nonexistent-model".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:8b".to_string(),
        chat_vision: None,
    });

    let result = config.validate_providers();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Default embedding model"));
}

#[test]
fn test_config_validate_providers_chat_fast_not_found() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "test".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![ModelCapability {
            model_id: "nomic-embed-text".to_string(),
            task: ModelTask::Embedding,
            context_window: 512,
            supports_vision: false,
            supports_audio: false,
            dimension: Some(768),
        }],
    });
    config.default_models = Some(DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "nonexistent-chat".to_string(),
        chat_smart: "nomic-embed-text".to_string(),
        chat_vision: None,
    });

    let result = config.validate_providers();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Default chat_fast model"));
}

#[test]
fn test_config_validate_providers_chat_smart_not_found() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "test".to_string(),
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
        ],
    });
    config.default_models = Some(DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "nonexistent-smart".to_string(),
        chat_vision: None,
    });

    let result = config.validate_providers();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Default chat_smart model"));
}

#[test]
fn test_config_validate_providers_duplicate_ids() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "duplicate".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 1,
        models: vec![ModelCapability {
            model_id: "model1".to_string(),
            task: ModelTask::Embedding,
            context_window: 512,
            supports_vision: false,
            supports_audio: false,
            dimension: Some(768),
        }],
    });
    config.llm_providers.push(LlmProviderConfig {
        id: "duplicate".to_string(), // Same ID!
        provider_type: ProviderType::OpenAi,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test".to_string()),
        timeout_secs: 60,
        priority: 2,
        models: vec![],
    });
    config.default_models = Some(DefaultModels {
        embedding: "model1".to_string(),
        chat_fast: "model1".to_string(),
        chat_smart: "model1".to_string(),
        chat_vision: None,
    });

    let result = config.validate_providers();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Duplicate provider ID"));
}

#[test]
fn test_config_validate_providers_valid() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "test".to_string(),
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
        ],
    });
    config.default_models = Some(DefaultModels {
        embedding: "nomic-embed-text".to_string(),
        chat_fast: "llama3.1:8b".to_string(),
        chat_smart: "llama3.1:8b".to_string(),
        chat_vision: None,
    });

    assert!(config.validate_providers().is_ok());
}

#[test]
fn test_config_get_provider_for_task_embedding() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "provider1".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 2,
        models: vec![ModelCapability {
            model_id: "nomic-embed-text".to_string(),
            task: ModelTask::Embedding,
            context_window: 512,
            supports_vision: false,
            supports_audio: false,
            dimension: Some(768),
        }],
    });

    let provider = config.get_provider_for_task(&ModelTask::Embedding);
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().id, "provider1");
}

#[test]
fn test_config_get_provider_for_task_priority_sorting() {
    let mut config = Config::default();
    config.llm_providers.push(LlmProviderConfig {
        id: "low_priority".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        api_key: None,
        timeout_secs: 120,
        priority: 10, // Lower priority
        models: vec![ModelCapability {
            model_id: "model1".to_string(),
            task: ModelTask::ChatSmall,
            context_window: 8192,
            supports_vision: false,
            supports_audio: false,
            dimension: None,
        }],
    });
    config.llm_providers.push(LlmProviderConfig {
        id: "high_priority".to_string(),
        provider_type: ProviderType::OpenAi,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test".to_string()),
        timeout_secs: 60,
        priority: 1, // Higher priority
        models: vec![ModelCapability {
            model_id: "gpt-4".to_string(),
            task: ModelTask::ChatSmall,
            context_window: 8192,
            supports_vision: false,
            supports_audio: false,
            dimension: None,
        }],
    });

    let provider = config.get_provider_for_task(&ModelTask::ChatSmall);
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().id, "high_priority");
}

#[test]
fn test_config_get_provider_for_task_not_found() {
    let config = Config::default();
    let provider = config.get_provider_for_task(&ModelTask::Vision);
    assert!(provider.is_none());
}

#[test]
fn test_config_get_rest_api_key_with_explicit_key() {
    let mut config = Config::default();
    config.rest_api_key = Some("explicit_rest_key".to_string());

    assert_eq!(config.get_rest_api_key(), "explicit_rest_key");
}

#[test]
fn test_config_get_rest_api_key_fallback_to_mcp() {
    let config = Config::default();
    assert_eq!(config.get_rest_api_key(), config.mcp_api_key);
}

#[test]
fn test_config_get_all_api_keys_unique() {
    let mut config = Config::default();
    config.mcp_api_key = "key1".to_string();
    config.rest_api_key = Some("key2".to_string());
    config.additional_api_keys = vec!["key3".to_string(), "key4".to_string()];

    let keys = config.get_all_api_keys();
    assert_eq!(keys.len(), 4);
    assert!(keys.contains(&"key1".to_string()));
    assert!(keys.contains(&"key2".to_string()));
    assert!(keys.contains(&"key3".to_string()));
    assert!(keys.contains(&"key4".to_string()));
}

#[test]
fn test_config_get_all_api_keys_deduplication() {
    let mut config = Config::default();
    config.mcp_api_key = "duplicate_key".to_string();
    config.rest_api_key = Some("duplicate_key".to_string());
    config.additional_api_keys = vec!["duplicate_key".to_string(), "unique_key".to_string()];

    let keys = config.get_all_api_keys();
    assert_eq!(keys.len(), 2); // Should deduplicate
    assert!(keys.contains(&"duplicate_key".to_string()));
    assert!(keys.contains(&"unique_key".to_string()));
}

#[test]
fn test_config_is_valid_api_key_true() {
    let mut config = Config::default();
    config.mcp_api_key = "valid_key_123".to_string();
    config.additional_api_keys = vec!["another_valid_key".to_string()];

    assert!(config.is_valid_api_key("valid_key_123"));
    assert!(config.is_valid_api_key("another_valid_key"));
}

#[test]
fn test_config_is_valid_api_key_false() {
    let config = Config::default();
    assert!(!config.is_valid_api_key("invalid_key"));
}

#[test]
fn test_default_rate_limit() {
    assert_eq!(default_rate_limit(), 1000);
}

#[test]
fn test_default_cors_enabled() {
    assert_eq!(default_cors_enabled(), true);
}

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 120);
}

#[test]
fn test_default_auto_fallback() {
    assert_eq!(default_auto_fallback(), true);
}

#[test]
fn test_default_require_vision() {
    assert_eq!(default_require_vision(), true);
}

#[test]
fn test_default_failure_threshold() {
    assert_eq!(default_failure_threshold(), 3);
}

#[test]
fn test_default_timeout_secs() {
    assert_eq!(default_timeout_secs(), 60);
}

#[test]
fn test_default_success_threshold() {
    assert_eq!(default_success_threshold(), 2);
}

#[test]
fn test_provider_type_clone() {
    let provider = ProviderType::Ollama;
    let cloned = provider.clone();
    assert_eq!(provider, cloned);
}

#[test]
fn test_model_task_clone() {
    let task = ModelTask::ChatSmall;
    let cloned = task.clone();
    assert_eq!(task, cloned);
}

#[test]
fn test_model_capability_clone() {
    let cap = ModelCapability {
        model_id: "test".to_string(),
        task: ModelTask::Embedding,
        context_window: 512,
        supports_vision: false,
        supports_audio: false,
        dimension: Some(768),
    };

    let cloned = cap.clone();
    assert_eq!(cap.model_id, cloned.model_id);
    assert_eq!(cap.dimension, cloned.dimension);
}

#[test]
fn test_reloadable_config_clone() {
    let reloadable = ReloadableConfig {
        summarization_enabled: true,
        pruning_enabled: false,
        log_level: "debug".to_string(),
    };

    let cloned = reloadable.clone();
    assert_eq!(reloadable.summarization_enabled, cloned.summarization_enabled);
    assert_eq!(reloadable.pruning_enabled, cloned.pruning_enabled);
    assert_eq!(reloadable.log_level, cloned.log_level);
}
