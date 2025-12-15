#!/bin/bash
set -e

# Sekha Docker Deployer (Tier 2)

echo "🚀 Deploying Sekha with Docker Compose..."

# Create project directory
mkdir -p sekha-deployment && cd sekha-deployment

# Download docker-compose.yml
curl -sSL https://raw.githubusercontent.com/sekha-ai/sekha-controller/main/docker-compose.yml -o docker-compose.yml

# Create .env file
if [ ! -f .env ]; then
    cat > .env <<EOF
SEKHA_PORT=8080
OLLAMA_HOST=http://host.docker.internal:11434
CHROMA_HOST=http://chroma:8000
REDIS_HOST=redis://redis:6379/0
EOF
fi

# Create data directory
mkdir -p data

# Pull and start
docker-compose pull
docker-compose up -d

echo "✅ Sekha deployed!"
echo "📊 API: http://localhost:8080"
echo "📊 LLM Bridge: http://localhost:5001"
echo "📊 Chroma: http://localhost:8000"
echo "📊 Redis: localhost:6379"
echo ""
echo "To view logs: docker-compose logs -f"
echo "To stop: docker-compose down"