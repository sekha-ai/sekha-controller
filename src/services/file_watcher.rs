use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::mpsc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::internal::{NewConversation, NewMessage};
use crate::storage::repository::ConversationRepository;
use crate::storage::repository::Stats;

// ============================================
// ChatGPT Export Format
// ============================================

#[derive(Debug, Deserialize)]
struct ChatGptExport {
    title: Option<String>,
    create_time: Option<f64>,
    update_time: Option<f64>,
    mapping: std::collections::HashMap<String, ChatGptNode>,
    #[serde(default)]
    conversation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatGptNode {
    id: String,
    message: Option<ChatGptMessage>,
    parent: Option<String>,
    children: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChatGptMessage {
    id: String,
    author: ChatGptAuthor,
    create_time: Option<f64>,
    content: ChatGptContent,
}

#[derive(Debug, Deserialize)]
struct ChatGptAuthor {
    role: String,
}

#[derive(Debug, Deserialize)]
struct ChatGptContent {
    content_type: String,
    parts: Option<Vec<String>>,
}

// ============================================
// Claude Export Format (XML-based)
// ============================================

#[derive(Debug, Deserialize)]
struct ClaudeExport {
    conversations: Vec<ClaudeConversation>,
}

#[derive(Debug, Deserialize)]
struct ClaudeConversation {
    title: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
    timestamp: Option<String>,
}

// ============================================
// Unified Import Format
// ============================================

#[derive(Debug, Clone)]
struct ParsedConversation {
    title: String,
    messages: Vec<ParsedMessage>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    source: ImportSource,
}

#[derive(Debug, Clone)]
struct ParsedMessage {
    role: String,
    content: String,
    timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ImportSource {
    ChatGPT,
    Claude,
    Unknown,
}

// ============================================
// File Watcher
// ============================================

pub struct ImportWatcher {
    watch_path: PathBuf,
    processor: Arc<ImportProcessor>,
}

impl ImportWatcher {
    pub fn new(watch_path: PathBuf, repo: Arc<dyn ConversationRepository + Send + Sync>) -> Self {
        Self {
            watch_path,
            processor: Arc::new(ImportProcessor::new(repo)),
        }
    }

    pub fn processor(&self) -> Arc<ImportProcessor> {
        self.processor.clone()
    }

    /// Start watching the import directory for new files
    pub async fn watch(&self) -> Result<()> {
        // Ensure directories exist
        self.ensure_directories().await?;

        // Create channel for file events
        let (tx, mut rx) = mpsc::channel(100);

        // Clone for async move
        let processor = self.processor.clone();

        // ✅ CORRECT: Spawn as Tokio task, not std::thread
        let watch_path = self.watch_path.clone();
        tokio::spawn(async move {
            let tx_clone = tx.clone();

            // Use spawn_blocking for the watcher setup (truly blocking operation)
            let watcher_handle = tokio::task::spawn_blocking(move || {
                let mut watcher: RecommendedWatcher = Watcher::new(
                    move |res: Result<Event, notify::Error>| {
                        if let Ok(event) = res {
                            if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                                for path in event.paths {
                                    if matches!(
                                        path.extension().and_then(|s| s.to_str()),
                                        Some("json") | Some("xml") | Some("md") | Some("txt")
                                    ) {
                                        // blocking_send works in any context
                                        let _ = tx_clone.blocking_send(path);
                                    }
                                }
                            }
                        }
                    },
                    notify::Config::default(),
                )
                .expect("Failed to create watcher");

                watcher
                    .watch(&watch_path, RecursiveMode::NonRecursive)
                    .expect("Failed to watch directory");

                tracing::info!("📁 Watching for imports in: {}", watch_path.display());

                // Keep blocking task alive
                std::thread::park();
            });

            // Wait for the blocking task to complete (it won't, by design)
            let _ = watcher_handle.await;
        });

        // Process initial files already in directory
        self.process_existing_files().await?;

        // Process new files as they arrive
        while let Some(path) = rx.recv().await {
            tracing::info!("📥 New file detected: {}", path.display());

            // Small delay to ensure file is fully written
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if let Err(e) = processor.process_file(&path).await {
                tracing::error!("❌ Failed to process {}: {}", path.display(), e);
            }
        }

        Ok(())
    }

    async fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.watch_path).await?;

        let imported_path = self.watch_path.parent().unwrap().join("imported");
        fs::create_dir_all(&imported_path).await?;

        tracing::info!("✅ Import directories ready");
        Ok(())
    }

