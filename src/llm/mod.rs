//! LLM integration module.
//!
//! This module provides integration with LLM services via the Bridge.

pub mod bridge_client;

pub use bridge_client::{
    BridgeClient,
    ChatMessage,
    ChatCompletionRequest,
    ChatCompletionResponse,
    EmbedRequest,
    EmbedResponse,
    RoutingRequest,
    RoutingResponse,
    ModelInfo,
};
