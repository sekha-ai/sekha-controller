# Sekha Controller

> **The Universal Memory System for AI That Never Forgets**

[![CI](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml/badge.svg)](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml)
[![CI Status](https://github.com/sekha-ai/sekha-controller/workflows/CI/badge.svg)](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sekha-ai/sekha-controller/branch/main/graph/badge.svg)](https://codecov.io/gh/sekha-ai/sekha-controller)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)
[![Crates.io](https://img.shields.io/crates/v/sekha-controller.svg)](https://crates.io/crates/sekha-controller)
[![Docker](https://img.shields.io/badge/docker-ready-green.svg)](https://github.com/orgs/sekha-ai/packages)


---

## 🎯 **What is Sekha?**

**Sekha Controller is a production-ready AI memory system that solves the fundamental limitations of conversational AI.**

### **The Problems It Solves**

Every AI conversation today faces critical failures:

1. **🔥 Broken Context** - Your LLM runs out of memory mid-conversation, losing critical details
2. **🧠 Forgotten Context** - Long-running conversations forget everything from earlier sessions
3. **⏱️ Agent Breakdowns** - AI agents fail on complex, multi-step tasks spanning hours or days
4. **🚫 No Continuity** - Each new chat starts from zero, wasting time re-explaining context
5. **📊 Lost Knowledge** - Years of valuable interactions vanish when you hit token limits

### **The Solution**

Sekha gives AI **persistent, searchable, infinite memory** - like a second brain that never forgets. Your conversations, whether they span minutes or years, maintain perfect continuity and context.

**Use Cases:**
- 💼 **Professionals**: Career-spanning AI assistant that remembers every project, decision, and insight
- 🤖 **AI Agents**: Self-improving agents that learn from every interaction and never repeat mistakes
- 🔬 **Researchers**: Persistent research assistant across multiple studies and experiments
- 👨‍💻 **Developers**: Code assistant that remembers your entire codebase evolution and decisions
- 📚 **Students**: Study companion that builds on months/years of coursework
- 🏥 **Healthcare**: Patient history tracking with perfect recall (HIPAA-ready architecture)
- 💡 **Creative Work**: Brainstorming partner that remembers every idea, draft, and iteration

**For important things that actually need to be completed. For problems that actually need to be solved.**

Sekha is NOT:
- A chatbot
- An LLM
- An agent framework

Sekha IS:
- A memory protocol layer
- Sits BETWEEN user/agent and LLM
- Captures, organizes, retrieves context
- Makes infinite context windows possible

---

## ✨ **Key Features**

### **🔒 Sovereign Memory**
Your conversations are **your intellectual property**. Self-hosted, local-first architecture means your data never leaves your infrastructure.

### **♾️ Infinite Context Windows**
Never hit token limits again. Conversations can span:
- Days → Weeks → Months → **Years**
- 100 messages → 1,000 messages → **1,000,000+ messages**
- Career-spanning interactions with perfect recall

### **🧠 Intelligent Context Assembly**
Advanced orchestration that automatically:
- Retrieves relevant past conversations via semantic search
- Prioritizes recent, important, and pinned messages
- Builds hierarchical summaries (daily → weekly → monthly)
- Suggests labels and organization strategies
- Recommends pruning low-value conversations

### **🔌 LLM Agnostic**
**Plug-and-play architecture** - use any LLM while retaining perfect memory:
- ✅ **Currently Implemented**: Ollama (local models: Llama, Mistral, etc.)
- 🔜 **Roadmap**: OpenAI, Anthropic, Google, Cohere, custom models
- 🎯 **Vision**: Switch between LLMs mid-conversation without losing context

### **🚀 Production Ready**
- 85%+ test coverage with comprehensive CI/CD
- Docker deployment with multi-arch support (amd64/arm64)
- RESTful API + Model Context Protocol (MCP) support
- Sub-100ms semantic queries on millions of messages
- Real-world benchmarks prepared for academic publication

### **🌐 Multi-Interface Support**
- REST API for any programming language
- MCP protocol for Claude Desktop, Cline, and compatible tools
- Python SDK for data science workflows
- JavaScript SDK for web applications
- VS Code extension for in-editor memory
- CLI tool for terminal power users

---
## 🏗️ **Architecture**

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SEKHA CONTROLLER (Rust)                       │
│                   Single Binary, Portable, ~50MB                     │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
         ┌─────────────────────┼─────────────────────┐
         │                     │                     │
         ▼                     ▼                     ▼
    ┌─────────┐          ┌──────────┐         ┌────────────┐
    │   REST  │          │   MCP    │         │  Internal  │
    │   API   │          │  Server  │         │  Services  │
    │ (17 eps)│          │ (7 tools)│         │            │
    └────┬────┘          └─────┬────┘         └──────┬─────┘
         │                     │                     │
         └─────────────────────┼─────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   MEMORY ORCHESTRATION ENGINE                        │
│                                                                       │
│  ┌──────────────────┬────────────────────┬────────────────────────┐ │
│  │ Context Assembly │  Hierarchical      │  Intelligent           │ │
│  │                  │  Summarization     │  Organization          │ │
│  ├──────────────────┼────────────────────┼────────────────────────┤ │
│  │ -  Semantic +     │ -  Daily rollups    │ -  Label suggestions    │ │
│  │   recency +      │ -  Weekly digests   │ -  Folder hierarchy     │ │
│  │   importance     │ -  Monthly reports  │ -  Importance scoring   │ │
│  │ -  Deduplication  │ -  Recursive        │ -  Pruning              │ │
│  │ -  Token limits   │   compression      │   recommendations      │ │
│  └──────────────────┴────────────────────┴────────────────────────┘ │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
          ▼                     ▼                     ▼
    ┌────────────┐       ┌─────────────┐      ┌──────────────┐
    │   SQLite   │       │  ChromaDB   │      │  LLM Bridge  │
    │  (SeaORM)  │       │   Vectors   │      │   (Python)   │
    └────────────┘       └─────────────┘      └──────────────┘
    │                    │                    │
    │ -  Metadata         │ -  768-dim          │ -  Ollama (v1)
    │ -  Relations        │   embeddings       │ -  Future: Any LLM
    │ -  Labels           │ -  Semantic         │ -  Summarization
    │ -  Folders          │   similarity       │ -  Embeddings
    │ -  Status           │ -  Sub-100ms        │ -  Tool calling
    │ -  Importance       │   queries          │
    │ -  Timestamps       │ -  Millions of      │
    │                    │   vectors          │
    └────────────────────┴────────────────────┴──────────────────┘
                                │
                   ┌────────────┴─────────────┐
                   │                          │
                   ▼                          ▼
            ┌─────────────┐           ┌──────────────┐
            │ File-based  │           │   Portable   │
            │   Storage   │           │  Single-file │
            │             │           │   Database   │
            │ ~/.sekha/   │           │   sekha.db   │
            │  ├─ data/   │           │              │
            │  ├─ logs/   │           │ Full backup  │
            │  └─ config/ │           │ via file copy│
            └─────────────┘           └──────────────┘
```

### **Data Flow: How Memory Works**

1. **Storage** → User sends conversation to `/api/v1/conversations`
2. **Embedding** → LLM Bridge generates vector embeddings via Ollama
3. **Indexing** → SQLite stores metadata, ChromaDB stores vectors
4. **Query** → User searches via semantic query or keywords
5. **Retrieval** → Orchestrator combines semantic + recency + importance
6. **Assembly** → Context builder creates optimal payload for LLM
7. **Interaction** → User gets perfectly contextualized AI response

### **Label & Folder Organization**
Label = think tags
Folder = think directories

/work
/project-alpha # Folder structure
(label: planning) # Labels within folders
(label: technical)
/personal
/learning
(label: rust)
(label: ai)


**Importance Scoring** (1-10 scale):
- `1-3`: Low priority, candidate for pruning
- `4-6`: Normal conversations
- `7-9`: High value, frequently referenced
- `10`: Pinned, never prune

---

## 🚀 **Quick Start**

### **Prerequisites**

- **Docker & Docker Compose** (recommended) OR
- **Rust 1.75+** + SQLite3 (for local development)

### **Option 1: Docker (Recommended for Users)**

```bash
# 1. Clone the deployment repository
git clone https://github.com/sekha-ai/sekha-docker.git
cd sekha-docker

# 2. Start the full stack (controller + LLM bridge + dependencies)
docker compose -f docker-compose.prod.yml up -d

# 3. Verify it's running
curl http://localhost:8080/health

# Expected output:
# {
#   "status": "healthy",
#   "timestamp": "2026-01-09T...",
#   "checks": {
#     "database": {"status": "ok"},
#     "chroma": {"status": "ok"}
#   }
# }

# 4. View interactive API documentation
open http://localhost:8080/swagger-ui/
```

What This Starts:

Sekha Controller (port 8080) - Core memory engine
Sekha LLM Bridge (port 5001) - LLM operations
ChromaDB (port 8000) - Vector database
Ollama (port 11434) - Local LLM runtime
Option 2: Local Development

### 1. Clone the controller
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

### 2. Start dependencies (Chroma + Ollama)
docker run -d --name chroma -p 8000:8000 chromadb/chroma
docker run -d --name ollama -p 11434:11434 ollama/ollama

### 3. Install embedding model
docker exec ollama ollama pull nomic-embed-text

### 4. Build and run the controller
cargo build --release
cargo run --release

### 5. Test the installation
curl -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dev-key-replace-in-production" \
  -d '{
    "label": "First Conversation",
    "folder": "/personal/test",
    "messages": [
      {
        "role": "user",
        "content": "Hello Sekha! This is my first message."
      },
      {
        "role": "assistant",
        "content": "Hello! I will remember this conversation forever."
      }
    ]
  }'

### Expected: {"id": "uuid...", "conversation_id": "uuid...", ...}

📖 How to Use Sekha
1. Storing Conversations
Every time you chat with an AI, store the conversation in Sekha:

### Store a conversation
curl -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "label": "Project Planning",
    "folder": "/work/new-feature",
    "messages": [
      {"role": "user", "content": "We need to build a new API endpoint"},
      {"role": "assistant", "content": "I recommend starting with..."}
    ]
  }'

Response:
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "label": "Project Planning",
  "folder": "/work/new-feature",
  "status": "active",
  "message_count": 2,
  "created_at": "2026-01-09T16:30:00"
}

2. Searching Your Memory

Semantic Search (finds meaning, not just keywords):

curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "query": "What did we discuss about API design?",
    "limit": 5
  }'


Full-Text Search (exact keywords):

curl -X POST http://localhost:8080/api/v1/search/fts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "query": "API endpoint",
    "limit": 10
  }'


3. Building Context for Your Next AI Chat
Get the most relevant past conversations to include in your next LLM prompt:

curl -X POST http://localhost:8080/api/v1/context/assemble \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "query": "Continue working on the new feature",
    "preferred_labels": ["Project Planning", "Technical Design"],
    "context_budget": 8000
  }'

This returns intelligently selected messages that fit within your token budget, prioritizing:

Semantic relevance to your query

Recent conversations

High importance scores

Preferred labels


4. Organizing Memory
Update Labels:
curl -X PUT http://localhost:8080/api/v1/conversations/{id}/label \
  -H "Content-Type: application/json" \
  -d '{"label": "Completed Feature", "folder": "/work/archive"}'

Pin Important Conversations:
curl -X PUT http://localhost:8080/api/v1/conversations/{id}/pin

Archive Old Conversations:
curl -X PUT http://localhost:8080/api/v1/conversations/{id}/archive


5. Getting AI-Powered Suggestions
Suggest Labels (AI analyzes content and suggests organization):

curl -X POST http://localhost:8080/api/v1/labels/suggest \
  -H "Content-Type: application/json" \
  -d '{"conversation_id": "uuid-here"}'

Pruning Recommendations (find low-value conversations to archive):

curl -X POST http://localhost:8080/api/v1/prune/dry-run \
  -H "Content-Type: application/json" \
  -d '{"threshold_days": 90}'


6. Generating Summaries
Create hierarchical summaries to compress long conversation histories:

### Daily summary
curl -X POST http://localhost:8080/api/v1/summarize \
  -H "Content-Type: application/json" \
  -d '{
    "conversation_id": "uuid-here",
    "level": "daily"
  }'

### Weekly digest
curl -X POST http://localhost:8080/api/v1/summarize \
  -H "Content-Type: application/json" \
  -d '{
    "conversation_id": "uuid-here",
    "level": "weekly"
  }'

7. Using with Claude Desktop (MCP)
Sekha includes native Model Context Protocol support. Add to your Claude Desktop config:

{
  "mcpServers": {
    "sekha": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "--network=host",
        "ghcr.io/sekha-ai/sekha-mcp:latest"
      ]
    }
  }
}


Now you can use these tools in Claude:

memory_store - Save conversations
memory_query - Search your memory
memory_get_context - Retrieve relevant context
memory_create_label - Organize conversations
memory_prune_suggest - Get cleanup recommendations
memory_export - Export your data
memory_stats - View usage statistics


⚙️ Configuration
Sekha auto-generates ~/.sekha/config.toml on first run. Customize it for your needs:

[server]
port = 8080
host = "0.0.0.0"
# IMPORTANT: Change this to a secure random string (min 32 characters)
api_key = "your-production-api-key-min-32-chars-long"

[database]
url = "sqlite://~/.sekha/data/sekha.db"
max_connections = 100

[vector_store]
chroma_url = "http://localhost:8000"
collection_name = "sekha_memory"

[llm]
# Current implementation: Ollama
ollama_url = "http://localhost:11434"
embedding_model = "nomic-embed-text:latest"
summarization_model = "llama3.1:8b"

# Future: Plug-and-play any LLM
# provider = "openai" | "anthropic" | "google" | "custom"
# api_key = "..."

[features]
# Enable/disable orchestration features
summarization_enabled = true
pruning_enabled = true
auto_embed = true
label_suggestions = true

[logging]
level = "info"  # trace | debug | info | warn | error
format = "json"  # json | pretty
output = "~/.sekha/logs/controller.log"

[rate_limiting]
requests_per_second = 100
burst_size = 200

[cors]
allowed_origins = ["http://localhost:3000", "https://your-app.com"]


Environment Variables (override config.toml):

export SEKHA_SERVER_PORT=8080
export SEKHA_API_KEY="production-key-change-this"
export SEKHA_DATABASE_URL="sqlite:///opt/sekha/data/sekha.db"
export SEKHA_CHROMA_URL="http://chroma:8000"
export SEKHA_OLLAMA_URL="http://ollama:11434"
export SEKHA_LOG_LEVEL="info"


📊 Current Status & Roadmap
✅ Production Ready (Current)
Core Storage & Retrieval:

 SQLite database with full ACID guarantees
 ChromaDB vector storage for semantic search
 Full-text search via SQLite FTS5
 Label and folder hierarchical organization
 Importance scoring (1-10 scale)
 Status tracking (active/archived)
 Conversation metadata (word count, timestamps, sessions)

APIs:

 17 REST endpoints (create, query, update, delete, search, stats)
 7 MCP protocol tools for Claude Desktop integration
 OpenAPI/Swagger documentation
 Rate limiting and CORS
 Bearer token authentication

Orchestration:

 Context assembly with semantic + recency + importance ranking
 Hierarchical summarization (daily → weekly → monthly)
 AI-powered label suggestions
 Pruning recommendations
 Deduplication and token budget optimization

LLM Integration:

 Ollama support (nomic-embed-text for embeddings)
 Llama 3.1 for summarization
 Async embedding pipeline with retry logic

Production Features:

 Docker multi-arch builds (amd64/arm64)
 Comprehensive CI/CD with 85%+ coverage
 Security audits (cargo-deny, cargo-audit)
 Health checks and Prometheus metrics
 Hot config reload
 Structured logging (JSON + pretty)

🎯 Roadmap 2026-2029
Q1 2026 - Multi-LLM Support

 OpenAI API integration (GPT-4, embeddings)
 Anthropic Claude integration
 Google Gemini support
 Plug-and-play LLM configuration
 LLM provider abstraction layer

Q2 2026 - Scale & Performance

 PostgreSQL backend option (multi-user)
 Redis caching layer
 Horizontal scaling architecture
 Kubernetes Helm charts
 Performance benchmarks published (academic paper submission)

Q3 2026 - Advanced Features

 Knowledge graph extraction from conversations
 Relationship mapping between conversations
 Temporal reasoning (time-aware context)
 Multi-modal memory (images, audio, video)
 Federated sync (S3, R2, self-hosted)

Q4 2026 - Enterprise & Collaboration

 Multi-tenant architecture
 Team collaboration features
 Role-based access control (RBAC)
 Audit logging and compliance (HIPAA, SOC2)
 WebSocket real-time updates

2027 - AI Agent Ecosystem

 Agent-to-agent memory sharing
 Autonomous agent memory management
 Self-improving agent frameworks
 Agent learning from collective experiences
 Cross-agent knowledge transfer

2028-2029 - Advanced Intelligence

 CRDT-based conflict resolution for distributed memory
 GPU-accelerated vector operations
 Plugin system for custom LLM backends
 Zero-knowledge encryption for privacy
 Blockchain-based provenance tracking (optional)
 Research: Contributions toward AGI architectures

🧪 Testing & Quality
Test Coverage# Run all tests
cargo test

# Run with coverage report
cargo tarpaulin --out Html --output-dir coverage
open coverage/index.html

# Run specific test suites
cargo test --test unit          # Pure logic tests
cargo test --test integration   # Database + API tests
cargo test --test api_test      # Full API validation
cargo test --test benchmark     # Performance tests

Current Coverage: 85%+ across all modules

Test Structure:
tests/
├── unit/           # Fast, no I/O, pure logic
├── integration/    # Database + Chroma + Ollama
├── api_test.rs     # Full REST API validation
├── benchmarks/     # Performance measurement
└── e2e/           # Docker stack smoke tests

Benchmarks
Performance benchmarks are in preparation for academic and industry paper submissions. Current focus:

Long-running conversation stability (days/weeks)
Million+ message scalability
Multi-user concurrent access patterns
Cross-repository integration latency
Real-world production workload simulation


🛠️ Development
Contributing
We are seeking contributors! 

How to Contribute:
Check open issues
Fork the repo and create a feature branch
Write tests for new functionality
Ensure cargo test passes with 80%+ coverage
Run cargo fmt and cargo clippy
Submit a pull request
See CONTRIBUTING.md for detailed guidelines.

Code Quality Standards
All code must pass:

cargo fmt -- --check          # Formatting
cargo clippy -- -D warnings   # Linting
cargo test                    # All tests pass
cargo tarpaulin --out Html    # 80%+ coverage
cargo deny check advisories   # Security audit


Development Setup

# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone the repo
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

# 3. Start dependencies
docker compose -f docker-compose.dev.yml up -d

# 4. Run in development mode (auto-reload)
cargo watch -x run

# 5. Run tests on file changes
cargo watch -x test


📚 Multi-Repository Ecosystem
Sekha is built as a modular ecosystem. This README covers the controller (core engine). See other repos for additional components:

| Repository       | Purpose                   | Status        |
| ---------------- | ------------------------- | ------------- |
| sekha-controller | Core memory engine (Rust) | ✅ Production  |
| sekha-llm-bridge | LLM operations (Python)   | ✅ Production  |
| sekha-docker     | Production deployment     | ✅ Production  |
| sekha-python-sdk | Python client library     | 🔜 Publishing |
| sekha-js-sdk     | JavaScript/TypeScript SDK | 🔜 Publishing |
| sekha-mcp        | MCP server for Claude     | 🔜 Publishing |
| sekha-vscode     | VS Code extension         | 🚧 Beta       |
| sekha-cli        | Command-line tool         | 🚧 Beta       |


🔒 Privacy & Security
Local-First Architecture
By default, all data stays on your machine:

SQLite database: ~/.sekha/data/sekha.db
ChromaDB vectors: Docker volume or local directory
No telemetry, no phone-home, no analytics (opt-in only)

Self-Hosted Deployment
Full control over your infrastructure:

Deploy on your own servers
Use your own LLMs (Ollama, vLLM, custom)
Air-gapped environments supported

GDPR/HIPAA-ready architecture
Security Features
Bearer token authentication
Rate limiting (per-IP, configurable)
CORS protection
Audit logging of all operations
Security audits via cargo-deny and cargo-audit
No external dependencies in production binary
Vulnerability Reporting: security@sekha.dev

📄 License
Dual License:
Open Source: AGPL-3.0 for personal, educational, and non-commercial use
Commercial: Contact hello@sekha.dev for enterprise licensing

🌐 Links & Resources
Website: https://sekha.dev - Product info, blog, use cases
Documentation: https://docs.sekha.dev - Full guides and API reference
GitHub: https://github.com/sekha-ai - All repositories
Discord: https://discord.gg/sekha - Community support
API Docs: http://localhost:8080/swagger-ui/ - Interactive API explorer (when running locally)

🙏 Acknowledgments
Built with world-class open-source tools:
Axum - Ergonomic async web framework
SeaORM - Rust async ORM
ChromaDB - Vector database for embeddings
Ollama - Local LLM runtime
SQLite - World's most deployed database
Utoipa - OpenAPI documentation generator
Special thanks to the Rust, AI, and open-source communities.

📞 Support
Issues: GitHub Issues
Discord: https://discord.gg/sekha
Email: hello@sekha.dev
Discussions: GitHub Discussions

📈 Project Stats
Lines of Code: ~15,000 (Rust controller) + ~5,000 (Python bridge)
Test Coverage: 85% (controller), 82% (bridge)
Dependencies: 47 Rust crates, 23 Python packages
First Commit: December 11, 2025
Current Version: v1.0.0
Contributors: Seeking contributors! Join us.
License: AGPL-3.0 / Commercial

<div align="center">
Built to enable AI that never forgets
From solving broken context windows to enabling career-spanning AI assistants and self-improving agents
⭐ Star us on GitHub • 📖 Read the Docs • 💬 Join Discord
Sekha Project • Website • GitHub • December 2025 - Present
</div> ```
