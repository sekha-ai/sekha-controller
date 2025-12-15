import httpx
import json
from typing import List, Optional, Dict, Any
from config import settings
import logging
from models import (
    EmbedResponse, SummarizeResponse, ScoreImportanceResponse,
    ExtractEntitiesResponse, TaskLevel
)

logger = logging.getLogger(__name__)

class OllamaClient:
    """Simple Ollama client for direct API access"""
    
    def __init__(self, base_url: str = settings.ollama_host):
        self.base_url = base_url
        self.client = httpx.AsyncClient(timeout=30.0)
    
    async def generate_embedding(self, text: str, model: str) -> List[float]:
        """Generate embedding using Ollama's embed endpoint"""
        try:
            response = await self.client.post(
                f"{self.base_url}/api/embed",
                json={"model": model, "input": text}
            )
            response.raise_for_status()
            data = response.json()
            return data["embeddings"][0]
        except Exception as e:
            logger.error(f"Embedding generation failed: {e}")
            raise
    
    async def generate_completion(self, prompt: str, model: str, **kwargs) -> str:
        """Generate text completion"""
        try:
            payload = {
                "model": model,
                "prompt": prompt,
                "stream": False,
                **kwargs
            }
            response = await self.client.post(
                f"{self.base_url}/api/generate",
                json=payload
            )
            response.raise_for_status()
            return response.json()["response"]
        except Exception as e:
            logger.error(f"Completion generation failed: {e}")
            raise
    
    async def list_models(self) -> List[str]:
        """Get list of available models"""
        try:
            response = await self.client.get(f"{self.base_url}/api/tags")
            response.raise_for_status()
            return [m["name"] for m in response.json()["models"]]
        except Exception as e:
            logger.error(f"Failed to list models: {e}")
            return []

class LLMBridge:
    """Main bridge that orchestrates LLM calls"""
    
    def __init__(self):
        self.ollama = OllamaClient()
    
    async def embed(self, text: str, model: Optional[str] = None) -> EmbedResponse:
        """Generate embeddings for text"""
        model = model or settings.default_embed_model
        
        embedding = await self.ollama.generate_embedding(text, model)
        
        # Estimate tokens (rough: 1 token ≈ 4 chars)
        tokens_used = len(text) // 4
        
        return EmbedResponse(
            embedding=embedding,
            model=model,
            tokens_used=tokens_used
        )
    
    async def summarize(
        self, 
        messages: List[str], 
        level: TaskLevel,
        model: Optional[str] = None,
        max_words: int = 200
    ) -> SummarizeResponse:
        """Generate hierarchical summary"""
        model = model or settings.default_summarize_model
        
        # Build prompt based on level (FIXED: No backslashes in f-string expressions)
        messages_str = "\n".join(f"- {msg}" for msg in messages)
        
        if level == TaskLevel.DAILY:
            prompt = f"""Summarize these messages from the past day. 
Focus on key decisions, TODOs, and insights. 
Maximum {max_words} words.

Messages:
{messages_str}"""
        elif level == TaskLevel.WEEKLY:
            prompt = f"""Synthesize these daily summaries from the past week.
Identify themes, progress, and blockers.
Maximum {max_words} words.

Summaries:
{messages_str}"""
        else:  # MONTHLY
            prompt = f"""Create an executive summary for the month.
Highlight achievements, learnings, and strategic direction.
Maximum {max_words} words.

Weekly summaries:
{messages_str}"""
        
        summary = await self.ollama.generate_completion(prompt, model)
        
        # Estimate tokens
        tokens_used = len(summary) // 4
        
        return SummarizeResponse(
            summary=summary.strip(),
            level=level,
            model=model,
            tokens_used=tokens_used
        )
    
    async def score_importance(
        self, 
        message: str, 
        context: Optional[str] = None,
        model: Optional[str] = None
    ) -> ScoreImportanceResponse:
        """Score message importance 1-10 with reasoning"""
        model = model or settings.default_importance_model
        
        context_str = f"\nContext: {context}" if context else ""
        
        prompt = f"""Rate the importance of this message on a scale of 1-10.
Consider: Does it contain decisions, action items, key insights, or 
critical information? Provide a brief reasoning.

Message: {message}{context_str}

Respond in JSON format:
{{"score": number, "reasoning": "explanation"}}"""
        
        result = await self.ollama.generate_completion(prompt, model)
        
        try:
            parsed = json.loads(result)
            return ScoreImportanceResponse(
                score=float(parsed["score"]),
                reasoning=parsed.get("reasoning"),
                model=model
            )
        except (json.JSONDecodeError, KeyError):
            # Fallback: return default if LLM doesn't give valid JSON
            logger.warning(f"LLM returned invalid JSON for importance scoring: {result}")
            return ScoreImportanceResponse(
                score=5.0,
                reasoning="LLM parsing failed, using default",
                model=model
            )
    
    async def extract_entities(
        self,
        text: str,
        entity_types: List[str],
        model: Optional[str] = None
    ) -> ExtractEntitiesResponse:
        """Extract entities from text"""
        model = model or settings.default_summarize_model
        
        # For now, return empty until we implement proper NER
        # This is a placeholder for Module 5 context assembly
        
        return ExtractEntitiesResponse(
            entities={etype: [] for etype in entity_types},
            model=model
        )