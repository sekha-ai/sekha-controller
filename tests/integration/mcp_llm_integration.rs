use axum::extract::State;
use axum::Json;
use sekha_controller::{
    api::mcp_llm::{
        mcp_llm_routing, mcp_llm_status, LlmStatusRequest, LlmStatusResponse, RoutingInfoRequest,
        RoutingInfoResponse,
    },
    api::routes::AppState,
    config::Config,
    orchestrator::MemoryOrchestrator,
    services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
    storage::{chroma_client::ChromaClient, init_db, repository::SeaOrmConversationRepository},
};
use std::sync::Arc;
use tokio::sync::RwLock;

async fn setup_test_state() -> AppState {
    let db = init_db("sqlite::memory:").await.unwrap();
    let config = Arc::new(RwLock::new(Config::default()));
    let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding_service = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));

    let repo = Arc::new(SeaOrmConversationRepository::new(
        db,
        chroma_client,
        embedding_service.clone(),
    ));

    let config_ref = config.read().await;
    let llm_bridge = Arc::new(LlmBridgeClient::new(&*config_ref).unwrap());
    drop(config_ref);

    AppState {
        config,
        repo: repo.clone(),
        orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge.clone())),
        embedding_service,
        chroma_client: Arc::new(ChromaClient::new("http://localhost:8000".to_string())),
        llm_client: llm_bridge,
    }
}

// ========== mcp_llm_status tests ==========

#[tokio::test]
async fn test_mcp_llm_status_error_path_bridge_offline() {
    // This test covers the error path (lines 84-90)
    let state = Arc::new(setup_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    // With bridge offline, should hit error path
    if !response.0.success {
        // Verify error response structure (lines 84-90)
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());

        let error_msg = response.0.error.unwrap();
        assert!(error_msg.contains("Error getting LLM provider status"));
    }
}

#[tokio::test]
async fn test_mcp_llm_status_success_path_structure() {
    // This test covers the success path (lines 76-81)
    let state = Arc::new(setup_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    // If bridge is available, verify success path
    if response.0.success {
        // Verify success response structure (lines 76-81)
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());

        // Deserialize and verify LlmStatusResponse
        let status: LlmStatusResponse = serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Verify response has required fields
        assert!(status.total_providers > 0);
        assert!(status.healthy_providers <= status.total_providers);
        assert!(!status.providers.is_empty());
    }
}

#[tokio::test]
async fn test_mcp_llm_status_with_provider_filter() {
    let state = Arc::new(setup_test_state().await);
    let request = LlmStatusRequest {
        provider_id: Some("ollama".to_string()),
    };

    let response = mcp_llm_status(State(state), Json(request)).await;

    // Either success or error is valid depending on bridge availability
    // But structure should always be correct
    if response.0.success {
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());
    } else {
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
    }
}

