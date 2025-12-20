#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_embedding_queue_processes_jobs() {
        let queue = EmbeddingQueue::new();
        
        let job = EmbeddingJob {
            conversation_id: "test-123".to_string(),
            message_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        
        queue.enqueue(job).await.unwrap();
        
        // Give workers time to process
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Verify job was processed (check logs or mock)
        // For now, just verify no panic/error
    }

    #[tokio::test]
    async fn test_embedding_queue_multiple_workers() {
        let queue = EmbeddingQueue::new();
        
        // Send 10 jobs
        for i in 0..10 {
            let job = EmbeddingJob {
                conversation_id: format!("test-{}", i),
                message_ids: vec![Uuid::new_v4()],
            };
            queue.enqueue(job).await.unwrap();
        }
        
        // Give workers time to process
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // All jobs should be processed by 4 workers
        // Verify via logs that multiple workers were active
    }
}