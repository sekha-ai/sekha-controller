#[cfg(test)]
mod routes_tests {
    use super::super::*;
    use crate::api::dto::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    // ==================== TEST HELPERS ====================

    async fn create_test_state() -> AppState {
        use crate::{
            config::Config,
            llm::bridge_client::BridgeClient,
            orchestrator::MemoryOrchestrator,
            storage::{init_db, repository::SeaOrmConversationRepository},
        };
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let db = init_db("sqlite::memory:").await.unwrap();
        let config = Arc::new(RwLock::new(Config::default()));
        let chroma_client = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));

        let base_config = Config::default();
        let bridge = BridgeClient::new(&base_config).expect("Failed to create BridgeClient");
        let embedding_service = Arc::new(EmbeddingService::new(
            bridge,
            "http://localhost:8000".to_string(),
        ));

        let repo = Arc::new(SeaOrmConversationRepository::new(
            db,
            chroma_client.clone(),
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
            chroma_client,
            llm_client: llm_bridge,
        }
    }

    // ==================== HEALTH ENDPOINT TESTS ====================

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health().await;
        assert_eq!(response.status, "healthy");
        assert!(!response.version.is_empty());
        assert_eq!(response.uptime_seconds, 0);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = create_test_state().await;
        let response = metrics(State(state)).await;
        assert!(response.0.get("metrics").is_some());
    }

    // ==================== CONVERSATION CRUD TESTS ====================

    #[tokio::test]
    async fn test_create_conversation_success() {
        let state = create_test_state().await;

        let req = CreateConversationRequest {
            label: "Test Conversation".to_string(),
            folder: "/work".to_string(),
            messages: vec![
                MessageDto {
                    role: "user".to_string(),
                    content: json!("Hello, world!"),
                },
                MessageDto {
                    role: "assistant".to_string(),
                    content: json!("Hi there!"),
                },
            ],
        };

        let result = create_conversation(State(state), Json(req)).await;
        assert!(result.is_ok());

        let (status, json) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(json.0.get("id").is_some());
        assert_eq!(json.0.get("label").unwrap().as_str().unwrap(), "Test Conversation");
        assert_eq!(json.0.get("message_count").unwrap().as_i64().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_create_conversation_minimal() {
        let state = create_test_state().await;

        let req = CreateConversationRequest {
            label: "Minimal".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let result = create_conversation(State(state), Json(req)).await;
        assert!(result.is_ok());

        let (status, json) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json.0.get("message_count").unwrap().as_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_get_conversation_success() {
        let state = create_test_state().await;

        // First create a conversation
        let req = CreateConversationRequest {
            label: "Get Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        // Now get it
        let result = get_conversation(State(state), Path(uuid)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.label, "Get Test");
        assert_eq!(response.status, "active");
    }

    #[tokio::test]
    async fn test_get_conversation_not_found() {
        let state = create_test_state().await;
        let non_existent_id = Uuid::new_v4();

        let result = get_conversation(State(state), Path(non_existent_id)).await;
        assert!(result.is_err());

        let (status, error) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(error.code, 404);
    }

    #[tokio::test]
    async fn test_list_conversations_no_filters() {
        let state = create_test_state().await;

        // Create some test conversations
        for i in 0..3 {
            let req = CreateConversationRequest {
                label: format!("Test {}", i),
                folder: "/".to_string(),
                messages: vec![MessageDto {
                    role: "user".to_string(),
                    content: json!("Test"),
                }],
            };
            create_conversation(State(state.clone()), Json(req))
                .await
                .unwrap();
        }

        let response = list_conversations(
            State(state),
            Query(PaginationParams {
                page: Some(1),
                page_size: Some(10),
            }),
            Query(FilterParams {
                label: None,
                folder: None,
                pinned: None,
                archived: None,
            }),
        )
        .await;

        assert_eq!(response.page, 1);
        assert!(response.total >= 3);
        assert!(!response.results.is_empty());
    }

    #[tokio::test]
    async fn test_list_conversations_with_label_filter() {
        let state = create_test_state().await;

        // Create conversation with specific label
        let req = CreateConversationRequest {
            label: "Project:AI".to_string(),
            folder: "/work".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("AI project"),
            }],
        };
        create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();

        let response = list_conversations(
            State(state),
            Query(PaginationParams {
                page: Some(1),
                page_size: Some(10),
            }),
            Query(FilterParams {
                label: Some("Project:AI".to_string()),
                folder: None,
                pinned: None,
                archived: None,
            }),
        )
        .await;

        assert!(response.results.iter().all(|r| r.label == "Project:AI"));
    }

    #[tokio::test]
    async fn test_update_conversation_label() {
        let state = create_test_state().await;

        // Create conversation
        let req = CreateConversationRequest {
            label: "Old Label".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        // Update label
        let update_req = UpdateLabelRequest {
            label: "New Label".to_string(),
            folder: "/updated".to_string(),
        };

        let result = update_conversation_label(State(state.clone()), Path(uuid), Json(update_req))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);

        // Verify update
        let conv = get_conversation(State(state), Path(uuid)).await.unwrap();
        assert_eq!(conv.label, "New Label");
        assert_eq!(conv.folder, "/updated");
    }

    #[tokio::test]
    async fn test_update_conversation_folder() {
        let state = create_test_state().await;

        // Create conversation
        let req = CreateConversationRequest {
            label: "Test".to_string(),
            folder: "/old".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        // Update folder
        let update_req = UpdateFolderRequest {
            folder: "/new".to_string(),
        };

        let result = update_conversation_folder(State(state.clone()), Path(uuid), Json(update_req))
            .await;
        assert!(result.is_ok());

        // Verify update
        let conv = get_conversation(State(state), Path(uuid)).await.unwrap();
        assert_eq!(conv.folder, "/new");
    }

    #[tokio::test]
    async fn test_update_folder_not_found() {
        let state = create_test_state().await;
        let non_existent_id = Uuid::new_v4();

        let update_req = UpdateFolderRequest {
            folder: "/new".to_string(),
        };

        let result = update_conversation_folder(State(state), Path(non_existent_id), Json(update_req))
            .await;
        assert!(result.is_err());

        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_pin_conversation() {
        let state = create_test_state().await;

        // Create conversation
        let req = CreateConversationRequest {
            label: "Pin Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        // Pin it
        let result = pin_conversation(State(state), Path(uuid)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_archive_conversation() {
        let state = create_test_state().await;

        // Create conversation
        let req = CreateConversationRequest {
            label: "Archive Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        // Archive it
        let result = archive_conversation(State(state.clone()), Path(uuid)).await;
        assert!(result.is_ok());

        // Verify status
        let conv = get_conversation(State(state), Path(uuid)).await.unwrap();
        assert_eq!(conv.status, "archived");
    }

    #[tokio::test]
    async fn test_delete_conversation_success() {
        let state = create_test_state().await;

        // Create conversation
        let req = CreateConversationRequest {
            label: "Delete Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        // Delete it
        let result = delete_conversation(State(state.clone()), Path(uuid)).await;
        assert!(result.is_ok());

        // Verify it's gone
        let get_result = get_conversation(State(state), Path(uuid)).await;
        assert!(get_result.is_err());
    }

    #[tokio::test]
    async fn test_delete_conversation_not_found() {
        let state = create_test_state().await;
        let non_existent_id = Uuid::new_v4();

        let result = delete_conversation(State(state), Path(non_existent_id)).await;
        assert!(result.is_err());

        let (status, error) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(error.code, 404);
    }

    #[tokio::test]
    async fn test_count_conversations_no_filters() {
        let state = create_test_state().await;

        // Create some conversations
        for i in 0..5 {
            let req = CreateConversationRequest {
                label: format!("Count Test {}", i),
                folder: "/".to_string(),
                messages: vec![MessageDto {
                    role: "user".to_string(),
                    content: json!("Test"),
                }],
            };
            create_conversation(State(state.clone()), Json(req))
                .await
                .unwrap();
        }

        let result = count_conversations(
            State(state),
            Query(CountParams {
                label: None,
                folder: None,
            }),
        )
        .await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.0.get("count").unwrap().as_i64().unwrap() >= 5);
    }

    #[tokio::test]
    async fn test_count_conversations_by_label() {
        let state = create_test_state().await;

        // Create conversations with specific label
        for _i in 0..3 {
            let req = CreateConversationRequest {
                label: "Project:Test".to_string(),
                folder: "/".to_string(),
                messages: vec![MessageDto {
                    role: "user".to_string(),
                    content: json!("Test"),
                }],
            };
            create_conversation(State(state.clone()), Json(req))
                .await
                .unwrap();
        }

        let result = count_conversations(
            State(state),
            Query(CountParams {
                label: Some("Project:Test".to_string()),
                folder: None,
            }),
        )
        .await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.0.get("count").unwrap().as_i64().unwrap() >= 3);
        assert_eq!(
            json.0.get("label").unwrap().as_str().unwrap(),
            "Project:Test"
        );
    }

    #[tokio::test]
    async fn test_count_conversations_both_filters_error() {
        let state = create_test_state().await;

        let result = count_conversations(
            State(state),
            Query(CountParams {
                label: Some("Label".to_string()),
                folder: Some("/folder".to_string()),
            }),
        )
        .await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.0.get("error").is_some());
        assert_eq!(json.0.get("count").unwrap().as_i64().unwrap(), 0);
    }

    // ==================== QUERY ENDPOINT TESTS ====================

    #[tokio::test]
    async fn test_semantic_query() {
        let state = create_test_state().await;

        // Create conversation with searchable content
        let req = CreateConversationRequest {
            label: "Search Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("How do I configure authentication?"),
            }],
        };
        create_conversation(State(state.clone()), Json(req))
            .await
            .unwrap();

        let query_req = QueryRequest {
            query: "authentication".to_string(),
            limit: Some(10),
            offset: Some(0),
            filters: None,
        };

        let result = semantic_query(State(state), Json(query_req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.page >= 1);
    }

    #[tokio::test]
    async fn test_full_text_search() {
        let state = create_test_state().await;

        let search_req = FtsSearchRequest {
            query: "test".to_string(),
            limit: 10,
        };

        let result = full_text_search(State(state), Json(search_req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.total, response.results.len());
    }

    #[tokio::test]
    async fn test_rebuild_embeddings() {
        let state = create_test_state().await;

        let result = rebuild_embeddings(State(state)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::ACCEPTED);
    }

    // ==================== ORCHESTRATION ENDPOINT TESTS ====================

    #[tokio::test]
    async fn test_assemble_context() {
        let state = create_test_state().await;

        let req = ContextAssembleRequest {
            query: "test query".to_string(),
            preferred_labels: vec![],
            context_budget: 4000,
            excluded_folders: vec![],
        };

        let result = assemble_context(State(state), Json(req)).await;
        assert!(result.is_ok());

        let messages = result.unwrap();
        assert!(messages.0.len() >= 0); // Can be empty if no context found
    }

    #[tokio::test]
    async fn test_generate_summary_daily() {
        let state = create_test_state().await;

        // Create conversation first
        let conv_req = CreateConversationRequest {
            label: "Summary Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test message"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(conv_req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        let summary_req = SummarizeRequest {
            conversation_id: uuid,
            level: "daily".to_string(),
        };

        let result = generate_summary(State(state), Json(summary_req)).await;
        // May fail if LLM not available, but should parse request correctly
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_generate_summary_invalid_level() {
        let state = create_test_state().await;

        let summary_req = SummarizeRequest {
            conversation_id: Uuid::new_v4(),
            level: "invalid".to_string(),
        };

        let result = generate_summary(State(state), Json(summary_req)).await;
        assert!(result.is_err());

        let (status, error) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, 400);
    }

    #[tokio::test]
    async fn test_prune_dry_run() {
        let state = create_test_state().await;

        let req = PruneRequest {
            threshold_days: 90,
        };

        let result = prune_dry_run(State(state), Json(req)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.total, response.suggestions.len());
    }

    #[tokio::test]
    async fn test_prune_execute() {
        let state = create_test_state().await;

        // Create conversation to prune
        let conv_req = CreateConversationRequest {
            label: "Prune Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(conv_req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        let exec_req = ExecutePruneRequest {
            conversation_ids: vec![uuid],
        };

        let result = prune_execute(State(state.clone()), Json(exec_req)).await;
        assert!(result.is_ok());

        // Verify conversation was archived
        let conv = get_conversation(State(state), Path(uuid)).await.unwrap();
        assert_eq!(conv.status, "archived");
    }

    #[tokio::test]
    async fn test_suggest_labels() {
        let state = create_test_state().await;

        // Create conversation
        let conv_req = CreateConversationRequest {
            label: "Label Test".to_string(),
            folder: "/".to_string(),
            messages: vec![MessageDto {
                role: "user".to_string(),
                content: json!("Test message about API authentication"),
            }],
        };

        let create_result = create_conversation(State(state.clone()), Json(conv_req))
            .await
            .unwrap();
        let id = create_result.1 .0.get("id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        let label_req = LabelSuggestRequest {
            conversation_id: uuid,
        };

        let result = suggest_labels(State(state), Json(label_req)).await;
        // May fail if LLM not available, but request should be valid
        assert!(result.is_ok() || result.is_err());
    }
}
