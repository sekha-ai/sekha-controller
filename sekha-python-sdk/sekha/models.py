"""
Pydantic models for type-safe API interaction
"""

from typing import List, Optional, Dict, Any, Union
from datetime import datetime
from pydantic import BaseModel, Field, ConfigDict
from enum import Enum


class MessageRole(str, Enum):
    USER = "user"
    ASSISTANT = "assistant"
    SYSTEM = "system"
    TOOL = "tool"


class ConversationStatus(str, Enum):
    ACTIVE = "active"
    ARCHIVED = "archived"
    PINNED = "pinned"


class SummaryLevel(str, Enum):
    DAILY = "daily"
    WEEKLY = "weekly"
    MONTHLY = "monthly"


class MessageDto(BaseModel):
    """Message data transfer object"""

    role: MessageRole
    content: str
    metadata: Optional[Dict[str, Any]] = None

    model_config = ConfigDict(use_enum_values=True)


class NewConversation(BaseModel):
    """Create a new conversation"""

    label: str = Field(..., min_length=1, max_length=200)
    folder: Optional[str] = Field(default="/", max_length=200)
    messages: List[MessageDto] = Field(..., min_length=1)
    metadata: Optional[Dict[str, Any]] = None

    model_config = ConfigDict(use_enum_values=True)


class ConversationResponse(BaseModel):
    """Conversation response from API"""

    id: str
    label: str
    folder: str
    status: ConversationStatus
    message_count: int
    created_at: datetime

    model_config = ConfigDict(use_enum_values=True)


class QueryRequest(BaseModel):
    """Smart query request"""

    query: str = Field(..., min_length=1)
    limit: Optional[int] = Field(default=10, ge=1, le=1000)
    filters: Optional[Dict[str, Any]] = None


class QueryResult(BaseModel):
    """Single search result"""

    conversation_id: str
    message_id: str
    score: float
    content: str
    metadata: Optional[Dict[str, Any]] = None
    label: str
    folder: str
    timestamp: datetime


class QueryResponse(BaseModel):
    """Smart query response"""

    results: List[QueryResult]
    total: int
    page: int
    page_size: int


class LabelSuggestion(BaseModel):
    """Auto-label suggestion"""

    label: str
    confidence: float
    is_existing: bool
    reason: Optional[str] = None


class PruningSuggestion(BaseModel):
    """Intelligent pruning suggestion"""

    conversation_id: str
    conversation_label: str
    last_accessed: datetime
    message_count: int
    token_estimate: int
    importance_score: float
    preview: str
    recommendation: str  # "keep" or "archive"


class HealthResponse(BaseModel):
    """Health check response"""

    status: str
    timestamp: str
    checks: Dict[str, Any]


class ImportanceScore(BaseModel):
    """Message importance score"""

    score: float = Field(..., ge=1.0, le=10.0)
    reasoning: Optional[str] = None
    model: str


class SummaryResponse(BaseModel):
    """Generated summary"""

    summary: str
    level: SummaryLevel
    model: str
    tokens_used: int
