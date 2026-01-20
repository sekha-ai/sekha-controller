#!/bin/bash
set -e

echo "🔍 Pre-publish checklist for crates.io..."

# Check for uncommitted changes
if [[ -n $(git status -s) ]]; then
    echo "❌ Uncommitted changes detected. Commit all changes first."
    exit 1
fi

# Check Cargo.toml
echo "✅ Checking Cargo.toml..."
cargo verify-project || exit 1

# Run tests
echo "✅ Running tests..."
cargo test --all-features

# Check formatting
echo "✅ Checking formatting..."
cargo fmt -- --check

# Run clippy
echo "✅ Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Build release
echo "✅ Building release..."
cargo build --release

# Check documentation
echo "✅ Generating docs..."
cargo doc --no-deps

# Dry-run publish
echo "✅ Dry-run publish to crates.io..."
cargo publish --dry-run

# Check package contents
echo "✅ Checking package contents..."
cargo package --list

echo ""
echo "🎉 All checks passed! Ready to publish with:"
echo "   cargo publish"
