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

# Test 3: Score position with ownership
echo -e "\n3. Score Position (with ownership):"
SCORE_RESPONSE=$(curl -s -X POST "http://$HOST/score/katago_gtp_bot" \
  -H "Content-Type: application/json" \
  -d '{
    "board_size": 19,
    "moves": ["R4", "D16"],
    "config": {"komi": 7.5, "ownership": true}
  }')

echo "$SCORE_RESPONSE" | jq '.'

# Validate the response
echo -e "\n3a. Validating ownership data:"

# Check probs array length (should be 361 for 19x19 board)
PROBS_COUNT=$(echo "$SCORE_RESPONSE" | jq '.probs | length')
echo -n "  - Ownership values count: $PROBS_COUNT (expected: 361) ... "
if [ "$PROBS_COUNT" -eq 361 ]; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Expected 361 ownership values for 19x19 board, got $PROBS_COUNT"
  exit 1
fi

# Check diagnostics are not default values
WINPROB=$(echo "$SCORE_RESPONSE" | jq '.diagnostics.winprob')
SCORE=$(echo "$SCORE_RESPONSE" | jq '.diagnostics.score')
BOT_MOVE=$(echo "$SCORE_RESPONSE" | jq -r '.diagnostics.bot_move')

echo -n "  - Winprob is not default (-1.0): $WINPROB ... "
if [ "$WINPROB" != "-1.0" ] && [ "$WINPROB" != "null" ]; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Winprob should not be -1.0 (default value)"
  exit 1
fi

echo -n "  - Score is not default (0.0): $SCORE ... "
# Use awk for float comparison since bash doesn't support it
if awk "BEGIN {exit !($SCORE != 0.0)}"; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Score should not be 0.0 (default value)"
  exit 1
fi

echo -n "  - Bot move is not empty: '$BOT_MOVE' ... "
if [ -n "$BOT_MOVE" ] && [ "$BOT_MOVE" != "null" ]; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Bot move should not be empty"
  exit 1
fi

echo -e "\n================================"
echo "All tests PASSED! ✓"
