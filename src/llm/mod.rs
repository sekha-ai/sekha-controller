//! LLM integration module.
//!
//! This module provides integration with LLM services via the Bridge.

pub mod bridge_client;

pub use bridge_client::{
    BridgeClient, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EmbedRequest,
    EmbedResponse, ModelInfo, RoutingRequest, RoutingResponse,
};
