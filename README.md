# Sekha

> **Give Your AI Perfect Memory**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![CI](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml/badge.svg)](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/badge/Discord-Join%20Us-7289da.svg)](https://discord.gg/sekha)

---

## What is Sekha?

**Sekha gives AI assistants perfect memory** - like giving your AI a brain that never forgets.

Talk to ChatGPT about Python in the morning. Switch to Claude for code review in the afternoon. Use a local AI for planning in the evening. **Sekha remembers it all** and automatically reminds each AI about relevant past conversations.

**No more:**
- ❌ "Can you remind me what we discussed yesterday?"
- ❌ Starting from scratch with each new chat
- ❌ Losing important conversations across different AI tools
- ❌ Manually copying context between conversations

**Instead:**
- ✅ Every AI remembers what you discussed across **all** your conversations
- ✅ Switch between ChatGPT, Claude, local LLMs seamlessly
- ✅ Your AI automatically recalls relevant details when needed
- ✅ **All your AI conversations in one searchable database**

---

## 👤 I Just Want to Use It

### Quick Install (5 Minutes)

**Requirements:** Just [Docker Desktop](https://www.docker.com/products/docker-desktop/) (free)

```bash
# 1. Download Sekha
git clone https://github.com/sekha-ai/sekha-docker.git
cd sekha-docker/docker

# 2. Install (one command)
./install-local.sh

# 3. Open the dashboard
open http://localhost:8081
```

**That's it!** Sekha is now running and ready to give your AI memory.

### What Just Happened?

Sekha installed:
- **Memory Engine** - Stores all your AI conversations
- **Smart Search** - Finds relevant past conversations automatically  
- **Web Dashboard** - View and search your conversation history
- **API Endpoint** - Point your AI apps here to add memory

---

## 🎯 How Do I Use It?

### Option 1: Use Claude Desktop (Easiest)

If you use [Claude Desktop](https://claude.ai/download), Sekha adds memory tools directly:

1. Install Sekha (see above)
2. Follow the [Claude Desktop setup guide](https://docs.sekha.dev/integrations/claude-desktop/)
3. Claude can now remember all your conversations!

Claude will automatically:
- Remember what you discussed previously
- Search through past conversations
- Build on previous work

### Option 2: Use the Proxy (Works with Any AI)

Point **any** AI app to Sekha instead of directly to OpenAI/Claude/etc:

**Before:**
```python
openai.api_base = "https://api.openai.com"  # Direct to OpenAI
```

**After:**
```python
openai.api_base = "http://localhost:8081"  # Through Sekha
```

Now **every** conversation is automatically:
- Saved to your local database
- Made searchable
- Used as context for future conversations

**Works with:** ChatGPT apps, custom tools, any app using OpenAI's API format

### Option 3: Use the Web Dashboard

Visit `http://localhost:8081` to:
- Browse all your AI conversations
- Search your conversation history  
- Organize conversations into folders
- Label important discussions
- See what context your AI is using

---

## 🌟 What Can Sekha Do?

### For Regular Users

- **Never Lose Context**: Switch between ChatGPT, Claude, local AI - they all remember your conversation history
- **Automatic Recall**: Your AI automatically remembers relevant past discussions
- **Privacy First**: All data stored **locally on your computer** (not in the cloud)
- **Search Everything**: Full-text and semantic search across all conversations
- **Organize & Label**: Tag important conversations, create folders
- **Multi-AI**: Use different AIs for different tasks while maintaining conversation continuity

### For Developers

- **REST API**: 17 endpoints for conversation storage, search, and retrieval
- **MCP Protocol**: Native integration with Claude Desktop and MCP-compatible tools
- **Python & JS SDKs**: Easy integration (coming soon)
- **Multi-LLM Support**: Works with OpenAI, Anthropic, Ollama, and 100+ providers
- **Vector Search**: ChromaDB-powered semantic search
- **Full-Text Search**: SQLite FTS5 for exact phrase matching
- **Self-Hosted**: Run on your laptop, server, or cloud

### For Frontier Labs

- **Universal Memory Layer**: Add persistent memory to your AI products
- **Provider Agnostic**: Switch between LLM providers without code changes
- **High Performance**: Rust core, sub-100ms queries, handles millions of messages
- **Production Ready**: 85%+ test coverage, CI/CD, comprehensive monitoring
- **Extensible Architecture**: Plugin system for custom storage, embeddings, LLMs
- **Open Source**: AGPL-3.0 (commercial licenses available)

---

## 🏗️ How It Works (Simple Version)

```
┌─────────────────────────────────────────────────┐
│  You: "What did we discuss about Python?"      │
│  (using ChatGPT, Claude, or local AI)           │
└────────────────────┬────────────────────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │   Sekha Memory Engine  │
        │   • Searches past      │
        │     conversations      │
        │   • Finds "Python"     │
        │     discussions        │
        └────────────┬───────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │  Relevant Context:     │
        │  "Two weeks ago you    │
        │   discussed type hints │
        │   and best practices" │
        └────────────┬───────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │  Your AI responds with │
        │  full context from     │
        │  previous discussions  │
        └────────────────────────┘
```

**Behind the scenes:**
1. Sekha captures all your AI conversations
2. Stores them in a local database (SQLite + ChromaDB)
3. When you ask a new question, searches for relevant past conversations
4. Automatically provides that context to your AI
5. Your AI responds with perfect recall

---

## 🔧 For Developers: Quick Start

### Full Stack (Recommended)

```bash
# Clone deployment repo
git clone https://github.com/sekha-ai/sekha-docker.git
cd sekha-docker/docker

# Configure
cp .env.example .env
nano .env  # Set your preferences

# Deploy
docker compose -f docker-compose.prod.yml up -d

# Verify
curl http://localhost:8080/health
```

### Just the Controller (Development)

```bash
# Clone this repo
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

# Start dependencies
docker run -d -p 8000:8000 chromadb/chroma
docker run -d -p 11434:11434 ollama/ollama

# Build & run
cargo build --release
cargo run --release

# Test
curl http://localhost:8080/health
```

**API Examples:**

```bash
# Store a conversation
curl -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "label": "Python Discussion",
    "folder": "/work",
    "messages": [
      {"role": "user", "content": "What are Python type hints?"},
      {"role": "assistant", "content": "Type hints are..."}
    ]
  }'

# Search conversations
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "Python type hints", "limit": 5}'

# Get relevant context (what the proxy uses)
curl -X POST http://localhost:8080/api/v1/context/assemble \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What did we discuss about Python?",
    "context_budget": 4000
  }'
```

**See [API Documentation](https://docs.sekha.dev/api-reference/rest-api/) for all 17 endpoints.**

---

## 🎨 Architecture (For the Curious)

### The Three Components

Sekha has three main parts:

1. **Controller** (this repo) - The memory brain (Rust)
   - Stores conversations in SQLite + ChromaDB
   - Provides REST API and MCP protocol
   - Orchestrates context assembly, search, summarization

2. **[LLM-Bridge](https://github.com/sekha-ai/sekha-llm-bridge)** (required) - Universal LLM adapter (Python)
   - Handles all LLM operations (embeddings, summarization, scoring)
   - Supports 100+ LLM providers via LiteLLM
   - Enables switching between OpenAI, Claude, Ollama, etc.

3. **[Proxy](https://github.com/sekha-ai/sekha-proxy)** (optional) - Transparent capture (Python)
   - Sits between your app and LLM
   - Auto-injects context from past conversations
   - Auto-saves all conversations
   - Provides web UI dashboard

### How They Work Together

```
┌──────────────────────────────────────────────────┐
│        Your Application / AI Client              │
│   (ChatGPT app, Claude Desktop, custom tool)     │
└───────────────────┬──────────────────────────────┘
                    │
        ┌───────────┴──────────┐
        │                      │
        ▼                      ▼
   [Direct API]          [Via Proxy]
   Use REST/MCP          Zero-config
   endpoints             capture
        │                      │
        └──────────┬───────────┘
                   ▼
        ┌─────────────────────┐
        │  Sekha Controller   │  ← Memory Brain
        │  (Rust - Port 8080) │
        └──────────┬──────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌──────────┐
   │LLM     │ │SQLite  │ │ChromaDB  │
   │Bridge  │ │(Meta)  │ │(Vectors) │
   └────────┘ └────────┘ └──────────┘
        │
        ├─→ Ollama (local LLMs)
        ├─→ OpenAI (GPT-4, etc.)
        ├─→ Anthropic (Claude)
        └─→ 100+ other providers
```

**Multi-LLM Example:**
- Morning: Use Claude for code review → Sekha captures
- Afternoon: Switch to ChatGPT for docs → Sekha provides context from morning
- Evening: Use local Ollama for planning → Still has full conversation history
- **Everything stored in one unified memory**

---

## 📦 The Sekha Ecosystem

| Component | What It Does | For Whom |
|-----------|--------------|----------|
| **[sekha-controller](https://github.com/sekha-ai/sekha-controller)** ⭐ | Core memory engine | Everyone (this repo) |
| **[sekha-llm-bridge](https://github.com/sekha-ai/sekha-llm-bridge)** 🔧 | Universal LLM adapter | Required component |
| **[sekha-proxy](https://github.com/sekha-ai/sekha-proxy)** 🌐 | Transparent capture + Web UI | End users |
| **[sekha-docker](https://github.com/sekha-ai/sekha-docker)** 🐳 | One-command deployment | Everyone |
| **[sekha-mcp](https://github.com/sekha-ai/sekha-mcp)** 🔌 | Claude Desktop integration | Claude users |
| [sekha-python-sdk](https://github.com/sekha-ai/sekha-python-sdk) 🐍 | Python client library | Python developers |
| [sekha-js-sdk](https://github.com/sekha-ai/sekha-js-sdk) ⚡ | JavaScript/TypeScript SDK | JS/TS developers |
| [sekha-cli](https://github.com/sekha-ai/sekha-cli) 💻 | Terminal interface | Power users |
| [sekha-vscode](https://github.com/sekha-ai/sekha-vscode) 📝 | VS Code extension | Developers |
| [sekha-obsidian](https://github.com/sekha-ai/sekha-obsidian) 📓 | Obsidian plugin | Note-takers |

---

## 🛠️ Development

### Prerequisites

- Rust 1.83+ ([rustup](https://rustup.rs/))
- Docker Desktop ([download](https://www.docker.com/products/docker-desktop/))
- Git

### Setup

```bash
# Clone
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

# Install dev dependencies
cargo install cargo-watch cargo-tarpaulin

# Start services
docker compose -f docker-compose.dev.yml up -d

# Run
cargo run
```

### Testing

```bash
# All tests
cargo test

# With coverage
cargo tarpaulin --out Html
open tarpaulin-report.html

# Specific test
cargo test test_create_conversation

# Integration tests only
cargo test --test integration
```

### Code Quality

```bash
# Format
cargo fmt

# Lint
cargo clippy -- -D warnings

# Security audit
cargo deny check advisories

# Pre-commit checks
./scripts/pre-commit.sh
```

### Project Structure

```
sekha-controller/
├── src/
│   ├── api/              # REST API endpoints
│   │   ├── routes.rs     # Main routes (17 endpoints)
│   │   └── mcp.rs        # MCP protocol server
│   ├── models/           # Data models
│   │   ├── api.rs        # API request/response types
│   │   └── internal.rs   # Internal domain models
│   ├── storage/          # Data layer
│   │   ├── entities/     # SeaORM database entities
│   │   ├── repository.rs # Repository trait
│   │   └── chroma_client.rs  # ChromaDB client
│   ├── services/         # Business logic
│   │   ├── conversation_service.rs
│   │   ├── embedding_service.rs
│   │   └── llm_bridge_client.rs
│   ├── orchestrator/     # Intelligence layer
│   │   ├── context_assembly.rs   # 4-phase context retrieval
│   │   ├── summarizer.rs         # Hierarchical summaries
│   │   ├── pruning_engine.rs     # Memory management
│   │   └── label_intelligence.rs # Auto-labeling
│   ├── config.rs         # Configuration
│   ├── errors.rs         # Error handling
│   └── main.rs           # Entry point
├── tests/
│   ├── unit/             # Pure logic tests
│   ├── integration/      # With database
│   └── e2e/              # Full stack
├── migrations/           # Database schemas
└── Cargo.toml
```

**See [Contributing Guide](https://docs.sekha.dev/development/contributing/) for detailed development workflow.**

---

## 📊 Performance

**Benchmarks** (on M1 MacBook Pro):

- **Store Conversation**: ~50ms (including embedding generation)
- **Semantic Search**: ~30ms (vector similarity search)
- **Context Assembly**: ~100ms (full 4-phase retrieval)
- **Full-Text Search**: ~10ms (SQLite FTS5)

**Scale:**
- Tested with **1M+ messages**
- Search remains sub-100ms
- SQLite handles billions of rows
- ChromaDB scales to millions of vectors

---

## 🔒 Security & Privacy

### Your Data Stays Yours

- **All data stored locally** on your machine (not in the cloud)
- **No telemetry or tracking** (100% offline-capable)
- **Your choice of LLM** (use local Ollama for complete privacy)
- **Open source** (audit the code yourself)

### Production Deployment

```bash
# Generate secure API key
openssl rand -base64 32

# Set in .env
SEKHA__MCP_API_KEY=your-secure-key-here

# Enable HTTPS (production)
SEKHA__SERVER_TLS_CERT=/path/to/cert.pem
SEKHA__SERVER_TLS_KEY=/path/to/key.pem
```

**See [Security Guide](https://docs.sekha.dev/deployment/security/) for production hardening.**

---

## 📚 Documentation

**Complete docs:** [docs.sekha.dev](https://docs.sekha.dev)

### For Users
- [Quick Start Guide](https://docs.sekha.dev/getting-started/quickstart/)
- [Installation](https://docs.sekha.dev/getting-started/installation/)
- [Claude Desktop Setup](https://docs.sekha.dev/integrations/claude-desktop/)
- [Using the Web UI](https://docs.sekha.dev/guides/web-ui/)

### For Developers
- [API Reference](https://docs.sekha.dev/api-reference/rest-api/)
- [Architecture Overview](https://docs.sekha.dev/architecture/overview/)
- [Python SDK](https://docs.sekha.dev/sdks/python/)
- [JavaScript SDK](https://docs.sekha.dev/sdks/javascript/)

### For Deployment
- [Docker Deployment](https://docs.sekha.dev/deployment/docker-compose/)
- [Production Best Practices](https://docs.sekha.dev/deployment/production/)
- [Configuration Reference](https://docs.sekha.dev/configuration/)

---

## 💬 Community

- **Discord**: [discord.gg/sekha](https://discord.gg/sekha) - Chat with users and developers
- **Discussions**: [GitHub Discussions](https://github.com/sekha-ai/sekha-controller/discussions) - Ask questions, share ideas
- **Issues**: [GitHub Issues](https://github.com/sekha-ai/sekha-controller/issues) - Report bugs, request features
- **Twitter/X**: [@sekha_ai](https://twitter.com/sekha_ai) - Updates and announcements

---

## 📄 License

**Dual License:**

### Free Use (AGPL-3.0)
Free for:
- Personal use
- Educational use  
- Open source projects
- Small businesses (<50 employees)

### Commercial License
Required for:
- SaaS products
- Closed-source commercial use
- Enterprises (50+ employees)

**[Contact us](mailto:hello@sekha.dev) for commercial licensing.**

**[Full License Details](https://docs.sekha.dev/about/license/)**

---

## 🙏 Acknowledgments

Built with amazing open source projects:

- [Rust](https://www.rust-lang.org) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Async web framework
- [SeaORM](https://github.com/SeaQL/sea-orm) - Database ORM
- [SQLite](https://sqlite.org) - Embedded database
- [ChromaDB](https://github.com/chroma-core/chroma) - Vector database
- [Ollama](https://ollama.ai) - Local LLM runtime
- [LiteLLM](https://litellm.ai) - Universal LLM gateway

**Thank you to all our [contributors](https://github.com/sekha-ai/sekha-controller/graphs/contributors)!**

---

<div align="center">

## ⭐ Star Us on GitHub!

If Sekha helps you, please star this repo. It helps others discover the project!

[![Star History](https://api.star-history.com/svg?repos=sekha-ai/sekha-controller&type=Date)](https://star-history.com/#sekha-ai/sekha-controller&Date)

---

**[Website](https://sekha.dev)** • **[Documentation](https://docs.sekha.dev)** • **[Discord](https://discord.gg/sekha)** • **[Twitter](https://twitter.com/sekha_ai)**

*Built for AI that never forgets • 2025-2026*

</div>
