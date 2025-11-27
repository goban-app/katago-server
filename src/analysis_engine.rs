use crate::api::{AnalysisRequest, AnalysisResponse, MoveInfo, RootInfo};
use crate::config::KatagoConfig;
use crate::error::{KatagoError, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::time::timeout;
use tracing::{debug, error, info};

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
    analyze_turns: Vec<u32>,
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
    response_rx: Arc<TokioMutex<mpsc::UnboundedReceiver<String>>>,
}

impl AnalysisEngine {
    pub fn new(config: KatagoConfig) -> Result<Self> {
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        let mut engine = Self {
            config: config.clone(),
            process: Arc::new(StdMutex::new(None)),
            stdin: Arc::new(StdMutex::new(None)),
            response_rx: Arc::new(TokioMutex::new(response_rx)),
        };

        engine.start_process(response_tx)?;

        // Wait a bit for initialization
        thread::sleep(Duration::from_millis(500));

        Ok(engine)
    }

    fn start_process(&mut self, response_tx: mpsc::UnboundedSender<String>) -> Result<()> {
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
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        debug!("KataGo analysis: {}", line);
                        if let Err(e) = response_tx.send(line) {
                            error!("Failed to send response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from KataGo analysis: {}", e);
                        break;
                    }
                }
            }
            info!("KataGo analysis stdout closed");
        });

        Ok(())
    }

    fn send_query(&self, query: &AnalysisQuery) -> Result<()> {
        let json = serde_json::to_string(query)?;
        debug!("Sending analysis query: {}", json);

        let mut stdin = self.stdin.lock().unwrap();
        let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;

        writeln!(stdin, "{}", json)?;
        stdin.flush()?;
        Ok(())
    }

    async fn wait_for_response(&self, id: &str, timeout_secs: u64) -> Result<AnalysisResult> {
        let duration = Duration::from_secs(timeout_secs);

        timeout(duration, async {
            loop {
                let mut rx = self.response_rx.lock().await;
                if let Some(response) = rx.recv().await {
                    debug!(
                        "Received response from KataGo (length: {}): {}",
                        response.len(),
                        if response.len() > 200 {
                            &response[..200]
                        } else {
                            &response
                        }
                    );

                    // Try to parse as JSON
                    match serde_json::from_str::<AnalysisResult>(&response) {
                        Ok(result) => {
                            if result.id == id {
                                debug!("Matched response ID, returning result");
                                return Ok(result);
                            } else {
                                debug!(
                                    "Response ID mismatch: expected '{}', got '{}'",
                                    id, result.id
                                );
                            }
                        }
                        Err(e) => {
                            debug!("Failed to parse as AnalysisResult: {}", e);
                            // Also handle error responses
                            if let Ok(error) = serde_json::from_str::<serde_json::Value>(&response)
                            {
                                if let Some(err_msg) = error.get("error") {
                                    error!("KataGo returned error: {}", err_msg);
                                    return Err(KatagoError::ResponseError(err_msg.to_string()));
                                }
                            }
                        }
                    }
                } else {
                    error!("Response channel closed - KataGo process died");
                    return Err(KatagoError::ProcessDied);
                }
            }
        })
        .await
        .map_err(|_| KatagoError::Timeout(timeout_secs))?
    }

    pub async fn analyze(&self, request: &AnalysisRequest) -> Result<AnalysisResponse> {
        let request_id = request
            .request_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Convert moves to KataGo format: [["B", "D4"], ["W", "Q16"], ...]
        let mut katago_moves = Vec::new();
        let mut color = "B";
        for mv in &request.moves {
            katago_moves.push(vec![color.to_string(), mv.clone()]);
            color = if color == "B" { "W" } else { "B" };
        }

        // Analyze the final position (after all moves)
        let turn_to_analyze = request.moves.len() as u32;

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
            analyze_turns: vec![turn_to_analyze],
            // Always include maxVisits - KataGo requires this to start analysis
            max_visits: Some(request.max_visits.unwrap_or(200)),
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

        // Send query (ensure mutex is dropped before await)
        {
            let mut stdin = self.stdin.lock().unwrap();
            let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;
            writeln!(stdin, "{}", json)?;
            stdin.flush()?;
        } // Mutex guard dropped here

        // Wait for version response
        let duration = Duration::from_secs(5);
        timeout(duration, async {
            loop {
                let mut rx = self.response_rx.lock().await;
                if let Some(response) = rx.recv().await {
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
                } else {
                    return Err(KatagoError::ProcessDied);
                }
            }
        })
        .await
        .map_err(|_| KatagoError::Timeout(5))?
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
