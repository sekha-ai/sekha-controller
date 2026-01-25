# Sekha Controller

> **The Universal AI Memory System - Rust Core Engine**

[![CI](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sekha-ai/sekha-controller/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/sekha-ai/sekha-controller/branch/main/graph/badge.svg)](https://codecov.io/gh/sekha-ai/sekha-controller)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-green.svg)](https://github.com/orgs/sekha-ai/packages)

---

## What is Sekha Controller?

**Sekha Controller is the core memory engine** that gives AI persistent, searchable, infinite memory. Written in Rust for maximum performance and reliability.

This is the **main repository** for Sekha's memory orchestration engine:

- ✅ REST API (17 endpoints)
- ✅ MCP Protocol server
- ✅ Semantic + full-text search
- ✅ Context assembly & summarization
- ✅ SQLite + ChromaDB storage
- ✅ 85%+ test coverage
- ✅ Sub-100ms queries at scale

**For important things that actually need to be completed. For problems that actually need to be solved.**

---

## 📚 Documentation

**Complete documentation is at [docs.sekha.dev](https://docs.sekha.dev)**

- [Quickstart](https://docs.sekha.dev/getting-started/quickstart/) - Get running in 5 minutes
- [Installation](https://docs.sekha.dev/getting-started/installation/) - Docker, binaries, from source
- [Architecture](https://docs.sekha.dev/architecture/overview/) - How Sekha works
- [API Reference](https://docs.sekha.dev/api-reference/rest-api/) - Complete REST API docs
- [Deployment](https://docs.sekha.dev/deployment/docker-compose/) - Production deployment
- [Contributing](https://docs.sekha.dev/development/contributing/) - How to contribute

---

## 🚀 Quick Start

### Docker (Recommended)

```bash
# Use the full stack deployment
git clone https://github.com/sekha-ai/sekha-docker.git
cd sekha-docker
docker compose up -d

# Verify
curl http://localhost:8080/health
```

### From Source

```bash
# Clone this repo
git clone https://github.com/sekha-ai/sekha-controller.git
cd sekha-controller

# Start dependencies
docker run -d --name chroma -p 8000:8000 chromadb/chroma
docker run -d --name ollama -p 11434:11434 ollama/ollama

# Build and run
cargo build --release
cargo run --release

# Test it
curl -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dev-key-replace-in-production" \
  -d '{"label": "Test", "messages": [{"role": "user", "content": "Hello Sekha!"}]}'
```

**See [Installation Guide](https://docs.sekha.dev/getting-started/installation/) for detailed instructions.**

---

## 🏗️ Repository Structure

```
sekha-controller/
├── src/
│   ├── api/           # REST API endpoints
│   ├── models/        # Database models (SeaORM)
│   ├── services/      # Business logic
│   ├── orchestration/ # Memory orchestration
│   ├── mcp/          # MCP protocol server
│   └── main.rs       # Entry point
├── tests/
│   ├── unit/         # Pure logic tests
│   ├── integration/  # DB + API tests
│   └── e2e/          # Full stack tests
├── migrations/       # Database migrations
└── Cargo.toml        # Dependencies
```

---

## 🧪 Development

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run lints
cargo fmt -- --check
cargo clippy -- -D warnings

# Run security audit
cargo deny check advisories

# Run in dev mode with auto-reload
cargo watch -x run
```

**See [Contributing Guide](https://docs.sekha.dev/development/contributing/) for detailed development setup.**

---

## 🌐 Sekha Ecosystem

| Repository | Purpose | Status |
|------------|---------|--------|
| **[sekha-controller](https://github.com/sekha-ai/sekha-controller)** | **Core memory engine (Rust)** | ✅ **Production** |
| [sekha-llm-bridge](https://github.com/sekha-ai/sekha-llm-bridge) | LLM operations (Python) | ✅ Production |
| [sekha-docker](https://github.com/sekha-ai/sekha-docker) | Deployment configs | ✅ Production |
| [sekha-mcp](https://github.com/sekha-ai/sekha-mcp) | MCP protocol server | ✅ Production |
| [sekha-python-sdk](https://github.com/sekha-ai/sekha-python-sdk) | Python client | 🔜 Publishing |
| [sekha-js-sdk](https://github.com/sekha-ai/sekha-js-sdk) | JavaScript/TypeScript SDK | 🔜 Publishing |
| [sekha-vscode](https://github.com/sekha-ai/sekha-vscode) | VS Code extension | 🚧 Beta |
| [sekha-cli](https://github.com/sekha-ai/sekha-cli) | Terminal tool | 🚧 Beta |
| [sekha-obsidian](https://github.com/sekha-ai/sekha-obsidian) | Obsidian plugin | 🚧 Beta |

---

## 📄 License

**Dual License:**

- **AGPL-3.0** - Free for personal, educational, and small business use (<50 employees)
- **Commercial License** - For enterprises, contact [hello@sekha.dev](mailto:hello@sekha.dev)

**See [License Details](https://docs.sekha.dev/about/license/) for full information.**

---

## 🔗 Links

- **Website:** [sekha.dev](https://sekha.dev)
- **Documentation:** [docs.sekha.dev](https://docs.sekha.dev)
- **Discord:** [discord.gg/sekha](https://discord.gg/sekha)
- **Discussions:** [GitHub Discussions](https://github.com/sekha-ai/sekha-controller/discussions)
- **Issues:** [GitHub Issues](https://github.com/sekha-ai/sekha-controller/issues)

---

## 🙏 Acknowledgments

Built with:

- [Axum](https://github.com/tokio-rs/axum) - Async web framework
- [SeaORM](https://github.com/SeaQL/sea-orm) - Rust ORM
- [ChromaDB](https://github.com/chroma-core/chroma) - Vector database
- [SQLite](https://sqlite.org) - Embedded database
- [Ollama](https://ollama.ai) - Local LLM runtime

---

<div align="center">

**Built for AI that never forgets**

[⭐ Star us on GitHub](https://github.com/sekha-ai/sekha-controller) • [📖 Read the Docs](https://docs.sekha.dev) • [💬 Join Discord](https://discord.gg/sekha)

*Sekha Project • 2025-2026*

</div>
