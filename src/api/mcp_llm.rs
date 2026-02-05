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
#[derive(Debug, Serialize)]
pub struct LlmStatusResponse {
    pub providers: Vec<ProviderStatus>,
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub total_models: usize,
}

#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
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

    // Check if bridge is healthy
    let is_healthy = bridge_client.health_check().await.unwrap_or(false);

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
