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
cargo run --release &
SERVER_PID=$!

# Wait for server to start
sleep 3

# 2. Test create conversation
echo "Testing POST /api/v1/conversations..."
RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/conversations \
  -H "Content-Type: application/json" \
  -d '{"label": "E2E-Test", "folder": "/test", "messages": [{"role": "user", "content": "End-to-end test"}]}')

echo "Response: $RESPONSE"
CONV_ID=$(echo $RESPONSE | jq -r '.id')

if [ -z "$CONV_ID" ] || [ "$CONV_ID" == "null" ]; then
    echo -e "${RED}❌ Failed to create conversation${NC}"
    kill $SERVER_PID
    exit 1
fi

echo -e "${GREEN}✅ Conversation created: $CONV_ID${NC}"

# 3. Test retrieve conversation
echo "Testing GET /api/v1/conversations/$CONV_ID..."
GET_RESPONSE=$(curl -s http://localhost:8080/api/v1/conversations/$CONV_ID)

if echo "$GET_RESPONSE" | grep -q "$CONV_ID"; then
    echo -e "${GREEN}✅ Retrieved conversation successfully${NC}"
else
    echo -e "${RED}❌ Failed to retrieve conversation${NC}"
    kill $SERVER_PID
    exit 1
fi

# 4. Test semantic search (mock)
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

# 5. Cleanup
echo "Stopping server..."
kill $SERVER_PID

echo -e "${GREEN}🎉 All tests passed!${NC}"
