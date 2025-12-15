from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from contextlib import asynccontextmanager
import logging
from typing import Optional

from config import settings
from llm_client import LLMBridge
from models import *
from health import router as health_router

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Global LLM Bridge instance
llm_bridge: Optional[LLMBridge] = None

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Manage LLM Bridge lifecycle"""
    global llm_bridge
    logger.info("Starting LLM Bridge...")
    llm_bridge = LLMBridge()
    
    # Test Ollama connection
    try:
        models = await llm_bridge.ollama.list_models()
        logger.info(f"Connected to Ollama. Available models: {len(models)}")
    except Exception as e:
        logger.error(f"Failed to connect to Ollama: {e}")
        logger.warning("LLM Bridge will start in degraded mode")
    
    yield
    
    logger.info("Shutting down LLM Bridge...")

app = FastAPI(
    title="Sekha LLM Bridge",
    description="Python service for LLM operations (embeddings, summarization, scoring)",
    version="0.1.0",
    lifespan=lifespan
)

# CORS for SDK clients
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # Configure for production
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include health router
app.include_router(health_router)

@app.post("/embed", response_model=EmbedResponse)
async def embed_text(request: EmbedRequest):
    """Generate embeddings for text"""
    if llm_bridge is None:
        raise HTTPException(status_code=503, detail="LLM Bridge not initialized")
    
    try:
        return await llm_bridge.embed(request.text, request.model)
    except Exception as e:
        logger.error(f"Embedding failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/summarize", response_model=SummarizeResponse)
async def summarize_text(request: SummarizeRequest):
    """Generate hierarchical summary"""
    if llm_bridge is None:
        raise HTTPException(status_code=503, detail="LLM Bridge not initialized")
    
    try:
        return await llm_bridge.summarize(
            request.messages,
            request.level,
            request.model,
            request.max_words
        )
    except Exception as e:
        logger.error(f"Summarization failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/score_importance", response_model=ScoreImportanceResponse)
async def score_importance(request: ScoreImportanceRequest):
    """Score message importance 1-10"""
    if llm_bridge is None:
        raise HTTPException(status_code=503, detail="LLM Bridge not initialized")
    
    try:
        return await llm_bridge.score_importance(
            request.message,
            request.context,
            request.model
        )
    except Exception as e:
        logger.error(f"Importance scoring failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/extract_entities", response_model=ExtractEntitiesResponse)
async def extract_entities(request: ExtractEntitiesRequest):
    """Extract entities from text"""
    if llm_bridge is None:
        raise HTTPException(status_code=503, detail="LLM Bridge not initialized")
    
    try:
        return await llm_bridge.extract_entities(
            request.text,
            request.entity_types,
            request.model
        )
    except Exception as e:
        logger.error(f"Entity extraction failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(
        "main:app",
        host=settings.host,
        port=settings.port,
        reload=True,
        log_level="info"
    )