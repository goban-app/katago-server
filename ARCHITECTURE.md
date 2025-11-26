# Project Structure

```
katago-server/
├── src/
│   ├── main.rs           # Application entry point and server setup
│   ├── api.rs            # REST API endpoints and request/response types
│   ├── katago_bot.rs     # KataGo process management and GTP protocol
│   ├── config.rs         # Configuration structures and loading
│   └── error.rs          # Error types and handling
├── Cargo.toml            # Rust dependencies and project metadata
├── config.toml.example   # Example server configuration
├── gtp_config.cfg.example # Example KataGo GTP configuration
├── setup.sh              # Automated setup script
├── test.sh               # API testing script
├── Makefile              # Build and deployment tasks
├── Dockerfile            # Container image definition
├── docker-compose.yml    # Docker Compose configuration
├── katago-server.service # systemd service file
├── README.md             # Main documentation
├── EXAMPLES.md           # Usage examples in various languages
├── LICENSE               # MIT License
└── .gitignore            # Git ignore patterns
```

## Architecture Overview

### Component Responsibilities

1. **main.rs**
   - Initialize logging and tracing
   - Load configuration from file or environment
   - Create KataGo bot instance
   - Set up Axum web server with middleware (CORS, tracing)
   - Start listening on configured host/port

2. **api.rs**
   - Define request/response structures with serde
   - Implement `/select-move/katago_gtp_bot` endpoint
   - Implement `/score/katago_gtp_bot` endpoint
   - Implement `/health` endpoint
   - Handle error conversion to HTTP responses

3. **katago_bot.rs**
   - Spawn and manage KataGo process lifecycle
   - Send GTP commands via stdin
   - Read and parse GTP responses from stdout
   - Parse diagnostics (win probability, score, move candidates)
   - Handle process death and timeout scenarios
   - Maintain game state (moves, komi, rules)

4. **config.rs**
   - Define configuration structures
   - Support loading from TOML files
   - Support loading from environment variables
   - Provide sensible defaults

5. **error.rs**
   - Define custom error types using thiserror
   - Handle process, parsing, IO, and timeout errors
   - Type alias for Result<T>

### Data Flow

```
HTTP Request
    ↓
Axum Router
    ↓
API Handler (api.rs)
    ↓
KatagoBot (katago_bot.rs)
    ↓
GTP Command → KataGo Process
    ↓
GTP Response ← KataGo Process
    ↓
Response Parser
    ↓
JSON Response
    ↓
HTTP Response
```

### Concurrency Model

- **Async/await**: Uses Tokio runtime for efficient I/O
- **Channels**: mpsc unbounded channels for process communication
- **Mutex Strategy**:
  - `tokio::sync::Mutex` for async-accessed data (response_rx, last_move_color)
  - `std::sync::Mutex` for sync-only data (stdin, process handles)
  - `std::sync::RwLock` for diagnostics (written from thread, read from async)
- **Thread spawning**: Separate thread reads KataGo stdout continuously
- **LazyLock**: Static regex patterns initialized once (Rust 1.80+)

### Key Design Decisions

1. **Single KataGo Instance**: One process shared across requests
   - Simpler than pooling
   - Sufficient for moderate load
   - Can scale by running multiple server instances

2. **Smart Mutex Usage**: Different mutex types for different access patterns
   - `tokio::sync::Mutex`: For data accessed in async contexts with `.await`
   - `std::sync::Mutex`: For sync-only access (better performance)
   - `std::sync::RwLock`: For frequent reads, infrequent writes (diagnostics)
   - Eliminates `Send` trait issues across `.await` boundaries

3. **Regex Compilation**: Static lazy initialization with LazyLock
   - Compiled once at first use (Rust 1.80+ feature)
   - No runtime overhead from lazy_static crate
   - Better performance than compiling per-request

4. **Process Supervision**: Managed child process lifecycle
   - Clean shutdown on server termination
   - Timeout handling for hung processes
   - Logs errors for debugging

5. **Minimal Dependencies**: Direct TOML parsing instead of config crate
   - Fewer transitive dependencies
   - Faster compilation
   - Zero duplicate dependencies in tree

## API Contract

