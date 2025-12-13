#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "🚀 Starting E2E Smoke Test..."

# 1. Build and start server
echo "Building project..."
cargo build --release

echo "Starting server in background..."
cargo run --release > server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 5

# 2. Test MCP endpoint
echo "Testing MCP memory_store tool..."
MCP_RESPONSE=$(curl -s -X POST http://localhost:8080/mcp/tools/memory_store \
  -H "Authorization: Bearer test_key_12345678901234567890123456789012" \
  -H "Content-Type: application/json" \
  -d '{"label": "E2E-MCP-Test", "folder": "/test", "messages": [{"role": "user", "content": "MCP end-to-end test"}]}')

echo "MCP Response: $MCP_RESPONSE"
MCP_CONV_ID=$(echo $MCP_RESPONSE | jq -r '.data.conversation_id')

if [ "$MCP_CONV_ID" == "null" ] || [ -z "$MCP_CONV_ID" ]; then
    echo -e "${RED}❌ MCP memory_store failed${NC}"
    kill $SERVER_PID
    exit 1
fi
echo -e "${GREEN}✅ MCP conversation created: $MCP_CONV_ID${NC}"

# 3. Test REST API create
echo "Testing POST /api/v1/conversations..."
REST_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -d '{"label": "E2E-REST-Test", "folder": "/test", "messages": [{"role": "user", "content": "REST end-to-end test"}]}')

echo "REST Response: $REST_RESPONSE"
REST_CONV_ID=$(echo $REST_RESPONSE | jq -r '.id')

if [ "$REST_CONV_ID" == "null" ] || [ -z "$REST_CONV_ID" ]; then
    echo -e "${RED}❌ REST create conversation failed${NC}"
    kill $SERVER_PID
    exit 1
fi
echo -e "${GREEN}✅ REST conversation created: $REST_CONV_ID${NC}"

# 4. Test retrieve via REST
echo "Testing GET /api/v1/conversations/$REST_CONV_ID..."
GET_RESPONSE=$(curl -s http://localhost:8080/api/v1/conversations/$REST_CONV_ID)

if echo "$GET_RESPONSE" | grep -q "$REST_CONV_ID"; then
    echo -e "${GREEN}✅ Retrieved conversation successfully${NC}"
else
    echo -e "${RED}❌ Failed to retrieve conversation${NC}"
    kill $SERVER_PID
    exit 1
fi

# 5. Test semantic search
echo "Testing POST /api/v1/query..."
SEARCH_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "test", "limit": 10}')

if echo "$SEARCH_RESPONSE" | grep -q "results"; then
    echo -e "${GREEN}✅ Semantic search working${NC}"
else
    echo -e "${RED}❌ Semantic search failed${NC}"
    kill $SERVER_PID
    exit 1
fi

# 6. Test health endpoint
echo "Testing GET /health..."
HEALTH_RESPONSE=$(curl -s http://localhost:8080/health)

if echo "$HEALTH_RESPONSE" | grep -q "healthy"; then
    echo -e "${GREEN}✅ Health check passed${NC}"
else
    echo -e "${RED}❌ Health check failed${NC}"
    kill $SERVER_PID
    exit 1
fi

# 7. Cleanup
echo "Stopping server..."
kill $SERVER_PID
sleep 2

echo -e "${GREEN}🎉 All E2E tests passed!${NC}"

# Show server logs if needed
# echo "Server logs:"
# cat server.log
