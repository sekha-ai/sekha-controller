# Testing Guide

## Test Structure
- **Unit tests**: Co-located in `src/` modules or dedicated `tests/` files
- **Integration tests**: `tests/api_test.rs`, `tests/integration/` (modular)
- **Benchmarks**: `tests/benchmarks/`

## Commands
```bash
# Run all tests
./scripts/test.sh

# Run specific test types
./scripts/test.sh unit
./scripts/test.sh integration
./scripts/test.sh bench

# With coverage
./scripts/test.sh all true

Requirements
Install tarpaulin: cargo install cargo-tarpaulin
Coverage target: >80%


---

#### **✅ sekha-cli (Python)**
**Current State:**
- Tests: `tests/test_client.py`, `test_commands.py`, `test_config.py`
- Has `pyproject.toml`

**Missing/Needs Verification:**
- [ ] pytest configuration
- [ ] Linting script (ruff/black)
- [ ] Test runner script
- [ ] coverage.py setup

**create file: `sekha-cli/scripts/test.sh`**
```bash
#!/bin/bash
set -e

echo "🧪 Running Sekha CLI Test Suite..."

TEST_TYPE=${1:-"all"}

# Install dev dependencies if needed
pip install -e ".[test]"

case $TEST_TYPE in
  "lint")
    echo "🔍 Running linters..."
    ruff check .
    ruff format --check .
    ;;
  "unit")
    echo "Running unit tests..."
    pytest tests/ -v --cov=sekha_cli --cov-report=term-missing
    ;;
  "all"|*)
    echo "Running linting and all tests..."
    ruff check .
    ruff format --check .
    pytest tests/ -v --cov=sekha_cli --cov-report=html --cov-report=term-missing
    ;;
esac

echo "✅ Tests complete!"