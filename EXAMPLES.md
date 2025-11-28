# Examples

This directory contains example code and usage patterns for the KataGo Server REST API.

## Basic Client (Python)

```python
import requests
import json

class KataGoClient:
    def __init__(self, base_url="http://localhost:2718"):
        self.base_url = base_url

    def analyze_position(self, moves, komi=7.5, board_x_size=19, board_y_size=19,
                        include_ownership=False, max_visits=None):
        """Analyze a Go position with comprehensive information."""
        url = f"{self.base_url}/api/v1/analysis"
        data = {
            "moves": moves,
            "komi": komi,
            "rules": "chinese",
            "boardXSize": board_x_size,
            "boardYSize": board_y_size,
            "includeOwnership": include_ownership
        }
        if max_visits:
            data["maxVisits"] = max_visits

        response = requests.post(url, json=data)
        response.raise_for_status()
        return response.json()

    def get_version(self):
        """Get server and KataGo version information."""
        url = f"{self.base_url}/api/v1/version"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()

    def health_check(self):
        """Check server health."""
        url = f"{self.base_url}/api/v1/health"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()

    def clear_cache(self):
        """Clear the neural network cache."""
        url = f"{self.base_url}/api/v1/cache/clear"
        response = requests.post(url)
        response.raise_for_status()
        return response.json()

# Usage
client = KataGoClient()

# Check health
health = client.health_check()
print(f"Server status: {health['status']}")

# Get version info
version = client.get_version()
print(f"Server version: {version['server']['version']}")
print(f"KataGo version: {version['katago']['version']}")

# Analyze a position
moves = ["Q16", "D4", "R4"]
result = client.analyze_position(moves, include_ownership=True, max_visits=100)

# Get best move
best_move = result['moveInfos'][0]
print(f"Best move: {best_move['moveCoord']}")
print(f"Win probability: {best_move['winrate']:.2%}")
print(f"Score lead: {best_move['scoreLead']:.1f}")
print(f"Visits: {best_move['visits']}")

# Get position evaluation
if 'rootInfo' in result:
    root = result['rootInfo']
    print(f"\nPosition evaluation:")
    print(f"  Winrate: {root['winrate']:.2%}")
    print(f"  Score lead: {root['scoreLead']:.1f}")
    print(f"  Current player: {root['currentPlayer']}")

# Get territory ownership
if 'ownership' in result:
    print(f"\nTerritory ownership available: {len(result['ownership'])} points")
```

## Basic Client (JavaScript)

```javascript
class KataGoClient {
    constructor(baseUrl = 'http://localhost:2718') {
        this.baseUrl = baseUrl;
    }

    async analyzePosition(moves, komi = 7.5, boardXSize = 19, boardYSize = 19,
                          includeOwnership = false, maxVisits = null) {
        const data = {
            moves,
            komi,
            rules: 'chinese',
            boardXSize,
            boardYSize,
            includeOwnership
        };

        if (maxVisits) {
            data.maxVisits = maxVisits;
        }

        const response = await fetch(`${this.baseUrl}/api/v1/analysis`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        return await response.json();
    }

    async getVersion() {
        const response = await fetch(`${this.baseUrl}/api/v1/version`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        return await response.json();
    }

    async healthCheck() {
        const response = await fetch(`${this.baseUrl}/api/v1/health`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        return await response.json();
    }

    async clearCache() {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/clear`, {
            method: 'POST'
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        return await response.json();
    }
}

// Usage
const client = new KataGoClient();

