.PHONY: build run test clean setup install help docker docker-run

# Default target
help:
	@echo "KataGo Server - Available targets:"
	@echo "  make setup        - Download KataGo and models, create configs"
	@echo "  make build        - Build the server in release mode"
	@echo "  make run          - Run the server"
	@echo "  make dev          - Run the server in development mode with logging"
	@echo "  make test         - Run unit tests"
	@echo "  make test-api     - Test the API endpoints (requires server running)"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make install      - Install as systemd service"
	@echo "  make docker       - Build Docker image"
	@echo "  make docker-run   - Run with Docker Compose"
	@echo "  make fmt          - Format code"
	@echo "  make clippy       - Run clippy linter"

setup:
	@echo "Running setup script..."
	./setup.sh

build:
	cargo build --release

run: build
	./target/release/katago-server

dev:
	RUST_LOG=debug cargo run

test:
	cargo test

test-api:
	@echo "Testing API endpoints..."
	./test.sh

clean:
	cargo clean
	rm -f katago katago_* *.bin.gz

install: build
	@echo "Installing systemd service..."
	@echo "Copying binary to /usr/local/bin..."
	sudo cp target/release/katago-server /usr/local/bin/
	@echo "Installing service file..."
	sudo cp katago-server.service /etc/systemd/system/
	@echo "Reloading systemd..."
	sudo systemctl daemon-reload
	@echo "Installation complete. Enable with: sudo systemctl enable katago-server"

docker:
	docker build -t katago-server:latest .

docker-run:
	docker-compose up -d

docker-stop:
	docker-compose down

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

watch:
	cargo watch -x run
