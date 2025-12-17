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

    config = ClientConfig(api_key="sk-sekha-test-key-for-testing-only")
    client = SekhaClient(config)

    # Create a mock that tracks calls but allows method assignment
    mock_httpx_client = AsyncMock()

    # Set up default mock response
    default_response = Mock()
    default_response.raise_for_status = Mock()
    default_response.json = Mock(
        return_value={
            "id": "test-uuid",
            "label": "Test",
            "folder": "/",
            "status": "active",
            "message_count": 1,
            "created_at": datetime.now().isoformat(),
        }
    )

    # Set default return values for all common methods
    mock_httpx_client.post = AsyncMock(return_value=default_response)
    mock_httpx_client.get = AsyncMock(return_value=default_response)
    mock_httpx_client.put = AsyncMock(return_value=default_response)
    mock_httpx_client.delete = AsyncMock(return_value=default_response)

    client.client = mock_httpx_client

    yield client

    await client.close()


@pytest.mark.asyncio
async def test_create_conversation(mock_client):
    """Test creating a conversation"""
    conv = NewConversation(
        label="Test",
        folder="/",
        messages=[MessageDto(role=MessageRole.USER, content="Hello")],
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
    mock_response.json = Mock(
        return_value={
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
        }
    )

    mock_client.client.post = AsyncMock(return_value=mock_response)

    result = await mock_client.smart_query("test query")

    assert result.total == 1
    assert len(result.results) == 1
    assert result.results[0].content == "test"


@pytest.mark.asyncio
async def test_pin_conversation(mock_client):
    """Test pinning a conversation"""
    # Setup mock for put
    mock_response = Mock()
    mock_response.raise_for_status = Mock()

    # Override the put method for this test
    mock_client.client.put = AsyncMock(return_value=mock_response)

    # Test pin
    await mock_client.pin("test-conv-123")

    # Verify the call
    assert mock_client.client.put.called
    call_args = mock_client.client.put.call_args
    assert "/conversations/test-conv-123/status" in call_args[0][0]
    assert call_args[1]["json"]["status"] == "pinned"


@pytest.mark.asyncio
async def test_archive_conversation(mock_client):
    """Test archiving a conversation"""
    # Setup mock for put
    mock_response = Mock()
    mock_response.raise_for_status = Mock()

    mock_client.client.put = AsyncMock(return_value=mock_response)

    # Test archive
    await mock_client.archive("test-conv-123")

    # Verify the call
    assert mock_client.client.put.called
    call_args = mock_client.client.put.call_args
    assert call_args[1]["json"]["status"] == "archived"


@pytest.mark.asyncio
async def test_export_conversations(mock_client):
    """Test export functionality"""
    # Setup mock for get
    mock_response = Mock()
    mock_response.raise_for_status = Mock()
    mock_response.json = Mock(
        return_value={
            "content": "# Test Export\n\n## Conversation 1",
            "format": "markdown",
            "conversation_count": 1,
        }
    )

    mock_client.client.get = AsyncMock(return_value=mock_response)

    # Test export
    result = await mock_client.export(label="Project:AI", format="markdown")

    assert result == "# Test Export\n\n## Conversation 1"
    assert mock_client.client.get.called
    call_args = mock_client.client.get.call_args
    assert call_args[1]["params"]["format"] == "markdown"
    assert call_args[1]["params"]["label"] == "Project:AI"


@pytest.mark.asyncio
async def test_export_conversations_json(mock_client):
    """Test export as JSON"""
    # Setup mock for get
    mock_response = Mock()
    mock_response.raise_for_status = Mock()
    mock_response.json = Mock(
        return_value={
            "content": '[{"id": "123", "label": "Test"}]',
            "format": "json",
            "conversation_count": 1,
        }
    )

    mock_client.client.get = AsyncMock(return_value=mock_response)

    result = await mock_client.export(format="json")

    assert '[{"id": "123"' in result
    assert mock_client.client.get.called
    call_args = mock_client.client.get.call_args
    assert call_args[1]["params"]["format"] == "json"
