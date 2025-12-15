# Sekha LLM Bridge

Python service for LLM operations (embeddings, summarization, importance scoring, entity extraction).

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Start Ollama (if not running)
ollama serve

# Pull required models
ollama pull nomic-embed-text:latest
ollama pull llama3.1:8b

# Start the bridge
python main.py

# Or with Docker
docker build -t sekha-llm-bridge .
docker run -p 5001:5001 --network host sekha-llm-bridge