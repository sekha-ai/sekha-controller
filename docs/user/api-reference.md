# Sekha Controller API Reference

Version: 0.1.0-alpha

Base URL: http://localhost:8080

## Authentication

MCP tool endpoints require API key authentication. Include the API key in the Authorization header:

Authorization: Bearer YOUR_API_KEY_HERE

Default API Key (for development): test_key_12345678901234567890123456789012

The API key must be at least 32 characters long. Configure via:
- SEKHA_MCP_API_KEY in .env
- mcp_api_key in config.toml

## REST API Endpoints

### POST /api/v1/conversations

Create and store a new conversation.

Request Body:
{
  "label": "Project:AI-Memory",
  "folder": "/Work/AI",
  "messages": [
    {
      "role": "user",
      "content": "Design the architecture for our new AI system"
    },
    {
      "role": "assistant", 
      "content": "Here's my proposed architecture..."
    }
  ]
}

Response (201 Created):
{
  "id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4",
  "label": "Project:AI-Memory",
  "folder": "/Work/AI",
  "status": "active",
  "importance_score": 5,
  "word_count": 150,
  "session_count": 1,
  "created_at": "2025-12-12T22:14:46.578803Z",
  "updated_at": "2025-12-12T22:14:46.578803Z"
}

Error Response (400 Bad Request):
{
  "error": "Missing required field: label"
}

### GET /api/v1/conversations/{id}

Retrieve a specific conversation by ID.

Path Parameter:
- id: UUID of the conversation (e.g., 6df49c90-bc08-45ab-aced-a7e615cd9ea4)

Response (200 OK):
{
  "id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4",
  "label": "Project:AI-Memory",
  "folder": "/Work/AI",
  "status": "active",
  "importance_score": 5,
  "word_count": 150,
  "session_count": 1,
  "created_at": "2025-12-12T22:14:46.578803Z",
  "updated_at": "2025-12-12T22:14:46.578803Z"
}

Error Response (404 Not Found):
{
  "error": "Conversation not found"
}

### POST /api/v1/query

Semantic search across all conversations (mock implementation until Module 5).

Request Body:
{
  "query": "token limits architecture",
  "filters": {"label": "Project:AI"},
  "limit": 10,
  "offset": 0
}

Response (200 OK):
{
  "query": "token limits architecture",
  "results": [
    {
      "conversation_id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4",
      "message_id": "48e840de-0dbb-4b54-bf41-43daa3871680",
      "score": 0.85,
      "content": "Design the architecture for our token limit system",
      "metadata": {
        "label": "Project:AI-Memory",
        "timestamp": "2025-12-11T21:00:00Z"
      }
    }
  ],
  "total": 1,
  "limit": 10,
  "offset": 0
}

### PUT /api/v1/conversations/{id}/label

Update a conversation's label and folder.

Path Parameter:
- id: UUID of the conversation

Request Body:
{
  "label": "Updated Project Name",
  "folder": "/Work/Updated"
}

Response (204 No Content)

Error Response (404 Not Found):
{
  "error": "Conversation not found"
}

### DELETE /api/v1/conversations/{id}

Delete a conversation.

Path Parameter:
- id: UUID of the conversation

Response (204 No Content)

Error Response (404 Not Found):
{
  "error": "Conversation not found"
}

### GET /health

Health check endpoint.

Response (200 OK):
{
  "status": "healthy",
  "timestamp": "2025-12-12T22:14:46.578803Z"
}

### GET /metrics

Prometheus metrics endpoint.

Response (200 OK):
# HELP sekha_conversations_total Total number of conversations
# TYPE sekha_conversations_total gauge
sekha_conversations_total{status="active"} 42

# HELP sekha_messages_total Total number of messages
# TYPE sekha_messages_total gauge
sekha_messages_total 156

## MCP Tools

All MCP tools require authentication via the Authorization: Bearer header.

