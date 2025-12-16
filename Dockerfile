FROM rust:1.91-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    libpq-dev \
    curl \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migration ./migration
COPY migrations ./migrations

# Build release binary
RUN cargo build --release --bin sekha-controller

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sekha-controller /usr/local/bin/

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["sekha-controller"]