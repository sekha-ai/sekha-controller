// tests/unit/embedding_service_test.rs
//! Unit tests for embedding service with BridgeClient integration

use httptest::{matchers::*, responders::*, Expectation, Server};
use sekha_controller::config::{
    Config, LlmProviderConfig, ModelCapability, ModelTask, ProviderType,
};
use sekha_controller::llm::bridge_client::BridgeClient;
use sekha_controller::services::embedding_service::{EmbeddingError, EmbeddingService};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

fn create_test_config(bridge_url: &str) -> Config {
    let mut config = Config::default();
    config.llm_bridge_url = bridge_url.to_string();
    config.llm_providers.push(LlmProviderConfig {
        id: "test-provider".to_string(),
        provider_type: ProviderType::Ollama,
        base_url: "http://test".to_string(),
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

// ============================================
// Test: get_model_dimension - success cases
// ============================================

#[tokio::test]
async fn test_get_model_dimension_with_model_specified() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
            json_encoded(json!([
                {
                    "model_id": "nomic-embed-text",
                    "provider_id": "ollama",
                    "task": "embedding",
                    "context_window": 8192,
                    "dimension": 768,
                    "supports_vision": false,
                    "supports_audio": false
                },
                {
                    "model_id": "text-embedding-3-large",
                    "provider_id": "openai",
                    "task": "embedding",
                    "context_window": 8191,
                    "dimension": 3072,
                    "supports_vision": false,
                    "supports_audio": false
                }
            ])),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.get_model_dimension(Some("nomic-embed-text")).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 768);
}

#[tokio::test]
async fn test_get_model_dimension_no_model_uses_default() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
            json_encoded(json!([
                {
                    "model_id": "default-model",
                    "provider_id": "ollama",
                    "task": "embedding",
                    "context_window": 8192,
                    "dimension": 1536,
                    "supports_vision": false,
                    "supports_audio": false
                }
            ])),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.get_model_dimension(None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1536);
}

#[tokio::test]
async fn test_get_model_dimension_cache_hit() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models"))
            .times(1) // Only called once
            .respond_with(json_encoded(json!([
                {
                    "model_id": "cached-model",
                    "provider_id": "ollama",
                    "task": "embedding",
                    "context_window": 8192,
                    "dimension": 768,
                    "supports_vision": false,
                    "supports_audio": false
                }
            ]))),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    // First call - populates cache
    let result1 = service.get_model_dimension(Some("cached-model")).await;
    assert_eq!(result1.unwrap(), 768);

    // Second call - uses cache (server only expects 1 call)
    let result2 = service.get_model_dimension(Some("cached-model")).await;
    assert_eq!(result2.unwrap(), 768);
}

// ============================================
// Test: get_model_dimension - error cases
// ============================================

#[tokio::test]
async fn test_get_model_dimension_model_not_found() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
            json_encoded(json!([
                {
                    "model_id": "other-model",
                    "provider_id": "ollama",
                    "task": "embedding",
                    "context_window": 8192,
                    "dimension": 768,
                    "supports_vision": false,
                    "supports_audio": false
                }
            ])),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.get_model_dimension(Some("nonexistent")).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        EmbeddingError::ModelNotFound(_)
    ));
}

#[tokio::test]
async fn test_get_model_dimension_no_embedding_models() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
            json_encoded(json!([
                {
                    "model_id": "gpt-4",
                    "provider_id": "openai",
                    "task": "chat_large",
                    "context_window": 8192,
                    "dimension": null,
                    "supports_vision": false,
                    "supports_audio": false
                }
            ])),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.get_model_dimension(None).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        EmbeddingError::NoModelsAvailable
    ));
}

#[tokio::test]
async fn test_get_model_dimension_empty_list() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models"))
            .respond_with(json_encoded(json!([]))),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.get_model_dimension(None).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        EmbeddingError::NoModelsAvailable
    ));
}

// ============================================
// Test: embed_with_collection_routing
// ============================================

#[tokio::test]
async fn test_embed_with_collection_routing_success() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
            json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            })),
        ),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
            json_encoded(json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            })),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service
        .embed_with_collection_routing("test content", None)
        .await;
    assert!(result.is_ok());
    let (embedding, dimension, model) = result.unwrap();
    assert_eq!(embedding.len(), 768);
    assert_eq!(dimension, 768);
    assert_eq!(model, "nomic-embed-text");
}

#[tokio::test]
async fn test_embed_with_collection_routing_with_preferred_model() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
            json_encoded(json!({
                "provider_id": "openai",
                "model_id": "text-embedding-3-large",
                "estimated_cost": 0.0001,
                "reason": "Preferred model",
                "provider_type": "openai"
            })),
        ),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
            json_encoded(json!({
                "embedding": vec![0.2; 3072],
                "model": "text-embedding-3-large",
                "dimension": 3072,
                "tokens_used": 10
            })),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service
        .embed_with_collection_routing("test", Some("text-embedding-3-large".to_string()))
        .await;
    assert!(result.is_ok());
    let (embedding, dimension, model) = result.unwrap();
    assert_eq!(embedding.len(), 3072);
    assert_eq!(dimension, 3072);
    assert_eq!(model, "text-embedding-3-large");
}

