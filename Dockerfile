# Multi-stage Dockerfile for LLM Research Lab
# Optimized for minimal image size and security

# ============================================================================
# Stage 1: Builder - Compile the Rust application
# ============================================================================
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY llm-research-lab/Cargo.toml ./llm-research-lab/
COPY llm-research-core/Cargo.toml ./llm-research-core/
COPY llm-research-api/Cargo.toml ./llm-research-api/
COPY llm-research-storage/Cargo.toml ./llm-research-storage/
COPY llm-research-metrics/Cargo.toml ./llm-research-metrics/
COPY llm-research-workflow/Cargo.toml ./llm-research-workflow/

# Copy source code
COPY llm-research-lab ./llm-research-lab
COPY llm-research-core ./llm-research-core
COPY llm-research-api ./llm-research-api
COPY llm-research-storage ./llm-research-storage
COPY llm-research-metrics ./llm-research-metrics
COPY llm-research-workflow ./llm-research-workflow

# Copy configuration files
COPY config ./config

# Build release binary with optimizations
RUN cargo build --release --bin llm-research-lab

# Strip debug symbols to reduce binary size
RUN strip /app/target/release/llm-research-lab

# ============================================================================
# Stage 2: Runtime - Minimal runtime environment
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN groupadd -r llmlab && useradd -r -g llmlab -u 1000 llmlab

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/llm-research-lab /app/llm-research-lab

# Copy configuration directory
COPY --from=builder /app/config /app/config

# Create data directories
RUN mkdir -p /app/data /app/logs \
    && chown -R llmlab:llmlab /app

# Switch to non-root user
USER llmlab

# Expose application port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info \
    LLM_RESEARCH_PORT=8080 \
    LLM_RESEARCH_LOG_LEVEL=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/llm-research-lab", "--health-check"] || exit 1

# Run the application
ENTRYPOINT ["/app/llm-research-lab"]
CMD []
