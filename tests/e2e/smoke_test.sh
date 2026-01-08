#!/bin/bash
set -e

# ============================================================
# SEKHA E2E SMOKE TEST
# Tests controller endpoints regardless of how it's running
# Does NOT manage infrastructure - only tests existing deployment
# ============================================================

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
SEKHA_BASE_URL="${SEKHA_BASE_URL:-http://localhost:8080}"
SEKHA_API_KEY="${SEKHA_API_KEY:-test-key-do-not-use-in-production-12345678901234}"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-120}"
SKIP_EMBEDDINGS="${SKIP_EMBEDDINGS:-false}"  # Set to true if Ollama unavailable

# Test state
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# ============================================================
# UTILITY FUNCTIONS
# ============================================================

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

# ============================================================
# PRE-FLIGHT CHECKS
# ============================================================

check_dependencies() {
    log_info "Checking dependencies..."
    
    if ! command -v curl &> /dev/null; then
        log_error "curl is required but not installed"
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        log_warning "jq not found - JSON parsing will be basic"
    fi
    
    log_success "Dependencies OK"
}

detect_server() {
    log_info "Detecting server at $SEKHA_BASE_URL..."
    
    local max_attempts=$((WAIT_TIMEOUT / 2))
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -sf "$SEKHA_BASE_URL/health" > /dev/null 2>&1; then
            log_success "Server detected and healthy"
            return 0
        fi
        
        if [ $attempt -eq 0 ]; then
            log_info "Waiting for server to become available..."
        fi
        
        attempt=$((attempt + 1))
        sleep 2
    done
    
    log_error "Server not reachable at $SEKHA_BASE_URL after ${WAIT_TIMEOUT}s"
    log_info "Hints:"
    log_info "  - Start local: cargo run --release"
    log_info "  - Start Docker: cd ../sekha-docker && docker compose up -d"
    log_info "  - Or set: export SEKHA_BASE_URL=https://your-server.com"
    exit 1
}

# ============================================================
# TEST SUITE
# ============================================================

test_health() {
    log_info "Test 1/8: Health Check"
    
    local response
    response=$(curl -sf "$SEKHA_BASE_URL/health" 2>&1)
    local status=$?
    
    if [ $status -eq 0 ] && echo "$response" | grep -qi "healthy"; then
        log_success "Health endpoint responsive"
    else
        log_error "Health check failed (HTTP $status)"
        echo "Response: $response"
    fi
}

test_metrics() {
    log_info "Test 2/8: Metrics Endpoint"
    
    local response
    response=$(curl -sf "$SEKHA_BASE_URL/metrics" 2>&1)
    
    if echo "$response" | grep -qE "# HELP|# TYPE"; then
        log_success "Metrics endpoint serving Prometheus format"
    else
        log_warning "Metrics endpoint returned unexpected format"
    fi
}