(async () => {
    try {
        // Check health
        const health = await client.healthCheck();
        console.log(`Server status: ${health.status}`);

        // Get version
        const version = await client.getVersion();
        console.log(`Server version: ${version.server.version}`);

        // Analyze position
        const moves = ['Q16', 'D4', 'R4'];
        const result = await client.analyzePosition(moves, 7.5, 19, 19, true, 100);

        // Best move
        const bestMove = result.moveInfos[0];
        console.log(`Best move: ${bestMove.moveCoord}`);
        console.log(`Win probability: ${(bestMove.winrate * 100).toFixed(2)}%`);
        console.log(`Score lead: ${bestMove.scoreLead.toFixed(1)}`);

        // Position evaluation
        if (result.rootInfo) {
            console.log(`\nPosition evaluation:`);
            console.log(`  Winrate: ${(result.rootInfo.winrate * 100).toFixed(2)}%`);
            console.log(`  Score lead: ${result.rootInfo.scoreLead.toFixed(1)}`);
        }
    } catch (error) {
        console.error('Error:', error.message);
    }
})();
```

## Basic Client (Rust)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct AnalysisRequest {
    moves: Vec<String>,
    komi: f32,
    rules: String,
    #[serde(rename = "boardXSize")]
    board_x_size: u8,
    #[serde(rename = "boardYSize")]
    board_y_size: u8,
    #[serde(rename = "includeOwnership")]
    include_ownership: bool,
    #[serde(rename = "maxVisits", skip_serializing_if = "Option::is_none")]
    max_visits: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnalysisResponse {
    #[serde(rename = "moveInfos")]
    move_infos: Vec<MoveInfo>,
    #[serde(rename = "rootInfo")]
    root_info: Option<RootInfo>,
    ownership: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct MoveInfo {
    #[serde(rename = "moveCoord")]
    move_coord: String,
    visits: u32,
    winrate: f32,
    #[serde(rename = "scoreLead")]
    score_lead: f32,
    prior: f32,
}

#[derive(Debug, Deserialize)]
struct RootInfo {
    winrate: f32,
    #[serde(rename = "scoreLead")]
    score_lead: f32,
    #[serde(rename = "currentPlayer")]
    current_player: String,
}

#[derive(Debug, Deserialize)]
struct VersionResponse {
    server: ServerInfo,
    katago: KataGoInfo,
}

#[derive(Debug, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct KataGoInfo {
    version: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:2718";

    // Get version
    let version: VersionResponse = client
        .get(format!("{}/api/v1/version", base_url))
        .send()
        .await?
        .json()
        .await?;

    println!("Server: {} {}", version.server.name, version.server.version);
    println!("KataGo: {}", version.katago.version);

    // Analyze position
    let request = AnalysisRequest {
        moves: vec!["Q16".to_string(), "D4".to_string(), "R4".to_string()],
        komi: 7.5,
        rules: "chinese".to_string(),
        board_x_size: 19,
        board_y_size: 19,
        include_ownership: true,
        max_visits: Some(100),
    };

    let response: AnalysisResponse = client
        .post(format!("{}/api/v1/analysis", base_url))
        .json(&request)
        .send()
        .await?
        .json()
        .await?;

    // Print best move
    if let Some(best_move) = response.move_infos.first() {
        println!("\nBest move: {}", best_move.move_coord);
        println!("Win probability: {:.2}%", best_move.winrate * 100.0);
        println!("Score lead: {:.1}", best_move.score_lead);
        println!("Visits: {}", best_move.visits);
    }

    // Print position evaluation
    if let Some(root) = response.root_info {
        println!("\nPosition evaluation:");
        println!("  Winrate: {:.2}%", root.winrate * 100.0);
        println!("  Score lead: {:.1}", root.score_lead);
        println!("  Current player: {}", root.current_player);
    }

    Ok(())
}
```

## SGF Integration Example

```python
import re
import requests

class KataGoClient:
    def __init__(self, base_url="http://localhost:2718"):
        self.base_url = base_url

    def analyze_position(self, moves, max_visits=100):
        url = f"{self.base_url}/api/v1/analysis"
        data = {
            "moves": moves,
            "komi": 7.5,
            "rules": "chinese",
            "boardXSize": 19,
            "boardYSize": 19,
            "maxVisits": max_visits
        }
        response = requests.post(url, json=data)
        response.raise_for_status()
        return response.json()

def sgf_to_moves(sgf_content):
    """Extract moves from SGF format."""
    moves = []
    pattern = r';[BW]\[([a-s]{2})\]'

    for match in re.finditer(pattern, sgf_content):
        coord = match.group(1)
        # Convert SGF coordinates to GTP format
        col = 'ABCDEFGHJKLMNOPQRST'[ord(coord[0]) - ord('a')]
        row = str(19 - (ord(coord[1]) - ord('a')))
        moves.append(f"{col}{row}")

    return moves

# Load SGF and analyze
with open('game.sgf', 'r') as f:
    sgf = f.read()

moves = sgf_to_moves(sgf)
client = KataGoClient()

# Analyze each position
for i in range(len(moves)):
    position = moves[:i+1]
    result = client.analyze_position(position)

    actual_move = moves[i]
    best_move = result['moveInfos'][0]
    suggested_move = best_move['moveCoord']
    winrate = best_move['winrate']
    score_lead = best_move['scoreLead']

    # Check if actual move matches suggestion
    match = "✓" if actual_move == suggested_move else "✗"

    print(f"Move {i+1}: {match} Played {actual_move}, "
          f"KataGo suggests {suggested_move} "
          f"(WR: {winrate:.1%}, Score: {score_lead:+.1f})")
```

