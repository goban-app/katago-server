# Multi-stage build for KataGo Server
# This image includes CPU-only KataGo and a lightweight 18-block model
# For GPU support or different models, mount them as volumes
FROM rust:1.83-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
# libgomp1 is required for OpenMP support in KataGo
RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    unzip \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/katago-server /app/

# Copy config templates
COPY config.toml.example /app/config.toml.example
COPY gtp_config.cfg.example /app/gtp_config.cfg.example

# Download KataGo and model in parallel, then configure
# Default: CPU-only version (eigen build for broad compatibility)
# For GPU or better performance: mount your own katago binary and model as volumes
ARG KATAGO_VERSION=v1.14.1
ARG KATAGO_BUILD=eigen

RUN set -ex && \
    # Download KataGo binary and model in parallel using background jobs
    wget -q https://github.com/lightvector/KataGo/releases/download/${KATAGO_VERSION}/katago-${KATAGO_VERSION}-${KATAGO_BUILD}-linux-x64.zip & \
    wget -q -O model.bin.gz https://katagotraining.org/api/networks/kata1-b15c192-s1672170752-d466197061/network_file & \
    wait && \
    # Extract and cleanup
    unzip -q katago-${KATAGO_VERSION}-${KATAGO_BUILD}-linux-x64.zip && \
    chmod +x katago && \
    rm katago-${KATAGO_VERSION}-${KATAGO_BUILD}-linux-x64.zip && \
    # Create default configs optimized for CPU usage
    cp config.toml.example config.toml && \
    cp gtp_config.cfg.example gtp_config.cfg && \
    sed -i 's/numSearchThreads = 4/numSearchThreads = 2/' gtp_config.cfg && \
    sed -i 's/maxVisits = 500/maxVisits = 200/' gtp_config.cfg

EXPOSE 2718

ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:2718/health || exit 1

CMD ["./katago-server"]
