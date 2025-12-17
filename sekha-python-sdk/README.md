# Sekha Python SDK

Python client library for the Sekha AI Memory System.

## Installation

```bash
pip install sekha-memory

## Quick Start

import asyncio
from sekha import SekhaClient, ClientConfig, NewConversation, MessageDto

async def main():
    # Configure client
    config = ClientConfig(
        api_key="sk-sekha-your-api-key-here",
        base_url="http://localhost:8080"
    )
    
    async with SekhaClient(config) as client:
        # Create a conversation
        conv = NewConversation(
            label="Project:AI",
            folder="/work",
            messages=[
                MessageDto(role="user", content="What is token limit for Claude?"),
                MessageDto(role="assistant", content="200K tokens"),
            ]
        )
        
        result = await client.create_conversation(conv)
        print(f"Created: {result.id}")
        
        # Smart query
        response = await client.smart_query("token limits")
        for msg in response.results:
            print(f"- {msg.content} (from {msg.label})")

# Run
asyncio.run(main())


Features

Core Operations
  Create/Get/Delete conversations
  Smart context assembly - Automatically ranks by importance, recency, and relevance
  Semantic search - Vector-based search across all messages
  Label management - Organize conversations hierarchically

Intelligence (Requires LLM Bridge)
  Importance scoring - LLM rates messages 1-10
  Hierarchical summarization - Daily/weekly/monthly summaries
  Pruning suggestions - Intelligent archive recommendations
  Auto-labeling - LLM suggests and applies labels

Advanced
  Rate limiting - Automatic (1000 req/min default)
  Retry logic - Exponential backoff with jitter
  Connection pooling - Reuses connections for performance
  Type hints - Full IDE support with autocomplete


## Configuration

config = ClientConfig(
    api_key="sk-sekha-...",           # Required
    base_url="http://localhost:8080",  # Default
    timeout=30.0,                      # Request timeout (seconds)
    max_retries=3,                     # Retry attempts
    rate_limit_requests=1000,          # Per minute
)

## Synchronous Usage

from sekha import SyncSekhaClient

with SyncSekhaClient(config) as client:
    # All async methods available synchronously
    result = client.create_conversation(conv)
    response = client.smart_query("token limits")

## Error Handling

from sekha import SekhaError, SekhaNotFoundError, SekhaAPIError

try:
    conv = await client.get_conversation("invalid-id")
except SekhaNotFoundError:
    print("Conversation not found")
except SekhaAPIError as e:
    print(f"API Error {e.status_code}: {e.message}")
except SekhaError as e:
    print(f"Unexpected error: {e}")


### **Pinning and Archiving Conversations**

```python
from sekha import SekhaClient

async with SekhaClient(config) as client:
    # Pin an important conversation
    await client.pin("conversation-uuid-here")
    
    # Archive an old conversation
    await client.archive("conversation-uuid-here")
    
    # Pinned conversations get boosted in search results
    # Archived conversations are excluded from default searches


### **Exporting Conversations**

from sekha import SekhaClient

async with SekhaClient(config) as client:
    # Export all conversations with a specific label as markdown
    markdown = await client.export(label="Project:AI", format="markdown")
    with open("backup.md", "w") as f:
        f.write(markdown)
    
    # Export all conversations as JSON
    json_data = await client.export(format="json")
    with open("backup.json", "w") as f:
        f.write(json_data)

    # Export without label filter (includes all conversations)
    all_markdown = await client.export(format="markdown")

## Development

git clone https://github.com/sekha-ai/sekha-controller
cd sekha-controller/sekha-python-sdk
pip install -r requirements.txt
pip install -e ".[dev]"

# Run tests
pytest tests/

# Type check
mypy sekha/

# Format code
black sekha/
isort sekha/

git clone https://github.com/sekha-ai/sekha-controller
cd sekha-controller/sekha-python-sdk
pip install -r requirements.txt
pip install -e ".[dev]"

# Run tests
pytest tests/

# Type check
mypy sekha/

# Format code
black sekha/
isort sekha/

## Lincense
AGPL-3.0 license