## Load Balancing Example (nginx)

```nginx
upstream katago_backend {
    least_conn;
    server 127.0.0.1:2718;
    server 127.0.0.1:2719;
    server 127.0.0.1:2720;
}

server {
    listen 80;
    server_name katago.example.com;

    location /api/ {
        proxy_pass http://katago_backend;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_connect_timeout 30s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;

        # Enable CORS if needed
        add_header Access-Control-Allow-Origin * always;
        add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always;
        add_header Access-Control-Allow-Headers "Content-Type" always;
    }
}
```

## Batch Processing Example

```python
import concurrent.futures
import requests

class KataGoClient:
    def __init__(self, base_url="http://localhost:2718"):
        self.base_url = base_url

    def analyze_position(self, moves, max_visits=100):
        url = f"{self.base_url}/api/v1/analysis"
        data = {
            "moves": moves,
            "komi": 7.5,
            "rules": "chinese",
            "boardXSize": 19,
            "boardYSize": 19,
            "maxVisits": max_visits
        }
        response = requests.post(url, json=data)
        response.raise_for_status()
        return response.json()

def analyze_single_position(client, moves):
    """Analyze a single position."""
    try:
        result = client.analyze_position(moves)
        best_move = result['moveInfos'][0]
        return {
            'moves': moves,
            'suggestion': best_move['moveCoord'],
            'winrate': best_move['winrate'],
            'score_lead': best_move['scoreLead'],
            'visits': best_move['visits']
        }
    except Exception as e:
        return {'error': str(e), 'moves': moves}

# Analyze multiple positions in parallel
client = KataGoClient()
positions = [
    ["Q16", "D4"],
    ["R4", "D16"],
    ["Q3", "D17"],
    ["Q16", "D4", "R4"],
    ["D4", "Q16", "Q4"],
    # ... more positions
]

print("Analyzing positions in parallel...")

with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
    futures = [executor.submit(analyze_single_position, client, pos)
               for pos in positions]
    results = [f.result() for f in concurrent.futures.as_completed(futures)]

# Print results
for result in results:
    if 'error' not in result:
        print(f"Position: {' '.join(result['moves'][:3])}... -> "
              f"{result['suggestion']} "
              f"(WR: {result['winrate']:.1%}, Score: {result['score_lead']:+.1f})")
    else:
        print(f"Error analyzing {result['moves']}: {result['error']}")
```

## Advanced Analysis Example

```python
import requests

def advanced_analysis(moves, komi=7.5):
    """Perform comprehensive analysis with all available data."""
    url = "http://localhost:2718/api/v1/analysis"

    data = {
        "moves": moves,
        "komi": komi,
        "rules": "chinese",
        "boardXSize": 19,
        "boardYSize": 19,
        "maxVisits": 500,
        "includeOwnership": True,
        "includePolicy": True,
        "includePVVisits": True,
        "analysisPVLen": 10,
        "requestId": f"analysis-{len(moves)}"
    }

    response = requests.post(url, json=data)
    response.raise_for_status()
    result = response.json()

    print(f"Analysis for position after {len(moves)} moves:")
    print(f"Request ID: {result.get('id', 'N/A')}")
    print()

    # Root position info
    if 'rootInfo' in result:
        root = result['rootInfo']
        print("Position Evaluation:")
        print(f"  Current player: {root['currentPlayer']}")
        print(f"  Winrate: {root['winrate']:.2%}")
        print(f"  Score lead: {root['scoreLead']:+.1f}")
        print(f"  Total visits: {root['visits']}")
        print()

    # Top move candidates
    print("Top 5 Move Candidates:")
    for i, move in enumerate(result['moveInfos'][:5], 1):
        print(f"{i}. {move['moveCoord']}")
        print(f"   Winrate: {move['winrate']:.2%}")
        print(f"   Score lead: {move['scoreLead']:+.1f}")
        print(f"   Visits: {move['visits']}")
        print(f"   Prior: {move['prior']:.3f}")

        if 'pv' in move and move['pv']:
            pv_str = ' '.join(move['pv'][:5])
            print(f"   Principal variation: {pv_str}")
        print()

    # Territory ownership
    if 'ownership' in result:
        ownership = result['ownership']
        black_territory = sum(1 for o in ownership if o > 0.7)
        white_territory = sum(1 for o in ownership if o < -0.7)
        print(f"Territory estimate:")
        print(f"  Black: ~{black_territory} points")
        print(f"  White: ~{white_territory} points")

# Example usage
moves = ["Q16", "D4", "R4", "D16", "Q3"]
advanced_analysis(moves)
```

