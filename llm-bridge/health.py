from fastapi import APIRouter
from models import HealthResponse
from config import settings
import httpx

router = APIRouter()

@router.get("/health", response_model=HealthResponse)
async def health_check():
    """Check health of LLM Bridge and Ollama"""
    ollama_healthy = False
    model_count = 0
    
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            response = await client.get(f"{settings.ollama_host}/api/tags")
            if response.status_code == 200:
                ollama_healthy = True
                model_count = len(response.json().get("models", []))
    except Exception:
        pass
    
    return HealthResponse(
        status="healthy" if ollama_healthy else "degraded",
        ollama_healthy=ollama_healthy,
        model_count=model_count
    )