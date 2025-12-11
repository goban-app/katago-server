#!/bin/bash
set -e

# Download KataGo model if KATAGO_MODEL is set
if [ -n "$KATAGO_MODEL" ]; then
  echo "Downloading KataGo model: $KATAGO_MODEL"
  wget -q -O "$KATAGO_MODEL" "https://media.katagotraining.org/uploaded/networks/models/kata1/$KATAGO_MODEL"
fi

# Download Human SL model if KATAGO_HUMAN_MODEL is set
if [ -n "$KATAGO_HUMAN_MODEL" ]; then
  echo "Downloading KataGo Human SL model: $KATAGO_HUMAN_MODEL"
  wget -q -O "$KATAGO_HUMAN_MODEL" "https://media.katagotraining.org/uploaded/networks/models/kata1/$KATAGO_HUMAN_MODEL"
fi

# Make katago executable if it exists
if [ -f "katago" ]; then
  chmod +x katago
fi

# Create config.toml from template and set model path
if [ -f "config.toml.example" ]; then
  cp config.toml.example config.toml
  if [ -n "$KATAGO_MODEL" ]; then
    sed -i "s|model_path = \".*\"|model_path = \"./$KATAGO_MODEL\"|" config.toml
  fi
fi

echo "Setup complete"
