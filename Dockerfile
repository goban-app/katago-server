# Multi-stage build for KataGo Server - CPU variant
# Builds on top of base image with pre-compiled binary
# This image includes CPU-only KataGo and a lightweight 18-block model

FROM ghcr.io/stubbi/katago-server:base AS runtime-base

# Install wget and unzip for downloading KataGo
RUN apt-get update && apt-get install -y \
    wget \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Download KataGo and model in parallel, then configure
# Default: CPU-only version (eigen build for broad compatibility)
# For GPU or better performance: mount your own katago binary and model as volumes
ARG KATAGO_VERSION=v1.14.1
ARG KATAGO_BUILD=eigen

RUN set -ex && \
    # Download KataGo binary and model in parallel using background jobs
    wget -q https://github.com/lightvector/KataGo/releases/download/${KATAGO_VERSION}/katago-${KATAGO_VERSION}-${KATAGO_BUILD}-linux-x64.zip & \
    wget -q -O kata1-b15c192-s1672170752-d466197061.bin.gz https://katagotraining.org/api/networks/kata1-b15c192-s1672170752-d466197061/network_file & \
    wait && \
    # Extract and cleanup
    unzip -q katago-${KATAGO_VERSION}-${KATAGO_BUILD}-linux-x64.zip && \
    chmod +x katago && \
    rm katago-${KATAGO_VERSION}-${KATAGO_BUILD}-linux-x64.zip && \
    # Create default configs optimized for CPU usage
    cp config.toml.example config.toml && \
    cp gtp_config.cfg.example gtp_config.cfg && \
    sed -i 's/numSearchThreads = 4/numSearchThreads = 2/' gtp_config.cfg && \
    sed -i 's/maxVisits = 500/maxVisits = 200/' gtp_config.cfg && \
    # Fix model path in config to match downloaded filename
    sed -i 's|model_path = ".*"|model_path = "./kata1-b15c192-s1672170752-d466197061.bin.gz"|' config.toml

EXPOSE 2718

ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:2718/health || exit 1

CMD ["./katago-server"]
