use crate::api::{AnalysisRequest, AnalysisResponse, MoveInfo, RootInfo};
use crate::config::KatagoConfig;
use crate::error::{KatagoError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// JSON request format for KataGo analysis engine
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisQuery {
    id: String,
    initial_stones: Vec<Vec<String>>,
    moves: Vec<Vec<String>>,
    rules: String,
    komi: f32,
    board_x_size: u8,
    board_y_size: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    analyze_turns: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_visits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_ownership: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_policy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_pv_visits: Option<bool>,
    /// Override KataGo search/analysis settings per-request
    /// Supports all KataGo analysis config options including human SL settings:
    /// - humanSLProfile: e.g., "preaz_5k", "rank_3d", "proyear_2020"
    /// - humanSLChosenMoveProp, humanSLRootExploreProbWeightless, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    override_settings: Option<serde_json::Value>,
}

/// JSON response format from KataGo analysis engine
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisResult {
    #[allow(dead_code)] // Used for routing responses, not directly accessed
    id: String,
    #[serde(default)]
    turn_number: u32,
    #[serde(default)]
    move_infos: Vec<KatagoMoveInfo>,
    #[serde(default)]
    root_info: Option<KatagoRootInfo>,
    #[serde(default)]
    ownership: Option<Vec<f32>>,
    #[serde(default)]
    policy: Option<Vec<f32>>,
    /// Human SL model policy (when human model is loaded and includePolicy=true)
    #[serde(default)]
    human_policy: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KatagoMoveInfo {
    #[serde(rename = "move")]
    move_coord: String,
    visits: u32,
    winrate: f32,
    score_mean: f32,
    #[serde(default)]
    score_stdev: f32,
    score_lead: f32,
    #[serde(default)]
    utility: f32,
    #[serde(default)]
    utility_lcb: f32,
    lcb: f32,
    prior: f32,
    /// Human SL model prior for this move (when human model is loaded)
    #[serde(default)]
    human_prior: Option<f32>,
    order: u32,
    #[serde(default)]
    pv: Vec<String>,
    #[serde(default)]
    pv_visits: Option<Vec<u32>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KatagoRootInfo {
    winrate: f32,
    score_lead: f32,
    #[serde(default)]
    utility: f32,
    visits: u32,
    current_player: String,
    #[serde(default)]
    raw_winrate: Option<f32>,
    #[serde(default)]
    raw_score_mean: Option<f32>,
    #[serde(default)]
    raw_st_score_error: Option<f32>,
    // Human SL model fields (when human model is loaded and humanSLProfile is set)
    #[serde(default)]
    human_winrate: Option<f32>,
    #[serde(default)]
    human_score_mean: Option<f32>,
    #[serde(default)]
    human_score_stdev: Option<f32>,
}

/// Keepalive interval in seconds - send periodic pings to keep KataGo alive
const KEEPALIVE_INTERVAL_SECS: u64 = 30;

pub struct AnalysisEngine {
    config: KatagoConfig,
    process: Arc<StdMutex<Option<Child>>>,
    stdin: Arc<StdMutex<Option<ChildStdin>>>,
    pending_requests: Arc<StdMutex<HashMap<String, oneshot::Sender<String>>>>,
    /// Flag indicating if KataGo process is alive
    process_alive: Arc<AtomicBool>,
}

impl AnalysisEngine {
    pub fn new(config: KatagoConfig) -> Result<Self> {
        let pending_requests = Arc::new(StdMutex::new(HashMap::new()));
        let process_alive = Arc::new(AtomicBool::new(false));

        let mut engine = Self {
            config: config.clone(),
            process: Arc::new(StdMutex::new(None)),
            stdin: Arc::new(StdMutex::new(None)),
            pending_requests: pending_requests.clone(),
            process_alive: process_alive.clone(),
        };

        engine.start_process(pending_requests.clone())?;

        // Wait a bit for initialization
        thread::sleep(Duration::from_millis(500));

        // Start process monitor thread (handles keepalive + auto-restart)
        let config_clone = config;
        let process_clone = engine.process.clone();
        let stdin_clone = engine.stdin.clone();
        let pending_clone = pending_requests;
        let alive_clone = process_alive;
        thread::spawn(move || {
            Self::process_monitor_loop(
                config_clone,
                process_clone,
                stdin_clone,
                pending_clone,
                alive_clone,
            );
        });

        Ok(engine)
    }

    /// Combined keepalive and process monitor loop
    /// Sends periodic pings and restarts KataGo if it dies
    fn process_monitor_loop(
        config: KatagoConfig,
        process: Arc<StdMutex<Option<Child>>>,
        stdin: Arc<StdMutex<Option<ChildStdin>>>,
        pending_requests: Arc<StdMutex<HashMap<String, oneshot::Sender<String>>>>,
        process_alive: Arc<AtomicBool>,
    ) {
        const MAX_RESTART_ATTEMPTS: u32 = 5;
        const RESTART_DELAY_SECS: u64 = 5;

        let mut restart_count: u32 = 0;

        loop {
            thread::sleep(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));

            // Check if process is dead and needs restart
            if !process_alive.load(Ordering::SeqCst) {
                if restart_count >= MAX_RESTART_ATTEMPTS {
                    error!(
                        "KataGo has failed {} times, giving up on restarts",
                        restart_count
                    );
                    continue;
                }

                warn!(
                    "KataGo process died, attempting restart (attempt {})",
                    restart_count + 1
                );
                thread::sleep(Duration::from_secs(RESTART_DELAY_SECS));

                // Clean up old process
                if let Some(mut old_process) = process.lock().unwrap().take() {
                    let _ = old_process.kill();
                    let _ = old_process.wait();
                }

                // Attempt to restart
                match Self::spawn_katago_process(&config) {
                    Ok((child, new_stdin, stdout, stderr)) => {
                        *stdin.lock().unwrap() = Some(new_stdin);
                        *process.lock().unwrap() = Some(child);
                        process_alive.store(true, Ordering::SeqCst);

                        // Start new reader threads
                        Self::spawn_reader_threads(
                            stdout,
                            stderr,
                            pending_requests.clone(),
                            process_alive.clone(),
                        );

                        info!("KataGo restarted successfully");
                        restart_count += 1;

                        // Wait for KataGo to initialize
                        thread::sleep(Duration::from_secs(5));
                    }
                    Err(e) => {
                        error!("Failed to restart KataGo: {}", e);
                        restart_count += 1;
                    }
                }
                continue;
            }

            // Process is alive, send keepalive ping
            let ping = serde_json::json!({
                "id": "keepalive",
                "action": "query_version"
            });

            let json = match serde_json::to_string(&ping) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize keepalive ping: {}", e);
                    continue;
                }
            };

            let mut stdin_guard = stdin.lock().unwrap();
            if let Some(ref mut stdin_ref) = *stdin_guard {
                if let Err(e) = writeln!(stdin_ref, "{}", json) {
                    warn!("Failed to send keepalive ping: {}", e);
                    process_alive.store(false, Ordering::SeqCst);
                } else if let Err(e) = stdin_ref.flush() {
                    warn!("Failed to flush keepalive ping: {}", e);
                    process_alive.store(false, Ordering::SeqCst);
                } else {
                    debug!("Sent keepalive ping to KataGo");
                    // Reset restart count on successful ping
                    restart_count = 0;
                }
            } else {
                debug!("No stdin available for keepalive ping");
            }
        }
    }

    /// Spawn the KataGo process and return handles to it
    fn spawn_katago_process(
        config: &KatagoConfig,
    ) -> Result<(
        Child,
        ChildStdin,
        std::process::ChildStdout,
        std::process::ChildStderr,
    )> {
        info!("Starting KataGo analysis engine");
        info!(
            "Config: katago={}, model={}, human_model={:?}, config={}",
            config.katago_path, config.model_path, config.human_model_path, config.config_path
        );

        let mut command = Command::new(&config.katago_path);
        command
            .arg("analysis")
            .arg("-model")
            .arg(&config.model_path);

        // Add human model if configured
        if let Some(ref human_model) = config.human_model_path {
            info!("Human SL model enabled: {}", human_model);
            command.arg("-human-model").arg(human_model);
        }

        let mut cmd = command
            .arg("-config")
            .arg(&config.config_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| KatagoError::ProcessStartFailed(e.to_string()))?;

        let stdout = cmd.stdout.take().ok_or(KatagoError::ProcessStartFailed(
            "Failed to capture stdout".to_string(),
        ))?;
        let stderr = cmd.stderr.take().ok_or(KatagoError::ProcessStartFailed(
            "Failed to capture stderr".to_string(),
        ))?;
        let stdin = cmd.stdin.take().ok_or(KatagoError::ProcessStartFailed(
            "Failed to capture stdin".to_string(),
        ))?;

        Ok((cmd, stdin, stdout, stderr))
    }

    /// Spawn reader threads for stdout and stderr
    fn spawn_reader_threads(
        stdout: std::process::ChildStdout,
        stderr: std::process::ChildStderr,
        pending_requests: Arc<StdMutex<HashMap<String, oneshot::Sender<String>>>>,
        process_alive: Arc<AtomicBool>,
    ) {
        // Spawn stderr reader thread
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        debug!("KataGo analysis stderr: {}", line);
                    }
                    Err(e) => {
                        error!("Error reading stderr from KataGo analysis: {}", e);
                        break;
                    }
                }
            }
            debug!("KataGo analysis stderr closed");
        });

        // Spawn stdout reader thread
        let process_alive_clone = process_alive;
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        info!("KataGo analysis stdout closed (EOF)");
                        // Mark process as dead
                        process_alive_clone.store(false, Ordering::SeqCst);
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        debug!("KataGo analysis raw output: {}", trimmed);

                        // Parse ID from response to route it
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(id) = value.get("id").and_then(|id| id.as_str()) {
                                let mut requests = pending_requests.lock().unwrap();
                                if let Some(sender) = requests.remove(id) {
                                    if sender.send(trimmed.to_string()).is_err() {
                                        warn!("Failed to send response to waiter for ID: {}", id);
                                    }
                                } else {
                                    // This might be a log message or unexpected response
                                    debug!("Received response for unknown or timed-out ID: {}", id);
                                }
                            } else {
                                // Maybe a log line or something without ID (like query_version response)
                                debug!("Received JSON without ID: {}", trimmed);
                            }
                        } else {
                            // Not JSON, probably a log line
                            debug!("Received non-JSON output: {}", trimmed);
                        }
                    }
                    Err(e) => {
                        error!("Error reading from KataGo analysis: {}", e);
                        process_alive_clone.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
            info!("KataGo analysis stdout reader thread exiting");
        });
    }

    fn start_process(
        &mut self,
        pending_requests: Arc<StdMutex<HashMap<String, oneshot::Sender<String>>>>,
    ) -> Result<()> {
        let (cmd, stdin, stdout, stderr) = Self::spawn_katago_process(&self.config)?;

        *self.stdin.lock().unwrap() = Some(stdin);
        *self.process.lock().unwrap() = Some(cmd);

        // Mark process as alive
        self.process_alive.store(true, Ordering::SeqCst);

        // Spawn reader threads
        Self::spawn_reader_threads(stdout, stderr, pending_requests, self.process_alive.clone());

        Ok(())
    }

    fn send_query(&self, query: &AnalysisQuery) -> Result<()> {
        // Check if process is alive before sending
        if !self.process_alive.load(Ordering::SeqCst) {
            return Err(KatagoError::ProcessDied);
        }

        let json = serde_json::to_string(query)?;
        debug!("Sending analysis query: {}", json);

        let mut stdin = self.stdin.lock().unwrap();
        let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;

        writeln!(stdin, "{}", json)?;
        debug!("Written query to stdin, flushing...");
        match stdin.flush() {
            Ok(_) => debug!("Stdin flushed successfully"),
            Err(e) => {
                error!("Failed to flush stdin: {}", e);
                self.process_alive.store(false, Ordering::SeqCst);
                return Err(KatagoError::ProcessDied);
            }
        }
        Ok(())
    }

    /// Check if KataGo process is running
    pub fn is_alive(&self) -> bool {
        self.process_alive.load(Ordering::SeqCst)
    }

    /// Validates if a move coordinate is valid for the given board size
    /// Go coordinates: A-Z (excluding I), 1-boardSize
    fn is_valid_move(move_str: &str, board_x_size: u8, board_y_size: u8) -> bool {
        if move_str.len() < 2 {
            return false;
        }

        // Handle special case "pass"
        if move_str.eq_ignore_ascii_case("pass") {
            return true;
        }

        // Parse column (letter) and row (number)
        let col_char = move_str.chars().next().unwrap().to_ascii_uppercase();
        let row_str = &move_str[1..];

        // Validate column (A-Z, excluding I)
        // Column A=1, B=2, ..., H=8, J=9, K=10, ...
        let col_num = if col_char < 'I' {
            col_char as u8 - b'A' + 1
        } else if col_char > 'I' {
            col_char as u8 - b'A' // B is offset by 1 less because we skip I
        } else {
            // I is not a valid column in Go
            return false;
        };

        if col_num > board_x_size {
            return false;
        }

        // Validate row
        if let Ok(row_num) = row_str.parse::<u8>() {
            row_num >= 1 && row_num <= board_y_size
        } else {
            false
        }
    }

    /// Returns the last valid column letter for a given board size
    fn column_letter_for_size(board_size: u8) -> char {
        // A=1, B=2, ..., H=8, J=9, K=10, ...
        if board_size <= 8 {
            (b'A' + board_size - 1) as char
        } else {
            // Skip 'I' after H
            (b'A' + board_size) as char
        }
    }

    async fn wait_for_response(&self, id: &str, timeout_secs: u64) -> Result<AnalysisResult> {
        let (tx, rx) = oneshot::channel();

        {
            let mut requests = self.pending_requests.lock().unwrap();
            requests.insert(id.to_string(), tx);
        }

        let duration = Duration::from_secs(timeout_secs);

        match timeout(duration, rx).await {
            Ok(Ok(response)) => {
                // Parse the response
                match serde_json::from_str::<AnalysisResult>(&response) {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        // Check for error response
                        if let Ok(error) = serde_json::from_str::<serde_json::Value>(&response) {
                            if let Some(err_msg) = error.get("error") {
                                error!("KataGo returned error: {}", err_msg);
                                return Err(KatagoError::ResponseError(err_msg.to_string()));
                            }
                        }
                        Err(KatagoError::ParseError(e.to_string()))
                    }
                }
            }
            Ok(Err(_)) => {
                // Sender dropped (process died?)
                Err(KatagoError::ProcessDied)
            }
            Err(_) => {
                // Timeout
                {
                    let mut requests = self.pending_requests.lock().unwrap();
                    requests.remove(id);
                }
                Err(KatagoError::Timeout(timeout_secs))
            }
        }
    }

    pub async fn analyze(&self, request: &AnalysisRequest) -> Result<AnalysisResponse> {
        let request_id = request
            .request_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Validate moves for the given board size
        for mv in &request.moves {
            if !Self::is_valid_move(mv, request.board_x_size, request.board_y_size) {
                warn!(
                    "Invalid move '{}' for {}x{} board (valid columns: A-{}, skipping I)",
                    mv,
                    request.board_x_size,
                    request.board_y_size,
                    Self::column_letter_for_size(request.board_x_size)
                );
            }
        }

        // Convert moves to KataGo format: [["b", "D4"], ["w", "Q16"], ...]
        // Note: KataGo requires lowercase b/w (confirmed by Python implementation and testing)
        // In handicap games (with initial_stones), White plays first
        let mut katago_moves = Vec::new();
        let has_handicap = request
            .initial_stones
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        // Use initial_player if provided, otherwise infer from handicap
        let first_player = request
            .initial_player
            .as_ref()
            .map(|p| p.to_lowercase())
            .unwrap_or_else(|| {
                if has_handicap {
                    "w".to_string() // White plays first in handicap games
                } else {
                    "b".to_string() // Black plays first normally
                }
            });
        let mut color = first_player.as_str();
        for mv in &request.moves {
            katago_moves.push(vec![color.to_string(), mv.clone()]);
            color = if color == "b" { "w" } else { "b" };
        }

        // Convert initial_stones from API format (tuples) to KataGo format (vecs)
        // API: [("B", "D16"), ("B", "Q4")] -> KataGo: [["B", "D16"], ["B", "Q4"]]
        let initial_stones: Vec<Vec<String>> = request
            .initial_stones
            .as_ref()
            .map(|stones| {
                stones
                    .iter()
                    .map(|(color, coord)| vec![color.clone(), coord.clone()])
                    .collect()
            })
            .unwrap_or_default();

        let query = AnalysisQuery {
            id: request_id.clone(),
            initial_stones,
            moves: katago_moves,
            rules: request.rules.clone().unwrap_or_else(|| {
                // Auto-detect rules from komi
                let komi = request.komi.unwrap_or(7.5);
                if komi == komi.floor() || (komi - 6.5).abs() < 0.01 {
                    "japanese".to_string()
                } else {
                    "chinese".to_string()
                }
            }),
            komi: request.komi.unwrap_or(7.5),
            board_x_size: request.board_x_size,
            board_y_size: request.board_y_size,
            // Let analyzeTurns default to analyzing the final position
            analyze_turns: None,
            // Always include maxVisits - KataGo requires this to start analysis
            // Default to 10 for fast CPU execution (increase for GPU or stronger analysis)
            max_visits: Some(request.max_visits.unwrap_or(10)),
            include_ownership: request.include_ownership,
            include_policy: request.include_policy,
            include_pv_visits: request.include_pv_visits,
            // Pass through override settings (e.g., humanSLProfile for human-style analysis)
            override_settings: request.override_settings.clone(),
        };

        self.send_query(&query)?;

        let result = self
            .wait_for_response(&request_id, self.config.move_timeout_secs)
            .await?;

        // Warn if KataGo returned empty move infos (might indicate invalid position/moves)
        if result.move_infos.is_empty() {
            warn!(
                "KataGo returned empty moveInfos for request {}: board={}x{}, moves={:?}",
                request_id, request.board_x_size, request.board_y_size, request.moves
            );
            if result.root_info.is_none() {
                warn!("No rootInfo either - the position may be invalid or moves may be illegal");
            }
        }

        // Convert KataGo response to our API format
        let move_infos = result
            .move_infos
            .into_iter()
            .map(|mi| MoveInfo {
                move_coord: mi.move_coord,
                visits: mi.visits,
                winrate: mi.winrate,
                score_mean: mi.score_mean,
                score_stdev: mi.score_stdev,
                score_lead: mi.score_lead,
                utility: mi.utility,
                utility_lcb: Some(mi.utility_lcb),
                lcb: mi.lcb,
                prior: mi.prior,
                human_prior: mi.human_prior,
                order: mi.order,
                pv: if mi.pv.is_empty() { None } else { Some(mi.pv) },
                pv_visits: mi.pv_visits,
                ownership: None, // Per-move ownership not implemented yet
            })
            .collect();

        let root_info = result.root_info.map(|ri| RootInfo {
            winrate: ri.winrate,
            score_lead: ri.score_lead,
            utility: ri.utility,
            visits: ri.visits,
            current_player: ri.current_player,
            raw_winrate: ri.raw_winrate,
            raw_score_mean: ri.raw_score_mean,
            raw_st_score_error: ri.raw_st_score_error,
            human_winrate: ri.human_winrate,
            human_score_mean: ri.human_score_mean,
            human_score_stdev: ri.human_score_stdev,
        });

        Ok(AnalysisResponse {
            id: request_id,
            turn_number: result.turn_number,
            is_during_search: false,
            move_infos: Some(move_infos),
            root_info,
            ownership: result.ownership,
            ownership_stdev: None, // Not provided by basic analysis
            policy: result.policy,
            human_policy: result.human_policy,
        })
    }

    pub async fn clear_cache(&self) -> Result<()> {
        info!("Clearing KataGo analysis cache");
        let query = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "action": "clear_cache"
        });

        let json = serde_json::to_string(&query)?;
        let mut stdin = self.stdin.lock().unwrap();
        let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;

        writeln!(stdin, "{}", json)?;
        stdin.flush()?;
        Ok(())
    }

    pub async fn query_version(&self) -> Result<(String, Option<String>)> {
        // KataGo requires an 'id' field for all requests including query_version
        let query = serde_json::json!({
            "id": "query_version",
            "action": "query_version"
        });

        let json = serde_json::to_string(&query)?;

        // For action commands, we can't use the pending_requests tracking
        // because the response doesn't have an id. Instead, we just send
        // the command and check if the process is still alive.
        {
            let mut stdin = self.stdin.lock().unwrap();
            let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;
            writeln!(stdin, "{}", json)?;
            stdin.flush()?;
            debug!("Sent query_version command");
        }

        // Give KataGo a moment to respond, then check if process is alive
        tokio::time::sleep(Duration::from_millis(100)).await;

        if !self.process_alive.load(Ordering::SeqCst) {
            return Err(KatagoError::ProcessDied);
        }

        // Return a placeholder - the actual version info will be in the response
        // but since we can't easily correlate it, we return what we know from startup logs
        Ok(("1.15.0".to_string(), None))
    }

    pub fn model_path(&self) -> &str {
        &self.config.model_path
    }
}

