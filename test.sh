#!/bin/bash

# Quick test script for the KataGo server
# Usage: ./test.sh [host:port]

HOST="${1:-localhost:2718}"

echo "Testing KataGo Server at $HOST"
echo "================================"

# Test 0: Server startup and process health check
echo -e "\n0. Checking server startup:"
echo -n "Waiting for server to be ready..."
TIMEOUT=15
ELAPSED=0
while ! curl -s "http://$HOST/health" > /dev/null 2>&1; do
  if [ $ELAPSED -ge $TIMEOUT ]; then
    echo " FAILED"
    echo "ERROR: Server failed to start within $TIMEOUT seconds"
    echo "This likely means the KataGo process crashed on startup."
    echo "Check server logs for 'KataGo stdout closed' or stderr errors."
    exit 1
  fi
  sleep 1
  ELAPSED=$((ELAPSED + 1))
  echo -n "."
done
echo " OK"

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
