from pydantic_settings import BaseSettings
from typing import Optional

class Settings(BaseSettings):
    # Server
    host: str = "0.0.0.0"
    port: int = 5001
    
    # LLM Providers
    ollama_host: str = "http://localhost:11434"
    default_embed_model: str = "nomic-embed-text:latest"
    default_summarize_model: str = "llama3.1:8b"
    default_importance_model: str = "llama3.1:8b"
    
    # Redis for Celery
    redis_url: str = "redis://localhost:6379/0"
    
    # Background processing
    enable_celery: bool = False  # Set to True when ready for async jobs
    
    class Config:
        env_file = ".env"
        env_prefix = "SEKHA_"

settings = Settings()