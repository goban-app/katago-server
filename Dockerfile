
# Builder stage for KataGo

FROM debian:bookworm-slim AS katago-builder
ARG KATAGO_MODEL=kata1-b28c512nbt-s11923456768-d5584765134.bin.gz
ENV KATAGO_MODEL=${KATAGO_MODEL}

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
# Build for CPU (Eigen), AVX2 nur für amd64
RUN ARCH=$(uname -m) && \
  if [ "$ARCH" = "x86_64" ] || [ "$ARCH" = "amd64" ]; then \
    cmake . -DUSE_BACKEND=EIGEN -DUSE_AVX2=1; \
  else \
    cmake . -DUSE_BACKEND=EIGEN -DUSE_AVX2=0; \
  fi && \
  make -j"$(nproc)" && \
  strip katago


# Final runtime stage
FROM ghcr.io/stubbi/katago-server:base AS runtime-base
ARG KATAGO_MODEL=kata1-b28c512nbt-s11923456768-d5584765134.bin.gz
ENV KATAGO_MODEL=${KATAGO_MODEL}

# Install runtime dependencies
RUN set -ex; \
  apt-get update; \
  if apt-get install -y --no-install-recommends libzip5; then :; else apt-get install -y --no-install-recommends libzip4; ln -s /usr/lib/$(uname -m)-linux-gnu/libzip.so.4 /usr/lib/$(uname -m)-linux-gnu/libzip.so.5; fi; \
  apt-get install -y --no-install-recommends wget libgomp1; \
  rm -rf /var/lib/apt/lists/*

# Copy compiled binary and configs
COPY --from=katago-builder /build/KataGo/cpp/katago /app/katago
COPY gtp_config.cfg.cpu /app/gtp_config.cfg
COPY docker-setup.sh /app/

# Download model and configure
RUN chmod +x docker-setup.sh && ./docker-setup.sh

EXPOSE 2718

ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:2718/health || exit 1

CMD ["./katago-server"]