    async fn process_existing_files(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.watch_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "json" || ext == "xml" {
                        tracing::info!("📄 Processing existing file: {}", path.display());

                        if let Err(e) = self.processor.process_file(&path).await {
                            tracing::error!("❌ Failed to process {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// ============================================
// Import Processor
// ============================================

#[derive(Clone)]
pub struct ImportProcessor {
    repo: Arc<dyn ConversationRepository + Send + Sync>,
}

impl ImportProcessor {
    pub fn new(repo: Arc<dyn ConversationRepository + Send + Sync>) -> Self {
        Self { repo }
    }

    pub fn repo(&self) -> Arc<dyn ConversationRepository> {
        self.repo.clone()
    }

    pub async fn process_file(&self, path: &Path) -> Result<()> {
        tracing::info!("🔍 Processing file: {}", path.display());

        // Read file content
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read file")?;

        // Detect format and parse
        let conversations = self.parse_file(&content, path)?;

        tracing::info!("📊 Found {} conversations", conversations.len());

        // Store each conversation
        let mut imported_count = 0;
        for conv in conversations {
            match self.import_conversation(conv).await {
                Ok(id) => {
                    imported_count += 1;
                    tracing::info!("✅ Imported conversation: {}", id);
                }
                Err(e) => {
                    tracing::error!("❌ Failed to import conversation: {}", e);
                }
            }
        }

        // Move processed file
        self.move_to_imported(path).await?;

        tracing::info!(
            "🎉 Successfully imported {} conversations from {}",
            imported_count,
            path.file_name().unwrap().to_str().unwrap()
        );

        Ok(())
    }

    fn parse_file(&self, content: &str, path: &Path) -> Result<Vec<ParsedConversation>> {
        // Try ChatGPT format first
        if let Ok(chatgpt_data) = serde_json::from_str::<Vec<ChatGptExport>>(content) {
            tracing::info!("🤖 Detected ChatGPT export format (array)");
            return Ok(chatgpt_data
                .into_iter()
                .filter_map(|export| self.parse_chatgpt_export(export).ok())
                .collect());
        }

        // Try single ChatGPT conversation
        if let Ok(chatgpt_export) = serde_json::from_str::<ChatGptExport>(content) {
            tracing::info!("🤖 Detected ChatGPT export format (single)");
            return Ok(vec![self.parse_chatgpt_export(chatgpt_export)?]);
        }

        // Try Claude XML format
        if content.trim_start().starts_with("<?xml")
            || content.trim_start().starts_with("<conversation")
        {
            tracing::info!("🧠 Detected Claude export format");
            return self.parse_claude_export(content);
        }

        // Try Claude JSON format
        if let Ok(claude_export) = serde_json::from_str::<ClaudeExport>(content) {
            tracing::info!("🧠 Detected Claude JSON export format");
            return Ok(claude_export
                .conversations
                .into_iter()
                .filter_map(|conv| self.parse_claude_conversation(conv).ok())
                .collect());
        }

        // Try Markdown format (ChatGPT exports) - ADD THIS
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            tracing::info!("📝 Detected Markdown export format");
            let filename = path.file_name().unwrap().to_str().unwrap();
            return Ok(vec![self.parse_markdown_export(content, filename)?]);
        }

        // Try TXT format (custom) - ADD THIS
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            tracing::info!("📄 Detected TXT export format");
            let filename = path.file_name().unwrap().to_str().unwrap();
            return Ok(vec![self.parse_txt_export(content, filename)?]);
        }

        anyhow::bail!("Unknown export format for file: {}", path.display())
    }

    fn parse_chatgpt_export(&self, export: ChatGptExport) -> Result<ParsedConversation> {
        let title = export
            .title
            .unwrap_or_else(|| "Untitled ChatGPT Conversation".to_string());

        // Build conversation tree from mapping
        let mut messages = Vec::new();

        // Find root node (node with no parent or parent = null)
        let root_id = export
            .mapping
            .iter()
            .find(|(_, node)| node.parent.is_none())
            .map(|(id, _)| id.clone());

        if let Some(root) = root_id {
            self.traverse_chatgpt_tree(&export.mapping, &root, &mut messages);
        }

        let created_at = export
            .create_time
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_else(|| chrono::Utc::now().naive_utc())
            })
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());

        let updated_at = export
            .update_time
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_else(|| chrono::Utc::now().naive_utc())
            })
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());

        Ok(ParsedConversation {
            title,
            messages,
            created_at,
            updated_at,
            source: ImportSource::ChatGPT,
        })
    }

    fn traverse_chatgpt_tree(
        &self,
        mapping: &std::collections::HashMap<String, ChatGptNode>,
        node_id: &str,
        messages: &mut Vec<ParsedMessage>,
    ) {
        if let Some(node) = mapping.get(node_id) {
            // Add message if it exists and has content
            if let Some(msg) = &node.message {
                if let Some(parts) = &msg.content.parts {
                    if !parts.is_empty() {
                        let content = parts.join("\n");

                        // Filter out empty messages
                        if !content.trim().is_empty() {
                            let timestamp = msg
                                .create_time
                                .and_then(|ts| {
                                    chrono::DateTime::from_timestamp(ts as i64, 0)
                                        .map(|dt| dt.naive_utc())
                                })
                                .unwrap_or_else(|| chrono::Utc::now().naive_utc());

                            messages.push(ParsedMessage {
                                role: msg.author.role.clone(),
                                content,
                                timestamp,
                            });
                        }
                    }
                }
            }

            // Traverse children (typically only one active child in ChatGPT exports)
            for child_id in &node.children {
                self.traverse_chatgpt_tree(mapping, child_id, messages);
            }
        }
    }

    fn parse_claude_export(&self, content: &str) -> Result<Vec<ParsedConversation>> {
        // Simple XML parser for Claude format
        // Note: For production, use a proper XML parser like quick-xml

        let mut conversations = Vec::new();

        // Basic XML parsing (simplified)
        if content.contains("<conversation>") {
            // Parse single conversation
            let title = self
                .extract_xml_tag(content, "title")
                .unwrap_or_else(|| "Untitled Claude Conversation".to_string());

            let messages = self.extract_claude_messages_xml(content);

            conversations.push(ParsedConversation {
                title,
                messages,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                source: ImportSource::Claude,
            });
        }

        Ok(conversations)
    }

    fn parse_claude_conversation(&self, conv: ClaudeConversation) -> Result<ParsedConversation> {
        let title = conv
            .title
            .unwrap_or_else(|| "Untitled Claude Conversation".to_string());

        let messages: Vec<ParsedMessage> = conv
            .messages
            .into_iter()
            .map(|msg| {
                let timestamp = msg
                    .timestamp
                    .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_else(|| chrono::Utc::now().naive_utc());

                ParsedMessage {
                    role: msg.role,
                    content: msg.content,
                    timestamp,
                }
            })
            .collect();

        let created_at = conv
            .created_at
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
            .map(|dt| dt.naive_utc())
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());

        let updated_at = conv
            .updated_at
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
            .map(|dt| dt.naive_utc())
            .unwrap_or(created_at);

        Ok(ParsedConversation {
            title,
            messages,
            created_at,
            updated_at,
            source: ImportSource::Claude,
        })
    }

    fn parse_markdown_export(&self, content: &str, filename: &str) -> Result<ParsedConversation> {
        let mut messages = Vec::new();
        let mut current_role = String::new();
        let mut current_content = String::new();

        for line in content.lines() {
            if line.starts_with("# ") {
                // Title line - skip
                continue;
            } else if line.starts_with("## ")
                || line.starts_with("**User:**")
                || line.starts_with("**Assistant:**")
            {
                // Save previous message
                if !current_content.is_empty() {
                    messages.push(ParsedMessage {
                        role: current_role.clone(),
                        content: current_content.trim().to_string(),
                        timestamp: chrono::Utc::now().naive_utc(),
                    });
                }

                // Detect role
                current_role = if line.contains("User") || line.contains("user") {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                };
                current_content = String::new();
            } else if !line.trim().is_empty() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Save last message
        if !current_content.is_empty() && !current_role.is_empty() {  // ADDED: && !current_role.is_empty()
            messages.push(ParsedMessage {
                role: current_role,
                content: current_content.trim().to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
            });
        }

        let title = filename.strip_suffix(".md").unwrap_or(filename).to_string();

        Ok(ParsedConversation {
            title,
            messages,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            source: ImportSource::ChatGPT,
        })
    }

    fn parse_txt_export(&self, content: &str, filename: &str) -> Result<ParsedConversation> {
        // Simple line-by-line parser for custom format
        // Expected format: "User: message" or "Assistant: message"
        let mut messages = Vec::new();
        let mut current_role = String::new();
        let mut current_content = String::new();

        for line in content.lines() {
            if line.starts_with("User:") || line.starts_with("user:") {
                if !current_content.is_empty() {
                    messages.push(ParsedMessage {
                        role: current_role.clone(),
                        content: current_content.trim().to_string(),
                        timestamp: chrono::Utc::now().naive_utc(),
                    });
                }
                current_role = "user".to_string();
                current_content = line
                    .trim_start_matches("User:")
                    .trim_start_matches("user:")
                    .trim()
                    .to_string();
            } else if line.starts_with("Assistant:") || line.starts_with("assistant:") {
                if !current_content.is_empty() {
                    messages.push(ParsedMessage {
                        role: current_role.clone(),
                        content: current_content.trim().to_string(),
                        timestamp: chrono::Utc::now().naive_utc(),
                    });
                }
                current_role = "assistant".to_string();
                current_content = line
                    .trim_start_matches("Assistant:")
                    .trim_start_matches("assistant:")
                    .trim()
                    .to_string();
            } else if !line.trim().is_empty() && !current_role.is_empty() {
                current_content.push('\n');
                current_content.push_str(line);
            }
        }

        // Save last message
        if !current_content.is_empty() {
            messages.push(ParsedMessage {
                role: current_role,
                content: current_content.trim().to_string(),
                timestamp: chrono::Utc::now().naive_utc(),
            });
        }

        let title = filename
            .strip_suffix(".txt")
            .unwrap_or(filename)
            .to_string();

        Ok(ParsedConversation {
            title,
            messages,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            source: ImportSource::Unknown,
        })
    }

    fn extract_xml_tag(&self, content: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start) = content.find(&start_tag) {
            if let Some(end) = content[start..].find(&end_tag) {
                let value = &content[start + start_tag.len()..start + end];
                return Some(value.trim().to_string());
            }
        }
        None
    }

    fn extract_claude_messages_xml(&self, content: &str) -> Vec<ParsedMessage> {
        let mut messages = Vec::new();

        // Simple extraction (for production, use quick-xml crate)
        let parts: Vec<&str> = content.split("<message>").collect();

        for part in parts.iter().skip(1) {
            if let Some(end) = part.find("</message>") {
                let msg_content = &part[..end];

                let role = self
                    .extract_xml_tag(msg_content, "role")
                    .unwrap_or_else(|| "user".to_string());

                let content = self
                    .extract_xml_tag(msg_content, "content")
                    .unwrap_or_default();

                if !content.is_empty() {
                    messages.push(ParsedMessage {
                        role,
                        content,
                        timestamp: chrono::Utc::now().naive_utc(),
                    });
                }
            }
        }

        messages
    }

    async fn import_conversation(&self, parsed: ParsedConversation) -> Result<Uuid> {
        let messages: Vec<NewMessage> = parsed
            .messages
            .into_iter()
            .map(|msg| NewMessage {
                role: msg.role,
                content: msg.content,
                timestamp: msg.timestamp,
                metadata: serde_json::json!({
                    "source": match parsed.source {
                        ImportSource::ChatGPT => "chatgpt",
                        ImportSource::Claude => "claude",
                        ImportSource::Unknown => "unknown",
                    },
                    "imported_at": chrono::Utc::now().to_rfc3339(),
                }),
            })
            .collect();

        let word_count: i32 = messages.iter().map(|m| m.content.len() as i32).sum();

        let new_conv = NewConversation {
            id: Some(Uuid::new_v4()),
            label: parsed.title,
            folder: match parsed.source {
                ImportSource::ChatGPT => "/imports/chatgpt".to_string(),
                ImportSource::Claude => "/imports/claude".to_string(),
                ImportSource::Unknown => "/imports/unknown".to_string(),
            },
            status: "active".to_string(),
            importance_score: Some(5),
            word_count,
            session_count: Some(1),
            created_at: parsed.created_at,
            updated_at: parsed.updated_at,
            messages,
        };

        self.repo
            .create_with_messages(new_conv)
            .await
            .context("Failed to store conversation in database")
    }

    async fn move_to_imported(&self, path: &Path) -> Result<()> {
        let imported_dir = path.parent().unwrap().parent().unwrap().join("imported");
        fs::create_dir_all(&imported_dir).await?;

        let filename = path.file_name().unwrap();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let new_filename = format!("{}_{}", timestamp, filename.to_str().unwrap());
        let new_path = imported_dir.join(new_filename);

        fs::rename(path, &new_path).await?;

        tracing::info!("📦 Moved to: {}", new_path.display());

        Ok(())
    }
}