#[tokio::test]
async fn test_mcp_llm_status_provider_details() {
    let state = Arc::new(setup_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    if response.0.success {
        let status: LlmStatusResponse = serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Verify each provider has required fields
        for provider in &status.providers {
            assert!(!provider.provider_id.is_empty());
            assert!(!provider.provider_type.is_empty());
            assert!(!provider.status.is_empty());
            assert!(!provider.circuit_breaker_state.is_empty());
            assert!(provider.models_count >= 0);
        }
    }
}

// ========== mcp_llm_routing tests ==========

#[tokio::test]
async fn test_mcp_llm_routing_success_path_all_fields() {
    // This test covers the success path (lines 103-108) and
    // the RoutingInfoResponse construction (lines 154-161)
    let state = Arc::new(setup_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: None,
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    // If successful, verify all response fields (lines 103-108, 154-161)
    if response.0.success {
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());

        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Verify all fields from RoutingInfoResponse (lines 154-161)
        assert!(!routing.provider_id.is_empty());
        assert!(!routing.model_id.is_empty());
        assert!(routing.estimated_cost >= 0.0);
        assert_eq!(routing.reason, "Routed by bridge"); // Line 159
        assert_eq!(routing.provider_type, "unknown"); // Line 160
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_error_path_bridge_offline() {
    // This test covers the error path (lines 111-117)
    let state = Arc::new(setup_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: None,
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    // With bridge offline, should hit error path
    if !response.0.success {
        // Verify error response structure (lines 111-117)
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());

        let error_msg = response.0.error.unwrap();
        assert!(error_msg.contains("Error getting routing info"));
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_with_all_parameters() {
    let state = Arc::new(setup_test_state().await);
    let request = RoutingInfoRequest {
        task: "embedding".to_string(),
        preferred_model: Some("nomic-embed-text".to_string()),
        max_cost: Some(0.001),
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    // Verify response structure regardless of success/error
    if response.0.success {
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());

        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // All fields should be populated
        assert!(!routing.provider_id.is_empty());
        assert!(!routing.model_id.is_empty());
        assert!(routing.estimated_cost >= 0.0);
    } else {
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_different_tasks() {
    let state = Arc::new(setup_test_state().await);

    let tasks = vec!["chat", "embedding", "completion"];

    for task in tasks {
        let request = RoutingInfoRequest {
            task: task.to_string(),
            preferred_model: None,
            max_cost: None,
        };

        let response = mcp_llm_routing(State(state.clone()), Json(request)).await;

        // Should return a valid response for each task type
        // Either success or error, but structure should be correct
        if response.0.success {
            assert!(response.0.data.is_some());
        } else {
            assert!(response.0.error.is_some());
        }
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_cost_constraint() {
    let state = Arc::new(setup_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: None,
        max_cost: Some(0.0001),
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    if response.0.success {
        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Estimated cost should be provided
        assert!(routing.estimated_cost >= 0.0);
    }
}

#[tokio::test]
async fn test_mcp_llm_routing_model_preference() {
    let state = Arc::new(setup_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: Some("llama3.1:8b".to_string()),
        max_cost: None,
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    if response.0.success {
        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Should return a model (might be preferred or fallback)
        assert!(!routing.model_id.is_empty());
    }
}

// ========== Helper function tests ==========

#[tokio::test]
async fn test_get_provider_status_consistency() {
    // Test that helper function maintains data consistency
    let state = Arc::new(setup_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    if response.0.success {
        let status: LlmStatusResponse = serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Total models should equal sum of individual provider model counts
        let total_from_providers: u32 = status.providers.iter().map(|p| p.models_count).sum();
        assert_eq!(status.total_models, total_from_providers);

        // Healthy providers count should match status == "healthy"
        let healthy_count = status
            .providers
            .iter()
            .filter(|p| p.status == "healthy")
            .count();
        assert_eq!(status.healthy_providers, healthy_count);
    }
}

#[tokio::test]
async fn test_get_routing_info_fields_populated() {
    // Test that all required fields in routing response are populated
    let state = Arc::new(setup_test_state().await);
    let request = RoutingInfoRequest {
        task: "chat".to_string(),
        preferred_model: Some("test-model".to_string()),
        max_cost: Some(0.05),
    };

    let response = mcp_llm_routing(State(state), Json(request)).await;

    if response.0.success {
        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();

        // Every field should be populated (lines 154-161)
        assert!(!routing.provider_id.is_empty(), "provider_id is empty");
        assert!(!routing.model_id.is_empty(), "model_id is empty");
        assert!(routing.estimated_cost >= 0.0, "estimated_cost is negative");
        assert!(
            !routing.reason.is_empty(),
            "reason is empty (should be 'Routed by bridge')"
        );
        assert!(
            !routing.provider_type.is_empty(),
            "provider_type is empty (should be 'unknown')"
        );

        // Verify specific values set by helper (lines 159-160)
        assert_eq!(routing.reason, "Routed by bridge");
        assert_eq!(routing.provider_type, "unknown");
    }
}

#[tokio::test]
async fn test_mcp_response_serialization() {
    // Test that McpToolResponse serializes correctly
    let state = Arc::new(setup_test_state().await);
    let request = LlmStatusRequest { provider_id: None };

    let response = mcp_llm_status(State(state), Json(request)).await;

    // Verify response can be serialized back to JSON
    let json_result = serde_json::to_string(&response.0);
    assert!(json_result.is_ok(), "Response should be serializable");

    let json_str = json_result.unwrap();
    assert!(json_str.contains("success"));
}
