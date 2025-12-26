#!/bin/bash
set -e

echo "🧪 Running Sekha Controller Test Suite..."

# Parse arguments
TEST_TYPE=${1:-"all"}
COVERAGE=${2:-"false"}

case $TEST_TYPE in
  "unit")
    echo "Running unit tests..."
    cargo test --lib
    ;;
  "integration")
    echo "Running integration tests..."
    cargo test --test '*'
    ;;
  "bench")
    echo "Running benchmarks..."
    cargo test --bench '*'
    ;;
  "all"|*)
    echo "Running all tests..."
    cargo test --all-features
    ;;
esac

# Run clippy linting
echo "🔍 Running clippy..."
cargo clippy -- -D warnings

# Coverage if requested
if [ "$COVERAGE" = "true" ]; then
  echo "📊 Collecting coverage..."
  cargo tarpaulin --out Html --output-dir ./coverage
fi

echo "✅ Tests complete!"