// ============================================
// Standalone CLI Tool (Optional)
// ============================================

#[cfg(feature = "cli")]
pub async fn run_import_watcher(repo: Arc<dyn ConversationRepository + Send + Sync>) -> Result<()> {
    let home_dir = dirs::home_dir().context("Failed to get home directory")?;
    let watch_path = home_dir.join(".sekha").join("import");

    let watcher = ImportWatcher::new(watch_path, repo);
    watcher.watch().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::internal::{Conversation, Message};
    use crate::storage::repository::{ConversationRepository, RepositoryError, SearchResult};
    use sea_orm::DatabaseConnection;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_parse_chatgpt_json() {
        let json = r#"{
            "title": "Test Conversation",
            "create_time": 1703073600.0,
            "update_time": 1703073600.0,
            "mapping": {
                "root": {
                    "id": "root",
                    "message": null,
                    "parent": null,
                    "children": ["msg1"]
                },
                "msg1": {
                    "id": "msg1",
                    "message": {
                        "id": "msg1",
                        "author": {"role": "user"},
                        "create_time": 1703073600.0,
                        "content": {
                            "content_type": "text",
                            "parts": ["Hello world"]
                        }
                    },
                    "parent": "root",
                    "children": []
                }
            }
        }"#;

        let export: ChatGptExport = serde_json::from_str(json).unwrap();
        assert_eq!(export.title.unwrap(), "Test Conversation");
    }

    #[test]
    fn test_extract_xml_tag() {
        let processor = ImportProcessor::new(Arc::new(MockRepo));
        let xml = "<conversation><title>Test</title></conversation>";

        assert_eq!(
            processor.extract_xml_tag(xml, "title"),
            Some("Test".to_string())
        );
    }

    #[test]
    fn test_traverse_chatgpt_tree_multiple_branches() {
        let processor = ImportProcessor::new(Arc::new(MockRepo));
        let mut mapping = HashMap::new();
        
        // Create tree with multiple children at root
        mapping.insert("root".to_string(), ChatGptNode {
            id: "root".to_string(),
            message: None,
            parent: None,
            children: vec!["msg1".to_string(), "msg2".to_string()],
        });
        
        mapping.insert("msg1".to_string(), ChatGptNode {
            id: "msg1".to_string(),
            message: Some(ChatGptMessage {
                id: "msg1".to_string(),
                author: ChatGptAuthor { role: "user".to_string() },
                create_time: Some(1703073600.0),
                content: ChatGptContent {
                    content_type: "text".to_string(),
                    parts: Some(vec!["Branch 1".to_string()]),
                },
            }),
            parent: Some("root".to_string()),
            children: vec![],
        });
        
        mapping.insert("msg2".to_string(), ChatGptNode {
            id: "msg2".to_string(),
            message: Some(ChatGptMessage {
                id: "msg2".to_string(),
                author: ChatGptAuthor { role: "assistant".to_string() },
                create_time: Some(1703073700.0),
                content: ChatGptContent {
                    content_type: "text".to_string(),
                    parts: Some(vec!["Branch 2".to_string()]),
                },
            }),
            parent: Some("root".to_string()),
            children: vec![],
        });
        
        let mut messages = Vec::new();
        processor.traverse_chatgpt_tree(&mapping, "root", &mut messages);
        
        assert_eq!(messages.len(), 2, "Should traverse both branches");
    }

    #[test]
    fn test_extract_xml_tag_various_formats() {
        let processor = ImportProcessor::new(Arc::new(MockRepo));
        
        // Simple tag - works correctly
        assert_eq!(
            processor.extract_xml_tag("<title>Test</title>", "title"),
            Some("Test".to_string())
        );
        
        // Nested content with spaces - trims correctly
        assert_eq!(
            processor.extract_xml_tag("<content>  Spaces  </content>", "content"),
            Some("Spaces".to_string())
        );
        
        // With attributes - simple parser limitation, cannot match
        // This is expected behavior - our basic string parser doesn't handle attributes
        assert_eq!(
            processor.extract_xml_tag(r#"<title attr="val">Test</title>"#, "title"),
            None  // ✅ CORRECTED: Returns None due to attribute presence
        );
        
        // Not found - returns None
        assert_eq!(
            processor.extract_xml_tag("<other>Test</other>", "title"),
            None
        );
    }

    #[test]
    fn test_parse_markdown_no_role_markers() {
        let processor = ImportProcessor::new(Arc::new(MockRepo));
        let content = r#"# Title only
    No role markers here
    Just plain text"#;

        let result = processor.parse_markdown_export(content, "no_roles.md");
        assert!(result.is_ok());
        
        let conv = result.unwrap();
        assert_eq!(conv.messages.len(), 0, "Should have no messages without role markers");
    }

    #[test]
    fn test_parse_txt_edge_cases() {
        let processor = ImportProcessor::new(Arc::new(MockRepo));
        
        // Empty content
        let result = processor.parse_txt_export("", "empty.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().messages.len(), 0);
        
        // Only malformed lines
        let result = processor.parse_txt_export(
            "No colon here\nAlso no colon\nStill no colon",
            "malformed.txt"
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().messages.len(), 0);
    }

    // Mock repository for tests
    struct MockRepo;

    #[async_trait::async_trait]
    impl ConversationRepository for MockRepo {
        async fn create(&self, _conv: Conversation) -> Result<Uuid, RepositoryError> {
            Ok(Uuid::new_v4())
        }

        async fn create_with_messages(
            &self,
            conv: NewConversation,
        ) -> Result<Uuid, RepositoryError> {
            Ok(conv.id.unwrap_or_else(Uuid::new_v4))
        }

        async fn delete(&self, _id: Uuid) -> Result<(), RepositoryError> {
            Ok(())
        }

        async fn count_by_label(&self, _label: &str) -> Result<u64, RepositoryError> {
            Ok(0)
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<Conversation>, RepositoryError> {
            Ok(None)
        }

        async fn find_by_label(
            &self,
            _label: &str,
            _limit: u64,
            _offset: u64,
        ) -> Result<Vec<Conversation>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn get_message_list(
            &self,
            _conversation_id: Uuid,
        ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
            Ok(vec![]) // Mock implementation
        }

        async fn get_conversation_messages(
            &self,
            _conversation_id: Uuid,
        ) -> Result<Vec<Message>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn find_message_by_id(&self, _id: Uuid) -> Result<Option<Message>, RepositoryError> {
            Ok(None)
        }

        async fn find_recent_messages(
            &self,
            _conversation_id: Uuid,
            _limit: usize,
        ) -> Result<Vec<Message>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn find_with_filters(
            &self,
            _filter: Option<String>,
            _limit: usize,
            _offset: u32,
        ) -> Result<(Vec<Conversation>, u64), RepositoryError> {
            Ok((Vec::new(), 0))
        }

        async fn update_label(
            &self,
            _id: Uuid,
            _new_label: &str,
            _new_folder: &str,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }

        async fn update_status(&self, _id: Uuid, _status: &str) -> Result<(), RepositoryError> {
            Ok(())
        }

        async fn update_importance(&self, _id: Uuid, _score: i32) -> Result<(), RepositoryError> {
            Ok(())
        }

        async fn count_messages_in_conversation(
            &self,
            _conversation_id: Uuid,
        ) -> Result<u64, RepositoryError> {
            Ok(0)
        }

        async fn full_text_search(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<Message>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn semantic_search(
            &self,
            _query: &str,
            _limit: usize,
            _filters: Option<Value>,
        ) -> Result<Vec<SearchResult>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn get_stats(
            &self,
            _folder: Option<String>,
        ) -> Result<Stats, Box<dyn std::error::Error>> {
            Ok(Stats {
                total_conversations: 0,
                average_importance: 0.0,
                group_type: "folder".to_string(),
                groups: vec![],
            })
        }

        async fn get_stats_by_folder(
            &self,
            folder: Option<String>,
        ) -> Result<Stats, Box<dyn std::error::Error>> {
            Ok(Stats {
                total_conversations: 5,
                average_importance: 3.5,
                group_type: "folder".to_string(),
                groups: folder.map(|f| vec![f]).unwrap_or_default(),
            })
        }

        async fn get_stats_by_label(
            &self,
            label: Option<String>,
        ) -> Result<Stats, Box<dyn std::error::Error>> {
            Ok(Stats {
                total_conversations: 3,
                average_importance: 4.0,
                group_type: "folder".to_string(),
                groups: label.map(|l| vec![l]).unwrap_or_default(),
            })
        }

        async fn get_all_labels(&self) -> Result<Vec<String>, RepositoryError> {
            Ok(Vec::new())
        }

        fn get_db(&self) -> &DatabaseConnection {
            panic!("MockRepo::get_db() should not be called in tests")
        }

        async fn find_by_folder(
            &self,
            _folder: &str,
            _limit: u64,
            _offset: u64,
        ) -> Result<Vec<Conversation>, RepositoryError> {
            Ok(Vec::new())
        }

        async fn get_all_folders(&self) -> Result<Vec<String>, RepositoryError> {
            Ok(Vec::new())
        }
    }
}
