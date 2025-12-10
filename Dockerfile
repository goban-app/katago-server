# ==============================================================================
# Multi-stage Dockerfile for katago-server
# Supports multiple build targets: base, cpu, gpu, minimal
# Usage: docker build --target <target> -t katago-server:<tag> .
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage: chef-planner
# Prepares the recipe for cargo-chef
# ------------------------------------------------------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1.83-slim AS chef-planner

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo chef prepare --recipe-path recipe.json

# ------------------------------------------------------------------------------
# Stage: rust-builder
# Builds the Rust katago-server binary using cargo-chef for optimal caching
# ------------------------------------------------------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1.83-slim AS rust-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies using cargo-chef (cached unless dependencies change)
COPY --from=chef-planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the actual application (only this layer rebuilds when code changes)
RUN cargo build --release

# ------------------------------------------------------------------------------
# Stage: base
# Minimal runtime with just the katago-server binary
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS base

WORKDIR /app

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libgomp1 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=rust-builder /app/target/release/katago-server /app/

# Copy config templates
COPY config.toml.example /app/config.toml.example
COPY analysis_config.cfg.example /app/analysis_config.cfg.example

EXPOSE 2718
ENV RUST_LOG=debug

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:2718/api/v1/health || exit 1

CMD ["./katago-server"]

# ------------------------------------------------------------------------------
# Stage: katago-cpu-builder
# Builds KataGo with CPU backend (Eigen + AVX2)
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS katago-cpu-builder

RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    cmake \
    libeigen3-dev \
    libzip-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Clone specific version (v1.15.0 required for human-style model support)
RUN git clone --depth 1 -b v1.15.0 https://github.com/lightvector/KataGo.git

WORKDIR /build/KataGo/cpp

# Build for CPU (Eigen), AVX2 only for amd64
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ] || [ "$ARCH" = "amd64" ]; then \
        cmake . -DUSE_BACKEND=EIGEN -DUSE_AVX2=1; \
    else \
        cmake . -DUSE_BACKEND=EIGEN -DUSE_AVX2=0; \
    fi && \
    make -j"$(nproc)" && \
    strip katago

# ------------------------------------------------------------------------------
# Stage: katago-gpu-builder
# Builds KataGo with CUDA backend
# Using CUDA 11.8 for broader driver compatibility (requires driver >= 450.80.02)
# ------------------------------------------------------------------------------
FROM nvidia/cuda:11.8.0-devel-ubuntu22.04 AS katago-gpu-builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    cmake \
    libzip-dev \
    libssl-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install cuDNN 8.9.7 for CUDA 11.x
RUN wget -q https://developer.download.nvidia.com/compute/cudnn/redist/cudnn/linux-x86_64/cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz \
    && tar -xf cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz \
    && cp cudnn-linux-x86_64-8.9.7.29_cuda11-archive/include/* /usr/local/cuda/include/ \
    && cp cudnn-linux-x86_64-8.9.7.29_cuda11-archive/lib/* /usr/local/cuda/lib64/ \
    && ldconfig \
    && rm -rf cudnn-linux-x86_64-8.9.7.29_cuda11-archive*

WORKDIR /build

# Clone specific version (v1.15.0 required for human-style model support)
RUN git clone --depth 1 -b v1.15.0 https://github.com/lightvector/KataGo.git

WORKDIR /build/KataGo/cpp

# Build for CUDA
RUN cmake . -DUSE_BACKEND=CUDA -DCUDNN_INCLUDE_DIR=/usr/local/cuda/include \
    && make -j"$(nproc)" \
    && strip katago

# ------------------------------------------------------------------------------
# Stage: cpu
# CPU variant with KataGo binary and model
# ------------------------------------------------------------------------------
FROM base AS cpu

ARG KATAGO_MODEL=kata1-b28c512nbt-s11923456768-d5584765134.bin.gz
ENV KATAGO_MODEL=${KATAGO_MODEL}

# Install runtime dependencies for KataGo
RUN set -ex; \
    apt-get update; \
    if apt-get install -y --no-install-recommends libzip5; then :; \
    else apt-get install -y --no-install-recommends libzip4; \
        ln -s /usr/lib/$(uname -m)-linux-gnu/libzip.so.4 /usr/lib/$(uname -m)-linux-gnu/libzip.so.5; \
    fi; \
    rm -rf /var/lib/apt/lists/*

# Copy KataGo binary
COPY --from=katago-cpu-builder /build/KataGo/cpp/katago /app/katago

# Copy configuration
COPY analysis_config.cfg.cpu /app/analysis_config.cfg
COPY docker-setup.sh /app/

# Download model and configure
RUN chmod +x docker-setup.sh && ./docker-setup.sh

# Create log directory with correct ownership for non-root user
RUN mkdir -p /app/analysis_logs && chown 1000:1000 /app/analysis_logs

# ------------------------------------------------------------------------------
# Stage: gpu
# GPU variant with CUDA-enabled KataGo binary and model
# Using CUDA 11.8 runtime for broader driver compatibility
# ------------------------------------------------------------------------------
FROM nvidia/cuda:11.8.0-base-ubuntu22.04 AS gpu

ARG KATAGO_MODEL=kata1-b28c512nbt-s11923456768-d5584765134.bin.gz
ENV KATAGO_MODEL=${KATAGO_MODEL}

WORKDIR /app

# Install runtime dependencies
RUN set -ex; \
    apt-get update; \
    if apt-get install -y --no-install-recommends libzip5; then :; \
    else apt-get install -y --no-install-recommends libzip4; \
        ln -s /usr/lib/$(uname -m)-linux-gnu/libzip.so.4 /usr/lib/$(uname -m)-linux-gnu/libzip.so.5; \
    fi; \
    apt-get install -y --no-install-recommends ca-certificates wget libgomp1; \
    rm -rf /var/lib/apt/lists/*

# Copy cuDNN libraries from builder (required at runtime)
COPY --from=katago-gpu-builder /usr/local/cuda/lib64/libcudnn* /usr/local/cuda/lib64/
RUN ldconfig

# Copy binaries
COPY --from=rust-builder /app/target/release/katago-server /app/
COPY --from=katago-gpu-builder /build/KataGo/cpp/katago /app/katago

# Copy configurations
COPY config.toml.example /app/config.toml.example
COPY analysis_config.cfg.gpu /app/analysis_config.cfg
COPY docker-setup.sh /app/

# Download model and configure
RUN chmod +x docker-setup.sh && ./docker-setup.sh

# Create log directory with correct ownership for non-root user
RUN mkdir -p /app/analysis_logs && chown 1000:1000 /app/analysis_logs

EXPOSE 2718
ENV RUST_LOG=debug
# Bind to IPv6 wildcard to support both IPv4 and IPv6 (required for Salad Cloud)
ENV KATAGO_SERVER_HOST="::"

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:2718/api/v1/health || exit 1

CMD ["./katago-server"]

# ------------------------------------------------------------------------------
# Stage: minimal
# Minimal variant - expects KataGo and model to be mounted as volumes
# ------------------------------------------------------------------------------
FROM base AS minimal

# Create default configs pointing to mounted paths
RUN cp config.toml.example config.toml && \
    sed -i 's|katago_path = "./katago"|katago_path = "/models/katago"|' config.toml && \
    sed -i 's|model_path = ".*"|model_path = "/models/model.bin.gz"|' config.toml && \
    sed -i 's|config_path = ".*"|config_path = "/models/analysis_config.cfg"|' config.toml
