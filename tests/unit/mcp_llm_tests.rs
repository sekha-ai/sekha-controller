use sekha_controller::{
    api::routes::AppState,
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, repository::MockConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;

// Helper to create test state
async fn create_llm_test_state() -> AppState {
    let config = Arc::new(RwLock::new(Config::default()));
    let mock_repo = Arc::new(MockConversationRepository::new());
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));

    let config_ref = config.read().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(&*config_ref).unwrap());
    drop(config_ref);

    AppState {
        config,
        repo: mock_repo.clone(),
        orchestrator: Arc::new(MemoryOrchestrator::new(mock_repo, llm_bridge.clone())),
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    }
}

#[tokio::test]
async fn test_llm_status_request_serialization() {
    use sekha_controller::api::mcp_llm::LlmStatusRequest;
    use serde_json::json;

    let json = json!({
        "provider_id": "ollama"
    });

    let request: LlmStatusRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.provider_id, Some("ollama".to_string()));
}

#[tokio::test]
async fn test_llm_status_request_without_provider() {
    use sekha_controller::api::mcp_llm::LlmStatusRequest;
    use serde_json::json;

    let json = json!({});

    let request: LlmStatusRequest = serde_json::from_value(json).unwrap();
    assert!(request.provider_id.is_none());
}

#[tokio::test]
async fn test_routing_info_request_serialization() {
    use sekha_controller::api::mcp_llm::RoutingInfoRequest;
    use serde_json::json;

    let json = json!({
        "task": "embedding",
        "preferred_model": "nomic-embed-text",
        "max_cost": 0.01
    });

    let request: RoutingInfoRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.task, "embedding");
    assert_eq!(
        request.preferred_model,
        Some("nomic-embed-text".to_string())
    );
    assert_eq!(request.max_cost, Some(0.01));
}

#[tokio::test]
async fn test_routing_info_request_minimal() {
    use sekha_controller::api::mcp_llm::RoutingInfoRequest;
    use serde_json::json;

    let json = json!({
        "task": "chat"
    });

    let request: RoutingInfoRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.task, "chat");
    assert!(request.preferred_model.is_none());
    assert!(request.max_cost.is_none());
}

#[tokio::test]
async fn test_provider_status_serialization() {
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let status = ProviderStatus {
        provider_id: "ollama".to_string(),
        provider_type: "ollama".to_string(),
        status: "healthy".to_string(),
        models_count: 5,
        circuit_breaker_state: "closed".to_string(),
    };

    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["provider_id"], "ollama");
    assert_eq!(json["models_count"], 5);
    assert_eq!(json["status"], "healthy");
}

#[tokio::test]
async fn test_llm_status_response_structure() {
    use sekha_controller::api::mcp_llm::{LlmStatusResponse, ProviderStatus};

    let response = LlmStatusResponse {
        providers: vec![
            ProviderStatus {
                provider_id: "ollama1".to_string(),
                provider_type: "ollama".to_string(),
                status: "healthy".to_string(),
                models_count: 3,
                circuit_breaker_state: "closed".to_string(),
            },
            ProviderStatus {
                provider_id: "ollama2".to_string(),
                provider_type: "ollama".to_string(),
                status: "degraded".to_string(),
                models_count: 2,
                circuit_breaker_state: "half_open".to_string(),
            },
        ],
        total_providers: 2,
        healthy_providers: 1,
        total_models: 5,
    };

    assert_eq!(response.total_providers, 2);
    assert_eq!(response.healthy_providers, 1);
    assert_eq!(response.total_models, 5);
    assert_eq!(response.providers.len(), 2);
}

#[tokio::test]
async fn test_routing_info_response_structure() {
    use sekha_controller::api::mcp_llm::RoutingInfoResponse;

    let response = RoutingInfoResponse {
        provider_id: "ollama_main".to_string(),
        model_id: "llama3.1:8b".to_string(),
        estimated_cost: 0.0001,
        reason: "Best cost-performance ratio".to_string(),
        provider_type: "ollama".to_string(),
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["provider_id"], "ollama_main");
    assert_eq!(json["model_id"], "llama3.1:8b");
    assert_eq!(json["estimated_cost"], 0.0001);
}

