#!/bin/bash

# Quick test script for the KataGo server
# Usage: ./test.sh [host:port]

HOST="${1:-localhost:2718}"

echo "Testing KataGo Server at $HOST"
echo "================================"

# Test 1: Health check
echo -e "\n1. Health Check:"
curl -s "http://$HOST/health" | jq '.'

# Test 2: Select move
echo -e "\n2. Select Move (Opening):"
curl -s -X POST "http://$HOST/select-move/katago_gtp_bot" \
  -H "Content-Type: application/json" \
  -d '{
    "board_size": 19,
    "moves": ["R4", "D16"],
    "config": {"komi": 7.5}
  }' | jq '.'

# Test 3: Score position
echo -e "\n3. Score Position:"
curl -s -X POST "http://$HOST/score/katago_gtp_bot" \
  -H "Content-Type: application/json" \
  -d '{
    "board_size": 19,
    "moves": ["R4", "D16"],
    "config": {"komi": 7.5, "ownership": true}
  }' | jq '.diagnostics'

echo -e "\n================================"
echo "Tests completed!"
