# Contributing to Sekha Controller

Thank you for your interest in contributing to Sekha! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/sekha-controller.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Build the project: `cargo build`

## Development Setup

### Prerequisites
- Rust 1.75+ (stable)
- Cargo
- Git
- Docker (for integration tests)

### Install Development Dependencies
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
cargo install cargo-watch cargo-nextest
```

## Testing

Run the test suite:

```bash
cargo test                         # Run all tests
cargo test --all-features          # Test all features
cargo nextest run                  # Faster test runner
cargo test -- --nocapture          # Show output
```

Tests should pass and maintain >80% coverage.

## Code Style

- **Formatter:** rustfmt (configured in rustfmt.toml)
- **Linter:** clippy (Rust's official linter)
- **Documentation:** cargo doc

Run all checks:

```bash
cargo fmt                          # Format code
cargo clippy -- -D warnings        # Lint with warnings as errors
cargo test                         # Run tests
cargo build --release              # Build optimized binary
```

## Pull Request Process

1. Ensure all tests pass: `cargo test`
2. Ensure code style compliance: `cargo fmt --check` and `cargo clippy`
3. Ensure documentation builds: `cargo doc --no-deps`
4. Update documentation if needed
5. Add test coverage for new functionality (aim for >80% coverage)
6. Submit PR with clear description of changes
7. Address review feedback promptly

## Commit Message Guidelines

- Use clear, descriptive commit messages
- Start with a verb in present tense: "Add feature", "Fix bug", "Update docs"
- Reference related issues: "Fixes #123"
- Keep commits focused on a single concern

## Reporting Issues

Use GitHub Issues to report bugs or suggest features.

Include:

- Rust version (`rustc --version`)
- Controller version
- Minimal reproducible example (for bugs)
- Expected vs actual behavior

## Code of Conduct

Please refer to CODE_OF_CONDUCT.md for our community standards.
