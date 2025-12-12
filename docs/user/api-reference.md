# API Reference

## Authentication

All API endpoints (except health/metrics) require MCP API key:


## REST Endpoints

### POST /api/v1/conversations
Store a new conversation.

**Request:**
```json
{
  "label": "Project:AI-Memory",
  "folder": "/Work/AI",
  "messages": [
    {
      "role": "user",
      "content": "Design the architecture"
    }
  ]
}

**Response:**
{
  "id": "8a49d44f-3329-4797-ae7d-50d977f6227a",
  "label": "Project:AI-Memory",
  "folder": "/Work/AI",
  "status": "active",
  "message_count": 1,
  "created_at": "2025-12-11T21:00:00Z"
}


### POST /api/v1/query
Semantic search (mock until Module 5).

**Request:**
```json
{
  "query": "token limits",
  "filters": {"label": "Project:AI"},
  "limit": 10
}

**Response:**
{
  "query": "token limits",
  "results": [
    {
      "conversation_id": "...",
      "message_id": "...",
      "score": 0.85,
      "content": "...",
      "metadata": {...}
    }
  ],
  "total": 1
}



MCP Tools
memory_store - Store conversation with metadata.
memory_query - Semantic search across all conversations.
memory_get_context - Get hierarchical context for a label/project.
