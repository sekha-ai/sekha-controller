use sekha_controller::api::dto::*;
use serde_json::{json, Value};
use uuid::Uuid;

// ========== ConversationRequest tests ==========

#[test]
fn test_conversation_request_minimal() {
    let req = ConversationRequest {
        label: Some("Test".to_string()),
        folder: None,
        messages: vec![],
        metadata: None,
    };

    assert_eq!(req.label, Some("Test".to_string()));
    assert!(req.folder.is_none());
    assert_eq!(req.messages.len(), 0);
}

#[test]
fn test_conversation_request_full() {
    let req = ConversationRequest {
        label: Some("Full Test".to_string()),
        folder: Some("/work/projects".to_string()),
        messages: vec![MessageRequest {
            role: "user".to_string(),
            content: "Hello".to_string(),
            metadata: None,
        }],
        metadata: Some(json!({"priority": "high"})),
    };

    assert!(req.label.is_some());
    assert!(req.folder.is_some());
    assert_eq!(req.messages.len(), 1);
    assert!(req.metadata.is_some());
}

#[test]
fn test_conversation_request_serialization() {
    let req = ConversationRequest {
        label: Some("Test".to_string()),
        folder: Some("/inbox".to_string()),
        messages: vec![],
        metadata: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("Test"));
    assert!(json.contains("/inbox"));
}

#[test]
fn test_conversation_request_deserialization() {
    let json = r#"{"label":"Test","folder":"/work","messages":[],"metadata":null}"#;
    let req: ConversationRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.label, Some("Test".to_string()));
    assert_eq!(req.folder, Some("/work".to_string()));
}

#[test]
fn test_conversation_request_empty_label() {
    let req = ConversationRequest {
        label: Some("".to_string()),
        folder: None,
        messages: vec![],
        metadata: None,
    };

    assert_eq!(req.label, Some("".to_string()));
}

// ========== MessageRequest tests ==========

#[test]
fn test_message_request_basic() {
    let msg = MessageRequest {
        role: "user".to_string(),
        content: "Hello world".to_string(),
        metadata: None,
    };

    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello world");
    assert!(msg.metadata.is_none());
}

#[test]
fn test_message_request_with_metadata() {
    let msg = MessageRequest {
        role: "assistant".to_string(),
        content: "Response".to_string(),
        metadata: Some(json!({"model": "gpt-4"})),
    };

    assert!(msg.metadata.is_some());
    assert_eq!(msg.metadata.unwrap()["model"], "gpt-4");
}

#[test]
fn test_message_request_all_roles() {
    let roles = vec!["user", "assistant", "system", "function", "tool"];
    for role in roles {
        let msg = MessageRequest {
            role: role.to_string(),
            content: "Test".to_string(),
            metadata: None,
        };
        assert_eq!(msg.role, role);
    }
}

#[test]
fn test_message_request_empty_content() {
    let msg = MessageRequest {
        role: "user".to_string(),
        content: "".to_string(),
        metadata: None,
    };

    assert_eq!(msg.content, "");
}

#[test]
fn test_message_request_long_content() {
    let long_content = "A".repeat(100000);
    let msg = MessageRequest {
        role: "user".to_string(),
        content: long_content.clone(),
        metadata: None,
    };

    assert_eq!(msg.content.len(), 100000);
}

#[test]
fn test_message_request_serialization() {
    let msg = MessageRequest {
        role: "user".to_string(),
        content: "Test message".to_string(),
        metadata: Some(json!({"timestamp": 123456})),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("user"));
    assert!(json.contains("Test message"));
}

// ========== Label tests ==========

#[test]
fn test_label_creation() {
    let label = Label {
        name: "Important".to_string(),
        color: Some("#FF0000".to_string()),
    };

    assert_eq!(label.name, "Important");
    assert_eq!(label.color, Some("#FF0000".to_string()));
}

#[test]
fn test_label_no_color() {
    let label = Label {
        name: "NoColor".to_string(),
        color: None,
    };

    assert!(label.color.is_none());
}

