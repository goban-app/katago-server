# ==============================================================================
# Multi-stage Dockerfile for katago-server
# Supports multiple build targets: base, cpu, gpu, minimal
# Usage: docker build --target <target> -t katago-server:<tag> .
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage: chef-planner
# Prepares the recipe for cargo-chef
# ------------------------------------------------------------------------------
FROM rust:1.83-slim AS chef-planner

RUN cargo install cargo-chef

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo chef prepare --recipe-path recipe.json

# ------------------------------------------------------------------------------
# Stage: rust-builder
# Builds the Rust katago-server binary using cargo-chef for optimal caching
# ------------------------------------------------------------------------------
FROM rust:1.83-slim AS rust-builder

RUN cargo install cargo-chef

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
COPY gtp_config.cfg.example /app/gtp_config.cfg.example

EXPOSE 2718
ENV RUST_LOG=debug

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:2718/health || exit 1

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

# Clone specific version
RUN git clone --depth 1 -b v1.14.1 https://github.com/lightvector/KataGo.git

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
# ------------------------------------------------------------------------------
FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 AS katago-gpu-builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    cmake \
    libzip-dev \
    libssl-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install cuDNN 8.9.7 for CUDA 12.2
RUN wget -q https://developer.download.nvidia.com/compute/cudnn/redist/cudnn/linux-x86_64/cudnn-linux-x86_64-8.9.7.29_cuda12-archive.tar.xz \
    && tar -xf cudnn-linux-x86_64-8.9.7.29_cuda12-archive.tar.xz \
    && cp cudnn-linux-x86_64-8.9.7.29_cuda12-archive/include/* /usr/local/cuda/include/ \
    && cp cudnn-linux-x86_64-8.9.7.29_cuda12-archive/lib/* /usr/local/cuda/lib64/ \
    && ldconfig \
    && rm -rf cudnn-linux-x86_64-8.9.7.29_cuda12-archive*

WORKDIR /build

# Clone specific version
RUN git clone --depth 1 -b v1.14.1 https://github.com/lightvector/KataGo.git

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
COPY gtp_config.cfg.cpu /app/gtp_config.cfg
COPY docker-setup.sh /app/

# Download model and configure
RUN chmod +x docker-setup.sh && ./docker-setup.sh

# ------------------------------------------------------------------------------
# Stage: gpu
# GPU variant with CUDA-enabled KataGo binary and model
# ------------------------------------------------------------------------------
FROM nvidia/cuda:12.2.0-base-ubuntu22.04 AS gpu

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

# Copy binaries
COPY --from=rust-builder /app/target/release/katago-server /app/
COPY --from=katago-gpu-builder /build/KataGo/cpp/katago /app/katago

# Copy configurations
COPY config.toml.example /app/config.toml.example
COPY gtp_config.cfg.gpu /app/gtp_config.cfg
COPY docker-setup.sh /app/

# Download model and configure
RUN chmod +x docker-setup.sh && ./docker-setup.sh

EXPOSE 2718
ENV RUST_LOG=debug

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:2718/health || exit 1

CMD ["./katago-server"]

# ------------------------------------------------------------------------------
# Stage: minimal
# Minimal variant - expects KataGo and model to be mounted as volumes
# ------------------------------------------------------------------------------
FROM base AS minimal

# Create default configs pointing to mounted paths
RUN cp config.toml.example config.toml && \
    cp gtp_config.cfg.example gtp_config.cfg && \
    sed -i 's|katago_path = "./katago"|katago_path = "/models/katago"|' config.toml && \
    sed -i 's|model_path = ".*"|model_path = "/models/model.bin.gz"|' config.toml && \
    sed -i 's|config_path = ".*"|config_path = "/models/gtp_config.cfg"|' config.toml
