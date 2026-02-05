use crate::models::internal::Message;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Images are not allowed in messages")]
    ImagesNotAllowed,
    #[error("Invalid metadata format")]
    InvalidMetadata,
}

/// Validate that messages do not contain images
pub fn validate_no_images(messages: &[Message]) -> Result<(), ValidationError> {
    for message in messages {
        if has_image_metadata(&message.metadata) {
            return Err(ValidationError::ImagesNotAllowed);
        }
    }
    Ok(())
}

/// Check if metadata contains image-related fields
fn has_image_metadata(metadata: &Value) -> bool {
    if let Some(obj) = metadata.as_object() {
        obj.contains_key("images")
            || obj.contains_key("image_urls")
            || obj.contains_key("attachments")
    } else {
        false
    }
}

/// Strip images from messages and return modified copies
pub fn strip_images(messages: &[Message]) -> Vec<Message> {
    messages
        .iter()
        .map(|m| {
            let mut new_message = m.clone();
            strip_image_metadata(&mut new_message.metadata);
            new_message
        })
        .collect()
}

/// Remove image-related fields from metadata
fn strip_image_metadata(metadata: &mut Value) {
    if let Some(obj) = metadata.as_object_mut() {
        obj.remove("images");
        obj.remove("image_urls");
        obj.remove("attachments");
        
        // Also remove data URIs from content if present
        if let Some(content_type) = obj.get("content_type") {
            if content_type.as_str() == Some("image") {
                obj.remove("content_type");
            }
        }
    }
}

/// Strip images from a single mutable message
pub fn strip_images_mut(message: &mut Message) -> bool {
    let had_images = has_image_metadata(&message.metadata);
    if had_images {
        strip_image_metadata(&mut message.metadata);
    }
    had_images
}

/// Count how many messages have images stripped
pub fn count_stripped_images(messages: &mut [Message]) -> usize {
    messages.iter_mut().filter(|m| strip_images_mut(m)).count()
}

/// Create a test message with no images
pub fn create_test_message(role: &str, content: &str) -> Message {
    Message {
        id: uuid::Uuid::new_v4(),
        conversation_id: uuid::Uuid::new_v4(),
        role: role.to_string(),
        content: content.to_string(),
        metadata: serde_json::json!({}),
        timestamp: chrono::Local::now().naive_local(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_no_images_pass() {
        let messages = vec![create_test_message("user", "Hello")];
        assert!(validate_no_images(&messages).is_ok());
    }

    #[test]
    fn test_validate_no_images_fail() {
        let mut message = create_test_message("user", "Hello");
        message.metadata = json!({"images": ["data:image/png;base64,abc"]});
        let messages = vec![message];
        assert!(validate_no_images(&messages).is_err());
    }

    #[test]
    fn test_strip_images() {
        let mut message = create_test_message("user", "Hello");
        message.metadata = json!({
            "images": ["data:image/png;base64,abc"],
            "other_field": "keep_this"
        });
        let messages = vec![message];
        let stripped = strip_images(&messages);
        
        assert_eq!(stripped.len(), 1);
        assert!(!has_image_metadata(&stripped[0].metadata));
        assert_eq!(stripped[0].metadata["other_field"], "keep_this");
    }

    #[test]
    fn test_count_stripped() {
        let mut messages = vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi"),
        ];
        messages[0].metadata = json!({"images": ["data:image/png;base64,abc"]});
        
        let count = count_stripped_images(&mut messages);
        assert_eq!(count, 1);
        assert!(!has_image_metadata(&messages[0].metadata));
    }
}
