#!/usr/bin/env bash
set -e

echo "🧪 Testing installation script..."

# Test in temporary directory
TEST_DIR=$(mktemp -d)
export SEKHA_INSTALL_DIR="$TEST_DIR/bin"
export HOME="$TEST_DIR"

# Mock cargo install
mkdir -p "$SEKHA_INSTALL_DIR"
cat > "$SEKHA_INSTALL_DIR/sekha-controller" << 'EOF'
#!/bin/bash
echo "Mock sekha-controller"
exit 0
EOF
chmod +x "$SEKHA_INSTALL_DIR/sekha-controller"

# Test CLI commands
"$SEKHA_INSTALL_DIR/sekha-controller"

# Verify config directory created
if [ ! -d "$HOME/.sekha" ]; then
    echo "❌ Config directory not created"
    exit 1
fi

# Verify directory structure
for dir in data logs import imported; do
    if [ ! -d "$HOME/.sekha/$dir" ]; then
        echo "❌ Missing directory: $dir"
        exit 1
    fi
done

# Cleanup
rm -rf "$TEST_DIR"

echo "✅ Installation script tests passed!"
