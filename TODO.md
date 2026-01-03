```markdown
# Sekha Controller - TODO for MCP Export & Stats Tools

## Overview
The MCP server (sekha-mcp) now supports 7 tools (store, search, update, context, prune, export, stats). The controller needs repository methods and API endpoints to support the 2 new tools: `memory_export` and `memory_stats`.

---

## 1. Repository Layer (`src/storage/repository.rs`)

### 1.1 Add to `ConversationRepository` trait:

```rust
/// Get all messages for a conversation (for export)
async fn get_message_list(
    &self,
    conversation_id: Uuid,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>;

/// Get memory statistics (global or by folder)
async fn get_stats(
    &self,
    folder: Option<String>,
) -> Result<Stats, Box<dyn std::error::Error>>;
```

### 1.2 Implement in `SeaOrmConversationRepository`:

```rust
// Add imports at top:
use sea_orm::{entity::*, query::*};

// Implementation for get_message_list
pub async fn get_message_list(
    &self,
    conversation_id: Uuid,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let messages = entity::message::Entity::find()
        .filter(entity::message::Column::ConversationId.eq(conversation_id))
        .order_by_asc(entity::message::Column::Timestamp)
        .all(&self.db)
        .await?;
    
    Ok(messages.into_iter().map(|msg| {
        serde_json::json!({
            "id": msg.id,
            "role": msg.role,
            "content": msg.content,
            "timestamp": msg.timestamp,
            "metadata": msg.metadata,
        })
    }).collect())
}

// Implementation for get_stats
pub async fn get_stats(
    &self,
    folder: Option<String>,
) -> Result<Stats, Box<dyn std::error::Error>> {
    let mut query = entity::conversation::Entity::find();
    
    if let Some(folder_path) = folder {
        query = query.filter(
            entity::conversation::Column::Folder.eq(folder_path)
        );
    }
    
    let conversations = query.all(&self.db).await?;
    let total = conversations.len();
    
    let avg_importance = if total > 0 {
        conversations.iter()
            .map(|c| c.importance_score)
            .sum::<i32>() as f32 / total as f32
    } else {
        0.0
    };
    
    // Get unique folders
    let folders: Vec<String> = {
        let folder_results: Vec<Option<String>> = entity::conversation::Entity::find()
            .select_only()
            .column(entity::conversation::Column::Folder)
            .distinct()
            .into_tuple()
            .all(&self.db)
            .await?;
        
        folder_results.into_iter()
            .flatten()
            .collect()
    };
    
    Ok(Stats {
        total_conversations: total,
        average_importance: avg_importance,
        folders,
    })
}
```

### 1.3 Add Stats struct:

```rust
// At top of file or in models module
#[derive(Serialize)]
pub struct Stats {
    pub total_conversations: usize,
    pub average_importance: f32,
    pub folders: Vec<String>,
}
```

### 1.4 Write repository tests:

```rust
#[tokio::test]
async fn test_get_message_list_success() {
    // Setup: create conversation with messages
    // Call get_message_list
    // Assert messages returned in correct order
}

#[tokio::test]
async fn test_get_message_list_empty() {
    // Setup: create conversation with no messages
    // Call get_message_list
    // Assert empty vector returned
}

#[tokio::test]
async fn test_get_message_list_not_found() {
    // Call with non-existent conversation_id
    // Assert error
}

#[tokio::test]
async fn test_get_stats_global() {
    // Setup: create multiple conversations across folders
    // Call get_stats(None)
    // Assert correct counts and folder list
}

#[tokio::test]
async fn test_get_stats_by_folder() {
    // Setup: create conversations in specific folder
    // Call get_stats(Some("/work"))
    // Assert only that folder's stats
}
```

---

## 2. API Layer (`src/api/mcp.rs`)

### 2.1 Add endpoint handlers:

Already added to file:
- `memory_export()` 
- `memory_stats()`

But need completion:
- `memory_export` needs to call `get_message_list()`
- `memory_stats` needs to call `get_stats()`

### 2.2 Update router:

Add to `create_mcp_router()`:
```rust
.route("/mcp/tools/memory_export", post(memory_export))
.route("/mcp/tools/memory_stats", post(memory_stats))
```

---

## 3. API Tests (`src/api/mcp.rs`)

### 3.1 Export tests:

```rust
#[tokio::test]
async fn test_memory_export_success() {
    // Setup: create conversation with messages
    // Call memory_export with valid ID
    // Assert success response with messages
}

#[tokio::test]
async fn test_memory_export_not_found() {
    // Call with non-existent UUID
    // Assert 404 response
}

#[tokio::test]
async fn test_memory_export_json_format() {
    // Call with format: "json"
    // Assert conversation data in JSON structure
}

#[tokio::test]
async fn test_memory_export_markdown_format() {
    // Call with format: "markdown"  
    // Assert markdown-formatted response
}
```

### 3.2 Stats tests:

```rust
#[tokio::test]
async fn test_memory_stats_global() {
    // Setup: create conversations across multiple folders
    // Call memory_stats with no folder
    // Assert correct totals and folder list
}

#[tokio::test]
async fn test_memory_stats_by_folder() {
    // Setup: create conversations in "/work" folder
    // Call memory_stats with folder: "/work"
    // Assert only "/work" stats
}

#[tokio::test]
async fn test_memory_stats_empty() {
    // Call on empty database
    // Assert zero totals and empty folder list
}
```

---

## 4. Integration Testing

### 4.1 Add integration test skeleton:

File: `tests/integration/test_mcp_endpoints.rs`

```rust
#[tokio::test]
async fn test_mcp_export_e2e() {
    // Start controller
    // Create conversation via MCP
    // Export via MCP
    // Verify export contains conversation
}

#[tokio::test]
async fn test_mcp_stats_e2e() {
    // Start controller
    // Create multiple conversations
    // Get stats via MCP
    // Verify counts match
}
```

---

## 5. Documentation

### 5.1 Update controller docs:

- `docs/architecture/mcp-protocol.md` - Add export/stats examples
- `docs/api/mcp-reference.md` - Document new endpoints
- `CHANGELOG.md` - Add v0.1.1 entry

---

## 6. Implementation Checklist

- [ ] Add `get_message_list` to repository trait
- [ ] Implement `get_message_list` in SeaOrm repo
- [ ] Add `get_stats` to repository trait
- [ ] Implement `get_stats` in SeaOrm repo
- [ ] Add `Stats` struct
- [ ] Write 6 repository unit tests
- [ ] Update `create_mcp_router` with 2 new routes
- [ ] Complete `memory_export` handler
- [ ] Complete `memory_stats` handler
- [ ] Write 6-8 API endpoint tests
- [ ] Run full test suite: `cargo test`
- [ ] Verify MCP integration: `pytest` in mcp repo
- [ ] Update documentation
- [ ] Update CHANGELOG

---

## Estimated Effort

Repository methods: 2 hours
API handlers: 1 hour
Tests: 3 hours
Documentation: 1 hour
Total: ~7 hours

## Notes

- Ensure all paths use proper UUID validation
- Handle empty result sets gracefully
- Follow existing error handling patterns
- Keep repository methods generic (not MCP-specific)
- Test both happy path and error cases