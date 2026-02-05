use sekha_controller::{models::internal::Message, validation::*};
use serde_json::json;
use uuid::Uuid;

fn create_message_with_metadata(role: &str, content: &str, metadata: serde_json::Value) -> Message {
    Message {
        id: Uuid::new_v4(),
        conversation_id: Uuid::new_v4(),
        role: role.to_string(),
        content: content.to_string(),
        metadata: Some(metadata),
        timestamp: chrono::Local::now().naive_local(),
        embedding_id: None,
    }
}

fn create_message_no_metadata(role: &str, content: &str) -> Message {
    Message {
        id: Uuid::new_v4(),
        conversation_id: Uuid::new_v4(),
        role: role.to_string(),
        content: content.to_string(),
        metadata: None,
        timestamp: chrono::Local::now().naive_local(),
        embedding_id: None,
    }
}

// ========== validate_no_images tests ==========

#[test]
fn test_validate_no_images_empty_messages() {
    let messages: Vec<Message> = vec![];
    assert!(validate_no_images(&messages).is_ok());
}

#[test]
fn test_validate_no_images_no_metadata() {
    let messages = vec![create_message_no_metadata("user", "Hello")];
    assert!(validate_no_images(&messages).is_ok());
}

#[test]
fn test_validate_no_images_empty_metadata() {
    let messages = vec![create_message_with_metadata("user", "Hello", json!({}))];
    assert!(validate_no_images(&messages).is_ok());
}

#[test]
fn test_validate_no_images_with_images_key() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Check this out",
        json!({"images": ["data:image/png;base64,abc123"]}),
    )];
    assert!(validate_no_images(&messages).is_err());
}

#[test]
fn test_validate_no_images_with_image_urls_key() {
    let messages = vec![create_message_with_metadata(
        "user",
        "See here",
        json!({"image_urls": ["https://example.com/image.jpg"]}),
    )];
    assert!(validate_no_images(&messages).is_err());
}

#[test]
fn test_validate_no_images_with_attachments_key() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Attachment",
        json!({"attachments": [{"type": "image", "url": "https://example.com/file.png"}]}),
    )];
    assert!(validate_no_images(&messages).is_err());
}

#[test]
fn test_validate_no_images_multiple_messages_all_clean() {
    let messages = vec![
        create_message_with_metadata("user", "Hello", json!({"type": "text"})),
        create_message_with_metadata("assistant", "Hi there", json!({"response": true})),
        create_message_no_metadata("user", "How are you?"),
    ];
    assert!(validate_no_images(&messages).is_ok());
}

#[test]
fn test_validate_no_images_multiple_messages_one_with_images() {
    let messages = vec![
        create_message_with_metadata("user", "Hello", json!({"type": "text"})),
        create_message_with_metadata("user", "Check this", json!({"images": ["img1"]})),
        create_message_with_metadata("assistant", "Got it", json!({})),
    ];
    assert!(validate_no_images(&messages).is_err());
}

#[test]
fn test_validate_no_images_non_object_metadata() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!("string_metadata"),
    )];
    // Should pass - non-object metadata doesn't contain images
    assert!(validate_no_images(&messages).is_ok());
}

#[test]
fn test_validate_no_images_array_metadata() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!([1, 2, 3]),
    )];
    // Should pass - array metadata doesn't contain images
    assert!(validate_no_images(&messages).is_ok());
}

#[test]
fn test_validate_no_images_null_metadata() {
    let messages = vec![create_message_with_metadata("user", "Hello", json!(null))];
    assert!(validate_no_images(&messages).is_ok());
}

// ========== strip_images tests ==========

#[test]
fn test_strip_images_empty_messages() {
    let messages: Vec<Message> = vec![];
    let stripped = strip_images(&messages);
    assert_eq!(stripped.len(), 0);
}

#[test]
fn test_strip_images_no_images_to_strip() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({"key": "value"}),
    )];
    let stripped = strip_images(&messages);
    assert_eq!(stripped.len(), 1);
    assert_eq!(stripped[0].content, "Hello");
}

