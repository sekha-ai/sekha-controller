#!/bin/bash
set -e

echo "🧪 Running Integration Tests..."

# Optional: Check if services are running
if curl -s http://localhost:8000/api/v1/heartbeat > /dev/null 2>&1; then
    echo "✅ Chroma is running"
else
    echo "⚠️  Chroma not detected - some tests may be skipped"
fi

if curl -s http://localhost:11434 > /dev/null 2>&1; then
    echo "✅ Ollama is running"
else
    echo "⚠️  Ollama not detected - some tests may be skipped"
fi

# Run integration tests
cargo test --test integration --all-features -- --nocapture

echo "✅ Integration tests complete"
