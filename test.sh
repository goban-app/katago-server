#!/bin/bash

# Quick test script for the KataGo server V1 API
# Usage: ./test.sh [host:port]

HOST="${1:-localhost:2718}"

echo "Testing KataGo Server V1 API at $HOST"
echo "========================================"

# Test 0: Server startup and process health check
echo -e "\n0. Checking server startup:"
echo -n "Waiting for server to be ready..."
TIMEOUT=15
ELAPSED=0
while ! curl -s "http://$HOST/api/v1/health" > /dev/null 2>&1; do
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
curl -s "http://$HOST/api/v1/health" | jq '.'

# Test 2: Version info
echo -e "\n2. Version Information:"
curl -s "http://$HOST/api/v1/version" | jq '.'

# Test 3: Analysis endpoint (basic)
echo -e "\n3. Position Analysis (Basic):"
ANALYSIS_RESPONSE=$(curl -s -X POST "http://$HOST/api/v1/analysis" \
  -H "Content-Type: application/json" \
  -d '{
    "moves": ["D4", "Q16"],
    "komi": 7.5,
    "rules": "chinese"
  }')

echo "$ANALYSIS_RESPONSE" | jq '.'

# Test 4: Analysis with ownership
echo -e "\n4. Position Analysis (with Ownership):"
OWNERSHIP_RESPONSE=$(curl -s -X POST "http://$HOST/api/v1/analysis" \
  -H "Content-Type: application/json" \
  -d '{
    "moves": ["D4", "Q16", "R4"],
    "komi": 7.5,
    "rules": "chinese",
    "includeOwnership": true
  }')

echo "$OWNERSHIP_RESPONSE" | jq '.'

# Validate the response
echo -e "\n4a. Validating analysis response:"

# Check moveInfos exists
HAS_MOVE_INFOS=$(echo "$OWNERSHIP_RESPONSE" | jq 'has("moveInfos")')
echo -n "  - Has moveInfos: $HAS_MOVE_INFOS ... "
if [ "$HAS_MOVE_INFOS" == "true" ]; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Response should contain moveInfos"
  exit 1
fi

# Check rootInfo exists
HAS_ROOT_INFO=$(echo "$OWNERSHIP_RESPONSE" | jq 'has("rootInfo")')
echo -n "  - Has rootInfo: $HAS_ROOT_INFO ... "
if [ "$HAS_ROOT_INFO" == "true" ]; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Response should contain rootInfo"
  exit 1
fi

# Check ownership array exists and has correct length
if [ "$HAS_MOVE_INFOS" == "true" ]; then
  HAS_OWNERSHIP=$(echo "$OWNERSHIP_RESPONSE" | jq 'has("ownership")')
  echo -n "  - Has ownership data: $HAS_OWNERSHIP ... "
  if [ "$HAS_OWNERSHIP" == "true" ]; then
    echo "✓ PASS"

    OWNERSHIP_COUNT=$(echo "$OWNERSHIP_RESPONSE" | jq '.ownership | length')
    echo -n "  - Ownership values count: $OWNERSHIP_COUNT (expected: 361) ... "
    if [ "$OWNERSHIP_COUNT" -eq 361 ]; then
      echo "✓ PASS"
    else
      echo "✗ FAIL"
      echo "ERROR: Expected 361 ownership values for 19x19 board, got $OWNERSHIP_COUNT"
      exit 1
    fi
  else
    echo "✗ FAIL"
    echo "ERROR: Response should contain ownership data when includeOwnership is true"
    exit 1
  fi
fi

# Check rootInfo values
WINRATE=$(echo "$OWNERSHIP_RESPONSE" | jq '.rootInfo.winrate')
SCORE_LEAD=$(echo "$OWNERSHIP_RESPONSE" | jq '.rootInfo.scoreLead')

echo -n "  - Winrate is valid: $WINRATE ... "
if [ "$WINRATE" != "null" ] && awk "BEGIN {exit !($WINRATE >= 0.0 && $WINRATE <= 1.0)}"; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Winrate should be between 0.0 and 1.0"
  exit 1
fi

echo -n "  - Score lead is not null: $SCORE_LEAD ... "
if [ "$SCORE_LEAD" != "null" ]; then
  echo "✓ PASS"
else
  echo "✗ FAIL"
  echo "ERROR: Score lead should not be null"
  exit 1
fi

# Test 5: Error handling (invalid request)
echo -e "\n5. Error Handling (Invalid Request):"
ERROR_RESPONSE=$(curl -s -X POST "http://$HOST/api/v1/analysis" \
  -H "Content-Type: application/json" \
  -d '{
    "invalid": "request"
  }')

echo "$ERROR_RESPONSE" | jq '.'

# Validate error response format (RFC 7807)
echo -e "\n5a. Validating error response format:"
HAS_TYPE=$(echo "$ERROR_RESPONSE" | jq 'has("type")')
HAS_TITLE=$(echo "$ERROR_RESPONSE" | jq 'has("title")')
HAS_STATUS=$(echo "$ERROR_RESPONSE" | jq 'has("status")')

echo -n "  - Has 'type' field (RFC 7807): $HAS_TYPE ... "
if [ "$HAS_TYPE" == "true" ]; then
  echo "✓ PASS"
else
  echo "⚠ WARNING (expected RFC 7807 format)"
fi

echo -n "  - Has 'title' field (RFC 7807): $HAS_TITLE ... "
if [ "$HAS_TITLE" == "true" ]; then
  echo "✓ PASS"
else
  echo "⚠ WARNING (expected RFC 7807 format)"
fi

echo -n "  - Has 'status' field (RFC 7807): $HAS_STATUS ... "
if [ "$HAS_STATUS" == "true" ]; then
  echo "✓ PASS"
else
  echo "⚠ WARNING (expected RFC 7807 format)"
fi

echo -e "\n========================================"
echo "All core tests PASSED! ✓"
echo ""
echo "Note: The error format warnings are non-critical but indicate"
echo "      that error responses should follow RFC 7807 format."
