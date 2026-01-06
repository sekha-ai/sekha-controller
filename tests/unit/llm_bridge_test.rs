use sekha_controller::services::llm_bridge_client::LlmBridgeClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use serde_json::json;

#[test]
fn test_llm_bridge_client_new() {
    let url = "http://localhost:11434";
    let _client = LlmBridgeClient::new(url.to_string());
    // Client should be created successfully (can't access private fields)
}

#[test]
fn test_llm_bridge_client_with_trailing_slash() {
    let _client = LlmBridgeClient::new("http://localhost:11434/".to_string());
    // Should handle trailing slash (no panic)
}

#[test]
fn test_llm_bridge_client_custom_port() {
    let _client = LlmBridgeClient::new("http://localhost:8080".to_string());
    // Should handle custom port
}

#[tokio::test]
async fn test_summarize_with_empty_messages() {
    let client = LlmBridgeClient::new("http://localhost:11434".to_string());

    // Test with empty messages vector
    let messages: Vec<String> = vec![];
    let _result = client.summarize(messages, "", None, None).await;
    // Just verify it doesn't panic (will likely error, which is fine)
}

#[tokio::test]
async fn test_summarize_with_messages() {
    let client = LlmBridgeClient::new("http://localhost:11434".to_string());

    // Test with some messages
    let messages = vec!["Hello".to_string(), "How are you?".to_string()];
    let _result = client
        .summarize(messages, "test conversation", None, None)
        .await;
    // Just verify method signature is correct
}

#[tokio::test]
async fn test_score_importance_with_empty_message() {
    let client = LlmBridgeClient::new("http://localhost:11434".to_string());

    // Test with empty message
    let _result = client.score_importance("", None, None).await;
    // Should handle gracefully (will error if Ollama not running)
}

#[test]
fn test_model_name_validation() {
    // Test that model names follow expected format
    let valid_models = vec!["llama3.1:8b", "llama3.2:3b", "nomic-embed-text:latest"];

    for model in valid_models {
        assert!(model.contains(':') || model.contains("latest"));
    }
}

#[test]
fn test_url_construction() {
    // Test various URL formats
    let urls = vec![
        "http://localhost:11434",
        "http://127.0.0.1:11434",
        "https://remote:11434",
    ];

    for url in urls {
        let _client = LlmBridgeClient::new(url.to_string());
        // Should construct without panic
    }
}

#[tokio::test]
async fn test_embed_text_success() {
    let mock_server = MockServer::start().await;
    let client = LlmBridgeClient::new(mock_server.uri());
    
    Mock::given(method("POST"))
        .and(path("/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embedding": [0.1, 0.2, 0.3],
            "model": "nomic-embed-text",
            "tokens_used": 10
        })))
        .mount(&mock_server)
        .await;
    
    let result = client.embed_text("test text", None).await.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 0.1);
}

#[tokio::test]
async fn test_summarize_success() {
    let mock_server = MockServer::start().await;
    let client = LlmBridgeClient::new(mock_server.uri());
    
    Mock::given(method("POST"))
        .and(path("/summarize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "summary": "Test summary",
            "level": "brief",
            "model": "llama3.1:8b",
            "tokens_used": 50
        })))
        .mount(&mock_server)
        .await;
    
    let result = client.summarize(
        vec!["message 1".to_string(), "message 2".to_string()],
        "brief",
        None,
        None
    ).await.unwrap();
    
    assert_eq!(result, "Test summary");
}

#[tokio::test]
async fn test_score_importance_success() {
    let mock_server = MockServer::start().await;
    let client = LlmBridgeClient::new(mock_server.uri());
    
    Mock::given(method("POST"))
        .and(path("/score_importance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "score": 0.85,
            "reasoning": "High importance",
            "model": "llama3.1:8b"
        })))
        .mount(&mock_server)
        .await;
    
    let result = client.score_importance("important message", None, None).await.unwrap();
    assert_eq!(result, 0.85);
}

#[tokio::test]
async fn test_embed_text_api_error() {
    let mock_server = MockServer::start().await;
    let client = LlmBridgeClient::new(mock_server.uri());
    
    Mock::given(method("POST"))
        .and(path("/embed"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
        .mount(&mock_server)
        .await;
    
    let result = client.embed_text("test", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_health_check_success() {
    let mock_server = MockServer::start().await;
    let client = LlmBridgeClient::new(mock_server.uri());
    
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;
    
    let result = client.health_check().await.unwrap();
    assert!(result);
}

#[tokio::test]
async fn test_health_check_failure() {
    let mock_server = MockServer::start().await;
    let client = LlmBridgeClient::new(mock_server.uri());
    
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;
    
    let result = client.health_check().await.unwrap();
    assert!(!result);
}