#[test]
fn test_strip_images_removes_images_key() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({"images": ["img1", "img2"], "other": "keep"}),
    )];
    let stripped = strip_images(&messages);

    assert_eq!(stripped.len(), 1);
    if let Some(ref metadata) = stripped[0].metadata {
        assert!(!metadata.as_object().unwrap().contains_key("images"));
        assert_eq!(metadata["other"], "keep");
    } else {
        panic!("Expected metadata");
    }
}

#[test]
fn test_strip_images_removes_image_urls_key() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({"image_urls": ["url1"], "data": "preserve"}),
    )];
    let stripped = strip_images(&messages);

    if let Some(ref metadata) = stripped[0].metadata {
        assert!(!metadata.as_object().unwrap().contains_key("image_urls"));
        assert_eq!(metadata["data"], "preserve");
    }
}

#[test]
fn test_strip_images_removes_attachments_key() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({"attachments": [{"file": "test.png"}], "text": "keep"}),
    )];
    let stripped = strip_images(&messages);

    if let Some(ref metadata) = stripped[0].metadata {
        assert!(!metadata.as_object().unwrap().contains_key("attachments"));
        assert_eq!(metadata["text"], "keep");
    }
}

#[test]
fn test_strip_images_removes_all_image_keys() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({
            "images": ["img"],
            "image_urls": ["url"],
            "attachments": ["file"],
            "normal_field": "keep_this"
        }),
    )];
    let stripped = strip_images(&messages);

    if let Some(ref metadata) = stripped[0].metadata {
        let obj = metadata.as_object().unwrap();
        assert!(!obj.contains_key("images"));
        assert!(!obj.contains_key("image_urls"));
        assert!(!obj.contains_key("attachments"));
        assert_eq!(metadata["normal_field"], "keep_this");
    }
}

#[test]
fn test_strip_images_removes_content_type_image() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({"content_type": "image", "other": "data"}),
    )];
    let stripped = strip_images(&messages);

    if let Some(ref metadata) = stripped[0].metadata {
        let obj = metadata.as_object().unwrap();
        assert!(!obj.contains_key("content_type"));
        assert_eq!(metadata["other"], "data");
    }
}

#[test]
fn test_strip_images_keeps_content_type_non_image() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!({"content_type": "text"}),
    )];
    let stripped = strip_images(&messages);

    if let Some(ref metadata) = stripped[0].metadata {
        assert_eq!(metadata["content_type"], "text");
    }
}

#[test]
fn test_strip_images_multiple_messages() {
    let messages = vec![
        create_message_with_metadata("user", "Msg1", json!({"images": ["img1"]})),
        create_message_with_metadata("user", "Msg2", json!({"normal": "field"})),
        create_message_with_metadata("user", "Msg3", json!({"image_urls": ["url"]})),
    ];
    let stripped = strip_images(&messages);

    assert_eq!(stripped.len(), 3);
    // First message should have images stripped
    if let Some(ref metadata) = stripped[0].metadata {
        assert!(!metadata.as_object().unwrap().contains_key("images"));
    }
    // Second message unchanged
    if let Some(ref metadata) = stripped[1].metadata {
        assert_eq!(metadata["normal"], "field");
    }
    // Third message should have image_urls stripped
    if let Some(ref metadata) = stripped[2].metadata {
        assert!(!metadata.as_object().unwrap().contains_key("image_urls"));
    }
}

#[test]
fn test_strip_images_non_object_metadata() {
    let messages = vec![create_message_with_metadata(
        "user",
        "Hello",
        json!("string"),
    )];
    let stripped = strip_images(&messages);
    assert_eq!(stripped.len(), 1);
    // Non-object metadata should remain unchanged
    assert_eq!(stripped[0].metadata, Some(json!("string")));
}

// ========== strip_images_mut tests ==========

#[test]
fn test_strip_images_mut_no_metadata() {
    let mut message = create_message_no_metadata("user", "Hello");
    let had_images = strip_images_mut(&mut message);
    assert_eq!(had_images, false);
}

#[test]
fn test_strip_images_mut_no_images() {
    let mut message = create_message_with_metadata("user", "Hello", json!({"key": "value"}));
    let had_images = strip_images_mut(&mut message);
    assert_eq!(had_images, false);
}

