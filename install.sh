#!/usr/bin/env bash
set -e

# Sekha Controller - Quick Install Script
# Usage: curl -sSL https://install.sekha.ai | bash

VERSION="${SEKHA_VERSION:-latest}"
INSTALL_DIR="${SEKHA_INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="$HOME/.sekha"

echo "🚀 Installing Sekha Controller..."

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)
        echo "❌ Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Download binary
BINARY_URL="https://github.com/sekha-ai/sekha-controller/releases/download/$VERSION/sekha-controller-$OS-$ARCH"

echo "📥 Downloading from $BINARY_URL..."
mkdir -p "$INSTALL_DIR"
curl -sSL "$BINARY_URL" -o "$INSTALL_DIR/sekha-controller"
chmod +x "$INSTALL_DIR/sekha-controller"

# Create config directory
mkdir -p "$CONFIG_DIR/data" "$CONFIG_DIR/logs" "$CONFIG_DIR/import" "$CONFIG_DIR/imported"

# Create default config if not exists
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "📝 Creating default configuration..."
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
[server]
host = "127.0.0.1"
port = 8080
api_key = "sk-dev-12345678901234567890123456789012"

[database]
url = "sqlite://$HOME/.sekha/data/sekha.db"

[vector_db]
url = "http://localhost:8000"
collection_name = "sekha_conversations"

[bridge]
url = "http://localhost:5001"
provider = "ollama"
model = "nomic-embed-text"

[storage]
data_dir = "$HOME/.sekha/data"
log_dir = "$HOME/.sekha/logs"
EOF
fi

# Add to PATH if not already there
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "⚠️  Add to your shell profile:"
    echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo ""
echo "✅ Sekha Controller installed successfully!"
echo ""
echo "Next steps:"
echo "  1. Review config: $CONFIG_DIR/config.toml"
echo "  2. Start server:  sekha-controller start"
echo "  3. Check status:  sekha-controller health"
echo ""
echo "📖 Documentation: https://docs.sekha.ai"
