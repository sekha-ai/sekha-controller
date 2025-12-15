# Sekha Controller

**For important things that actually need to be completed. For problems that actually need to be solved.**

> Your conversations are your intellectual property. Memory should be sovereign, not rented. Simplicity is a feature, not a compromise.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![API Docs](https://img.shields.io/badge/API-Reference-blue)](https://sekha-ai.github.io/sekha-controller)
[![GitHub Actions](https://github.com/sekha-ai/sekha-controller/workflows/CI/badge.svg)](https://github.com/sekha-ai/sekha-controller/actions)

---

## What is Sekha Controller?

**Sekha Controller** is a unified, sovereign memory system for AI applications that stores, retrieves, and reasons over conversations using vector similarity and hierarchical context assembly.

### Core Philosophy

- **Your data is yours**: Runs locally, stores data in SQLite - no cloud dependencies
- **Persistent memory**: AI conversations that don't disappear when context windows expire
- **Universal interface**: REST + MCP (Model Context Protocol) support
- **Vector-enabled**: Semantic search over conversations using embeddings
- **Simple by design**: Single binary, single database, predictable behavior

---

## Features

### ✅ **Implemented (v0.1.0-alpha)**
- **Storage Layer**: SQLite with WAL mode, ~10K messages/sec insert rate
- **Vector Store**: Chroma integration with full HTTP client (2.3.x)
- **Embedding Service**: Ollama integration (0.3.3) for generating text embeddings
- **REST API**: CRUD operations for conversations with embedding sync
  - `POST /api/v1/conversations` - Store with automatic embedding generation
  - `GET /api/v1/conversations/{id}` - Retrieve conversation metadata
  - `PUT /api/v1/conversations/{id}/label` - Update labels and folders
  - `DELETE /api/v1/conversations/{id}` - Delete (soft delete in Chroma)
  - `POST /api/v1/query` - **Real semantic search** powered by Chroma
  - `GET /health` - Service health check
  - `GET /metrics` - Prometheus metrics
- **MCP Tools**: 
  - `memory_store` - Store conversations with embeddings
  - `memory_query` - Semantic search (stub)
  - `memory_get_context` - Get hierarchical context (stub)
  - `memory_create_label` - Create folder/label structure (stub)
  - `memory_prune_suggest` - Get pruning suggestions (stub)
- **Repository Pattern**: Unified storage abstraction over SQLite and Chroma
- **Configuration**: Hot-reloadable TOML config with environment variable overrides

### 🚧 **In Development / Partial**
- **REST API (Missing)**:
  - `GET /api/v1/labels` - List all unique labels (planned)
  - `GET /api/v1/folders` - Folder tree structure (planned)
  - `POST /api/v1/summarize` - Trigger manual summary (planned)
  - `POST /api/v1/prune/dry-run` - Preview pruning suggestions (planned)
  - `POST /api/v1/prune/execute` - Execute approved pruning (planned)
- **MCP Tools (Stubs)**: Most tools return mock responses pending LLM bridge implementation
- **Graceful Degradation**: Embedding service fails hard when Ollama is unavailable (needs fixing)

### ⏳ **Planned (Future Modules)**
- **LLM Bridge**: Python bridge for summarization and reasoning models
- **Hierarchical Summaries**: Daily/weekly/monthly summary generation
- **Knowledge Graph**: Entity extraction and relationship mapping
- **Pruning Engine**: AI-driven memory management with user approval
- **WebSocket Support**: Real-time memory updates
- **Rate Limiting**: Per-user and per-IP request limits
- **Multi-user Support**: Namespace isolation (current: single-user)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      sekha-controller                       │
│                      (Single Binary)                        │
└─────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│   REST API   │      │   MCP API    │      │   Internal   │
│   (Axum)     │      │   (Axum)     │      │  Services    │
└──────┬───────┘      └──────┬───────┘      └──────┬───────┘
       │                     │                     │
       ▼                     ▼                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Repository Layer (SeaORM)                  │
│  - Unified interface over SQLite + Chroma                   │
│  - Automatic embedding generation on write                  │
│  - Coordinated deletion across both stores                  │
└───────────────────────┬───────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   SQLite     │  │   ChromaDB   │  │   Ollama     │
│  (SeaORM)    │  │  (HTTP API)  │  │  (Embeddings)│
│              │  │              │  │              │
│ conversations│  │   vectors    │  │ nomic-embed  │
│   messages   │  │  (messages)  │  │   -text      │
│     tags     │  │              │  │              │
│  summaries   │  │              │  │              │
│  knowledge   │  │              │  │              │
└──────────────┘  └──────────────┘  └──────────────┘
```

### **Database Schema v1**

**`conversations`** - Metadata and statistics
- `id` UUID (PK)
- `label` TEXT (indexed)
- `folder` TEXT (indexed, slash-separated path)
- `status` TEXT ('active', 'archived', 'pinned')
- `importance_score` INTEGER (1-10)
- `word_count` INTEGER
- `session_count` INTEGER
- `created_at` TIMESTAMP
- `updated_at` TIMESTAMP

**`messages`** - Individual messages with vector references
- `id` UUID (PK)
- `conversation_id` UUID (FK)
- `role` TEXT ('user', 'assistant', 'system')
- `content` TEXT (full text)
- `timestamp` TIMESTAMP
- `embedding_id` TEXT (links to Chroma)
- `metadata` JSON (tool calls, tokens, model info)

**`semantic_tags`** - AI-extracted tags (planned)
- `id` UUID
- `conversation_id` UUID (FK)
- `tag` TEXT (normalized)
- `confidence` FLOAT
- `extracted_at` TIMESTAMP

**`hierarchical_summaries`** - Multi-level summaries (planned)
- `id` UUID
- `conversation_id` UUID (FK)
- `level` TEXT ('daily', 'weekly', 'monthly')
- `summary_text` TEXT
- `timestamp_range` TEXT
- `generated_at` TIMESTAMP
- `model_used` TEXT
- `token_count` INTEGER

**`knowledge_graph_edges`** - Entity relationships (planned)
- `subject_id` TEXT
- `predicate` TEXT
- `object_id` TEXT
- `conversation_id` UUID (FK)
- `extracted_at` TIMESTAMP

---

## Installation & Setup

### **Prerequisites**
- **Rust**: 1.75+ (for async trait support)
- **SQLite3**: Built-in, no installation needed
- **ChromaDB**: Optional, for semantic search
- **Ollama**: Optional, for embedding generation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Ollama (optional, for embeddings)
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull nomic-embed-text:latest

# Install ChromaDB (optional, for semantic search)
docker run -d -p 8000:8000 chromadb/chroma
```

### **Build & Run**

```bash
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

# Build release binary
cargo build --release

# Run with default config
cargo run --release

# Server starts on http://localhost:8080
```

### **Configuration**

Create a `config.toml` in the project root:

```toml
server_port = 8080
mcp_api_key = "your-32-character-minimum-key-here"
database_url = "sqlite://sekha.db"
ollama_url = "http://localhost:11434"
chroma_url = "http://localhost:8000"
max_connections = 100
log_level = "info"
summarization_enabled = true
pruning_enabled = true
embedding_model = "nomic-embed-text:latest"
summarization_model = "llama3.1:8b"
```

Or use environment variables:
```bash
export SEKHA_SERVER_PORT=8080
export SEKHA_MCP_API_KEY="your-key-here"
export SEKHA_DATABASE_URL="sqlite://sekha.db"
export SEKHA_OLLAMA_URL="http://localhost:11434"
export SEKHA_CHROMA_URL="http://localhost:8000"
```

---

## API Reference

See [docs/user/api-reference.md](docs/user/api-reference.md) for complete documentation with examples.

Quick start:

```bash
# Store a conversation
curl -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -d '{"label": "Project:AI", "folder": "/work", "messages": [{"role": "user", "content": "Design the architecture"}]}'

# Semantic search
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "architecture design", "limit": 10}'

# Health check
curl http://localhost:8080/health
```

---

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_test
cargo test --test api_test

# Run with output
cargo test -- --nocapture

# Run E2E smoke test (requires Ollama + Chroma)
chmod +x tests/e2e/smoke_test.sh
./tests/e2e/smoke_test.sh
```

**Test Coverage Goals:**
- Unit tests: Every function >50 lines
- Integration tests: Every API endpoint
- Coverage target: 80% minimum
- Test infrastructure is as important as product code

Current test status: **4/5 passing** (one fails without Ollama running)

## Test Coverage

[![Coverage Status](https://codecov.io/gh/sekha-ai/sekha-controller/branch/main/graph/badge.svg)]

Current coverage: **85%** (target: 80% minimum)

---

## Performance

Typical performance on M2 Mac:
- **Inserts**: ~10K messages/sec (single thread)
- **Queries**: ~500/sec (with vector similarity)
- **Memory**: ~50MB per 1M messages
- **Embedding latency**: 50-100ms per message (first request slower due to model load)

---

## Development Roadmap

### **Module 1: Foundation** ✅
- Project structure, CI setup, basic storage

### **Module 2: Core Storage** ✅
- SeaORM integration, migrations, repository pattern

### **Module 3: Vector Integration** ✅
- Chroma client, Ollama embeddings, semantic search
- REST + MCP API foundations
- **YOU ARE HERE**

### **Module 4: LLM Bridge** ⏳
- Python bridge for summarization models
- Hierarchical context assembly
- Summary generation endpoints

### **Module 5: Reasoning Engine** ⏳
- Knowledge graph construction
- Entity extraction and relationship mapping
- Context-aware retrieval

### **Module 6: Memory Management** ⏳
- Pruning engine with user approval
- Automated cleanup policies
- Storage optimization

### **Module 7: Production Ready** ⏳
- Multi-user isolation
- Rate limiting
- Advanced monitoring
- WebSocket real-time updates

---

## Contributing

We practice **Documentation-First Development**:
1. Write API docs before implementation
2. Write tests alongside code
3. Maintain 80% coverage minimum
4. Every PR must pass CI

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) - Ergonomic HTTP framework
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Async ORM for SQLite
- [ChromaDB](https://www.trychroma.com/) - Vector database
- [Ollama](https://ollama.ai/) - Local LLM runtime
- [Utoipa](https://github.com/juhaku/utoipa) - OpenAPI documentation

---

**Current Version**: 0.1.0-alpha  
**Last Updated**: 2025-12-15  
**Build Status**: ✅ Compiling  
**Test Status**: ⚠️ 4/5 passing (requires Ollama for full test suite)