impl Drop for AnalysisEngine {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.lock().unwrap().take() {
            info!("Terminating KataGo analysis process");
            let _ = process.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_validation_9x9_board() {
        // Valid moves on 9x9 board
        assert!(AnalysisEngine::is_valid_move("A1", 9, 9));
        assert!(AnalysisEngine::is_valid_move("D4", 9, 9));
        assert!(AnalysisEngine::is_valid_move("J9", 9, 9)); // J is the 9th column (I is skipped)
        assert!(AnalysisEngine::is_valid_move("H5", 9, 9));
        assert!(AnalysisEngine::is_valid_move("pass", 9, 9));
        assert!(AnalysisEngine::is_valid_move("PASS", 9, 9));

        // Invalid moves on 9x9 board
        assert!(!AnalysisEngine::is_valid_move("R4", 9, 9)); // R is column 17 (skipping I)
        assert!(!AnalysisEngine::is_valid_move("K1", 9, 9)); // K would be column 10
        assert!(!AnalysisEngine::is_valid_move("A10", 9, 9)); // Row 10 doesn't exist on 9x9
        assert!(!AnalysisEngine::is_valid_move("I5", 9, 9)); // I is never a valid column
        assert!(!AnalysisEngine::is_valid_move("A0", 9, 9)); // Row 0 doesn't exist
    }

    #[test]
    fn test_move_validation_19x19_board() {
        // Valid moves on 19x19 board
        assert!(AnalysisEngine::is_valid_move("A1", 19, 19));
        assert!(AnalysisEngine::is_valid_move("D4", 19, 19));
        assert!(AnalysisEngine::is_valid_move("R4", 19, 19)); // R is valid on 19x19
        assert!(AnalysisEngine::is_valid_move("T19", 19, 19)); // T is the 19th column
        assert!(AnalysisEngine::is_valid_move("Q16", 19, 19));

        // Invalid moves on 19x19 board
        assert!(!AnalysisEngine::is_valid_move("U1", 19, 19)); // U would be column 20
        assert!(!AnalysisEngine::is_valid_move("A20", 19, 19)); // Row 20 doesn't exist
        assert!(!AnalysisEngine::is_valid_move("I5", 19, 19)); // I is never valid
    }

    #[test]
    fn test_column_letter_for_size() {
        assert_eq!(AnalysisEngine::column_letter_for_size(9), 'J'); // A-H, J (skip I)
        assert_eq!(AnalysisEngine::column_letter_for_size(19), 'T'); // A-H, J-T
        assert_eq!(AnalysisEngine::column_letter_for_size(5), 'E');
    }
}