#[test]
fn test_label_serialization() {
    let label = Label {
        name: "Work".to_string(),
        color: Some("#0000FF".to_string()),
    };

    let json = serde_json::to_string(&label).unwrap();
    assert!(json.contains("Work"));
    assert!(json.contains("#0000FF"));
}

// ========== FolderInfo tests ==========

#[test]
fn test_folder_info_basic() {
    let folder = FolderInfo {
        path: "/work/projects".to_string(),
        conversation_count: 5,
    };

    assert_eq!(folder.path, "/work/projects");
    assert_eq!(folder.conversation_count, 5);
}

#[test]
fn test_folder_info_zero_count() {
    let folder = FolderInfo {
        path: "/empty".to_string(),
        conversation_count: 0,
    };

    assert_eq!(folder.conversation_count, 0);
}

#[test]
fn test_folder_info_large_count() {
    let folder = FolderInfo {
        path: "/archive".to_string(),
        conversation_count: 10000,
    };

    assert_eq!(folder.conversation_count, 10000);
}

// ========== MetadataUpdate tests ==========

#[test]
fn test_metadata_update_basic() {
    let update = MetadataUpdate {
        metadata: json!({"key": "value"}),
    };

    assert_eq!(update.metadata["key"], "value");
}

#[test]
fn test_metadata_update_complex() {
    let update = MetadataUpdate {
        metadata: json!({
            "tags": ["important", "work"],
            "priority": 5,
            "nested": {"field": "value"}
        }),
    };

    assert!(update.metadata.is_object());
    assert_eq!(update.metadata["priority"], 5);
}

#[test]
fn test_metadata_update_empty() {
    let update = MetadataUpdate {
        metadata: json!({}),
    };

    assert!(update.metadata.as_object().unwrap().is_empty());
}

// ========== SearchRequest tests ==========

#[test]
fn test_search_request_minimal() {
    let req = SearchRequest {
        query: "test search".to_string(),
        limit: None,
        folder: None,
        preferred_labels: None,
        context_budget: None,
    };

    assert_eq!(req.query, "test search");
    assert!(req.limit.is_none());
}

#[test]
fn test_search_request_with_limit() {
    let req = SearchRequest {
        query: "search".to_string(),
        limit: Some(10),
        folder: None,
        preferred_labels: None,
        context_budget: None,
    };

    assert_eq!(req.limit, Some(10));
}

#[test]
fn test_search_request_with_folder() {
    let req = SearchRequest {
        query: "search".to_string(),
        limit: None,
        folder: Some("/work".to_string()),
        preferred_labels: None,
        context_budget: None,
    };

    assert_eq!(req.folder, Some("/work".to_string()));
}

#[test]
fn test_search_request_with_preferred_labels() {
    let req = SearchRequest {
        query: "search".to_string(),
        limit: None,
        folder: None,
        preferred_labels: Some(vec!["important".to_string(), "urgent".to_string()]),
        context_budget: None,
    };

    assert_eq!(req.preferred_labels.as_ref().unwrap().len(), 2);
}

#[test]
fn test_search_request_with_context_budget() {
    let req = SearchRequest {
        query: "search".to_string(),
        limit: None,
        folder: None,
        preferred_labels: None,
        context_budget: Some("LARGE".to_string()),
    };

    assert_eq!(req.context_budget, Some("LARGE".to_string()));
}

#[test]
fn test_search_request_full() {
    let req = SearchRequest {
        query: "comprehensive search".to_string(),
        limit: Some(50),
        folder: Some("/archive".to_string()),
        preferred_labels: Some(vec!["tag1".to_string()]),
        context_budget: Some("MEDIUM".to_string()),
    };

    assert!(req.limit.is_some());
    assert!(req.folder.is_some());
    assert!(req.preferred_labels.is_some());
    assert!(req.context_budget.is_some());
}

#[test]
fn test_search_request_empty_query() {
    let req = SearchRequest {
        query: "".to_string(),
        limit: None,
        folder: None,
        preferred_labels: None,
        context_budget: None,
    };

    assert_eq!(req.query, "");
}

