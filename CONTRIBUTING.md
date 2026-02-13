# Contributing to Sekha Python SDK

Thank you for your interest in contributing to Sekha! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/sekha-python-sdk.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Install development dependencies: `pip install -e ".[dev]"`

## Development Setup

### Prerequisites
- Python 3.12+
- pip or uv
- Git

### Install Development Dependencies
```bash
pip install -e ".[dev]"

Testing
Run the test suite:

bash
pytest
pytest --cov=sekha --cov-report=html  # With coverage

Tests should pass and maintain >80% coverage.

Code Style
Formatter: Black (black sekha tests)
Import Sorting: isort (isort sekha tests)
Type Checking: mypy (mypy sekha)
Linting: Ruff (configured in pyproject.toml)

Run all checks:

bash
black sekha tests
isort sekha tests
mypy sekha
pytest --cov=sekha

Pull Request Process
1. Ensure all tests pass: pytest
2. Ensure code style compliance: black, isort, mypy
3. Update documentation if needed
4. Add test coverage for new functionality (aim for >80% coverage)
5. Submit PR with clear description of changes
6. Address review feedback promptly

Commit Message Guidelines
Use clear, descriptive commit messages
Start with a verb in present tense: "Add feature", "Fix bug", "Update docs"
Reference related issues: "Fixes #123"
Keep commits focused on a single concern
Reporting Issues
Use GitHub Issues to report bugs or suggest features

Include:

Python version
SDK version
Minimal reproducible example (for bugs)
Expected vs actual behavior

Code of Conduct
Please refer to CODE_OF_CONDUCT.md for our community standards.