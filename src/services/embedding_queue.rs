use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

pub struct EmbeddingJob {
    pub conversation_id: String,
    pub message_ids: Vec<String>,
}

pub struct EmbeddingQueue {
    sender: mpsc::Sender<EmbeddingJob>,
}

impl EmbeddingQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<EmbeddingJob>(100);
        let receiver = Arc::new(tokio::sync::Mutex::new(receiver));

        for worker_id in 0..4 {
            let rx = receiver.clone();
            tokio::spawn(async move {
                info!("Embedding worker {} started", worker_id);
                loop {
                    let mut lock = rx.lock().await;
                    match lock.recv().await {
                        Some(job) => {
                            drop(lock);
                            info!(
                                "Worker {} processing job for conversation {}",
                                worker_id, job.conversation_id
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        None => break,
                    }
                }
            });
        }

        Self { sender }
    }

    pub async fn enqueue(
        &self,
        job: EmbeddingJob,
    ) -> Result<(), mpsc::error::SendError<EmbeddingJob>> {
        self.sender.send(job).await?;
        Ok(())
    }
}

impl Default for EmbeddingQueue {
    fn default() -> Self {
        Self::new()
    }
}
