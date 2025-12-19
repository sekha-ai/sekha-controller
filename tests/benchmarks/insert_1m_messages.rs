use chrono::Utc;
use sekha_controller::models::internal::{NewConversation, NewMessage};
use sekha_controller::services::embedding_service::EmbeddingService;
use sekha_controller::storage::chroma_client::ChromaClient;
use sekha_controller::storage::{init_db, SeaOrmConversationRepository};
use sekha_controller::ConversationRepository;
use std::sync::Arc;
use tokio::time::Instant;

#[tokio::main]
async fn main() {
    println!("🚀 Starting 1M message benchmark...");

    let db = init_db("sqlite://benchmark.db").await.unwrap();
    let chroma = Arc::new(ChromaClient::new("http://localhost:8000".to_string()));
    let embedding = Arc::new(EmbeddingService::new(
        "http://localhost:11434".to_string(),
        "http://localhost:8000".to_string(),
    ));
    let repo = Arc::new(SeaOrmConversationRepository::new(db, chroma, embedding));

    let start = Instant::now();
    let mut total_messages = 0;

    // FIX: Use a proper iterator with variable in scope
    for conv_idx in 0..10_000 {
        if conv_idx % 100 == 0 {
            println!(
                "Progress: {} conversations, {:.2} msg/sec",
                conv_idx,
                total_messages as f64 / start.elapsed().as_secs_f64()
            );
        }

        // FIX: now must be inside the loop body
        let now = Utc::now().naive_utc();

        // FIX: Collect messages first to calculate word_count
        let messages: Vec<NewMessage> = (0..100)
            .map(|msg_idx| {
                total_messages += 1;
                NewMessage {
                    role: "user".to_string(),
                    content: format!("Benchmark message {} in conversation {}", msg_idx, conv_idx),
                    metadata: serde_json::json!({}),
                    timestamp: now,
                }
            })
            .collect();

        // Calculate word count
        let word_count: i32 = messages.iter().map(|m| m.content.len() as i32).sum();

        let conv = NewConversation {
            id: None,
            label: format!("Bench-{}", conv_idx),
            folder: "/benchmark".to_string(),
            status: "active".to_string(),
            importance_score: Some(5),
            word_count,
            session_count: Some(1),
            created_at: now,
            updated_at: now,
            messages,
        };

        repo.create_with_messages(conv).await.unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "✅ Complete: {} messages in {:.2?}",
        total_messages, elapsed
    );
    println!(
        "📊 Throughput: {:.2} messages/sec",
        total_messages as f64 / elapsed.as_secs_f64()
    );

    // Cleanup
    std::fs::remove_file("benchmark.db").ok();
}
