# KataGo Server (Rust)

[![CI](https://github.com/stubbi/katago-server/actions/workflows/ci.yml/badge.svg)](https://github.com/stubbi/katago-server/actions/workflows/ci.yml)
[![Docker Image](https://img.shields.io/badge/docker-latest%20%7C%20latest--gpu%20%7C%20latest--minimal-44cc11)](https://github.com/stubbi/katago-server/pkgs/container/katago-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

A high-performance REST API server for KataGo, written in Rust using Axum. Provides a comprehensive, versioned API following REST best practices with RFC 7807 error handling for position analysis, move suggestions, and territory evaluation.

## Features

- 🚀 **High Performance**: Built with Rust and async/await using Tokio
- 🔒 **Type Safety**: Full type checking with comprehensive error handling
- 🌐 **Versioned REST API**: `/api/v1/` endpoints with comprehensive position analysis
- 📊 **Rich Analysis**: Returns win probability, score estimates, move candidates with visits and priors
- 🎯 **Territory Ownership**: Detailed ownership predictions for each intersection
- ⚙️ **Configurable**: Support for TOML config files and environment variables
- 🔄 **Process Management**: Automatic KataGo process lifecycle management
- 🏗️ **Standards Compliant**: RFC 7807 Problem Details for HTTP error responses
- 🎯 **Production Ready**: CORS support, structured logging, health checks, and cache management

## Prerequisites

- Rust 1.70 or later
- KataGo binary (download from [KataGo releases](https://github.com/lightvector/KataGo/releases))
- A KataGo neural network model (e.g., `kata1-b18c384nbt-s*.bin.gz`)
- A KataGo GTP config file

## Installation

### Quick Start with Docker

The easiest way to get started is using the pre-built Docker image:

```bash
# CPU version (recommended for getting started)
docker pull ghcr.io/stubbi/katago-server:latest
docker run -p 2718:2718 ghcr.io/stubbi/katago-server:latest

# GPU version (requires NVIDIA GPU and nvidia-docker)
docker pull ghcr.io/stubbi/katago-server:latest-gpu
docker run --gpus all -p 2718:2718 ghcr.io/stubbi/katago-server:latest-gpu
```

The server will be available at `http://localhost:2718`.

See the [Docker Image](#docker-image) section for more variants and configuration options.

### 1. Clone the Repository

```bash
git clone https://github.com/stubbi/katago-server
cd katago-server
```

### 2. Download KataGo

Download the appropriate KataGo binary for your platform:

```bash
# Linux (CPU - eigen build for compatibility)
wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/katago-v1.14.1-eigen-linux-x64.zip
unzip katago-v1.14.1-eigen-linux-x64.zip

# Linux (GPU - requires CUDA 12.1)
wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/katago-v1.14.1-cuda12.1-cudnn8.9.7-linux-x64.zip
unzip katago-v1.14.1-cuda12.1-cudnn8.9.7-linux-x64.zip

# macOS
wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/katago-v1.14.1-osx-x64.zip
unzip katago-v1.14.1-osx-x64.zip
```

Make it executable:
```bash
chmod +x katago
```

### 3. Download a Neural Network Model

Download from the KataGo training networks:

```bash
# 15-block model (faster, ~120MB, suitable for CPU)
wget -O model.bin.gz https://katagotraining.org/api/networks/kata1-b15c192-s1672170752-d466197061/network_file

# 40-block model (stronger, ~450MB, recommended for GPU)
wget -O model.bin.gz https://katagotraining.org/api/networks/kata1-b40c256-s11840935168-d2898845681/network_file
```

### 4. Create a GTP Configuration File

Create `gtp_config.cfg`:

```ini
# Basic GTP config for KataGo
# Adjust based on your hardware

# Number of threads to use
numSearchThreads = 4

# Maximum number of visits to search
maxVisits = 500

# Pondering (thinking during opponent's turn) - false for server
ponderingEnabled = false

# GPU settings (set to -1 to use all GPUs, or specific ID)
# numNNServerThreadsPerModel = 1
# nnMaxBatchSize = 16

# For CPU-only (slower):
# numNNServerThreadsPerModel = 2
# nnMaxBatchSize = 8

# Rules
rules = chinese
koRule = POSITIONAL
scoringRule = AREA
taxRule = NONE
multiStoneSuicideLegal = false

# Analysis settings
analysisWideRootNoise = 0.0
```

### 5. Build the Project

```bash
cargo build --release
```

## Configuration

### Option 1: Configuration File

Create `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 2718

[katago]
katago_path = "./katago"
model_path = "./kata1-b18c384nbt-s9131461376-d4087399203.bin.gz"
config_path = "./gtp_config.cfg"
move_timeout_secs = 20
```

### Option 2: Environment Variables

```bash
export KATAGO_SERVER_HOST="0.0.0.0"
export KATAGO_SERVER_PORT="2718"
export KATAGO_KATAGO_PATH="./katago"
export KATAGO_MODEL_PATH="./model.bin.gz"
export KATAGO_CONFIG_PATH="./gtp_config.cfg"
export KATAGO_MOVE_TIMEOUT_SECS="20"
```

## Usage

### Start the Server

```bash
# With config file
./target/release/katago-server

# With debug logging
RUST_LOG=debug ./target/release/katago-server
```

The server will start on `http://0.0.0.0:2718` (or your configured port).

## API Endpoints

The server provides a versioned REST API (`/api/v1/`) following best practices with RFC 7807 error handling.

### 1. Comprehensive Analysis

Analyze a Go position with comprehensive information including move candidates, win probability, score estimation, and optional territory ownership.

**Endpoint:** `POST /api/v1/analysis`

**Request:**
```json
{
  "moves": ["D4", "Q16", "R4"],
  "komi": 7.5,
  "rules": "chinese",
  "boardXSize": 19,
  "boardYSize": 19,
  "includeOwnership": true,
  "includePolicy": false,
  "maxVisits": 100,
  "requestId": "optional-id"
}
```

**Request Parameters:**
- `moves` (required): Array of moves in coordinate notation (e.g., ["D4", "Q16"])
- `komi` (optional, default: 7.5): Komi value
- `rules` (optional): Game rules ("chinese", "japanese", "korean", "tromp-taylor", etc.)
- `boardXSize` (optional, default: 19): Board width
- `boardYSize` (optional, default: 19): Board height
- `includeOwnership` (optional): Include territory ownership predictions
- `includePolicy` (optional): Include raw neural network policy
- `includePVVisits` (optional): Include visit counts in principal variations
- `maxVisits` (optional): Override search visit limit
- `analysisPVLen` (optional): Length of principal variation to return
- `requestId` (optional): Request identifier echoed back in response

**Response:**
```json
{
  "id": "optional-id",
  "turnNumber": 3,
  "isDuringSearch": false,
  "moveInfos": [
    {
      "moveCoord": "D16",
      "visits": 142,
      "winrate": 0.523,
      "scoreMean": 2.5,
      "scoreStdev": 8.2,
      "scoreLead": 2.5,
      "utility": 0.031,
      "utilityLcb": 0.025,
      "lcb": 0.515,
      "prior": 0.18,
      "order": 0,
      "pv": ["D16", "Q4", "D10"],
      "pvVisits": [142, 95, 82]
    }
  ],
  "rootInfo": {
    "winrate": 0.512,
    "scoreLead": 1.5,
    "utility": 0.015,
    "visits": 500,
    "currentPlayer": "B",
    "rawWinrate": 0.508,
    "rawScoreMean": 1.2
  },
  "ownership": [0.85, 0.92, -0.15, ...]
}
```

### 2. Version Information

Get server and KataGo version information.

**Endpoint:** `GET /api/v1/version`

**Response:**
```json
{
  "server": {
    "name": "katago-server",
    "version": "0.1.0"
  },
  "katago": {
    "version": "1.15.3",
    "gitHash": "abc123def"
  }
}
```

### 3. Health Check

**Endpoint:** `GET /api/v1/health`

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-11-27T12:34:56Z",
  "uptime": 3600
}
```

### 4. Clear Cache

Clear the KataGo neural network cache to free memory.

**Endpoint:** `POST /api/v1/cache/clear`

**Response:**
```json
{
  "status": "cleared",
  "timestamp": "2025-11-27T12:34:56Z"
}
```

## Testing with curl

```bash
# Analyze a position
curl -X POST http://localhost:2718/api/v1/analysis \
  -H "Content-Type: application/json" \
  -d '{
    "moves": ["D4", "Q16"],
    "komi": 7.5,
    "rules": "chinese",
    "includeOwnership": true
  }'

# Get version info
curl http://localhost:2718/api/v1/version

# Health check
curl http://localhost:2718/api/v1/health

# Clear cache
curl -X POST http://localhost:2718/api/v1/cache/clear
```

## Error Responses

All errors follow RFC 7807 Problem Details format:

```json
{
  "type": "https://katago-server/problems/timeout",
  "title": "Analysis Timeout",
  "status": 504,
  "detail": "KataGo analysis timed out after 20 seconds",
  "instance": "/api/v1/analysis",
  "requestId": "req-123"
}
```

Common error types:
- `invalid-request` (400): Malformed request
- `timeout` (504): Analysis timeout
- `process-died` (503): KataGo process crashed
- `internal-error` (500): Unexpected server error

## Architecture

### Components

- **`main.rs`**: Application entry point, server initialization
- **`api.rs`**: REST API endpoints and request/response types
- **`katago_bot.rs`**: KataGo process management and GTP protocol handling
- **`config.rs`**: Configuration structures and loading
- **`error.rs`**: Error types and handling

### Design Decisions

1. **Async Architecture**: Uses Tokio for efficient concurrent request handling
2. **Type Safety**: Strongly typed API with serde for JSON serialization
3. **Process Management**: Spawns KataGo as a child process with stdin/stdout communication
4. **Error Handling**: Custom error types with proper error propagation
5. **Logging**: Structured logging with tracing for observability
6. **CORS**: Enabled for cross-origin requests from web frontends

## Production Deployment

### As a systemd Service

Create `/etc/systemd/system/katago-server.service`:

```ini
[Unit]
Description=KataGo Server
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/var/www/katago-server
ExecStart=/var/www/katago-server/target/release/katago-server
Restart=on-failure
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable katago-server
sudo systemctl start katago-server
sudo systemctl status katago-server
```

View logs:
```bash
journalctl -u katago-server -f
```

### Behind a Reverse Proxy (nginx)

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:2718;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Performance Tuning

### KataGo Configuration

Adjust `gtp_config.cfg` based on your hardware:

**For GPU:**
```ini
numSearchThreads = 8
maxVisits = 800
numNNServerThreadsPerModel = 2
nnMaxBatchSize = 32
```

**For CPU:**
```ini
numSearchThreads = 4
maxVisits = 400
numNNServerThreadsPerModel = 1
nnMaxBatchSize = 8
```

### Server Configuration

- Increase `move_timeout_secs` for stronger analysis
- Use smaller neural networks for faster responses
- Run multiple instances behind a load balancer for high traffic

## Troubleshooting

### KataGo Process Fails to Start

- Verify `katago_path` points to the correct binary
- Check that `model_path` and `config_path` exist
- Ensure KataGo binary has execute permissions
- Check system logs: `journalctl -xe`

### Timeout Errors

- Increase `move_timeout_secs` in config
- Reduce `maxVisits` in KataGo config
- Use a smaller neural network model

### High Memory Usage

- Reduce `nnMaxBatchSize` in KataGo config
- Use a smaller neural network (fewer blocks/channels)
- Limit `numSearchThreads`

## Development

```bash
# Run in development mode with hot reload (install cargo-watch)
cargo install cargo-watch
cargo watch -x run

# Run tests
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

## Comparison with Python Version

| Feature | Python (Flask) | Rust (Axum) |
|---------|---------------|-------------|
| Performance | ~100 req/s | ~5000 req/s |
| Memory Usage | ~100 MB | ~10 MB |
| Startup Time | ~1s | ~100ms |
| Type Safety | Runtime | Compile-time |
| Concurrency | Threading/GIL | Async/await |

## License

This project is provided as-is for educational and production use. Please ensure compliance with KataGo's license when using the neural network models.

## Docker Image

Pre-built Docker images are automatically published to GitHub Container Registry with three variants:

### Image Variants

#### 1. CPU (Default) - `latest`
**Best for**: Testing, development, moderate usage
- **Model**: 15-block network (~120MB) for faster downloads
- **KataGo**: Eigen build (broad CPU compatibility)
- **Performance**: Suitable for casual play and development
- **Size**: ~80MB compressed
- **Requirements**: Any x86_64 or ARM64 system
- **Memory**: ~500MB RAM

```bash
docker pull ghcr.io/stubbi/katago-server:latest
docker run -p 2718:2718 ghcr.io/stubbi/katago-server:latest
```

#### 2. GPU - `latest-gpu`
**Best for**: Production, high-performance analysis
- **Model**: 40-block network (~450MB) for strong play
- **KataGo**: CUDA 12.1 with cuDNN 8.9.7
- **Performance**: Professional strength, suitable for dan-level play
- **Size**: ~2GB compressed
- **Requirements**: NVIDIA GPU with CUDA 12.1+, nvidia-docker
- **Memory**: ~1GB RAM + 2-4GB VRAM

```bash
docker pull ghcr.io/stubbi/katago-server:latest-gpu
docker run --gpus all -p 2718:2718 ghcr.io/stubbi/katago-server:latest-gpu
```

#### 3. Minimal - `latest-minimal`
**Best for**: Custom configurations, different models
- **Model**: None (bring your own)
- **KataGo**: Not included (mount your own)
- **Size**: ~20MB compressed
- **Requirements**: Mount `/app` directory with katago, model, and config

```bash
docker pull ghcr.io/stubbi/katago-server:latest-minimal
docker run -p 2718:2718 \
  -v /path/to/katago:/app/katago:ro \
  -v /path/to/model.bin.gz:/app/model.bin.gz:ro \
  -v /path/to/gtp_config.cfg:/app/gtp_config.cfg:ro \
  ghcr.io/stubbi/katago-server:latest-minimal
```

### Usage Examples

**Quick Start (CPU)**:
```bash
docker run -p 2718:2718 ghcr.io/stubbi/katago-server:latest
# Server available at http://localhost:2718
```

**GPU with Custom Config**:
```bash
docker run --gpus all -p 2718:2718 \
  -v $(pwd)/gtp_config.cfg:/app/gtp_config.cfg:ro \
  ghcr.io/stubbi/katago-server:latest-gpu
```

**Custom Model (Minimal)**:
```bash
# Your directory structure:
# /my-models/
#   ├── katago (binary)
#   ├── my-custom-model.bin.gz
#   └── gtp_config.cfg

docker run -p 2718:2718 \
  -v /my-models:/models:ro \
  ghcr.io/stubbi/katago-server:latest-minimal
```

**Override Environment Variables**:
```bash
docker run -p 2718:2718 \
  -e RUST_LOG=debug \
  -e KATAGO_SERVER__HOST=0.0.0.0 \
  -e KATAGO_SERVER__PORT=2718 \
  ghcr.io/stubbi/katago-server:latest
```

### Docker Compose

```yaml
version: '3.8'
services:
  katago-cpu:
    image: ghcr.io/stubbi/katago-server:latest
    ports:
      - "2718:2718"
    environment:
      - RUST_LOG=info
    restart: unless-stopped

  katago-gpu:
    image: ghcr.io/stubbi/katago-server:latest-gpu
    ports:
      - "2719:2718"
    runtime: nvidia
    environment:
      - RUST_LOG=info
      - NVIDIA_VISIBLE_DEVICES=all
    restart: unless-stopped
```

### Building Custom Images

You can build your own image with a different model:

```bash
# Build with specific model
docker build --target cpu -t my-katago-server \
  --build-arg KATAGO_MODEL=kata1-b10c128-s*.bin.gz \
  .

# Build GPU version
docker build --target gpu -t my-katago-server:gpu .

# Build minimal version (no model included)
docker build --target minimal -t my-katago-server:minimal .

# Build base version (server binary only)
docker build --target base -t my-katago-server:base .
```

### Mounting Custom Models

All variants support mounting custom models and configurations:

```bash
docker run -p 2718:2718 \
  -v $(pwd)/my-model.bin.gz:/app/my-model.bin.gz:ro \
  -v $(pwd)/my-config.cfg:/app/gtp_config.cfg:ro \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  ghcr.io/stubbi/katago-server:latest
```

Then update `config.toml` to point to `/app/my-model.bin.gz`.

### CPU vs GPU Configuration

**CPU Configuration** (`gtp_config.cfg`):
```ini
numSearchThreads = 2
maxVisits = 200
numNNServerThreadsPerModel = 2
nnMaxBatchSize = 8
```

**GPU Configuration** (`gtp_config.cfg`):
```ini
numSearchThreads = 8
maxVisits = 800
numNNServerThreadsPerModel = 2
nnMaxBatchSize = 32
```

### Available Tags

- `latest` - CPU version with 15-block model (recommended for getting started)
- `latest-gpu` - GPU version with 40-block model (production-grade strength)
- `latest-minimal` - No bundled model, bring your own
- `v*.*.*` - Semantic version tags (all variants available)
- `main` - Latest main branch build (development)

### Multi-Architecture Support

All images support both `linux/amd64` and `linux/arm64` architectures, automatically selecting the correct one for your system.

## Performance Characteristics

### Optimized Build Settings
- **LTO**: Thin link-time optimization for smaller binaries
- **Strip**: Debug symbols removed for production
- **Panic**: Abort on panic for reduced binary size
- **Codegen**: Single codegen unit for maximum optimization

### Expected Performance
- **Binary Size**: ~3.4MB (stripped, optimized)
- **Memory Usage**: ~10MB (server) + KataGo overhead
- **Startup Time**: <100ms
- **Throughput**: ~5000 req/s (without KataGo bottleneck)
- **Latency**: <1ms (API overhead only)

### Benchmarks (on moderate hardware)
- **15-block model (CPU)**: ~2-5 seconds per move
- **40-block model (GPU)**: ~0.5-1 second per move

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests at [github.com/stubbi/katago-server](https://github.com/stubbi/katago-server).

See EXAMPLES.md for usage patterns and client implementations.
See ARCHITECTURE.md for design decisions and implementation details.

## References

- [This Project](https://github.com/stubbi/katago-server)
- [KataGo](https://github.com/lightvector/KataGo)
- [Original Python katago-server](https://github.com/hauensteina/katago-server)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [GTP Protocol](https://www.lysator.liu.se/~gunnar/gtp/)