#[tokio::test]
async fn test_mcp_llm_status_data_structures() {
    // Test all enum values for provider status
    let statuses = vec!["healthy", "unhealthy", "degraded"];

    for status_str in statuses {
        let status = sekha_controller::api::mcp_llm::ProviderStatus {
            provider_id: "test".to_string(),
            provider_type: "test".to_string(),
            status: status_str.to_string(),
            models_count: 1,
            circuit_breaker_state: "closed".to_string(),
        };

        assert_eq!(status.status, status_str);
    }
}

#[tokio::test]
async fn test_circuit_breaker_states() {
    // Test all circuit breaker states
    let states = vec!["closed", "open", "half_open"];

    for state_str in states {
        let status = sekha_controller::api::mcp_llm::ProviderStatus {
            provider_id: "test".to_string(),
            provider_type: "test".to_string(),
            status: "healthy".to_string(),
            models_count: 1,
            circuit_breaker_state: state_str.to_string(),
        };

        assert_eq!(status.circuit_breaker_state, state_str);
    }
}

#[tokio::test]
async fn test_routing_with_cost_constraint() {
    use sekha_controller::api::mcp_llm::RoutingInfoRequest;
    use serde_json::json;

    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: Some(0.001),
    };

    // Verify cost constraint is preserved
    assert_eq!(request.max_cost, Some(0.001));

    // Serialize and deserialize
    let json = serde_json::to_value(&request).unwrap();
    let deserialized: RoutingInfoRequest = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.max_cost, Some(0.001));
}

#[tokio::test]
async fn test_routing_with_preferred_model() {
    use sekha_controller::api::mcp_llm::RoutingInfoRequest;

    let request = RoutingInfoRequest {
        task: "embedding".to_string(),
        preferred_model: Some("nomic-embed-text".to_string()),
        max_cost: None,
    };

    assert_eq!(
        request.preferred_model,
        Some("nomic-embed-text".to_string())
    );
}

#[tokio::test]
async fn test_llm_status_response_with_no_providers() {
    use sekha_controller::api::mcp_llm::LlmStatusResponse;

    let response = LlmStatusResponse {
        providers: vec![],
        total_providers: 0,
        healthy_providers: 0,
        total_models: 0,
    };

    assert_eq!(response.providers.len(), 0);
    assert_eq!(response.total_providers, 0);
    assert_eq!(response.healthy_providers, 0);
}

#[tokio::test]
async fn test_provider_status_json_format() {
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let status = ProviderStatus {
        provider_id: "openai".to_string(),
        provider_type: "openai".to_string(),
        status: "healthy".to_string(),
        models_count: 10,
        circuit_breaker_state: "closed".to_string(),
    };

    let json_str = serde_json::to_string(&status).unwrap();
    assert!(json_str.contains("openai"));
    assert!(json_str.contains("healthy"));
    assert!(json_str.contains("closed"));
}

#[tokio::test]
async fn test_routing_response_cost_estimation() {
    use sekha_controller::api::mcp_llm::RoutingInfoResponse;

    let response = RoutingInfoResponse {
        provider_id: "anthropic".to_string(),
        model_id: "claude-3-haiku".to_string(),
        estimated_cost: 0.00025,
        reason: "Lowest cost for task".to_string(),
        provider_type: "anthropic".to_string(),
    };

    // Verify cost is reasonable
    assert!(response.estimated_cost > 0.0);
    assert!(response.estimated_cost < 1.0);
}

#[tokio::test]
async fn test_provider_types_coverage() {
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let provider_types = vec!["ollama", "openai", "anthropic", "litellm", "openrouter"];

    for ptype in provider_types {
        let status = ProviderStatus {
            provider_id: format!("{}_test", ptype),
            provider_type: ptype.to_string(),
            status: "healthy".to_string(),
            models_count: 5,
            circuit_breaker_state: "closed".to_string(),
        };

        assert_eq!(status.provider_type, ptype);
    }
}
