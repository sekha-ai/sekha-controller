// Unit tests for services
mod chroma_client_test;
mod embedding_queue_test;
mod embedding_service_test;
mod file_watcher_test;
// mod llm_bridge_comprehensive_test;
mod llm_bridge_test;

// Unit tests for orchestrator - comprehensive coverage
mod context_assembly_test;
mod importance_engine_test;
mod label_intelligence_test;
mod memory_orchestrator_test;
mod pruning_engine_test;
mod summarizer_test;

// Unit tests for API and Config - 100% coverage
mod auth_comprehensive_test;
mod auth_test;
mod config_comprehensive_test;
mod config_test;
mod mcp_llm_tests;
mod mcp_tests;
mod rate_limiter_test;
mod route_test;
mod routes_test;
mod validation_comprehensive_test;

// mod storage;
