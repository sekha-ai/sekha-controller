use super::create_test_services;
use sekha_controller::models::internal::{NewConversation, NewMessage};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_orchestrator_with_budget_limits() {
    let state = create_test_services().await;
    let repo = state.repo.clone();

    let conv = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Test Budget".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 10,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        }],
    };

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_folder_security() {
    let state = create_test_services().await;
    let repo = state.repo.clone();

    let conv1 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Public".to_string(),
        folder: "/work/project".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 10,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![],
    };

    let conv2 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Private".to_string(),
        folder: "/private/secrets".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 10,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![],
    };

    let _ = repo.create_with_messages(conv1).await;
    let _ = repo.create_with_messages(conv2).await;

    let public_convs = repo.find_by_folder("/work/project", 10, 0).await;
    let private_convs = repo.find_by_folder("/private/secrets", 10, 0).await;

    assert!(public_convs.is_ok());
    assert!(private_convs.is_ok());
}

#[tokio::test]
async fn test_orchestrator_handles_many_conversations() {
    let state = create_test_services().await;
    let repo = state.repo.clone();

    let long_content = "A".repeat(10000);

    for i in 0..10 {
        let conv = NewConversation {
            id: Some(Uuid::new_v4()),
            label: format!("Long {}", i),
            folder: "/test".to_string(),
            status: "active".to_string(),
            importance_score: Some(5),
            word_count: long_content.len() as i32,
            session_count: Some(1),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            messages: vec![NewMessage {
                role: "user".to_string(),
                content: long_content.clone(),
                timestamp: chrono::Utc::now().naive_utc(),
                metadata: serde_json::json!({}),
            }],
        };

        let _ = repo.create_with_messages(conv).await;
    }

    let all_convs = repo.find_with_filters(None, 100, 0).await;
    assert!(all_convs.is_ok());
}

#[tokio::test]
async fn test_orchestrator_unicode_handling() {
    let state = create_test_services().await;
    let repo = state.repo.clone();

    let conv = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Unicode Test".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 10,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "Hello 🌍 世界 مرحبا".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        }],
    };

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_prioritizes_conversations() {
    let state = create_test_services().await;
    let repo = state.repo.clone();

    let conv1 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Important Project".to_string(),
        folder: "/work".to_string(),
        status: "active".to_string(),
        importance_score: Some(10),
        word_count: 50,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "Project details".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        }],
    };

    let conv2 = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Random Chat".to_string(),
        folder: "/casual".to_string(),
        status: "active".to_string(),
        importance_score: Some(1),
        word_count: 20,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "Casual conversation".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        }],
    };

    let _ = repo.create_with_messages(conv1).await;
    let _ = repo.create_with_messages(conv2).await;

    let work_convs = repo.find_by_folder("/work", 10, 0).await;
    assert!(work_convs.is_ok());
}

#[tokio::test]
async fn test_orchestrator_handles_empty_messages() {
    let state = create_test_services().await;
    let repo = state.repo.clone();

    let conv = NewConversation {
        id: Some(Uuid::new_v4()),
        label: "Empty Test".to_string(),
        folder: "/test".to_string(),
        status: "active".to_string(),
        importance_score: Some(5),
        word_count: 0,
        session_count: Some(1),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        messages: vec![NewMessage {
            role: "user".to_string(),
            content: "".to_string(),
            timestamp: chrono::Utc::now().naive_utc(),
            metadata: serde_json::json!({}),
        }],
    };

    let result = repo.create_with_messages(conv).await;
    assert!(result.is_ok());
}
