#!/usr/bin/env bash
set -e

echo "🧪 Testing installation script..."

# Create temporary test environment
TEST_DIR=$(mktemp -d)
export SEKHA_INSTALL_DIR="$TEST_DIR/bin"
export HOME="$TEST_DIR"
export SEKHA_VERSION="v0.1.0"

# Mock the binary download (simulate GitHub release)
mkdir -p "$SEKHA_INSTALL_DIR"
cat > "$SEKHA_INSTALL_DIR/sekha-controller" << 'EOF'
#!/bin/bash
case "$1" in
    --version) echo "sekha-controller v0.1.0" ;;
    start) echo "Starting controller..." ;;
    health) echo '{"status":"healthy"}' ;;
    *) echo "Mock sekha-controller" ;;
esac
exit 0
EOF
chmod +x "$SEKHA_INSTALL_DIR/sekha-controller"

# Override curl to use local mock binary instead of downloading
curl() {
    if [[ "$*" == *"releases/download"* ]]; then
        # Simulate successful download
        local output_file=$(echo "$@" | grep -oP '(?<=-o )\S+')
        cp "$SEKHA_INSTALL_DIR/sekha-controller" "$output_file"
        return 0
    else
        # Call real curl for other uses
        command curl "$@"
    fi
}
export -f curl

# Run the actual install script
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bash install.sh

# Verify binary was "installed"
if [ ! -x "$SEKHA_INSTALL_DIR/sekha-controller" ]; then
    echo "❌ Binary not installed"
    rm -rf "$TEST_DIR"
    exit 1
fi

# Verify binary works
if ! "$SEKHA_INSTALL_DIR/sekha-controller" --version | grep -q "v0.1.0"; then
    echo "❌ Binary version check failed"
    rm -rf "$TEST_DIR"
    exit 1
fi

# Verify config directory structure
if [ ! -d "$HOME/.sekha" ]; then
    echo "❌ Config directory not created"
    rm -rf "$TEST_DIR"
    exit 1
fi

for dir in data logs import imported; do
    if [ ! -d "$HOME/.sekha/$dir" ]; then
        echo "❌ Missing directory: $dir"
        rm -rf "$TEST_DIR"
        exit 1
    fi
done

# Verify config file was created
if [ ! -f "$HOME/.sekha/config.toml" ]; then
    echo "❌ Config file not created"
    rm -rf "$TEST_DIR"
    exit 1
fi

# Verify config contains required sections
for section in "[server]" "[database]" "[vector_db]" "[bridge]" "[storage]"; do
    if ! grep -q "$section" "$HOME/.sekha/config.toml"; then
        echo "❌ Config missing section: $section"
        rm -rf "$TEST_DIR"
        exit 1
    fi
done

# Verify security warning is shown (check it exists in install.sh)
if ! grep -q "SECURITY WARNING" "$(dirname "${BASH_SOURCE[0]}")/../install.sh"; then
    echo "❌ Install script missing security warning"
    rm -rf "$TEST_DIR"
    exit 1
fi

# Cleanup
rm -rf "$TEST_DIR"

echo "✅ Installation script tests passed!"
