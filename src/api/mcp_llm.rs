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

    // Return basic status
    Ok(LlmStatusResponse {
        providers: vec![ProviderStatus {
            provider_id: "bridge".to_string(),
            provider_type: "bridge".to_string(),
            status: if is_healthy { "healthy" } else { "unhealthy" }.to_string(),
            models_count: 0,
            circuit_breaker_state: "closed".to_string(),
        }],
        total_providers: 1,
        healthy_providers: if is_healthy { 1 } else { 0 },
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

    async fn create_test_state_with_bad_bridge() -> Arc<AppState> {
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

        // Create bridge client pointing to invalid URL to force errors
        let mut bad_config = Config::default();
        bad_config.llm_bridge_url = "http://localhost:1".to_string(); // Invalid port
        let llm_bridge = Arc::new(LlmBridgeClient::new(&bad_config).unwrap());

        Arc::new(AppState {
            config,
            repo: repo.clone(),
            orchestrator: Arc::new(MemoryOrchestrator::new(repo, llm_bridge.clone())),
            embedding_service,
            chroma_client,
            llm_client: llm_bridge,
        })
    }

    /// FORCES lines 83-88 (error path in mcp_llm_status)
    #[tokio::test]
    async fn test_mcp_llm_status_error_path_forced() {
        let state = create_test_state_with_bad_bridge().await;
        let request = LlmStatusRequest { provider_id: None };

        // Bridge is unreachable, so get_provider_status will return Err
        // This FORCES execution through lines 83-88
        let response = mcp_llm_status(State(state), Json(request)).await;

        // Lines 83-88 executed
        assert!(!response.0.success);
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
    }

    /// FORCES lines 113-119 (error path in mcp_llm_routing)
    #[tokio::test]
    async fn test_mcp_llm_routing_error_path_forced() {
        let state = create_test_state_with_bad_bridge().await;
        let request = RoutingInfoRequest {
            task: "chat".to_string(),
            preferred_model: None,
            max_cost: None,
        };

        // Bridge is unreachable, so get_routing_info will return Err
        // This FORCES execution through lines 113-119
        let response = mcp_llm_routing(State(state), Json(request)).await;

        // Lines 113-119 executed
        assert!(!response.0.success);
        assert!(response.0.data.is_none());
        assert!(response.0.error.is_some());
    }
}
