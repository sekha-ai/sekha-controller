//! MCP endpoints for LLM provider information.
//!
//! These endpoints expose LLM provider status, routing, and cost information
//! via the MCP (Model Context Protocol) interface.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

use crate::api::mcp::McpToolResponse;
use crate::api::routes::AppState;

/// Request for LLM provider status
#[derive(Debug, Deserialize, Serialize)]
pub struct LlmStatusRequest {
    /// Optional provider ID to get specific provider status
    pub provider_id: Option<String>,
}

/// Response with LLM provider status
#[derive(Debug, Deserialize, Serialize)]
pub struct LlmStatusResponse {
    pub providers: Vec<ProviderStatus>,
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub total_models: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProviderStatus {
    pub provider_id: String,
    pub provider_type: String,
    pub status: String, // "healthy", "unhealthy", "degraded"
    pub models_count: usize,
    pub circuit_breaker_state: String,
}

/// Request for routing information
#[derive(Debug, Deserialize, Serialize)]
pub struct RoutingInfoRequest {
    pub task: String,
    pub preferred_model: Option<String>,
    pub max_cost: Option<f64>,
}

/// Response with routing recommendation
#[derive(Debug, Deserialize, Serialize)]
pub struct RoutingInfoResponse {
    pub provider_id: String,
    pub model_id: String,
    pub estimated_cost: f64,
    pub reason: String,
    pub provider_type: String,
}

/// MCP tool: Get LLM provider status
///
/// Returns information about configured LLM providers including:
/// - Provider health
/// - Circuit breaker states
/// - Available models
/// - Total provider count
pub async fn mcp_llm_status(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LlmStatusRequest>,
) -> Json<McpToolResponse> {
    debug!("MCP: Getting LLM provider status");

    // Get bridge client from state
    let bridge_client = &state.llm_client;

    // Try to get provider health from bridge
    match get_provider_status(bridge_client, request.provider_id).await {
        Ok(response) => {
            let result = serde_json::to_value(&response).unwrap_or_default();
            Json(McpToolResponse {
                success: true,
                data: Some(result),
                error: None,
            })
        }
        Err(e) => {
            error!("Failed to get LLM provider status: {}", e);
            Json(McpToolResponse {
                success: false,
                data: None,
                error: Some(format!("Error getting LLM provider status: {}", e)),
            })
        }
    }
}

/// MCP tool: Get routing recommendation
///
/// Returns the optimal provider and model for a given task with cost estimate.
pub async fn mcp_llm_routing(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RoutingInfoRequest>,
) -> Json<McpToolResponse> {
    debug!("MCP: Getting routing info for task: {}", request.task);

    let bridge_client = &state.llm_client;

    match get_routing_info(bridge_client, request).await {
        Ok(response) => {
            let result = serde_json::to_value(&response).unwrap_or_default();
            Json(McpToolResponse {
                success: true,
                data: Some(result),
                error: None,
            })
        }
        Err(e) => {
            error!("Failed to get routing info: {}", e);
            Json(McpToolResponse {
                success: false,
                data: None,
                error: Some(format!("Error getting routing info: {}", e)),
            })
        }
    }
}

// Helper functions

async fn get_provider_status(
    bridge_client: &crate::services::llm_bridge_client::LlmBridgeClient,
    _provider_id: Option<String>,
) -> anyhow::Result<LlmStatusResponse> {
    // For now, return basic status
    // In a full implementation, this would call bridge /api/v1/health/providers

    // Check if bridge is healthy - propagate error if health check fails completely
    let is_healthy = bridge_client.health_check().await?;

    // If bridge is unreachable (health_check returns Ok(false)), treat as error
    if !is_healthy {
        return Err(anyhow::anyhow!("Bridge is unreachable or unhealthy"));
    }

    // Return basic status
    Ok(LlmStatusResponse {
        providers: vec![ProviderStatus {
            provider_id: "bridge".to_string(),
            provider_type: "bridge".to_string(),
            status: "healthy".to_string(),
            models_count: 0,
            circuit_breaker_state: "closed".to_string(),
        }],
        total_providers: 1,
        healthy_providers: 1,
        total_models: 0,
    })
}

async fn get_routing_info(
    bridge_client: &crate::services::llm_bridge_client::LlmBridgeClient,
    request: RoutingInfoRequest,
) -> anyhow::Result<RoutingInfoResponse> {
    // Get routing from bridge
    let routing = bridge_client
        .get_routing(&request.task, request.preferred_model, request.max_cost)
        .await?;

    Ok(RoutingInfoResponse {
        provider_id: routing.provider_id,
        model_id: routing.model_id,
        estimated_cost: routing.estimated_cost,
        reason: "Routed by bridge".to_string(),
        provider_type: "unknown".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        orchestrator::MemoryOrchestrator,
        services::{embedding_service::EmbeddingService, llm_bridge_client::LlmBridgeClient},
        storage::{chroma_client::ChromaClient, init_db, repository::SeaOrmConversationRepository},
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    async fn create_test_state_with_mock_bridge(bridge_url: String) -> Arc<AppState> {
        let db = init_db("sqlite::memory:").await.unwrap();
        let config = Arc::new(RwLock::new(Config::default()));
        let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
        let embedding_service = Arc::new(EmbeddingService::new(
            "http://localhost:11434".to_string(),
            "http://localhost:8000".to_string(),
        ));

        let repo = Arc::new(SeaOrmConversationRepository::new(
            db,
            chroma_client.clone(),
            embedding_service.clone(),
        ));

        let mut test_config = Config::default();
        test_config.llm_bridge_url = bridge_url;
        let llm_bridge = Arc::new(LlmBridgeClient::new(&test_config).unwrap());

        Arc::new(AppState {
            config,
            repo: repo.clone(),
            orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge.clone())),
            embedding_service,
            chroma_client,
            llm_client: llm_bridge,
        })
    }

    /// FORCES lines 76-81 (success path in mcp_llm_status)
    #[tokio::test]
    async fn test_mcp_llm_status_success() {
        let mock_server = MockServer::start().await;

        // Mock health check - correct endpoint is /health (not /api/v1/health)
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let state = create_test_state_with_mock_bridge(mock_server.uri()).await;
        let request = LlmStatusRequest { provider_id: None };

        let response = mcp_llm_status(State(state), Json(request)).await;

        // Lines 76-81 EXECUTED
        assert!(response.0.success);
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());
    }

