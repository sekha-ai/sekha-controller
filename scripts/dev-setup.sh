#!/bin/bash
set -e  # Exit on any error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Sekha Development Environment Setup ===${NC}"

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$HOME/.sekha/config.toml"
DATA_DIR="$HOME/.sekha/data"

# 1. Check required tools
echo -e "\n${YELLOW}Checking toolchain...${NC}"
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}✗ $1 not found. Please install it first.${NC}"
        exit 1
    else
        echo -e "${GREEN}✓ $1 found: $($1 --version 2>&1 | head -n1)${NC}"
    fi
}

check_tool cargo
check_tool ollama
check_tool python3
check_tool node
check_tool docker
check_tool gh

# Verify Ollama models
echo -e "\n${YELLOW}Verifying Ollama models...${NC}"
for model in nomic-embed-text llama3.1:70b llama3.1:8b; do
    if ollama list | grep -q "$model"; then
        echo -e "${GREEN}✓ Model $model is available${NC}"
    else
        echo -e "${RED}✗ Model $model not found. Run: ollama pull $model${NC}"
        exit 1
    fi
done

# 2. Create project structure if needed
echo -e "\n${YELLOW}Setting up project structure...${NC}"
mkdir -p "$DATA_DIR/chroma"
mkdir -p "$PROJECT_ROOT/target"

# 3. Initialize Rust environment
echo -e "\n${YELLOW}Setting up Rust environment...${NC}"
cd "$PROJECT_ROOT"
if [ ! -f "Cargo.toml" ]; then
    echo -e "${YELLOW}Initializing new Rust project...${NC}"
    cargo init --name sekha-controller .
fi

# Install Rust dependencies if needed
cargo fetch

# 4. Python bridge setup
echo -e "\n${YELLOW}Setting up Python environment...${NC}"
cd "$PROJECT_ROOT"
if [ ! -d "python-bridge" ]; then
    mkdir -p python-bridge
    cd python-bridge
    cat > pyproject.toml << EOF
[tool.poetry]
name = "sekha-python-bridge"
version = "0.1.0"
description = "Python bridge for Sekha ML integrations"
authors = ["Dev <dev@sekha.ai>"]

[tool.poetry.dependencies]
python = "^3.11"
requests = "^2.31"
pydantic = "^2.5"
pytest = "^7.4"

[tool.poetry.group.dev.dependencies]
black = "^23.0"
flake8 = "^6.0"

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"
EOF
else
    cd python-bridge
fi

# Setup Poetry virtualenv
poetry install --no-root
echo -e "${GREEN}✓ Python environment ready at python-bridge/.venv${NC}"

# 5. Node.js SDK setup
echo -e "\n${YELLOW}Setting up Node.js environment...${NC}"
cd "$PROJECT_ROOT"
if [ ! -f "package.json" ]; then
    cat > package.json << EOF
{
  "name": "sekha-sdks",
  "version": "0.1.0",
  "private": true,
  "workspaces": [
    "sdks/*"
  ],
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.3"
  }
}
EOF
    mkdir -p sdks/typescript
    pnpm install
fi

# 6. Database initialization
echo -e "\n${YELLOW}Initializing database...${NC}"
cd "$PROJECT_ROOT"

# Check if sea-orm-cli is installed
if ! command -v sea-orm-cli &> /dev/null; then
    echo -e "${YELLOW}Installing sea-orm-cli...${NC}"
    cargo install sea-orm-cli
fi

# Run migrations (if any exist)
if [ -d "migration" ]; then
    echo -e "${YELLOW}Running database migrations...${NC}"
    cd migration
    DATABASE_URL="sqlite://$DATA_DIR/sekha.db" sea-orm-cli migrate up
    cd ..
else
    echo -e "${YELLOW}No migrations found. Skipping...${NC}"
fi

# Create empty DB file if it doesn't exist
touch "$DATA_DIR/sekha.db"

# 7. Seed test data
echo -e "\n${YELLOW}Seeding test data...${NC}"
cat > "$DATA_DIR/seed.json" << EOF
{
  "test_conversations": [
    {"id": "test-001", "title": "First Test Conversation", "messages": 5},
    {"id": "test-002", "title": "Second Test Conversation", "messages": 3}
  ],
  "test_vectors": [
    {"id": "vec-001", "content": "Sample embedding for testing"}
  ]
}
EOF
echo -e "${GREEN}✓ Test data seeded to $DATA_DIR/seed.json${NC}"

# 8. Validate configuration
echo -e "\n${YELLOW}Validating configuration...${NC}"
if [ -f "$CONFIG_PATH" ]; then
    echo -e "${GREEN}✓ Config file exists at $CONFIG_PATH${NC}"
    # Basic TOML validation (try to parse with Python)
    if python3 -c "import tomllib; tomllib.load(open('$CONFIG_PATH', 'rb'))" 2>/dev/null; then
        echo -e "${GREEN}✓ Config file is valid TOML${NC}"
    else
        echo -e "${RED}✗ Config file has TOML syntax errors${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Config file not found at $CONFIG_PATH${NC}"
    exit 1
fi

# 9. Set up git hooks (optional)
echo -e "\n${YELLOW}Setting up git hooks...${NC}"
mkdir -p .git/hooks
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
echo "Running pre-commit checks..."
cargo fmt -- --check
cargo clippy -- -D warnings
EOF
chmod +x .git/hooks/pre-commit
echo -e "${GREEN}✓ Pre-commit hook installed${NC}"

echo -e "\n${GREEN}=== Setup Complete! ===${NC}"
echo -e "Run ${YELLOW}./scripts/dev-run.sh${NC} to start development environment"
echo -e "Data directory: $DATA_DIR"
echo -e "Config file: $CONFIG_PATH"
