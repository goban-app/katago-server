# KataGo Server API Review & Improvements

## Executive Summary

This document outlines the comprehensive review of the KataGo Server API and proposes improvements to:
1. Expose all reasonable KataGo capabilities
2. Follow REST API best practices
3. Maintain backward compatibility where possible

## Current State Analysis

### Existing Endpoints
1. `POST /select-move/katago_gtp_bot` - Get best move with basic diagnostics
2. `POST /score/katago_gtp_bot` - Get territory ownership probabilities
3. `GET /health` - Health check

### Current Limitations

#### Missing KataGo Features
- **No JSON Analysis Engine support** - Currently only uses GTP protocol
- **Limited analysis data** - Missing:
  - Policy network raw probabilities
  - Principal Variations (PV) with visit counts
  - Per-move statistics (visits, utility, LCB, prior)
  - Score distributions and standard deviations
  - Root position statistics
- **No per-request configuration** - Cannot override:
  - maxVisits (search depth)
  - rootPolicyTemperature (exploration)
  - analysisPVLen (variation depth)
- **No move filtering** - Cannot specify avoidMoves or allowMoves
- **No human SL model support** - Cannot request human-style predictions
- **No batch analysis** - Cannot analyze multiple positions in one request
- **No cache management** - Cannot clear neural net cache via API
- **Rules auto-determined** - Cannot explicitly specify rule set

#### REST API Best Practices Issues
- **No API versioning** - No `/api/v1/` prefix for future compatibility
- **Non-RESTful naming** - `/select-move/katago_gtp_bot` is not semantic or resource-oriented
- **Limited HTTP methods** - Only POST and GET, no DELETE, PUT, PATCH
- **Basic error responses** - Simple JSON errors, not RFC 7807 Problem Details
- **Inconsistent naming** - `bot_move` vs `best_ten`, `winprob` vs `score`
- **No metadata** - Missing version, timestamp, etc. in responses
- **Limited CORS configuration** - Wide open, could be more controlled

## Implemented Improvements

### New API Structure (Clean Slate)

Since the API was not yet in production, all legacy endpoints have been removed for a cleaner implementation:

```
GET    /api/v1/health           - Health check with detailed status
GET    /api/v1/version          - Server and KataGo version information
POST   /api/v1/analysis         - Comprehensive position analysis
POST   /api/v1/cache/clear      - Clear neural network cache
```

### Enhanced Analysis Endpoint

**POST /api/v1/analysis**

Comprehensive endpoint supporting all KataGo features in one unified interface.

#### Request Format
```json
{
  "moves": ["D4", "Q16", "R4"],
  "rules": "chinese",
  "komi": 7.5,
  "boardXSize": 19,
  "boardYSize": 19,
  "initialStones": [],
  "initialPlayer": "B",
  "analyzeTurns": [3],

  "maxVisits": 100,
  "rootPolicyTemperature": 1.0,
  "rootFpuReductionMax": 0.2,
  "analysisPVLen": 15,

  "includeOwnership": true,
  "includeOwnershipStdev": false,
  "includeMovesOwnership": false,
  "includePolicy": true,
  "includePVVisits": true,

  "avoidMoves": [
    {"player": "B", "moves": ["C3", "Q4"], "untilDepth": 5}
  ],
  "allowMoves": [
    {"player": "W", "moves": ["D4", "Q16"], "untilDepth": 10}
  ],

  "overrideSettings": {
    "playoutDoublingAdvantage": 0.0,
    "wideRootNoise": 0.0,
    "cpuctExploration": 1.5,
    "humanSLProfile": "rank_5d"
  },

  "reportDuringSearchEvery": 0,
  "priority": 0,
  "requestId": "unique-request-id"
}
```