// ========== ProviderInfo tests ==========

#[test]
fn test_provider_info_basic() {
    let info = ProviderInfo {
        id: "ollama_local".to_string(),
        provider_type: "ollama".to_string(),
        base_url: "http://localhost:11434".to_string(),
        priority: 1,
        model_count: 3,
    };

    assert_eq!(info.id, "ollama_local");
    assert_eq!(info.priority, 1);
    assert_eq!(info.model_count, 3);
}

#[test]
fn test_provider_info_different_types() {
    let types = vec!["ollama", "openai", "anthropic", "litellm", "openrouter"];
    for ptype in types {
        let info = ProviderInfo {
            id: format!("{}_instance", ptype),
            provider_type: ptype.to_string(),
            base_url: "http://localhost".to_string(),
            priority: 1,
            model_count: 5,
        };
        assert_eq!(info.provider_type, ptype);
    }
}

#[test]
fn test_provider_info_serialization() {
    let info = ProviderInfo {
        id: "test".to_string(),
        provider_type: "openai".to_string(),
        base_url: "https://api.openai.com".to_string(),
        priority: 2,
        model_count: 10,
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("openai"));
}

// ========== ProviderListResponse tests ==========

#[test]
fn test_provider_list_response_empty() {
    let response = ProviderListResponse {
        providers: vec![],
        total_count: 0,
    };

    assert_eq!(response.providers.len(), 0);
    assert_eq!(response.total_count, 0);
}

#[test]
fn test_provider_list_response_single() {
    let response = ProviderListResponse {
        providers: vec![ProviderInfo {
            id: "test".to_string(),
            provider_type: "ollama".to_string(),
            base_url: "http://localhost:11434".to_string(),
            priority: 1,
            model_count: 5,
        }],
        total_count: 1,
    };

    assert_eq!(response.providers.len(), 1);
    assert_eq!(response.total_count, 1);
}

#[test]
fn test_provider_list_response_multiple() {
    let response = ProviderListResponse {
        providers: vec![
            ProviderInfo {
                id: "provider1".to_string(),
                provider_type: "ollama".to_string(),
                base_url: "http://localhost:11434".to_string(),
                priority: 1,
                model_count: 3,
            },
            ProviderInfo {
                id: "provider2".to_string(),
                provider_type: "openai".to_string(),
                base_url: "https://api.openai.com".to_string(),
                priority: 2,
                model_count: 10,
            },
        ],
        total_count: 2,
    };

    assert_eq!(response.providers.len(), 2);
    assert_eq!(response.total_count, 2);
}

#[test]
fn test_provider_list_response_serialization() {
    let response = ProviderListResponse {
        providers: vec![ProviderInfo {
            id: "test".to_string(),
            provider_type: "ollama".to_string(),
            base_url: "http://localhost:11434".to_string(),
            priority: 1,
            model_count: 5,
        }],
        total_count: 1,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("providers"));
    assert!(json.contains("total_count"));
}

// ========== Integration/round-trip tests ==========

#[test]
fn test_conversation_request_round_trip() {
    let original = ConversationRequest {
        label: Some("Test".to_string()),
        folder: Some("/work".to_string()),
        messages: vec![MessageRequest {
            role: "user".to_string(),
            content: "Hello".to_string(),
            metadata: Some(json!({"test": true})),
        }],
        metadata: Some(json!({"version": 1})),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ConversationRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(original.label, deserialized.label);
    assert_eq!(original.folder, deserialized.folder);
    assert_eq!(original.messages.len(), deserialized.messages.len());
}

#[test]
fn test_search_request_round_trip() {
    let original = SearchRequest {
        query: "test".to_string(),
        limit: Some(10),
        folder: Some("/inbox".to_string()),
        preferred_labels: Some(vec!["tag".to_string()]),
        context_budget: Some("SMALL".to_string()),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(original.query, deserialized.query);
    assert_eq!(original.limit, deserialized.limit);
    assert_eq!(original.folder, deserialized.folder);
}
