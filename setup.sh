#!/bin/bash

# Setup script for KataGo Server
# This script helps download and configure KataGo

set -e

echo "KataGo Server Setup"
echo "==================="

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
    ARCH="x64"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="osx"
    ARCH="x64"
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

KATAGO_VERSION="v1.14.1"
KATAGO_URL="https://github.com/lightvector/KataGo/releases/download/${KATAGO_VERSION}/katago-${KATAGO_VERSION}-${OS}-${ARCH}.zip"

echo -e "\n1. Downloading KataGo ${KATAGO_VERSION} for ${OS}..."
if [ ! -f "katago" ]; then
    wget -q --show-progress "$KATAGO_URL" -O katago.zip
    unzip -q katago.zip
    chmod +x katago
    rm katago.zip
    echo "✓ KataGo binary downloaded"
else
    echo "✓ KataGo binary already exists"
fi

echo -e "\n2. Downloading neural network model..."
MODEL_NAME="kata1-b18c384nbt-s9131461376-d4087399203.bin.gz"
MODEL_URL="https://github.com/lightvector/KataGo/releases/download/${KATAGO_VERSION}/${MODEL_NAME}"

if [ ! -f "$MODEL_NAME" ]; then
    wget -q --show-progress "$MODEL_URL"
    echo "✓ Model downloaded: $MODEL_NAME"
else
    echo "✓ Model already exists: $MODEL_NAME"
fi

echo -e "\n3. Creating configuration files..."
if [ ! -f "config.toml" ]; then
    cp config.toml.example config.toml
    # Update model path in config
    sed -i.bak "s|kata1-b18c384nbt-s9131461376-d4087399203.bin.gz|$MODEL_NAME|g" config.toml
    rm config.toml.bak 2>/dev/null || true
    echo "✓ Created config.toml"
else
    echo "✓ config.toml already exists"
fi

if [ ! -f "gtp_config.cfg" ]; then
    cp gtp_config.cfg.example gtp_config.cfg
    echo "✓ Created gtp_config.cfg"
else
    echo "✓ gtp_config.cfg already exists"
fi

echo -e "\n4. Building the server..."
cargo build --release
echo "✓ Server built successfully"

echo -e "\n================================"
echo "Setup complete!"
echo ""
echo "To start the server:"
echo "  ./target/release/katago-server"
echo ""
echo "To test the server:"
echo "  ./test.sh"
echo ""
echo "See README.md for more information."
