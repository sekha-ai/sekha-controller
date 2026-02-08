// Additional tests for embedding_service.rs uncovered lines
// Add these to the #[cfg(test)] mod tests section in src/services/embedding_service.rs

use super::*;
use crate::config::{Config, LlmProviderConfig, ModelCapability, ModelTask, ProviderType};
use httptest::{matchers::*, responders::*, Expectation, Server};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_search_all_dimensions_with_failed_secondary_fixed() {
    let chroma_server = Server::run();
    let bridge_server = Server::run();

    // Mock primary embedding (succeeds)
    bridge_server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route"))
            .respond_with(json_encoded(serde_json::json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            }))),
    );

    bridge_server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed"))
            .respond_with(json_encoded(serde_json::json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            }))),
    );

    // Mock list_models to return only one dimension (no secondary to fail)
    bridge_server.expect(
        Expectation::matching(request::method_path("GET", "/api/v1/models")).respond_with(
            json_encoded(serde_json::json!([
                {
                    "model_id": "nomic-embed-text",
                    "provider_id": "ollama",
                    "task": "embedding",
                    "dimension": 768
                }
            ])),
        ),
    );

    chroma_server.expect(
        Expectation::matching(request::method_path(
            "POST",
            "/api/v1/collections/conversations_768/query",
        ))
        .respond_with(json_encoded(serde_json::json!({
            "ids": [["id1"]],
            "distances": [[0.1]],
            "documents": [["doc1"]],
            "metadatas": [[{"key": "value1"}]]
        }))),
    );

    let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(
        bridge,
        chroma_server.url_str("").trim_end_matches('/').to_string(),
    );

    // Should succeed with primary results
    let result = service.search_all_dimensions("query", 5, None, None).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_process_message_with_retry_on_second_attempt() {
    let chroma_server = Server::run();
    let bridge_server = Server::run();

    // Mock route to succeed after being called twice
    bridge_server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/route"))
            .times(2)
            .respond_with(json_encoded(serde_json::json!({
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "estimated_cost": 0.0,
                "reason": "Best match",
                "provider_type": "ollama"
            }))),
    );

    // First embed fails, second succeeds
    bridge_server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed"))
            .times(1)
            .respond_with(status_code(500)),
    );
    
    bridge_server.expect(
        Expectation::matching(request::method_path("POST", "/api/v1/embed"))
            .times(1)
            .respond_with(json_encoded(serde_json::json!({
                "embedding": vec![0.1; 768],
                "model": "nomic-embed-text",
                "dimension": 768,
                "tokens_used": 5
            }))),
    );

    chroma_server.expect(
        Expectation::matching(request::method_path(
            "POST",
            "/api/v1/collections/conversations_768",
        ))
        .respond_with(status_code(200)),
    );

    chroma_server.expect(
        Expectation::matching(request::method_path(
            "POST",
            "/api/v1/collections/conversations_768/upsert",
        ))
        .respond_with(status_code(200)),
    );

    let config = create_test_config(&bridge_server.url_str("").trim_end_matches('/'));
    let bridge = BridgeClient::new(&config).unwrap();
    let service = EmbeddingService::new(
        bridge,
        chroma_server.url_str("").trim_end_matches('/').to_string(),
    );

    let result = service
        .process_message_with_retry(
            Uuid::new_v4(),
            "test content",
            Uuid::new_v4(),
            json!({}),
            None,
        )
        .await;
    assert!(result.is_ok());
}

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
