#!/bin/bash
set -e

# Sekha Local Installer (Tier 1)

INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.sekha"
DATA_DIR="$HOME/.sekha/data"

echo "🚀 Installing Sekha Controller (Local Binary)..."

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
    x86_64) ARCH="amd64" ;;
    aarch64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Download latest release
LATEST_VERSION=$(curl -s https://api.github.com/repos/sekha-ai/sekha-controller/releases/latest | grep '"tag_name":' | cut -d'"' -f4)
DOWNLOAD_URL="https://github.com/sekha-ai/sekha-controller/releases/download/${LATEST_VERSION}/sekha-controller-${OS}-${ARCH}"

echo "📥 Downloading ${LATEST_VERSION}..."
curl -L -o /tmp/sekha-controller "$DOWNLOAD_URL"
chmod +x /tmp/sekha-controller

# Install binary
mkdir -p "$INSTALL_DIR"
mv /tmp/sekha-controller "$INSTALL_DIR/"

# Create directories
mkdir -p "$CONFIG_DIR" "$DATA_DIR"

# Create default config if it doesn't exist
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cat > "$CONFIG_DIR/config.toml" <<EOF
[server]
port = 8080

[database]
url = "sqlite://${DATA_DIR}/sekha.db"

[storage]
chroma_url = "http://localhost:8000"
ollama_url = "http://localhost:11434"

[embedding]
model = "nomic-embed-text:latest"
EOF
fi

echo "✅ Installed to $INSTALL_DIR/sekha-controller"
echo "📁 Config: $CONFIG_DIR/config.toml"
echo "📁 Data: $DATA_DIR"
echo ""
echo "Next steps:"
echo "1. Ensure Ollama is running: ollama serve"
echo "2. Pull embedding model: ollama pull nomic-embed-text:latest"
echo "3. Start Sekha: $INSTALL_DIR/sekha-controller"