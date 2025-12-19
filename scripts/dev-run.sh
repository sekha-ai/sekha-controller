#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$HOME/.sekha/config.toml"
DATA_DIR="$HOME/.sekha/data"
TMUX_SESSION="sekha-dev"

echo -e "${GREEN}=== Sekha Development Runner ===${NC}"

# Check if tmux is installed
if ! command -v tmux &> /dev/null; then
    echo -e "${RED}✗ tmux is required but not installed.${NC}"
    echo -e "Install it with: sudo apt install tmux  # Ubuntu/Debian"
    echo -e "Or: brew install tmux  # macOS"
    exit 1
fi

# Read config values (naive but works for our TOML)
get_config() {
    grep -A10 "$1" "$CONFIG_PATH" | grep "$2" | cut -d'=' -f2 | tr -d ' "' | head -n1
}

API_HOST=$(get_config "\[api\]" "host")
API_PORT=$(get_config "\[api\]" "port")

# Kill existing session if running
if tmux has-session -t "$TMUX_SESSION" 2>/dev/null; then
    echo -e "${YELLOW}Existing tmux session found. Killing it...${NC}"
    tmux kill-session -t "$TMUX_SESSION"
fi

# Create new session (detached)
echo -e "${YELLOW}Creating tmux session: $TMUX_SESSION${NC}"
tmux new-session -d -s "$TMUX_SESSION" -n "api" -c "$PROJECT_ROOT"

# Window 1: Main API Server (with hot reload)
tmux send-keys -t "$TMUX_SESSION:api" "echo 'Starting API server with hot reload...'" C-m
tmux send-keys -t "$TMUX_SESSION:api" "cargo watch -x run --features dev" C-m

# Window 2: Database Monitor
tmux new-window -t "$TMUX_SESSION" -n "db" -c "$PROJECT_ROOT"
tmux send-keys -t "$TMUX_SESSION:db" "echo 'Database monitor (SQLite)'" C-m
tmux send-keys -t "$TMUX_SESSION:db" "watch -n 5 'echo \"Row count:\"; sqlite3 $DATA_DIR/sekha.db \"SELECT COUNT(*) FROM conversations;\" 2>/dev/null || echo \"No conversations table yet\"'" C-m

# Window 3: Ollama Service Monitor
tmux new-window -t "$TMUX_SESSION" -n "ollama" -c "$PROJECT_ROOT"
tmux send-keys -t "$TMUX_SESSION:ollama" "echo 'Ollama service logs...'" C-m
tmux send-keys -t "$TMUX_SESSION:ollama" "journalctl -u ollama -f 2>/dev/null || tail -f ~/.ollama/logs/server.log 2>/dev/null || echo \"Watching Ollama (if available)\"" C-m

# Window 4: Python Bridge
tmux new-window -t "$TMUX_SESSION" -n "python" -c "$PROJECT_ROOT/python-bridge"
tmux send-keys -t "$TMUX_SESSION:python" "echo 'Python bridge service'" C-m
tmux send-keys -t "$TMUX_SESSION:python" "poetry run python -m uvicorn main:app --reload --port 8001" C-m

# Window 5: Build & Test Watcher
tmux new-window -t "$TMUX_SESSION" -n "tests" -c "$PROJECT_ROOT"
tmux send-keys -t "$TMUX_SESSION:tests" "echo 'Running tests on file changes...'" C-m
tmux send-keys -t "$TMUX_SESSION:tests" "cargo watch -x test" C-m

# Window 6: Logs & Debugging
tmux new-window -t "$TMUX_SESSION" -n "logs" -c "$PROJECT_ROOT"
tmux send-keys -t "$TMUX_SESSION:logs" "echo 'Application logs will appear here...'" C-m
tmux send-keys -t "$TMUX_SESSION:logs" "tail -f $DATA_DIR/*.log 2>/dev/null || echo \"Waiting for logs...\"" C-m

# Select the API window as default
tmux select-window -t "$TMUX_SESSION:api"

echo -e "${GREEN}✓ Tmux session created successfully!${NC}"
echo -e "\n${YELLOW}To attach to the session:${NC}"
echo -e "  tmux attach-session -t $TMUX_SESSION"
echo -e "\n${YELLOW}Or use these shortcuts inside tmux:${NC}"
echo -e "  Ctrl+b, then n  → Next window"
echo -e "  Ctrl+b, then p  → Previous window"
echo -e "  Ctrl+b, then 0-5 → Jump to window number"
echo -e "  Ctrl+b, then d  → Detach session"
echo -e "\n${YELLOW}To kill the session:${NC}"
echo -e "  tmux kill-session -t $TMUX_SESSION"

# Optional: auto-attach if requested
if [ "$1" == "--attach" ]; then
    tmux attach-session -t "$TMUX_SESSION"
else
    echo -e "\n${BLUE}Session running in background. Use --attach flag to connect immediately.${NC}"
fi

# Create a stop script
cat > "$PROJECT_ROOT/scripts/dev-stop.sh" << EOF
#!/bin/bash
tmux kill-session -t $TMUX_SESSION 2>/dev/null && echo "Development session stopped" || echo "No session running"
EOF
chmod +x "$PROJECT_ROOT/scripts/dev-stop.sh"

echo -e "\n${GREEN}=== Development environment is running ===${NC}"
echo -e "API will be available at http://${API_HOST}:${API_PORT}"
