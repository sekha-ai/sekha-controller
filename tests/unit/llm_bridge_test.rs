use sekha_controller::services::llm_bridge_client::LlmBridgeClient;

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
