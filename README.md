# Sekha Controller

> **The Memory Engine for AI - Persistent, Searchable, Infinite Context**

[![CI](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sekha-ai/sekha-controller/branch/main/graph/badge.svg)](https://codecov.io/gh/sekha-ai/sekha-controller)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-green.svg)](https://github.com/orgs/sekha-ai/packages)

---

## What is Sekha Controller?

Sekha Controller is a **high-performance memory orchestration engine** written in Rust that provides persistent, searchable memory for AI systems. It enables AI assistants to remember conversations across sessions, recall relevant context automatically, and maintain perfect continuity regardless of which LLM provider you use.

**Core Capabilities:**
- **Universal Memory Layer**: Works with any LLM (OpenAI, Anthropic, Ollama, etc.)
- **Intelligent Context Assembly**: 4-phase retrieval algorithm combining semantic search, recency, and importance
- **Dual Storage**: SQLite for metadata + ChromaDB for vector embeddings
- **REST API + MCP Protocol**: 17 REST endpoints + native Claude Desktop integration
- **Production Ready**: 85%+ test coverage, <100ms query times, scales to millions of messages

---

## Quick Links

- 🚀 **[Get Started](https://docs.sekha.dev/getting-started/quickstart/)** - Install and run in 5 minutes
- 📚 **[Full Documentation](https://docs.sekha.dev)** - Complete guides and API reference
- 🐳 **[Deployment Options](https://github.com/sekha-ai/sekha-docker)** - Docker Compose, Kubernetes, local binary
- 🐛 **[Report Issues](https://github.com/sekha-ai/sekha-controller/issues)** - Bug reports and feature requests
- 💬 **[Community](https://discord.gg/sekha)** - Discord, discussions, and support

---

## Architecture Overview

Sekha Controller is the **central orchestration engine** in a three-component architecture:

```
┌───────────────────────────────────────────────┐
│          Applications & Clients              │
│   (Claude Desktop, ChatGPT, custom apps)     │
└─────────────────┬──────────────────────────────┘
                 │
     ┌───────────┼──────────┐
     │            │           │
     ▼            ▼           ▼
  REST API      MCP       Proxy
  Direct     Protocol   (Optional)
     │            │           │
     └───────────┼──────────┘
                 ▼
┌───────────────────────────────────────────────┐
│       Sekha Controller (Port 8080)          │
│                                             │
│  • Memory Orchestration                    │
│  • Context Assembly (4-phase algorithm)    │
│  • Storage Management (SQLite + Chroma)    │
│  • Pruning & Summarization                 │
│  • Label Intelligence                      │
└────────────────┬──────────────────────────────┘
                 │
     ┌───────────┼──────────┐
     │            │           │
     ▼            ▼           ▼
┌────────┐  ┌────────┐  ┌──────────┐
│  LLM   │  │ SQLite │  │ ChromaDB │
│ Bridge │  │  FTS5  │  │  Vectors │
└────────┘  └────────┘  └──────────┘
 Required    Metadata   Embeddings
```

**Component Relationships:**

| Component | Purpose | Status |
|-----------|---------|--------|
| **Controller** (this repo) | Memory orchestration engine | Core component |
| [LLM-Bridge](https://github.com/sekha-ai/sekha-llm-bridge) | Universal LLM adapter | Required dependency |
| [Proxy](https://github.com/sekha-ai/sekha-proxy) | Transparent capture + Web UI | Optional tool |

**See [Architecture Documentation](https://docs.sekha.dev/architecture/overview/) for detailed system design.**

---

## Deployment Options

Choose the deployment tier that matches your needs:

### Tier 1: Local Binary *(Coming Soon)*
Single executable with minimal dependencies.
```bash
# Via Homebrew (planned)
brew install sekha

# Or download binary
curl -L https://sekha.dev/install.sh | bash
```
**Status**: Not yet available (pending crate publishing dependencies)  
**Target**: Quick evaluation, offline use, individual developers

### Tier 2: Docker Compose *(Recommended)*
Complete stack with one command.
```bash
git clone https://github.com/sekha-ai/sekha-docker.git
cd sekha-docker/docker
./deploy-docker.sh
```
**Status**: Production ready  
**Target**: Most installations, development, testing  
**See**: [Docker Deployment Guide](https://docs.sekha.dev/deployment/docker-compose/)

### Tier 3: Cloud Native *(Kubernetes)*
Horizontal scaling for teams and enterprises.
```bash
helm install sekha sekha/sekha-controller \
  --set replicas=3 \
  --set persistence.enabled=true
```
**Status**: Production ready  
**Target**: Enterprise deployments, teams, high-availability  
**See**: [Kubernetes Guide](https://docs.sekha.dev/deployment/kubernetes/)

### Tier 4: Hybrid *(Local + Cloud)*
Local storage with cloud LLM providers.
```bash
# .env configuration
BRIDGE_PROVIDER=anthropic
BRIDGE_FALLBACK=ollama
```
**Status**: Production ready  
**Target**: Privacy + cloud model access  
**See**: [Hybrid Configuration](https://docs.sekha.dev/deployment/hybrid/)

### Tier 5: Federated *(Future)*
Multi-instance sync for distributed teams.
```bash
# Coming in Q2 2026
SEKHA_SYNC_BACKEND=s3://team-memory
SEKHA_SYNC_STRATEGY=crdt
```
**Status**: Planned  
**Target**: Team collaboration, shared memory

**For detailed installation instructions, see [sekha-docker repository](https://github.com/sekha-ai/sekha-docker).**

---

## Key Features

### Memory Orchestration
- **4-Phase Context Assembly**: Semantic search → Ranking → Assembly → Enhancement
- **Hierarchical Summarization**: Daily/weekly/monthly conversation summaries
- **Smart Pruning**: Importance-based memory management
- **Label Intelligence**: AI-powered conversation categorization

### Storage & Retrieval
- **Hybrid Search**: Vector similarity (ChromaDB) + full-text (SQLite FTS5)
- **Sub-100ms Queries**: Optimized for real-time context injection
- **Millions of Messages**: Tested at scale with production workloads
- **Persistent State**: Local SQLite database (~/.sekha/sekha.db)

### API & Integration
- **17 REST Endpoints**: Complete conversation lifecycle management
- **MCP Protocol**: Native Claude Desktop integration
- **OpenAPI Spec**: Auto-generated API documentation
- **SDKs**: Python, JavaScript/TypeScript (publishing soon)

### Production Quality
- **85%+ Test Coverage**: Unit, integration, and E2E tests
- **CI/CD Pipeline**: Automated testing, linting, security audits
- **Observability**: Prometheus metrics, structured logging
- **Type Safety**: Rust's compile-time guarantees

---

## API Overview

### Core Endpoints

| Endpoint | Method | Purpose |
|----------|--------|----------|
| `/api/v1/conversations` | POST | Store conversation with auto-labeling |
| `/api/v1/conversations/{id}` | GET | Retrieve conversation by ID |
| `/api/v1/query` | POST | Semantic + full-text search |
| `/api/v1/context/assemble` | POST | Get relevant context for query |
| `/api/v1/prune/by-importance` | POST | Remove low-value conversations |
| `/api/v1/summarize/daily` | POST | Generate daily summary |
| `/api/v1/labels/suggest` | POST | AI-powered label suggestions |
| `/health` | GET | Health check + metrics |

**Example: Store Conversation**
```bash
curl -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "label": "Python Type Hints",
    "folder": "/work/python",
    "messages": [
      {"role": "user", "content": "Explain Python type hints"},
      {"role": "assistant", "content": "Type hints are..."}
    ]
  }'
```

**Example: Context Assembly**
```bash
curl -X POST http://localhost:8080/api/v1/context/assemble \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What did we discuss about Python?",
    "context_budget": 4000,
    "preferred_labels": ["python", "coding"]
  }'
```

**Full API documentation: [docs.sekha.dev/api-reference](https://docs.sekha.dev/api-reference/rest-api/)**

---

## Development

### Prerequisites
- Rust 1.83+ ([install rustup](https://rustup.rs/))
- Docker Desktop ([download](https://www.docker.com/products/docker-desktop/))
- Git

### Setup

```bash
# Clone repository
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

# Start dependencies
docker compose -f docker-compose.dev.yml up -d

# Build and run
cargo build --release
cargo run --release

# Verify
curl http://localhost:8080/health
```

### Testing

```bash
# All tests
cargo test

# With coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Specific test suite
cargo test --test integration_tests

# Watch mode (auto-rerun on changes)
cargo install cargo-watch
cargo watch -x test
```

### Code Quality

```bash
# Format
cargo fmt --check

# Lint
cargo clippy -- -D warnings

# Security audit
cargo install cargo-deny
cargo deny check advisories

# Pre-commit checks
./scripts/pre-commit.sh
```

### Project Structure

```
sekha-controller/
├── src/
│   ├── api/                  # HTTP endpoints
│   │   ├── routes.rs         # REST API (17 endpoints)
│   │   └── mcp.rs            # MCP protocol server
│   ├── models/               # Data structures
│   │   ├── api.rs            # Request/response types
│   │   └── internal.rs       # Domain models
│   ├── storage/              # Persistence layer
│   │   ├── entities/         # SeaORM entities
│   │   ├── repository.rs     # Repository trait
│   │   └── chroma_client.rs  # Vector DB client
│   ├── services/             # Business logic
│   │   ├── conversation_service.rs
│   │   ├── embedding_service.rs
│   │   └── llm_bridge_client.rs
│   ├── orchestrator/         # Intelligence
│   │   ├── context_assembly.rs
│   │   ├── summarizer.rs
│   │   ├── pruning_engine.rs
│   │   └── label_intelligence.rs
│   ├── config.rs             # Configuration
│   ├── errors.rs             # Error types
│   └── main.rs               # Entry point
├── tests/
│   ├── unit/                 # Pure logic tests
│   ├── integration/          # Database tests
│   └── e2e/                  # Full stack tests
├── migrations/               # SQL migrations
├── scripts/
│   ├── install-local.sh      # Tier 1 installer
│   ├── deploy-docker.sh      # Tier 2 deployment
│   └── deploy-k8s.sh         # Tier 3 deployment
└── Cargo.toml                # Dependencies
```

**Contributing guide: [docs.sekha.dev/development/contributing](https://docs.sekha.dev/development/contributing/)**

---

## Performance Benchmarks

**Test Environment**: M1 MacBook Pro, 1M messages in database

| Operation | Latency | Notes |
|-----------|---------|-------|
| Store Conversation | ~50ms | Including embedding generation |
| Semantic Search | ~30ms | ChromaDB vector similarity |
| Full-Text Search | ~10ms | SQLite FTS5 index |
| Context Assembly | ~100ms | Full 4-phase retrieval |
| Daily Summary | ~2s | LLM-dependent |

**Scalability:**
- SQLite handles billions of rows
- ChromaDB scales to millions of vectors
- Search performance remains sub-100ms at scale

---

## Ecosystem

### Official Repositories

| Repository | Description | Status |
|------------|-------------|--------|
| **[sekha-controller](https://github.com/sekha-ai/sekha-controller)** | Core memory engine (Rust) | ✅ Production |
| **[sekha-llm-bridge](https://github.com/sekha-ai/sekha-llm-bridge)** | Universal LLM adapter (Python) | ✅ Production |
| **[sekha-proxy](https://github.com/sekha-ai/sekha-proxy)** | Transparent proxy + Web UI (Python) | ✅ Production |
| **[sekha-docker](https://github.com/sekha-ai/sekha-docker)** | Deployment configurations | ✅ Production |
| [sekha-mcp](https://github.com/sekha-ai/sekha-mcp) | MCP server for Claude | ✅ Production |
| [sekha-python-sdk](https://github.com/sekha-ai/sekha-python-sdk) | Python client library | 📦 Publishing |
| [sekha-js-sdk](https://github.com/sekha-ai/sekha-js-sdk) | JavaScript/TypeScript SDK | 📦 Publishing |
| [sekha-cli](https://github.com/sekha-ai/sekha-cli) | Command-line interface | 🚧 Beta |
| [sekha-vscode](https://github.com/sekha-ai/sekha-vscode) | VS Code extension | 🚧 Beta |
| [sekha-obsidian](https://github.com/sekha-ai/sekha-obsidian) | Obsidian plugin | 🚧 Beta |

### Community & Support

- **Documentation**: [docs.sekha.dev](https://docs.sekha.dev) - Complete guides and API reference
- **Discord**: [discord.gg/sekha](https://discord.gg/sekha) - Real-time chat and support
- **Discussions**: [GitHub Discussions](https://github.com/sekha-ai/sekha-controller/discussions) - Q&A and ideas
- **Issues**: [GitHub Issues](https://github.com/sekha-ai/sekha-controller/issues) - Bug reports and features
- **Twitter/X**: [@sekha_ai](https://twitter.com/sekha_ai) - Updates and announcements

---

## License

**AGPL-3.0** with commercial licensing option.

### Open Source Use (Free)
Sekha Controller is free for:
- Personal use
- Educational and research use
- Open source projects
- Businesses with fewer than 50 employees

### Commercial License (Required)
Businesses with 50 or more employees (including frontier labs) require a commercial license.

**Contact**: [hello@sekha.dev](mailto:hello@sekha.dev)  
**Details**: [docs.sekha.dev/about/license](https://docs.sekha.dev/about/license/)

---

## Built With

Sekha Controller is built on exceptional open source projects:

- [Rust](https://www.rust-lang.org) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Ergonomic async web framework  
- [SeaORM](https://github.com/SeaQL/sea-orm) - Async ORM for Rust
- [SQLite](https://sqlite.org) - Embedded relational database
- [ChromaDB](https://github.com/chroma-core/chroma) - AI-native vector database
- [Tokio](https://tokio.rs) - Async runtime

**Thank you to all [contributors](https://github.com/sekha-ai/sekha-controller/graphs/contributors)!**

---

<div align="center">

**[Website](https://sekha.dev)** • **[Documentation](https://docs.sekha.dev)** • **[Discord](https://discord.gg/sekha)** • **[Twitter](https://twitter.com/sekha_ai)**

*Persistent memory for AI systems • 2025-2026*

</div>
