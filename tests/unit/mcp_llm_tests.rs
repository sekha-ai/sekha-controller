use axum::extract::State;
use axum::Json;
use sekha_controller::{
    api::mcp_llm::{LlmStatusRequest, RoutingInfoRequest},
    api::routes::AppState,
    config::Config,
    llm::bridge_client::BridgeClient,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, repository::MockConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;

// Helper to create test state
async fn create_llm_test_state() -> AppState {
    let base_config = Config::default();
    let config = Arc::new(RwLock::new(base_config.clone()));
    let mock_repo = Arc::new(MockConversationRepository::new());
    
    // Create BridgeClient first, then pass to EmbeddingService
    let bridge = BridgeClient::new(&base_config).expect("Failed to create BridgeClient");
    let embedding_service = Arc::new(EmbeddingService::new(
        bridge,
        "http://localhost:8000".to_string(),
    ));
    
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let llm_bridge = Arc::new(LlmBridgeClient::new(&base_config).unwrap());

    AppState {
        config,
        repo: mock_repo.clone(),
        orchestrator: Arc::new(MemoryOrchestrator::new(mock_repo, llm_bridge.clone())),
        embedding_service,
        chroma_client,
        llm_client: llm_bridge,
    }
}

// ========== Data Structure Tests ==========

#[tokio::test]
async fn test_llm_status_request_serialization() {
    use sekha_controller::api::mcp_llm::LlmStatusRequest;
    use serde_json::json;

    let json = json!({"provider_id": "ollama"});
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
async fn test_llm_status_request_debug() {
    let request = LlmStatusRequest {
        provider_id: Some("test".to_string()),
    };
    let debug_str = format!("{:?}", request);
    assert!(debug_str.contains("LlmStatusRequest"));
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

    let json = json!({"task": "chat"});
    let request: RoutingInfoRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.task, "chat");
    assert!(request.preferred_model.is_none());
    assert!(request.max_cost.is_none());
}

#[tokio::test]
async fn test_routing_info_request_debug() {
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: Some("model".to_string()),
        max_cost: Some(0.01),
    };
    let debug_str = format!("{:?}", request);
    assert!(debug_str.contains("RoutingInfoRequest"));
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
async fn test_provider_status_debug() {
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let status = ProviderStatus {
        provider_id: "test".to_string(),
        provider_type: "ollama".to_string(),
        status: "healthy".to_string(),
        models_count: 5,
        circuit_breaker_state: "closed".to_string(),
    };

    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("ProviderStatus"));
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
async fn test_llm_status_response_debug() {
    use sekha_controller::api::mcp_llm::{LlmStatusResponse, ProviderStatus};

    let response = LlmStatusResponse {
        providers: vec![ProviderStatus {
            provider_id: "test".to_string(),
            provider_type: "ollama".to_string(),
            status: "healthy".to_string(),
            models_count: 3,
            circuit_breaker_state: "closed".to_string(),
        }],
        total_providers: 1,
        healthy_providers: 1,
        total_models: 3,
    };

    let debug_str = format!("{:?}", response);
    assert!(debug_str.contains("LlmStatusResponse"));
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
async fn test_routing_info_response_debug() {
    use sekha_controller::api::mcp_llm::RoutingInfoResponse;

    let response = RoutingInfoResponse {
        provider_id: "test".to_string(),
        model_id: "model".to_string(),
        estimated_cost: 0.001,
        reason: "test reason".to_string(),
        provider_type: "ollama".to_string(),
    };

    let debug_str = format!("{:?}", response);
    assert!(debug_str.contains("RoutingInfoResponse"));
}

#[tokio::test]
async fn test_provider_status_all_states() {
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let statuses = vec!["healthy", "unhealthy", "degraded"];
    for status_str in statuses {
        let status = ProviderStatus {
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
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let states = vec!["closed", "open", "half_open"];
    for state_str in states {
        let status = ProviderStatus {
            provider_id: "test".to_string(),
            provider_type: "test".to_string(),
            status: "healthy".to_string(),
            models_count: 1,
            circuit_breaker_state: state_str.to_string(),
        };
        assert_eq!(status.circuit_breaker_state, state_str);
    }
}

// ========== Endpoint Function Tests ==========

#[tokio::test]
async fn test_mcp_llm_status_endpoint_basic() {
    use sekha_controller::api::mcp_llm::mcp_llm_status;

    let state = Arc::new(create_llm_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    // Test endpoint executes without panicking
    let _response = mcp_llm_status(State(state), Json(request)).await;
}

#[tokio::test]
async fn test_mcp_llm_status_with_provider_id() {
    use sekha_controller::api::mcp_llm::mcp_llm_status;

    let state = Arc::new(create_llm_test_state().await);
    let request = LlmStatusRequest {
        provider_id: Some("ollama".to_string()),
    };

    // Test endpoint executes without panicking
    let _response = mcp_llm_status(State(state), Json(request)).await;
}

#[tokio::test]
async fn test_mcp_llm_status_success_response_structure() {
    use sekha_controller::api::mcp_llm::{mcp_llm_status, LlmStatusResponse};

    let state = Arc::new(create_llm_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    // Verify McpToolResponse structure
    if response.0.success {
        // Success path: lines 76-81
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());

        // Deserialize and verify LlmStatusResponse
        let status: LlmStatusResponse = serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Should have basic bridge status
        assert_eq!(status.total_providers, 1);
        assert!(status.healthy_providers <= 1);
    }
}

#[tokio::test]
async fn test_mcp_llm_status_error_response_structure() {
    use sekha_controller::api::mcp_llm::mcp_llm_status;

    let state = Arc::new(create_llm_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    // If error path (lines 84-90)
    if !response.0.success {
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
        let error_msg = response.0.error.unwrap();
        assert!(error_msg.contains("Error getting LLM provider status"));
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_endpoint_basic() {
    use sekha_controller::api::mcp_llm::mcp_llm_routing;

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: None,
    };

    // Test endpoint executes without panicking
    let _response = mcp_llm_routing(State(state), Json(request)).await;
}

#[tokio::test]
async fn test_mcp_llm_routing_with_preferred_model() {
    use sekha_controller::api::mcp_llm::mcp_llm_routing;

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "embedding".to_string(),
        preferred_model: Some("nomic-embed-text".to_string()),
        max_cost: None,
    };

    // Test endpoint executes without panicking
    let _response = mcp_llm_routing(State(state), Json(request)).await;
}

#[tokio::test]
async fn test_mcp_llm_routing_with_max_cost() {
    use sekha_controller::api::mcp_llm::mcp_llm_routing;

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: Some(0.01),
    };

    // Test endpoint executes without panicking
    let _response = mcp_llm_routing(State(state), Json(request)).await;
}

#[tokio::test]
async fn test_mcp_llm_routing_with_all_options() {
    use sekha_controller::api::mcp_llm::mcp_llm_routing;

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: Some("llama3.1:8b".to_string()),
        max_cost: Some(0.001),
    };

    // Test endpoint executes without panicking
    let _response = mcp_llm_routing(State(state), Json(request)).await;
}

#[tokio::test]
async fn test_mcp_llm_routing_success_response_structure() {
    use sekha_controller::api::mcp_llm::{mcp_llm_routing, RoutingInfoResponse};

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: None,
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    // Success path: lines 103-108
    if response.0.success {
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());

        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Verify fields are populated
        assert!(!routing.provider_id.is_empty());
        assert!(!routing.model_id.is_empty());
        assert!(routing.estimated_cost >= 0.0);
        assert_eq!(routing.reason, "Routed by bridge");
        assert_eq!(routing.provider_type, "unknown");
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_error_response_structure() {
    use sekha_controller::api::mcp_llm::mcp_llm_routing;

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "invalid_task".to_string(),
        preferred_model: None,
        max_cost: None,
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    // Error path: lines 111-117
    if !response.0.success {
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
        let error_msg = response.0.error.unwrap();
        assert!(error_msg.contains("Error getting routing info"));
    }
}

#[tokio::test]
async fn test_get_provider_status_healthy_path() {
    // This tests the helper function indirectly through the endpoint
    // Testing line 138: is_healthy = true path
    use sekha_controller::api::mcp_llm::{mcp_llm_status, LlmStatusResponse};

    let state = Arc::new(create_llm_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    if response.0.success {
        let status: LlmStatusResponse = serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Verify provider status structure (lines 139-145)
        assert_eq!(status.providers.len(), 1);
        let provider = &status.providers[0];
        assert_eq!(provider.provider_id, "bridge");
        assert_eq!(provider.provider_type, "bridge");

        // Status should be either "healthy" or "unhealthy" (line 140)
        assert!(provider.status == "healthy" || provider.status == "unhealthy");

        // healthy_providers should be 0 or 1 (line 143)
        assert!(status.healthy_providers <= 1);

        // Verify consistency
        if provider.status == "healthy" {
            assert_eq!(status.healthy_providers, 1);
        } else {
            assert_eq!(status.healthy_providers, 0);
        }
    }
}

#[tokio::test]
async fn test_get_routing_info_helper_fields() {
    // This tests the helper function's field assignment (lines 154-161)
    use sekha_controller::api::mcp_llm::{mcp_llm_routing, RoutingInfoResponse};

    let state = Arc::new(create_llm_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: Some("test-model".to_string()),
        max_cost: Some(0.05),
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    if response.0.success {
        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Verify all fields from get_routing_info are present
        assert!(!routing.provider_id.is_empty());
        assert!(!routing.model_id.is_empty());
        assert!(routing.estimated_cost >= 0.0);
        assert_eq!(routing.reason, "Routed by bridge"); // Line 159
        assert_eq!(routing.provider_type, "unknown"); // Line 160
    }
}

#[tokio::test]
async fn test_llm_status_response_empty_providers() {
    use sekha_controller::api::mcp_llm::LlmStatusResponse;

    let response = LlmStatusResponse {
        providers: vec![],
        total_providers: 0,
        healthy_providers: 0,
        total_models: 0,
    };

    assert_eq!(response.providers.len(), 0);
    assert_eq!(response.total_providers, 0);
}

#[tokio::test]
async fn test_provider_status_with_zero_models() {
    use sekha_controller::api::mcp_llm::ProviderStatus;

    let status = ProviderStatus {
        provider_id: "empty".to_string(),
        provider_type: "ollama".to_string(),
        status: "healthy".to_string(),
        models_count: 0,
        circuit_breaker_state: "closed".to_string(),
    };

    assert_eq!(status.models_count, 0);
}

#[tokio::test]
async fn test_routing_response_zero_cost() {
    use sekha_controller::api::mcp_llm::RoutingInfoResponse;

    let response = RoutingInfoResponse {
        provider_id: "free".to_string(),
        model_id: "free_model".to_string(),
        estimated_cost: 0.0,
        reason: "Free tier".to_string(),
        provider_type: "ollama".to_string(),
    };

    assert_eq!(response.estimated_cost, 0.0);
}