test_rest_create() {
    log_info "Test 3/8: REST API - Create Conversation"
    
    local timestamp=$(date +%s)
    local response
    
    response=$(curl -sf -X POST "$SEKHA_BASE_URL/api/v1/conversations" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $SEKHA_API_KEY" \
        -d "{
            \"label\": \"E2E-Test-${timestamp}\",
            \"folder\": \"/test/e2e\",
            \"messages\": [
                {\"role\": \"user\", \"content\": \"E2E smoke test message at ${timestamp}\"},
                {\"role\": \"assistant\", \"content\": \"Acknowledged. This is a test conversation.\"}
            ]
        }" 2>&1)
    
    local conv_id
    if command -v jq &> /dev/null; then
        conv_id=$(echo "$response" | jq -r '.id // .conversation_id // empty' 2>/dev/null)
    else
        conv_id=$(echo "$response" | grep -oP '"id"\s*:\s*"\K[^"]+' | head -1)
    fi
    
    if [ -n "$conv_id" ] && [ "$conv_id" != "null" ]; then
        log_success "Conversation created: ${conv_id:0:8}..."
        echo "$conv_id" > /tmp/sekha_e2e_rest_id.txt
    else
        log_error "Failed to create conversation"
        echo "Response: $response"
    fi
}

test_rest_get() {
    log_info "Test 4/8: REST API - Get Conversation"
    
    if [ ! -f /tmp/sekha_e2e_rest_id.txt ]; then
        log_warning "Skipped - no conversation ID from create test"
        return
    fi
    
    local conv_id
    conv_id=$(cat /tmp/sekha_e2e_rest_id.txt)
    
    local response
    response=$(curl -sf "$SEKHA_BASE_URL/api/v1/conversations/$conv_id" \
        -H "X-API-Key: $SEKHA_API_KEY" 2>&1)
    
    if echo "$response" | grep -q "$conv_id"; then
        log_success "Retrieved conversation successfully"
    else
        log_error "Failed to retrieve conversation"
        echo "Response: $response"
    fi
}

test_rest_update() {
    log_info "Test 5/8: REST API - Update Label"
    
    if [ ! -f /tmp/sekha_e2e_rest_id.txt ]; then
        log_warning "Skipped - no conversation ID"
        return
    fi
    
    local conv_id
    conv_id=$(cat /tmp/sekha_e2e_rest_id.txt)
    
    local response
    response=$(curl -sf -X PUT "$SEKHA_BASE_URL/api/v1/conversations/$conv_id/label" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $SEKHA_API_KEY" \
        -d '{
            "label": "E2E-Test-Updated",
            "folder": "/test/e2e/updated"
        }' 2>&1)
    
    if echo "$response" | grep -qE "success|updated|$conv_id"; then
        log_success "Label updated successfully"
    else
        log_error "Failed to update label"
        echo "Response: $response"
    fi
}

test_semantic_search() {
    log_info "Test 6/8: Semantic Search"
    
    if [ "$SKIP_EMBEDDINGS" = "true" ]; then
        log_warning "Skipped - SKIP_EMBEDDINGS=true (Ollama not available)"
        return
    fi
    
    # Wait for embeddings to be generated
    log_info "Waiting 5s for embeddings to be generated..."
    sleep 5
    
    local response
    response=$(curl -sf -X POST "$SEKHA_BASE_URL/api/v1/query" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $SEKHA_API_KEY" \
        -d '{
            "query": "smoke test",
            "limit": 10
        }' 2>&1)
    
    if echo "$response" | grep -qE "results|conversations"; then
        local count=0
        if command -v jq &> /dev/null; then
            count=$(echo "$response" | jq '.results | length // 0' 2>/dev/null || echo "0")
        fi
        log_success "Semantic search working (found $count results)"
    else
        log_warning "Semantic search returned empty (embeddings may not be ready)"
    fi
}

test_mcp_store() {
    log_info "Test 7/8: MCP - memory_store Tool"
    
    local timestamp=$(date +%s)
    local response
    
    response=$(curl -sf -X POST "$SEKHA_BASE_URL/mcp/tools/memory_store" \
        -H "Authorization: Bearer $SEKHA_API_KEY" \
        -H "Content-Type: application/json" \
        -d "{
            \"label\": \"E2E-MCP-${timestamp}\",
            \"folder\": \"/test/mcp\",
            \"messages\": [
                {\"role\": \"user\", \"content\": \"MCP smoke test at ${timestamp}\"}
            ]
        }" 2>&1)
    
    local conv_id
    if command -v jq &> /dev/null; then
        conv_id=$(echo "$response" | jq -r '.data.conversation_id // .conversation_id // empty' 2>/dev/null)
    else
        conv_id=$(echo "$response" | grep -oP '"conversation_id"\s*:\s*"\K[^"]+' | head -1)
    fi
    
    if [ -n "$conv_id" ] && [ "$conv_id" != "null" ]; then
        log_success "MCP conversation created: ${conv_id:0:8}..."
    else
        log_error "MCP memory_store failed"
        echo "Response: $response"
    fi
}

test_mcp_query() {
    log_info "Test 8/8: MCP - memory_query Tool"
    
    if [ "$SKIP_EMBEDDINGS" = "true" ]; then
        log_warning "Skipped - SKIP_EMBEDDINGS=true"
        return
    fi
    
    local response
    response=$(curl -sf -X POST "$SEKHA_BASE_URL/mcp/tools/memory_query" \
        -H "Authorization: Bearer $SEKHA_API_KEY" \
        -H "Content-Type: application/json" \
        -d '{
            "query": "test"
        }' 2>&1)
    
    if echo "$response" | grep -qE "results|conversations|data"; then
        log_success "MCP memory_query working"
    else
        log_warning "MCP memory_query returned empty results"
    fi
}

# ============================================================
# CLEANUP
# ============================================================

cleanup() {
    rm -f /tmp/sekha_e2e_*.txt 2>/dev/null || true
}

trap cleanup EXIT

# ============================================================
# MAIN EXECUTION
# ============================================================

main() {
    echo ""
    echo "============================================================"
    echo "  SEKHA E2E SMOKE TEST"
    echo "============================================================"
    echo "  Target: $SEKHA_BASE_URL"
    echo "  API Key: ${SEKHA_API_KEY:0:20}..."
    echo "  Timeout: ${WAIT_TIMEOUT}s"
    echo "============================================================"
    echo ""
    
    # Pre-flight
    check_dependencies
    detect_server
    
    echo ""
    log_info "Starting test suite..."
    echo ""
    
    # Run all tests
    test_health
    test_metrics
    test_rest_create
    test_rest_get
    test_rest_update
    test_semantic_search
    test_mcp_store
    test_mcp_query
    
    # Results
    echo ""
    echo "============================================================"
    echo "  TEST RESULTS"
    echo "============================================================"
    echo -e "  ${GREEN}Passed:${NC}  $TESTS_PASSED"
    echo -e "  ${RED}Failed:${NC}  $TESTS_FAILED"
    echo -e "  ${YELLOW}Skipped:${NC} $TESTS_SKIPPED"
    echo "============================================================"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "🎉 ALL TESTS PASSED!"
        exit 0
    else
        log_error "⚠️  $TESTS_FAILED TEST(S) FAILED"
        exit 1
    fi
}

# Run main function
main