#### Response Format
```json
{
  "id": "unique-request-id",
  "turnNumber": 3,
  "moveInfos": [
    {
      "move": "D16",
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
      "pv": ["D16", "Q4", "D10", "R9"],
      "pvVisits": [142, 95, 82, 71],
      "ownership": [0.95, 0.89, ...],
      "ownershipStdev": [0.05, 0.08, ...]
    }
  ],
  "rootInfo": {
    "winrate": 0.512,
    "scoreLead": 1.5,
    "utility": 0.015,
    "visits": 500,
    "currentPlayer": "B",
    "rawWinrate": 0.508,
    "rawScoreMean": 1.2,
    "rawStScoreError": 8.5
  },
  "ownership": [0.85, 0.92, -0.15, ...],
  "ownershipStdev": [0.05, 0.04, 0.12, ...],
  "policy": [0.001, 0.002, 0.18, ...]
}
```

### Error Handling (RFC 7807)

All errors follow RFC 7807 Problem Details for HTTP APIs:

```json
{
  "type": "https://katago-server/problems/timeout",
  "title": "Analysis Timeout",
  "status": 504,
  "detail": "KataGo analysis timed out after 20 seconds",
  "instance": "/api/v1/analysis",
  "requestId": "unique-request-id"
}
```

Error types:
- `invalid-request` - Malformed request (400)
- `timeout` - Analysis timeout (504)
- `process-died` - KataGo process crashed (503)
- `parse-error` - Failed to parse KataGo response (500)
- `internal-error` - Unexpected server error (500)

### Version Endpoint

**GET /api/v1/version**

Returns server and KataGo version information:

```json
{
  "server": {
    "name": "katago-server",
    "version": "1.0.0",
    "buildDate": "2025-11-27"
  },
  "katago": {
    "version": "1.15.3",
    "gitHash": "abc123def456"
  }
}
```

### Cache Management

**POST /api/v1/cache/clear**

Clears the KataGo neural network cache to free memory:

```json
{
  "status": "cleared",
  "timestamp": "2025-11-27T12:34:56Z"
}
```

### Improved Health Endpoint

**GET /api/v1/health**

Enhanced health check with more details:

```json
{
  "status": "healthy",
  "timestamp": "2025-11-27T12:34:56Z",
  "uptime": 3600,
  "katago": {
    "running": true,
    "responsive": true
  }
}
```

## Implementation Status

All phases completed:

1. ✅ Create comprehensive request/response types
2. ✅ Implement `/api/v1/analysis` with full KataGo features
3. ✅ Add `/api/v1/version` endpoint
4. ✅ Add `/api/v1/cache/clear` endpoint
5. ✅ Improve `/api/v1/health` endpoint
6. ✅ Implement RFC 7807 error handling
7. ✅ Remove legacy endpoints (not in production yet)
8. ✅ Add comprehensive tests
9. 🔄 Update README with new API documentation (in progress)
10. 🔄 Update EXAMPLES with new endpoint usage (in progress)

## Naming Conventions

All JSON fields use camelCase to match KataGo's JSON analysis engine format:
- `moveInfos` (not `move_infos`)
- `scoreMean` (not `score_mean`)
- `rootInfo` (not `root_info`)

This provides consistency with KataGo's own API and is more idiomatic for JSON APIs.

## Benefits

1. **Complete Feature Parity** - Exposes all KataGo capabilities
2. **Future-Proof** - Versioned API allows breaking changes in v2
3. **Standards Compliant** - Follows REST and RFC 7807 best practices
4. **Better Developer Experience** - Comprehensive documentation and examples
5. **More Flexible** - Per-request configuration for all parameters
6. **Better Errors** - Structured error responses with actionable information
7. **Clean Implementation** - No legacy code to maintain

## Example Usage

### Analyzing a Position

```bash
curl -X POST http://localhost:2718/api/v1/analysis \
  -H "Content-Type: application/json" \
  -d '{
    "moves": ["D4", "Q16"],
    "komi": 7.5,
    "rules": "chinese",
    "includeOwnership": true,
    "maxVisits": 100
  }'
```

The endpoint returns comprehensive analysis including:
- All candidate moves with winrate, score lead, visits, and prior probability
- Position-level statistics (root info)
- Territory ownership predictions
- Move sequences (principal variations)

### Getting Server Version

```bash
curl http://localhost:2718/api/v1/version
```

### Clearing the Cache

```bash
curl -X POST http://localhost:2718/api/v1/cache/clear
```