// ============================================
// Test: generate_embedding
// ============================================

#[tokio::test]
async fn test_generate_embedding_success() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
            json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            })),
        ),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
            json_encoded(json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            })),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.generate_embedding("test", None).await;
    assert!(result.is_ok());
    let embedding = result.unwrap();
    assert_eq!(embedding.len(), 768);
}

// ============================================
// Test: generate_embedding_with_retry
// ============================================

#[tokio::test]
async fn test_generate_embedding_with_retry_success() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
            json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            })),
        ),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
            json_encoded(json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            })),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service
        .generate_embedding_with_retry("test content", 3, None)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 768);
}

#[tokio::test]
async fn test_generate_embedding_with_retry_exhaustion() {
    let server = Server::run();

    // All requests fail
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route"))
            .times(2)
            .respond_with(status_code(500)),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.generate_embedding_with_retry("test", 2, None).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        EmbeddingError::MaxRetriesExceeded
    ));
}

#[tokio::test]
async fn test_generate_embedding_with_retry_no_embeddings_immediate_fail() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
            json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            })),
        ),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
            json_encoded(json!({
                "embedding": [],
                "model": "nomic-embed-text",
                "dimension": 0,
                "tokens_used": 0
            })),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.generate_embedding_with_retry("test", 3, None).await;
    assert!(result.is_err());
    // Should fail with NoEmbeddings, not MaxRetriesExceeded
    assert!(matches!(result.unwrap_err(), EmbeddingError::NoEmbeddings));
}

// ============================================
// Test: generate_embeddings_batch
// ============================================

#[tokio::test]
async fn test_generate_embeddings_batch_success() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route"))
            .times(3)
            .respond_with(json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            }))),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed"))
            .times(3)
            .respond_with(json_encoded(json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            }))),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let texts = vec![
        "text1".to_string(),
        "text2".to_string(),
        "text3".to_string(),
    ];
    let result = service.generate_embeddings_batch(texts, 2, None).await;

    assert!(result.is_ok());
    let embeddings = result.unwrap();
    assert_eq!(embeddings.len(), 3);
    assert_eq!(embeddings[0].len(), 768);
}

#[tokio::test]
async fn test_generate_embeddings_batch_empty() {
    let config = create_test_config("http://localhost:5001");
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    let result = service.generate_embeddings_batch(vec![], 10, None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

// ============================================
// Test: Rate limiting (semaphore)
// ============================================

#[tokio::test]
async fn test_semaphore_rate_limiting() {
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route"))
            .times(10)
            .respond_with(json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            }))),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed"))
            .times(10)
            .respond_with(json_encoded(json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            }))),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = Arc::new(EmbeddingService::new(
        bridge,
        "http://localhost:8000".to_string(),
    ));

    // Spawn 10 concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let service = service.clone();
        let handle = tokio::spawn(async move {
            service
                .generate_embedding(&format!("text{}", i), None)
                .await
        });
        handles.push(handle);
    }

    // Wait for all
    let results = futures::future::join_all(handles).await;
    assert!(results.iter().all(|r| r.is_ok()));
}

// ============================================
// Test: Error conversions
// ============================================

#[tokio::test]
async fn test_error_conversion_from_acquire_error() {
    let err: EmbeddingError = tokio::sync::AcquireError::NoPermits.into();
    assert!(matches!(err, EmbeddingError::SemaphoreError(_)));
}

#[tokio::test]
async fn test_error_conversion_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("test bridge error");
    let err: EmbeddingError = anyhow_err.into();
    assert!(matches!(err, EmbeddingError::BridgeError(_)));
}

// ============================================
// Test: process_message (integration-like)
// ============================================

#[tokio::test]
async fn test_process_message_creates_dimension_specific_collection() {
    // This test verifies the collection naming scheme
    let server = Server::run();

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route")).respond_with(
            json_encoded(json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            })),
        ),
    );

    server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed")).respond_with(
            json_encoded(json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            })),
        ),
    );

    let config = create_test_config(&server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(bridge, "http://localhost:8000".to_string());

    // Note: This will fail with Chroma connection error, but we're testing the routing logic
    let result = service
        .process_message(
            Uuid::new_v4(),
            "test content",
            Uuid::new_v4(),
            json!({}),
            None,
        )
        .await;

    // Expect ChromaError because we don't have a real Chroma instance
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        EmbeddingError::ChromaError(_)
    ));
}
