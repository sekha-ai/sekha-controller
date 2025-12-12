┌──────────────────────────────────────────────────────────────┐
│                    AI Application Layer                       │
│  (Claude Desktop, Ollama UI, Custom Agents, etc.)          │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           │ HTTP/MCP Protocol
                           │
┌──────────────────────────▼───────────────────────────────────┐
│          AI MEMORY CONTROLLER (Standalone Service)           │
│                                                              │
│  ┌─────────────┐    ┌──────────────────┐    ┌─────────────┐ │
│  │ API Gateway │───▶│  Memory Core     │───▶│  LLM Bridge │ │
│  │ (FastAPI)   │    │  (Rust/Go)       │    │  (Python)   │ │
│  └─────────────┘    └──────────────────┘    └─────────────┘ │
│         │                 │   ▲   │               │          │
│         │                 │   │   │               │          │
│  ┌──────▼──────┐  ┌──────▼───┴─▼───▼──────┐  ┌──▼─────────┐│
│  │ Conversatio │  │ Semantic  │  Knowledge│  │  Local LLM ││
│  │ n Store     │  │ Vector    │  │  Graph  │  │  (Ollama)  ││
│  │ (PostgreSQL)│  │ Store     │  │ (Neo4j) │  │  Optional  ││
│  └─────────────┘  │ (Chroma)  │  └─────────┘  └────────────┘│
│                   └───────────┘                              │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           │ Local Filesystem
                           │
┌──────────────────────────▼───────────────────────────────────┐
│              User Data Directory (~/.ai-memory/)             │
│  /conversations/  /vectors/  /graphs/  /exports/             │
└──────────────────────────────────────────────────────────────┘