### POST /select-move/katago_gtp_bot

**Request:**
```json
{
  "board_size": 19,
  "moves": ["R4", "D16"],
  "config": {
    "komi": 7.5,
    "client": "optional-client-id",
    "request_id": "optional-request-id"
  }
}
```

**Response:**
```json
{
  "bot_move": "Q16",
  "diagnostics": {
    "winprob": 0.5234,
    "score": 2.5,
    "bot_move": "Q16",
    "best_ten": [
      {"move": "Q16", "psv": 842},
      {"move": "Q4", "psv": 835}
    ]
  },
  "request_id": "optional-request-id"
}
```

### POST /score/katago_gtp_bot

**Request:**
```json
{
  "board_size": 19,
  "moves": ["R4", "D16"],
  "config": {
    "komi": 7.5,
    "ownership": true,
    "request_id": "optional-request-id"
  }
}
```

**Response:**
```json
{
  "probs": [0.95, 0.89, -0.12, ...],
  "diagnostics": {
    "winprob": 0.5123,
    "score": 1.5,
    "bot_move": "",
    "best_ten": []
  },
  "request_id": "optional-request-id"
}
```

## Performance Characteristics

### Benchmarks (Approximate)

| Metric | Value |
|--------|-------|
| Request throughput | ~5000 req/s (empty queue) |
| Memory footprint | ~10 MB (server only) |
| Startup time | ~100 ms |
| Move latency | Depends on KataGo config |

### Scaling Strategies

1. **Vertical Scaling**
   - More CPU cores → increase numSearchThreads
   - More RAM → increase maxVisits
   - Better GPU → faster NN evaluation

2. **Horizontal Scaling**
   - Run multiple server instances
   - Use nginx load balancer (least_conn)
   - Each instance has dedicated KataGo process

3. **Optimization**
   - Reduce maxVisits for faster (weaker) responses
   - Use smaller neural networks
   - Adjust search parameters in gtp_config.cfg

## Security Considerations

1. **Input Validation**
   - Validate board_size (9, 13, 19)
   - Validate move format (regex)
   - Limit moves array length

2. **Resource Limits**
   - Configure move_timeout_secs
   - Set maxVisits in KataGo config
   - Monitor memory usage

3. **Network Security**
   - Use reverse proxy (nginx)
   - Enable HTTPS (Let's Encrypt)
   - Implement rate limiting
   - Set up firewall rules

4. **Process Isolation**
   - Run as non-root user
   - Use systemd security features
   - Consider containerization

## Monitoring and Observability

### Logging

- Structured logging with tracing crate
- Levels: error, warn, info, debug, trace
- Configure via RUST_LOG environment variable

### Metrics to Track

- Request rate (requests/second)
- Response latency (p50, p95, p99)
- Error rate
- KataGo process restarts
- Memory usage
- CPU usage

### Health Check

- GET /health endpoint
- Returns {"status": "ok"}
- Use for load balancer health checks

## Maintenance

### Updating KataGo

1. Download new binary and model
2. Update config.toml paths
3. Restart service: `sudo systemctl restart katago-server`

### Viewing Logs

```bash
# Systemd logs
journalctl -u katago-server -f

# Docker logs
docker logs -f katago-server
```

### Backup

Important files to backup:
- config.toml (server config)
- gtp_config.cfg (KataGo config)
- Neural network models (large files)

## Troubleshooting

### Server Won't Start

- Check config.toml paths are correct
- Verify KataGo binary has execute permissions
- Check ports aren't already in use
- Review logs: `journalctl -u katago-server`

### Slow Responses

- Reduce maxVisits in gtp_config.cfg
- Use smaller neural network
- Check CPU/GPU usage
- Increase numSearchThreads if CPU underutilized

### High Memory Usage

- Reduce nnMaxBatchSize
- Use smaller neural network
- Limit maxVisits
- Check for memory leaks (unlikely in Rust)

## Contributing

See EXAMPLES.md for usage patterns and client implementations.

## References

- [KataGo](https://github.com/lightvector/KataGo)
- [GTP Protocol](https://www.lysator.liu.se/~gunnar/gtp/)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [Tokio Async Runtime](https://tokio.rs/)
