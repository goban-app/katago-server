# Multi-stage build for KataGo Server
FROM rust:1.75-slim as builder

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

# Download KataGo and model (you can customize this)
ARG KATAGO_VERSION=v1.14.1
ARG MODEL_NAME=kata1-b18c384nbt-s9131461376-d4087399203.bin.gz

RUN wget -q https://github.com/lightvector/KataGo/releases/download/${KATAGO_VERSION}/katago-${KATAGO_VERSION}-linux-x64.zip && \
    unzip -q katago-${KATAGO_VERSION}-linux-x64.zip && \
    chmod +x katago && \
    rm katago-${KATAGO_VERSION}-linux-x64.zip

RUN wget -q https://github.com/lightvector/KataGo/releases/download/${KATAGO_VERSION}/${MODEL_NAME}

# Create default configs if they don't exist
RUN cp config.toml.example config.toml && \
    cp gtp_config.cfg.example gtp_config.cfg

EXPOSE 2718

ENV RUST_LOG=info

CMD ["./katago-server"]
