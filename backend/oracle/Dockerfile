# Multi-stage build for PIFP Oracle Service
# Produces a minimal container with just the binary and required dependencies

# Stage 1: Build the Rust binary
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build for release
RUN cargo build --release

# Stage 2: Create minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false oracle

# Copy binary from builder
COPY --from=builder /app/target/release/pifp-oracle /usr/local/bin/pifp-oracle
RUN chmod +x /usr/local/bin/pifp-oracle

# Switch to non-root user
USER oracle

# Set working directory
WORKDIR /home/oracle

# Default environment variables (override at runtime)
ENV RUST_LOG=info
ENV IPFS_GATEWAY=https://ipfs.io
ENV TIMEOUT_SECS=30

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD pifp-oracle --help || exit 1

# Entry point
ENTRYPOINT ["pifp-oracle"]

# Default command (can be overridden)
CMD ["--help"]
