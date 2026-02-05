use sekha_controller::{config::Config, services::llm_bridge_client::LlmBridgeClient};

#[test]
fn test_llm_bridge_client_creation() {
    let config = Config::default();
    let client = LlmBridgeClient::new(&config);
    assert!(client.is_ok());
}

#[test]
fn test_llm_bridge_base_url_from_config() {
    let config = Config::default();
    let client = LlmBridgeClient::new(&config);
    assert!(client.is_ok());
}
