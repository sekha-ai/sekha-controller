use sekha_controller::services::embedding_queue::{EmbeddingJob, EmbeddingQueue};

#[tokio::test]
async fn test_embedding_queue_creation() {
    let queue = EmbeddingQueue::new();
    // Should create without panic
}

#[tokio::test]
async fn test_embedding_queue_default() {
    let queue = EmbeddingQueue::default();
    // Should create via Default trait
}

#[tokio::test]
async fn test_enqueue_job() {
    let queue = EmbeddingQueue::new();

    let job = EmbeddingJob {
        conversation_id: "test-123".to_string(),
        message_ids: vec!["msg1".to_string(), "msg2".to_string()],
    };

    let result = queue.enqueue(job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_enqueue_multiple_jobs() {
    let queue = EmbeddingQueue::new();

    for i in 0..10 {
        let job = EmbeddingJob {
            conversation_id: format!("conv-{}", i),
            message_ids: vec![format!("msg-{}", i)],
        };

        queue.enqueue(job).await.unwrap();
    }

    // All jobs should be enqueued
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    // Workers process in background
}

#[tokio::test]
async fn test_embedding_job_structure() {
    let job = EmbeddingJob {
        conversation_id: "test".to_string(),
        message_ids: vec!["a".to_string(), "b".to_string()],
    };

    assert_eq!(job.conversation_id, "test");
    assert_eq!(job.message_ids.len(), 2);
}
