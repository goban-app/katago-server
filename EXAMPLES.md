# Examples

This directory contains example code and usage patterns for the KataGo Server.

## Basic Client (Python)

```python
import requests
import json

class KataGoClient:
    def __init__(self, base_url="http://localhost:2718"):
        self.base_url = base_url
    
    def select_move(self, moves, komi=7.5, board_size=19):
        """Get the best move for the current position."""
        url = f"{self.base_url}/select-move/katago_gtp_bot"
        data = {
            "board_size": board_size,
            "moves": moves,
            "config": {"komi": komi}
        }
        response = requests.post(url, json=data)
        return response.json()
    
    def get_score(self, moves, komi=7.5, board_size=19):
        """Get territory ownership probabilities."""
        url = f"{self.base_url}/score/katago_gtp_bot"
        data = {
            "board_size": board_size,
            "moves": moves,
            "config": {"komi": komi, "ownership": True}
        }
        response = requests.post(url, json=data)
        return response.json()

# Usage
client = KataGoClient()

# Play some moves
moves = ["Q16", "D4", "R4"]
result = client.select_move(moves)
print(f"Best move: {result['bot_move']}")
print(f"Win probability: {result['diagnostics']['winprob']:.2%}")

# Get territory info
score_result = client.get_score(moves + [result['bot_move']])
print(f"Score estimate: {score_result['diagnostics']['score']:.1f}")
```

## Basic Client (JavaScript)

```javascript
class KataGoClient {
    constructor(baseUrl = 'http://localhost:2718') {
        this.baseUrl = baseUrl;
    }

    async selectMove(moves, komi = 7.5, boardSize = 19) {
        const response = await fetch(`${this.baseUrl}/select-move/katago_gtp_bot`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                board_size: boardSize,
                moves: moves,
                config: { komi }
            })
        });
        return await response.json();
    }

    async getScore(moves, komi = 7.5, boardSize = 19) {
        const response = await fetch(`${this.baseUrl}/score/katago_gtp_bot`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                board_size: boardSize,
                moves: moves,
                config: { komi, ownership: true }
            })
        });
        return await response.json();
    }
}

// Usage
const client = new KataGoClient();

(async () => {
    const moves = ['Q16', 'D4', 'R4'];
    const result = await client.selectMove(moves);
    console.log(`Best move: ${result.bot_move}`);
    console.log(`Win probability: ${(result.diagnostics.winprob * 100).toFixed(2)}%`);
})();
```

## Basic Client (Rust)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct SelectMoveRequest {
    board_size: u8,
    moves: Vec<String>,
    config: RequestConfig,
}

#[derive(Debug, Serialize)]
struct RequestConfig {
    komi: f32,
}

#[derive(Debug, Deserialize)]
struct SelectMoveResponse {
    bot_move: String,
    diagnostics: Diagnostics,
}

#[derive(Debug, Deserialize)]
struct Diagnostics {
    winprob: f32,
    score: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let request = SelectMoveRequest {
        board_size: 19,
        moves: vec!["Q16".to_string(), "D4".to_string(), "R4".to_string()],
        config: RequestConfig { komi: 7.5 },
    };
    
    let response = client
        .post("http://localhost:2718/select-move/katago_gtp_bot")
        .json(&request)
        .send()
        .await?
        .json::<SelectMoveResponse>()
        .await?;
    
    println!("Best move: {}", response.bot_move);
    println!("Win probability: {:.2}%", response.diagnostics.winprob * 100.0);
    
    Ok(())
}
```

## SGF Integration Example

```python
import re
from katago_client import KataGoClient

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

# Get suggested move at each position
for i in range(len(moves)):
    position = moves[:i+1]
    result = client.select_move(position)
    actual_move = moves[i] if i < len(moves) else None
    suggested_move = result['bot_move']
    winprob = result['diagnostics']['winprob']
    
    print(f"Move {i+1}: Played {actual_move}, KataGo suggests {suggested_move} ({winprob:.1%})")
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

    location / {
        proxy_pass http://katago_backend;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }
}
```

## Batch Processing Example

```python
import concurrent.futures
from katago_client import KataGoClient

def analyze_position(client, moves):
    """Analyze a single position."""
    try:
        result = client.select_move(moves)
        return {
            'moves': moves,
            'suggestion': result['bot_move'],
            'winprob': result['diagnostics']['winprob']
        }
    except Exception as e:
        return {'error': str(e)}

# Analyze multiple positions in parallel
client = KataGoClient()
positions = [
    ["Q16", "D4"],
    ["R4", "D16"],
    ["Q3", "D17"],
    # ... more positions
]

with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
    futures = [executor.submit(analyze_position, client, pos) for pos in positions]
    results = [f.result() for f in concurrent.futures.as_completed(futures)]

for result in results:
    if 'error' not in result:
        print(f"Position: {result['moves']} -> {result['suggestion']} ({result['winprob']:.1%})")
```
