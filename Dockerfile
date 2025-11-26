# Multi-stage build for KataGo Server - CPU variant
# Builds on top of base image with pre-compiled binary
# This image includes CPU-only KataGo and a lightweight 18-block model

# Builder stage for KataGo
FROM debian:bookworm-slim AS katago-builder

RUN apt-get update && apt-get install -y \
  git \
  build-essential \
  cmake \
  libeigen3-dev \
  libzip-dev \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /build
# Clone specific version
RUN git clone --depth 1 -b v1.14.1 https://github.com/lightvector/KataGo.git

WORKDIR /build/KataGo/cpp
# Build for CPU (Eigen)
RUN cmake . -DUSE_BACKEND=EIGEN -DUSE_AVX2=1 \
  && make -j"$(nproc)" \
  && strip katago

# Final runtime stage
FROM ghcr.io/stubbi/katago-server:base AS runtime-base

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  wget \
  libzip4 \
  libgomp1 \
  && rm -rf /var/lib/apt/lists/*

# Copy compiled binary
COPY --from=katago-builder /build/KataGo/cpp/katago /app/katago

# Download model and configure
RUN set -ex && \
  wget -q -O kata1-b15c192-s1672170752-d466197061.bin.gz https://katagotraining.org/api/networks/kata1-b15c192-s1672170752-d466197061/network_file && \
  chmod +x katago && \
  # Create default configs optimized for CPU usage
  cp config.toml.example config.toml && \
  cp gtp_config.cfg.example gtp_config.cfg && \
  sed -i 's/numSearchThreads = 4/numSearchThreads = 2/' gtp_config.cfg && \
  sed -i 's/maxVisits = 500/maxVisits = 200/' gtp_config.cfg && \
  # Fix model path in config to match downloaded filename
  sed -i 's|model_path = ".*"|model_path = "./kata1-b28c512nbt-s11803203328-d5553431682.bin.gz"|' config.toml

EXPOSE 2718

ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:2718/health || exit 1

CMD ["./katago-server"]