### POST /mcp/tools/memory_store

Store a conversation with metadata in the memory system.

Request Body:
{
  "label": "Project:AI-Memory",
  "folder": "/projects",
  "messages": [
    {
      "role": "user",
      "content": "Important discussion about AI capabilities"
    }
  ]
}

Response (200 OK):
{
  "success": true,
  "data": {
    "conversation_id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4"
  },
  "error": null
}

Error Response (401 Unauthorized):
{
  "success": false,
  "data": null,
  "error": "Invalid or missing API key"
}

### POST /mcp/tools/memory_query

Query conversations using semantic search. (Mock implementation until Module 5)

Request Body:
{
  "query": "AI capabilities discussion",
  "filters": {"folder": "/projects"},
  "limit": 10
}

Response (200 OK):
{
  "success": true,
  "data": {
    "results": [
      {
        "conversation_id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4",
        "message_id": "48e840de-0dbb-4b54-bf41-43daa3871680",
        "score": 0.87,
        "summary": "Discussion about AI capabilities",
        "metadata": {
          "label": "Project:AI-Memory",
          "timestamp": "2025-12-11T21:00:00Z"
        }
      }
    ]
  },
  "error": null
}

### POST /mcp/tools/memory_get_context

Retrieve hierarchical context for a specific conversation.

Request Body:
{
  "conversation_id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4"
}

Response (200 OK):
{
  "success": true,
  "data": {
    "conversation_id": "6df49c90-bc08-45ab-aced-a7e615cd9ea4",
    "label": "Project:AI-Memory",
    "status": "active",
    "message_count": 42,
    "related_conversations": [],
    "hierarchical_summary": null
  },
  "error": null
}

Error Response (404 Not Found):
{
  "success": false,
  "data": null,
  "error": "Conversation not found"
}

## Configuration

Configuration is loaded in this priority order:
1. Environment variables (highest priority)
2. .env file
3. config.toml file
4. Built-in defaults (lowest priority)

### Environment Variables

Copy .env.example to .env and configure:

Variable: SEKHA_SERVER_PORT
Description: HTTP server port
Default: 8080

Variable: SEKHA_MCP_API_KEY
Description: API key for MCP endpoints (minimum 32 characters)
Default: test_key_12345678901234567890123456789012

Variable: SEKHA_DATABASE_URL
Description: SQLite database location. Use sqlite://file.db for relative, sqlite:///absolute/path.db for absolute
Default: sqlite://sekha.db

Variable: SEKHA_OLLAMA_URL
Description: Ollama endpoint for embeddings
Default: http://localhost:11434

## Testing

### Run Unit Tests
cargo test

### Run Integration Tests
cargo test --test integration_test
cargo test --test api_test

### Run E2E Smoke Test
chmod +x tests/e2e/smoke_test.sh
./tests/e2e/smoke_test.sh

### Performance Benchmarks
cargo bench

Typical performance on M2 Mac:
- Inserts: ~10K messages/sec (single thread)
- Queries: ~500/sec (with vector similarity)
- Memory: ~50MB per 1M messages

## Error Codes

HTTP Status: 200
Error Code: OK
Description: Success

HTTP Status: 201
Error Code: Created
Description: Resource created successfully

HTTP Status: 204
Error Code: No Content
Description: Operation successful, no content to return

HTTP Status: 400
Error Code: Bad Request
Description: Invalid request format or missing required fields

HTTP Status: 401
Error Code: Unauthorized
Description: Missing or invalid API key

HTTP Status: 404
Error Code: Not Found
Description: Resource does not exist

HTTP Status: 500
Error Code: Internal Server Error
Description: Database error or unexpected exception

## Rate Limiting

Currently no rate limiting is implemented. This is planned for a future release (v0.2.0).

## WebSocket Support

WebSocket connections for real-time memory updates are planned for v0.3.0.

Generated: 2025-12-12
API Version: 0.1.0-alpha