"""
Main client for interacting with Sekha API
"""

import asyncio
import httpx
from typing import Optional, List, Dict, Any, Union
from dataclasses import dataclass
import backoff

from .models import *
from .errors import *
from .utils import RateLimiter, ExponentialBackoff, validate_api_key


@dataclass
class ClientConfig:
    """Client configuration"""

    api_key: str
    base_url: str = "http://localhost:8080"
    timeout: float = 30.0
    max_retries: int = 3
    rate_limit_requests: int = 1000  # per minute
    rate_limit_window: float = 60.0


class SekhaClient:
    """
    Main client for Sekha AI Memory System

    Supports both sync and async operations.
    Automatically handles auth, retries, and rate limiting.
    """

    def __init__(self, config: ClientConfig):
        """
        Initialize the client

        Args:
            config: ClientConfiguration object
        """
        validate_api_key(config.api_key)

        self.config = config
        self.rate_limiter = RateLimiter(
            config.rate_limit_requests, config.rate_limit_window
        )
        self.backoff = ExponentialBackoff(base_delay=0.5, max_delay=10.0, factor=2.0)

        # Create httpx client with connection pooling
        self.client = httpx.AsyncClient(
            base_url=config.base_url,
            timeout=config.timeout,
            headers={
                "Authorization": f"Bearer {config.api_key}",
                "User-Agent": "Sekha-Python-SDK/0.5.0",
            },
        )

        # For sync operations, we'll create clients on-demand
        self._sync_client: Optional[httpx.Client] = None

    async def __aenter__(self):
        """Async context manager entry"""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        await self.close()

    async def close(self):
        """Close the async client"""
        if hasattr(self, "client") and self.client:
            await self.client.aclose()

    @property
    def sync_client(self) -> httpx.Client:
        """Lazy-load sync client"""
        if self._sync_client is None:
            self._sync_client = httpx.Client(
                base_url=self.config.base_url,
                timeout=self.config.timeout,
                headers={
                    "Authorization": f"Bearer {self.config.api_key}",
                    "User-Agent": "Sekha-Python-SDK/0.5.0",
                },
            )
        return self._sync_client

    # ============== Conversation Operations ==============

    @backoff.on_exception(
        backoff.expo, (SekhaConnectionError, httpx.TimeoutException), max_tries=3
    )
    async def create_conversation(
        self,
        conversation: NewConversation,
    ) -> ConversationResponse:
        """
        Create a new conversation with messages

        Args:
            conversation: NewConversation object

        Returns:
            Created conversation with ID

        Raises:
            SekhaValidationError: Invalid input
            SekhaAPIError: API returned error
        """
        await self.rate_limiter.acquire()

        try:
            response = await self.client.post(
                "/api/v1/conversations",
                json=conversation.model_dump(),
            )
            response.raise_for_status()
            return ConversationResponse(**response.json())

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 400:
                raise SekhaValidationError("Invalid conversation data", e.response.text)
            elif e.response.status_code == 401:
                raise SekhaAuthError("Invalid API key")
            else:
                raise SekhaAPIError(
                    f"Failed to create conversation: {e.response.text}",
                    e.response.status_code,
                    e.response.text,
                )
        except httpx.TimeoutException:
            raise SekhaConnectionError("Request timed out")
        except Exception as e:
            raise SekhaError(f"Unexpected error: {e}")

    async def get_conversation(self, conversation_id: str) -> ConversationResponse:
        """Get conversation by ID"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.get(f"/api/v1/conversations/{conversation_id}")
            response.raise_for_status()
            return ConversationResponse(**response.json())

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise SekhaNotFoundError(f"Conversation {conversation_id} not found")
            raise SekhaAPIError(
                "Failed to get conversation", e.response.status_code, e.response.text
            )

    async def list_conversations(
        self,
        label: Optional[str] = None,
        page: int = 1,
        page_size: int = 50,
    ) -> List[ConversationResponse]:
        """List conversations with optional filtering"""
        await self.rate_limiter.acquire()

        params: Dict[str, Any] = {"page": page, "page_size": page_size}
        if label:
            params["label"] = label

        try:
            response = await self.client.get(
                "/api/v1/conversations",
                params=params,
            )
            response.raise_for_status()
            data = response.json()
            return [ConversationResponse(**conv) for conv in data.get("results", [])]

        except Exception as e:
            raise SekhaError(f"Failed to list conversations: {e}")

    async def update_label(
        self,
        conversation_id: str,
        new_label: str,
        new_folder: Optional[str] = None,
    ) -> None:
        """Update conversation label and folder"""
        await self.rate_limiter.acquire()

        body = {"label": new_label}
        if new_folder:
            body["folder"] = new_folder

        try:
            response = await self.client.put(
                f"/api/v1/conversations/{conversation_id}/label",
                json=body,
            )
            response.raise_for_status()

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise SekhaNotFoundError(f"Conversation {conversation_id} not found")
            raise SekhaAPIError(
                "Failed to update label", e.response.status_code, e.response.text
            )

    async def delete_conversation(self, conversation_id: str) -> None:
        """Delete a conversation"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.delete(
                f"/api/v1/conversations/{conversation_id}"
            )
            response.raise_for_status()

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise SekhaNotFoundError(f"Conversation {conversation_id} not found")
            raise SekhaAPIError(
                "Failed to delete conversation", e.response.status_code, e.response.text
            )

    # ============== Smart Query ==============

    async def smart_query(
        self,
        query: str,
        limit: Optional[int] = None,
        filters: Optional[Dict[str, Any]] = None,
    ) -> QueryResponse:
        """
        Intelligent context assembly using MemoryOrchestrator

        Args:
            query: Search query
            limit: Max results
            filters: Metadata filters

        Returns:
            Query response with assembled context
        """
        await self.rate_limiter.acquire()

        body = QueryRequest(query=query, limit=limit, filters=filters)

        try:
            response = await self.client.post(
                "/api/v1/query/smart",
                json=body.model_dump(),
            )
            response.raise_for_status()
            return QueryResponse(**response.json())

        except Exception as e:
            raise SekhaError(f"Smart query failed: {e}")

    # ============== Importance Scoring ==============

    async def score_message_importance(
        self,
        message_id: str,
    ) -> ImportanceScore:
        """Score a message's importance 1-10 using LLM"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.post(
                f"/api/v1/messages/{message_id}/importance",
            )
            response.raise_for_status()
            return ImportanceScore(**response.json())

        except Exception as e:
            raise SekhaError(f"Failed to score message: {e}")

    # ============== Summarization ==============

    async def generate_summary(
        self,
        conversation_id: str,
        level: SummaryLevel = SummaryLevel.DAILY,
    ) -> SummaryResponse:
        """Generate hierarchical summary for conversation"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.post(
                f"/api/v1/conversations/{conversation_id}/summary",
                params={"level": level.value},
            )
            response.raise_for_status()
            return SummaryResponse(**response.json())

        except Exception as e:
            raise SekhaError(f"Failed to generate summary: {e}")

    # ============== Pruning ==============

    async def get_pruning_suggestions(
        self,
        threshold_days: int = 90,
        importance_threshold: float = 3.0,
    ) -> List[PruningSuggestion]:
        """Get intelligent pruning suggestions"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.get(
                "/api/v1/prune/suggestions",
                params={
                    "threshold_days": threshold_days,
                    "importance_threshold": importance_threshold,
                },
            )
            response.raise_for_status()
            data = response.json()
            return [PruningSuggestion(**s) for s in data]

        except Exception as e:
            raise SekhaError(f"Failed to get pruning suggestions: {e}")

    # ============== Label Intelligence ==============

    async def suggest_labels(
        self,
        conversation_id: str,
    ) -> List[LabelSuggestion]:
        """Get auto-label suggestions for conversation"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.post(
                f"/api/v1/conversations/{conversation_id}/suggest-labels",
            )
            response.raise_for_status()
            data = response.json()
            return [LabelSuggestion(**s) for s in data]

        except Exception as e:
            raise SekhaError(f"Failed to suggest labels: {e}")

    async def auto_label(
        self,
        conversation_id: str,
        threshold: float = 0.7,
    ) -> Optional[str]:
        """
        Auto-apply label if confidence exceeds threshold

        Returns:
            Applied label or None if no label met threshold
        """
        suggestions = await self.suggest_labels(conversation_id)

        for suggestion in suggestions:
            if suggestion.confidence >= threshold:
                await self.update_label(
                    conversation_id,
                    suggestion.label,
                )
                return suggestion.label

        return None

    # ============== Pin, Archive, Export ==================
    async def pin(self, conversation_id: str) -> None:
        """Pin a conversation (status = 'pinned')"""
        await self._update_status(conversation_id, "pinned")

    async def archive(self, conversation_id: str) -> None:
        """Archive a conversation (status = 'archived')"""
        await self._update_status(conversation_id, "archived")

    async def export(
        self, label: Optional[str] = None, format: str = "markdown"
    ) -> str:
        """Export conversations to specified format

        Args:
            label: Optional label filter (exports all if None)
            format: "markdown" or "json"

        Returns:
            Exported content as string

        Raises:
            SekhaValidationError: Invalid format parameter
            SekhaAPIError: API returned error
        """
        await self.rate_limiter.acquire()

        params = {"format": format}
        if label:
            params["label"] = label

        try:
            response = await self.client.get(
                "/api/v1/export",
                params=params,
            )
            response.raise_for_status()

            data = response.json()
            return data["content"]

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 400:
                raise SekhaValidationError("Invalid export parameters", e.response.text)
            raise SekhaAPIError(
                "Export failed", e.response.status_code, e.response.text
            )
        except Exception as e:
            raise SekhaError(f"Export failed: {e}")

    async def _update_status(self, conversation_id: str, status: str) -> None:
        """Internal method to update conversation status"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.put(
                f"/api/v1/conversations/{conversation_id}/status",
                json={"status": status},
            )
            response.raise_for_status()

        except httpx.HTTPStatusError as e:
            if e.response.status_code == 404:
                raise SekhaNotFoundError(f"Conversation {conversation_id} not found")
            elif e.response.status_code == 400:
                raise SekhaValidationError("Invalid status", e.response.text)
            raise SekhaAPIError(
                "Status update failed", e.response.status_code, e.response.text
            )
        except Exception as e:
            raise SekhaError(f"Status update failed: {e}")

    # ============== MCP Integration (future) ==============

    async def get_mcp_tools(self) -> List[Dict[str, Any]]:
        """List available MCP tools"""
        await self.rate_limiter.acquire()

        try:
            response = await self.client.get("/mcp/tools")
            response.raise_for_status()
            return response.json()

        except Exception as e:
            raise SekhaError(f"Failed to get MCP tools: {e}")


# ============== Sync Wrapper ==============


class SyncSekhaClient:
    """
    Synchronous wrapper for SekhaClient

    All async methods are available as sync methods.
    Uses httpx.Client internally with same config.
    """

    def __init__(self, config: ClientConfig):
        self._async_client = SekhaClient(config)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self._async_client.sync_client.close()

    def __getattr__(self, name: str):
        """Delegate to async client"""
        async_method = getattr(self._async_client, name)

        if not asyncio.iscoroutinefunction(async_method):
            return async_method

        def sync_wrapper(*args, **kwargs):
            # Simpler than maintaining full sync/async duplication
            import asyncio

            return asyncio.run(async_method(*args, **kwargs))

        return sync_wrapper
