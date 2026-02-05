//! Message validation to prevent image storage
//!
//! Vision support in v2.0 is pass-through only - images are processed
//! by LLM providers but never stored in the database.

use crate::models::internal::Message;

/// Error type for validation failures
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Images cannot be stored in messages. Images are processed in real-time but not persisted.")]
    ImageStorageNotAllowed,
}

/// Validate that a message does not contain images before storage
///
/// # Arguments
/// * `message` - The message to validate
///
/// # Returns
/// * `Ok(())` if valid (no images)
/// * `Err(ValidationError::ImageStorageNotAllowed)` if images detected
///
/// # Example
/// ```ignore
/// use sekha_controller::validation::validate_no_images;
/// use sekha_controller::models::internal::Message;
///
/// let message = Message {
///     id: uuid::Uuid::new_v4(),
///     conversation_id: uuid::Uuid::new_v4(),
///     role: "user".to_string(),
///     content: "Hello".to_string(),
///     metadata: None,
///     ..Default::default()
/// };
///
/// assert!(validate_no_images(&message).is_ok());
/// ```
pub fn validate_no_images(message: &Message) -> Result<(), ValidationError> {
    // Check message metadata for image indicators
    if let Some(metadata) = &message.metadata {
        // Check for common image-related keys
        if metadata.contains_key("images")
            || metadata.contains_key("image_urls")
            || metadata.contains_key("attachments")
        {
            return Err(ValidationError::ImageStorageNotAllowed);
        }

        // Check if metadata suggests image content
        if let Some(content_type) = metadata.get("content_type") {
            if content_type.as_str().map_or(false, |s| s.starts_with("image/")) {
                return Err(ValidationError::ImageStorageNotAllowed);
            }
        }
    }

    // Check content for base64-encoded images
    if message.content.contains("data:image/") {
        return Err(ValidationError::ImageStorageNotAllowed);
    }

    // Check content for image URLs (heuristic)
    // This is a best-effort check - not foolproof but catches common cases
    let content_lower = message.content.to_lowercase();
    if (content_lower.contains("http://") || content_lower.contains("https://"))
        && (content_lower.contains(".jpg")
            || content_lower.contains(".jpeg")
            || content_lower.contains(".png")
            || content_lower.contains(".gif")
            || content_lower.contains(".webp"))
    {
        // This might be a false positive for legitimate text containing image URLs
        // Only flag if it looks like structured image data
        if content_lower.contains("image_url") || content_lower.contains("\"url\":") {
            return Err(ValidationError::ImageStorageNotAllowed);
        }
    }

    Ok(())
}

/// Strip any image-related data from a message before storage
///
/// This is a defensive measure to ensure no image data leaks into storage.
///
/// # Arguments
/// * `message` - Mutable reference to the message to sanitize
///
/// # Returns
/// * `true` if any data was stripped, `false` if message was clean
///
/// # Example
/// ```ignore
/// let mut message = create_message();
/// let was_modified = strip_images(&mut message);
/// if was_modified {
///     log::warn!("Image data was stripped from message before storage");
/// }
/// ```
pub fn strip_images(message: &mut Message) -> bool {
    let mut modified = false;

    // Remove image-related metadata keys
    if let Some(metadata) = &mut message.metadata {
        let keys_to_remove = ["images", "image_urls", "attachments"];
        for key in &keys_to_remove {
            if metadata.remove(*key).is_some() {
                modified = true;
            }
        }

        // Check and remove image content_type
        if let Some(content_type) = metadata.get("content_type") {
            if content_type.as_str().map_or(false, |s| s.starts_with("image/")) {
                metadata.remove("content_type");
                modified = true;
            }
        }
    }

    // Strip base64 images from content (replace with placeholder)
    if message.content.contains("data:image/") {
        // Replace base64 image data with placeholder
        let re = regex::Regex::new(r"data:image/[^;]+;base64,[A-Za-z0-9+/=]+").unwrap();
        let new_content = re.replace_all(&message.content, "[IMAGE_REMOVED]");
        if new_content != message.content {
            message.content = new_content.to_string();
            modified = true;
        }
    }

    modified
}

/// Validate a batch of messages
///
/// # Arguments
/// * `messages` - Slice of messages to validate
///
/// # Returns
/// * `Ok(())` if all valid
/// * `Err(ValidationError)` on first validation failure
pub fn validate_messages_no_images(messages: &[Message]) -> Result<(), ValidationError> {
    for message in messages {
        validate_no_images(message)?;
    }
    Ok(())
}

/// Strip images from a batch of messages
///
/// # Arguments
/// * `messages` - Mutable slice of messages to sanitize
///
/// # Returns
/// * Number of messages that were modified
pub fn strip_images_from_messages(messages: &mut [Message]) -> usize {
    messages.iter_mut().filter(|m| strip_images(m)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn create_test_message(content: String) -> Message {
        Message {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            role: "user".to_string(),
            content,
            timestamp: chrono::Utc::now(),
            metadata: None,
            embedding_id: None,
            importance_score: None,
        }
    }

    #[test]
    fn test_validate_text_only_message() {
        let message = create_test_message("Hello, how are you?".to_string());
        assert!(validate_no_images(&message).is_ok());
    }

    #[test]
    fn test_reject_base64_image() {
        let message = create_test_message(
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==".to_string(),
        );
        assert!(validate_no_images(&message).is_err());
    }

    #[test]
    fn test_reject_message_with_images_metadata() {
        let mut message = create_test_message("Test".to_string());
        message.metadata = Some(json!({
            "images": ["https://example.com/image.jpg"]
        }));
        assert!(validate_no_images(&message).is_err());
    }

    #[test]
    fn test_strip_images_from_metadata() {
        let mut message = create_test_message("Test".to_string());
        message.metadata = Some(json!({
            "images": ["https://example.com/image.jpg"],
            "other_data": "keep this"
        }));

        let was_modified = strip_images(&mut message);
        assert!(was_modified);
        assert!(message.metadata.as_ref().unwrap().get("images").is_none());
        assert!(message
            .metadata
            .as_ref()
            .unwrap()
            .get("other_data")
            .is_some());
    }

    #[test]
    fn test_strip_base64_from_content() {
        let mut message = create_test_message(
            "Here is an image: data:image/png;base64,iVBORw0KGgoAAAANSUhEUg== and more text"
                .to_string(),
        );

        let was_modified = strip_images(&mut message);
        assert!(was_modified);
        assert!(message.content.contains("[IMAGE_REMOVED]"));
        assert!(!message.content.contains("data:image/"));
        assert!(message.content.contains("and more text"));
    }

    #[test]
    fn test_batch_validation() {
        let messages = vec![
            create_test_message("Message 1".to_string()),
            create_test_message("Message 2".to_string()),
        ];

        assert!(validate_messages_no_images(&messages).is_ok());

        let mut messages_with_image = messages;
        messages_with_image.push(create_test_message(
            "data:image/png;base64,abc".to_string(),
        ));

        assert!(validate_messages_no_images(&messages_with_image).is_err());
    }

    #[test]
    fn test_batch_stripping() {
        let mut messages = vec![
            create_test_message("Clean message".to_string()),
            create_test_message("data:image/png;base64,test".to_string()),
            create_test_message("Another clean".to_string()),
        ];

        let modified_count = strip_images_from_messages(&mut messages);
        assert_eq!(modified_count, 1);
        assert!(!messages[1].content.contains("data:image/"));
    }
}
