use crate::api::{AnalysisRequest, AnalysisResponse, MoveInfo, RootInfo};
use crate::config::KatagoConfig;
use crate::error::{KatagoError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
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
}

pub struct AnalysisEngine {
    config: KatagoConfig,
    process: Arc<StdMutex<Option<Child>>>,
    stdin: Arc<StdMutex<Option<ChildStdin>>>,
    pending_requests: Arc<StdMutex<HashMap<String, oneshot::Sender<String>>>>,
}

impl AnalysisEngine {
    pub fn new(config: KatagoConfig) -> Result<Self> {
        let pending_requests = Arc::new(StdMutex::new(HashMap::new()));

        let mut engine = Self {
            config: config.clone(),
            process: Arc::new(StdMutex::new(None)),
            stdin: Arc::new(StdMutex::new(None)),
            pending_requests: pending_requests.clone(),
        };

        engine.start_process(pending_requests)?;

        // Wait a bit for initialization
        thread::sleep(Duration::from_millis(500));

        Ok(engine)
    }

    fn start_process(
        &mut self,
        pending_requests: Arc<StdMutex<HashMap<String, oneshot::Sender<String>>>>,
    ) -> Result<()> {
        info!("Starting KataGo analysis engine");
        info!(
            "Config: katago={}, model={}, config={}",
            self.config.katago_path, self.config.model_path, self.config.config_path
        );

        let mut cmd = Command::new(&self.config.katago_path)
            .arg("analysis")
            .arg("-model")
            .arg(&self.config.model_path)
            .arg("-config")
            .arg(&self.config.config_path)
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

        *self.stdin.lock().unwrap() = Some(stdin);
        *self.process.lock().unwrap() = Some(cmd);

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
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        info!("KataGo analysis stdout closed (EOF)");
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
                                // Maybe a log line or something without ID
                                debug!("Received JSON without ID: {}", trimmed);
                            }
                        } else {
                            // Not JSON, probably a log line
                            debug!("Received non-JSON output: {}", trimmed);
                        }
                    }
                    Err(e) => {
                        error!("Error reading from KataGo analysis: {}", e);
                        break;
                    }
                }
            }
            info!("KataGo analysis stdout reader thread exiting");
        });

        Ok(())
    }

    fn send_query(&self, query: &AnalysisQuery) -> Result<()> {
        let json = serde_json::to_string(query)?;
        debug!("Sending analysis query: {}", json);

        let mut stdin = self.stdin.lock().unwrap();
        let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;

        writeln!(stdin, "{}", json)?;
        debug!("Written query to stdin, flushing...");
        match stdin.flush() {
            Ok(_) => debug!("Stdin flushed successfully"),
            Err(e) => error!("Failed to flush stdin: {}", e),
        }
        Ok(())
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

        // Convert moves to KataGo format: [["b", "D4"], ["w", "Q16"], ...]
        // Note: KataGo requires lowercase b/w (confirmed by Python implementation and testing)
        let mut katago_moves = Vec::new();
        let mut color = "b";
        for mv in &request.moves {
            katago_moves.push(vec![color.to_string(), mv.clone()]);
            color = if color == "b" { "w" } else { "b" };
        }

        let query = AnalysisQuery {
            id: request_id.clone(),
            initial_stones: vec![], // Empty for standard games (could support handicap via API later)
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
        };

        self.send_query(&query)?;

        let result = self
            .wait_for_response(&request_id, self.config.move_timeout_secs)
            .await?;

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
        let query_id = uuid::Uuid::new_v4().to_string();
        let query = serde_json::json!({
            "id": query_id,
            "action": "query_version"
        });

        let json = serde_json::to_string(&query)?;

        let (tx, rx) = oneshot::channel();
        {
            let mut requests = self.pending_requests.lock().unwrap();
            requests.insert(query_id.clone(), tx);
        }

        // Send query (ensure mutex is dropped before await)
        {
            let mut stdin = self.stdin.lock().unwrap();
            let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;
            writeln!(stdin, "{}", json)?;
            debug!("Written version query to stdin, flushing...");
            match stdin.flush() {
                Ok(_) => debug!("Stdin flushed successfully"),
                Err(e) => error!("Failed to flush stdin: {}", e),
            }
        } // Mutex guard dropped here

        // Wait for version response
        let duration = Duration::from_secs(5);

        match timeout(duration, rx).await {
            Ok(Ok(response)) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response) {
                    if value.get("id").and_then(|id| id.as_str()) == Some(&query_id) {
                        let version = value
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let git_hash = value
                            .get("git_hash")
                            .and_then(|h| h.as_str())
                            .map(|s| s.to_string());
                        return Ok((version, git_hash));
                    }
                }
                Err(KatagoError::ParseError(
                    "Failed to parse version response".to_string(),
                ))
            }
            Ok(Err(_)) => Err(KatagoError::ProcessDied),
            Err(_) => {
                {
                    let mut requests = self.pending_requests.lock().unwrap();
                    requests.remove(&query_id);
                }
                Err(KatagoError::Timeout(5))
            }
        }
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
