# Multi-stage build for Sekha Controller
# Stage 1: Build
FROM rust:1.83-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source (skip dummy build, causes issues with workspaces)
COPY . .

# Build for release
RUN cargo build --release --bin sekha-controller

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/sekha-controller /usr/local/bin/sekha-controller

# Create data directory
RUN mkdir -p /data

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run as non-root user
RUN useradd -m -u 1000 sekha && \
    chown -R sekha:sekha /app /data
USER sekha

# Set environment defaults
ENV SEKHA_SERVER_PORT=8080 \
    SEKHA_DATABASE_URL="sqlite:///data/sekha.db" \
    RUST_LOG=info

CMD ["sekha-controller"]
