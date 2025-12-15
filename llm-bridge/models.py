from pydantic import BaseModel, Field
from typing import List, Optional, Dict
from enum import Enum

class TaskLevel(str, Enum):
    DAILY = "daily"
    WEEKLY = "weekly"
    MONTHLY = "monthly"

class EmbedRequest(BaseModel):
    text: str = Field(..., min_length=1, max_length=100_000)
    model: Optional[str] = None

class EmbedResponse(BaseModel):
    embedding: List[float]
    model: str
    tokens_used: int

class SummarizeRequest(BaseModel):
    messages: List[str] = Field(..., min_items=1, max_items=1000)
    level: TaskLevel
    model: Optional[str] = None
    max_words: int = Field(default=200, ge=50, le=2000)

class SummarizeResponse(BaseModel):
    summary: str
    level: TaskLevel
    model: str
    tokens_used: int

class ScoreImportanceRequest(BaseModel):
    message: str = Field(..., min_length=1, max_length=10_000)
    context: Optional[str] = None
    model: Optional[str] = None

class ScoreImportanceResponse(BaseModel):
    score: float = Field(..., ge=1.0, le=10.0)
    reasoning: Optional[str] = None
    model: str

class ExtractEntitiesRequest(BaseModel):
    text: str = Field(..., min_length=1, max_length=50_000)
    entity_types: List[str] = Field(default=["person", "org", "project", "deadline"])
    model: Optional[str] = None

class ExtractEntitiesResponse(BaseModel):
    entities: Dict[str, List[str]]
    model: str

class HealthResponse(BaseModel):
    status: str
    ollama_healthy: bool
    model_count: int
    version: str = "0.1.0"