#[test]
fn test_strip_images_mut_has_images() {
    let mut message =
        create_message_with_metadata("user", "Hello", json!({"images": ["img1"], "keep": "this"}));
    let had_images = strip_images_mut(&mut message);

    assert_eq!(had_images, true);
    if let Some(ref metadata) = message.metadata {
        assert!(!metadata.as_object().unwrap().contains_key("images"));
        assert_eq!(metadata["keep"], "this");
    }
}

#[test]
fn test_strip_images_mut_has_image_urls() {
    let mut message = create_message_with_metadata("user", "Hello", json!({"image_urls": ["url"]}));
    let had_images = strip_images_mut(&mut message);
    assert_eq!(had_images, true);
}

#[test]
fn test_strip_images_mut_has_attachments() {
    let mut message =
        create_message_with_metadata("user", "Hello", json!({"attachments": ["file"]}));
    let had_images = strip_images_mut(&mut message);
    assert_eq!(had_images, true);
}

// ========== count_stripped_images tests ==========

#[test]
fn test_count_stripped_images_none() {
    let mut messages = vec![
        create_message_with_metadata("user", "Hello", json!({"text": "data"})),
        create_message_no_metadata("assistant", "Hi"),
    ];
    let count = count_stripped_images(&mut messages);
    assert_eq!(count, 0);
}

#[test]
fn test_count_stripped_images_one() {
    let mut messages = vec![
        create_message_with_metadata("user", "Msg1", json!({"images": ["img"]})),
        create_message_with_metadata("user", "Msg2", json!({"normal": "data"})),
    ];
    let count = count_stripped_images(&mut messages);
    assert_eq!(count, 1);
    // Verify images were actually stripped
    if let Some(ref metadata) = messages[0].metadata {
        assert!(!metadata.as_object().unwrap().contains_key("images"));
    }
}

#[test]
fn test_count_stripped_images_multiple() {
    let mut messages = vec![
        create_message_with_metadata("user", "Msg1", json!({"images": ["img1"]})),
        create_message_with_metadata("user", "Msg2", json!({"image_urls": ["url"]})),
        create_message_with_metadata("user", "Msg3", json!({"normal": "data"})),
        create_message_with_metadata("user", "Msg4", json!({"attachments": ["file"]})),
    ];
    let count = count_stripped_images(&mut messages);
    assert_eq!(count, 3);
}

#[test]
fn test_count_stripped_images_all() {
    let mut messages = vec![
        create_message_with_metadata("user", "Msg1", json!({"images": ["img"]})),
        create_message_with_metadata("user", "Msg2", json!({"image_urls": ["url"]})),
    ];
    let count = count_stripped_images(&mut messages);
    assert_eq!(count, 2);
}

#[test]
fn test_count_stripped_images_empty() {
    let mut messages: Vec<Message> = vec![];
    let count = count_stripped_images(&mut messages);
    assert_eq!(count, 0);
}

// ========== create_test_message tests ==========

#[test]
fn test_create_test_message_basic() {
    let message = create_test_message("user", "Test content");
    assert_eq!(message.role, "user");
    assert_eq!(message.content, "Test content");
    assert!(message.metadata.is_some());
    assert!(message.embedding_id.is_none());
}

#[test]
fn test_create_test_message_different_roles() {
    let roles = vec!["user", "assistant", "system", "function"];
    for role in roles {
        let message = create_test_message(role, "Content");
        assert_eq!(message.role, role);
    }
}

#[test]
fn test_create_test_message_empty_content() {
    let message = create_test_message("user", "");
    assert_eq!(message.content, "");
}

#[test]
fn test_create_test_message_long_content() {
    let long_content = "A".repeat(10000);
    let message = create_test_message("user", &long_content);
    assert_eq!(message.content.len(), 10000);
}

#[test]
fn test_create_test_message_unique_ids() {
    let msg1 = create_test_message("user", "Content");
    let msg2 = create_test_message("user", "Content");
    assert_ne!(msg1.id, msg2.id);
    assert_ne!(msg1.conversation_id, msg2.conversation_id);
}

// ========== ValidationError tests ==========

#[test]
fn test_validation_error_images_not_allowed() {
    let error = ValidationError::ImagesNotAllowed;
    assert_eq!(error.to_string(), "Images are not allowed in messages");
}

#[test]
fn test_validation_error_invalid_metadata() {
    let error = ValidationError::InvalidMetadata;
    assert_eq!(error.to_string(), "Invalid metadata format");
}
