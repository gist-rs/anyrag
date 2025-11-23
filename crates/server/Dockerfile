# syntax=docker/dockerfile:1
# Simplified Dockerfile for anyrag-server

# Use the standard Rust image
FROM rust:1.88.0-bullseye as builder

# Install essential system dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the application
RUN cargo build --bin server --release

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates wget && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN addgroup --system app && \
    adduser --system --uid 1000 --ingroup app app

WORKDIR /app

# Copy credentials and binary
COPY gcp_creds.json /app/gcp_creds.json
COPY --from=builder /app/target/release/server /app/server

# Set permissions
RUN chown -R app:app /app

# Expose port
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD wget -q --spider http://localhost:9090/health || exit 1

# Run as non-root user
USER app

# Entry point
ENTRYPOINT ["/app/server"]
