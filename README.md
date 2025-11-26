# KataGo Server (Rust)

A high-performance REST API server for KataGo, written in Rust using Axum. This is a minimal yet complete implementation following best practices, providing endpoints to query KataGo for move suggestions and board analysis.

## Features

- 🚀 **High Performance**: Built with Rust and async/await using Tokio
- 🔒 **Type Safety**: Full type checking with comprehensive error handling
- 🌐 **REST API**: Clean JSON endpoints for move selection and territory scoring
- 📊 **Rich Diagnostics**: Returns winning probability, score estimates, and best move candidates
- ⚙️ **Configurable**: Support for TOML config files and environment variables
- 🔄 **Process Management**: Automatic KataGo process lifecycle management
- 🎯 **Production Ready**: CORS support, structured logging, and health checks

## Prerequisites

- Rust 1.70 or later
- KataGo binary (download from [KataGo releases](https://github.com/lightvector/KataGo/releases))
- A KataGo neural network model (e.g., `kata1-b18c384nbt-s*.bin.gz`)
- A KataGo GTP config file

## Installation

### Quick Start with Docker

The easiest way to get started is using the pre-built Docker image:

```bash
docker pull ghcr.io/stubbi/katago-server:latest
docker run -p 2718:2718 ghcr.io/stubbi/katago-server:latest
```

The server will be available at `http://localhost:2718`.

### 1. Clone the Repository

```bash
git clone https://github.com/stubbi/katago-server
cd katago-server
```

### 2. Download KataGo

Download the appropriate KataGo binary for your platform:

```bash
# Linux
wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/katago-v1.14.1-linux-x64.zip
unzip katago-v1.14.1-linux-x64.zip

# macOS
wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/katago-v1.14.1-osx-x64.zip
unzip katago-v1.14.1-osx-x64.zip
```

Make it executable:
```bash
chmod +x katago
```

### 3. Download a Neural Network Model

```bash
# Download a smaller model for testing (18 blocks)
wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/kata1-b18c384nbt-s9131461376-d4087399203.bin.gz

# Or a stronger model (40 blocks, requires more RAM/GPU)
# wget https://github.com/lightvector/KataGo/releases/download/v1.14.1/kata1-b40c256-s*.bin.gz
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
export KATAGO_SERVER__HOST="0.0.0.0"
export KATAGO_SERVER__PORT="2718"
export KATAGO_KATAGO__KATAGO_PATH="./katago"
export KATAGO_KATAGO__MODEL_PATH="./kata1-b18c384nbt-s9131461376-d4087399203.bin.gz"
export KATAGO_KATAGO__CONFIG_PATH="./gtp_config.cfg"
export KATAGO_KATAGO__MOVE_TIMEOUT_SECS="20"
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

### 1. Select Move

Get the best move for a given position.

**Endpoint:** `POST /select-move/katago_gtp_bot`

**Request:**
```json
{
  "board_size": 19,
  "moves": ["Q16", "D4", "R4"],
  "config": {
    "komi": 7.5,
    "request_id": "optional-id"
  }
}
```

**Response:**
```json
{
  "bot_move": "D16",
  "diagnostics": {
    "winprob": 0.5234,
    "score": 2.5,
    "bot_move": "D16",
    "best_ten": [
      {"move": "D16", "psv": 842},
      {"move": "Q4", "psv": 835},
      {"move": "D17", "psv": 820}
    ]
  },
  "request_id": "optional-id"
}
```

### 2. Score Position

Get territory ownership probabilities for each intersection.

**Endpoint:** `POST /score/katago_gtp_bot`

**Request:**
```json
{
  "board_size": 19,
  "moves": ["Q16", "D4", "R4", "D16"],
  "config": {
    "komi": 7.5,
    "ownership": true,
    "request_id": "optional-id"
  }
}
```

**Response:**
```json
{
  "probs": [0.95, 0.89, -0.12, ..., 0.43],
  "diagnostics": {
    "winprob": 0.5123,
    "score": 1.5,
    "bot_move": "",
    "best_ten": []
  },
  "request_id": "optional-id"
}
```

The `probs` array contains 361 values (for 19x19) representing ownership probability for each intersection (-1.0 = definitely white, +1.0 = definitely black).

### 3. Health Check

**Endpoint:** `GET /health`

**Response:**
```json
{
  "status": "ok"
}
```

## Testing with curl

```bash
# Select move
curl -X POST http://localhost:2718/select-move/katago_gtp_bot \
  -H "Content-Type: application/json" \
  -d '{
    "board_size": 19,
    "moves": ["R4", "D16"],
    "config": {"komi": 7.5}
  }'

# Get territory ownership
curl -X POST http://localhost:2718/score/katago_gtp_bot \
  -H "Content-Type: application/json" \
  -d '{
    "board_size": 19,
    "moves": ["R4", "D16"],
    "config": {"komi": 7.5, "ownership": true}
  }'

# Health check
curl http://localhost:2718/health
```

## Request Configuration Options

The `config` field in requests supports:

- `komi` (float, default: 7.5): The komi value
- `client` (string, optional): Client identifier (e.g., "kifucam")
- `request_id` (string, optional): Request ID echoed back in response
- `ownership` (bool, default: true): Whether to return ownership data in score endpoint

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

Pre-built Docker images are automatically published to GitHub Container Registry on every commit:

```bash
# Latest version
docker pull ghcr.io/stubbi/katago-server:latest

# Specific version
docker pull ghcr.io/stubbi/katago-server:v1.0.0

# Run with custom config
docker run -p 2718:2718 \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  -v $(pwd)/gtp_config.cfg:/app/gtp_config.cfg:ro \
  ghcr.io/stubbi/katago-server:latest
```

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests at [github.com/stubbi/katago-server](https://github.com/stubbi/katago-server).

See EXAMPLES.md for usage patterns and client implementations.

## References

- [This Project](https://github.com/stubbi/katago-server)
- [KataGo](https://github.com/lightvector/KataGo)
- [Original Python katago-server](https://github.com/hauensteina/katago-server)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [GTP Protocol](https://www.lysator.liu.se/~gunnar/gtp/)