    /// FORCES lines 83-88 (error path in mcp_llm_status)
    #[tokio::test]
    async fn test_mcp_llm_status_error() {
        let mock_server = MockServer::start().await;
        let server_uri = mock_server.uri();

        // Don't mount any mock - this causes connection errors
        // Drop the server to ensure connection refused
        drop(mock_server);

        let state = create_test_state_with_mock_bridge(server_uri).await;
        let request = LlmStatusRequest { provider_id: None };

        let response = mcp_llm_status(State(state), Json(request)).await;

        // Lines 83-88 EXECUTED - connection failure -> health_check returns Ok(false) -> error path
        assert!(!response.0.success);
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
    }

    /// FORCES lines 106-111 AND 161-166 (success paths)
    #[tokio::test]
    async fn test_mcp_llm_routing_success() {
        let mock_server = MockServer::start().await;

        // Mock routing endpoint - correct path is /api/v1/route (not /routing)
        Mock::given(method("POST"))
            .and(path("/api/v1/route"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "provider_id": "test_provider",
                "model_id": "test_model",
                "estimated_cost": 0.001,
                "reason": "test",
                "provider_type": "test"
            })))
            .mount(&mock_server)
            .await;

        let state = create_test_state_with_mock_bridge(mock_server.uri()).await;
        let request = RoutingInfoRequest {
            task: "chat".to_string(),
            preferred_model: None,
            max_cost: None,
        };

        let response = mcp_llm_routing(State(state), Json(request)).await;

        // Lines 106-111 EXECUTED
        assert!(response.0.success);
        assert!(response.0.data.is_some());
        assert!(response.0.error.is_none());

        // Verify lines 161-166 (RoutingInfoResponse construction)
        let routing: RoutingInfoResponse =
            serde_json::from_value(response.0.data.unwrap()).unwrap();
        assert_eq!(routing.provider_id, "test_provider");
        assert_eq!(routing.model_id, "test_model");
        assert_eq!(routing.reason, "Routed by bridge"); // Line 164
        assert_eq!(routing.provider_type, "unknown"); // Line 165
    }

    /// FORCES lines 113-119 (error path in mcp_llm_routing)
    #[tokio::test]
    async fn test_mcp_llm_routing_error() {
        let mock_server = MockServer::start().await;
        let server_uri = mock_server.uri();

        // Drop server to cause connection failure
        drop(mock_server);

        let state = create_test_state_with_mock_bridge(server_uri).await;
        let request = RoutingInfoRequest {
            task: "chat".to_string(),
            preferred_model: None,
            max_cost: None,
        };

        let response = mcp_llm_routing(State(state), Json(request)).await;

        // Lines 113-119 EXECUTED
        assert!(!response.0.success);
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
    }
}
