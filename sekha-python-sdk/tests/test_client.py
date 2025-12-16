"""
Tests for SekhaClient
"""

import pytest
import asyncio
from unittest.mock import Mock, AsyncMock, patch
from datetime import datetime

from sekha import (
    SekhaClient,
    SekhaAPIError,
    SekhaNotFoundError,
    NewConversation,
    MessageDto,
    ConversationResponse,
    QueryRequest,
    ConversationStatus,
    MessageRole,
)


@pytest.fixture
async def mock_client():
    """Create a client with mocked httpx"""
    from sekha.client import ClientConfig
    
    config = ClientConfig(api_key="sk-sekha-test-key")
    client = SekhaClient(config)
    
    # Mock the httpx client
    mock_response = Mock()
    mock_response.raise_for_status = Mock()
    mock_response.json = Mock(return_value={
        "id": "test-uuid",
        "label": "Test",
        "folder": "/",
        "status": "active",
        "message_count": 1,
        "created_at": datetime.now().isoformat(),
    })
    
    client.client = AsyncMock()
    client.client.post = AsyncMock(return_value=mock_response)
    client.client.get = AsyncMock(return_value=mock_response)
    
    yield client
    
    await client.close()


@pytest.mark.asyncio
async def test_create_conversation(mock_client):
    """Test creating a conversation"""
    conv = NewConversation(
        label="Test",
        folder="/",
        messages=[
            MessageDto(role=MessageRole.USER, content="Hello")
        ]
    )
    
    result = await mock_client.create_conversation(conv)
    
    assert isinstance(result, ConversationResponse)
    assert result.label == "Test"
    assert mock_client.client.post.called


@pytest.mark.asyncio
async def test_smart_query(mock_client):
    """Test smart query"""
    # Mock the response
    mock_response = Mock()
    mock_response.raise_for_status = Mock()
    mock_response.json = Mock(return_value={
        "results": [
            {
                "conversation_id": "conv-123",
                "message_id": "msg-456",
                "score": 0.8,
                "content": "test",
                "metadata": {},
                "label": "Test",
                "folder": "/",
                "timestamp": datetime.now().isoformat(),
            }
        ],
        "total": 1,
        "page": 1,
        "page_size": 1,
    })
    
    mock_client.client.post = AsyncMock(return_value=mock_response)
    
    result = await mock_client.smart_query("test query")
    
    assert result.total == 1
    assert len(result.results) == 1
    assert result.results[0].content == "test"