## Error Handling Example

```python
import requests
from typing import Optional

class KataGoAPIError(Exception):
    """Custom exception for KataGo API errors."""
    def __init__(self, status: int, error_type: str, title: str, detail: str):
        self.status = status
        self.error_type = error_type
        self.title = title
        self.detail = detail
        super().__init__(f"{title}: {detail}")

def analyze_with_error_handling(moves: list[str]) -> Optional[dict]:
    """Analyze position with proper error handling."""
    url = "http://localhost:2718/api/v1/analysis"
    data = {
        "moves": moves,
        "komi": 7.5,
        "rules": "chinese",
        "maxVisits": 100
    }

    try:
        response = requests.post(url, json=data, timeout=30)

        # Check for RFC 7807 error response
        if not response.ok:
            error = response.json()
            raise KataGoAPIError(
                status=error.get('status', response.status_code),
                error_type=error.get('type', 'unknown'),
                title=error.get('title', 'API Error'),
                detail=error.get('detail', response.text)
            )

        return response.json()

    except requests.Timeout:
        print("Request timed out - KataGo analysis took too long")
        return None
    except requests.ConnectionError:
        print("Connection error - is the server running?")
        return None
    except KataGoAPIError as e:
        print(f"API Error ({e.status}): {e.title}")
        print(f"Details: {e.detail}")
        print(f"Error type: {e.error_type}")
        return None
    except Exception as e:
        print(f"Unexpected error: {e}")
        return None

# Usage
result = analyze_with_error_handling(["Q16", "D4", "R4"])
if result:
    best_move = result['moveInfos'][0]
    print(f"Best move: {best_move['moveCoord']}")
```

## TypeScript Client Example

```typescript
interface AnalysisRequest {
  moves: string[];
  komi?: number;
  rules?: string;
  boardXSize?: number;
  boardYSize?: number;
  includeOwnership?: boolean;
  maxVisits?: number;
}

interface MoveInfo {
  moveCoord: string;
  visits: number;
  winrate: number;
  scoreLead: number;
  scoreStdev?: number;
  prior: number;
  pv?: string[];
}

interface RootInfo {
  winrate: number;
  scoreLead: number;
  currentPlayer: string;
  visits: number;
}

interface AnalysisResponse {
  id?: string;
  turnNumber: number;
  moveInfos: MoveInfo[];
  rootInfo?: RootInfo;
  ownership?: number[];
}

class KataGoClient {
  constructor(private baseUrl: string = 'http://localhost:2718') {}

  async analyzePosition(request: AnalysisRequest): Promise<AnalysisResponse> {
    const response = await fetch(`${this.baseUrl}/api/v1/analysis`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        moves: request.moves,
        komi: request.komi ?? 7.5,
        rules: request.rules ?? 'chinese',
        boardXSize: request.boardXSize ?? 19,
        boardYSize: request.boardYSize ?? 19,
        includeOwnership: request.includeOwnership ?? false,
        maxVisits: request.maxVisits,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`${error.title}: ${error.detail}`);
    }

    return await response.json();
  }

  async getVersion() {
    const response = await fetch(`${this.baseUrl}/api/v1/version`);
    if (!response.ok) throw new Error('Failed to get version');
    return await response.json();
  }

  async healthCheck() {
    const response = await fetch(`${this.baseUrl}/api/v1/health`);
    if (!response.ok) throw new Error('Health check failed');
    return await response.json();
  }
}

// Usage
const client = new KataGoClient();

(async () => {
  const result = await client.analyzePosition({
    moves: ['Q16', 'D4', 'R4'],
    maxVisits: 100,
    includeOwnership: true,
  });

  console.log(`Best move: ${result.moveInfos[0].moveCoord}`);
  console.log(`Winrate: ${(result.moveInfos[0].winrate * 100).toFixed(2)}%`);
})();
```
