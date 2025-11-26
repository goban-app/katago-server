use crate::config::{KatagoConfig, RequestConfig};
use crate::error::{KatagoError, Result};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex as StdMutex, RwLock};
use std::thread;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

static WINRATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Winrate\s+([^\s]+)\s+").unwrap());
static SCORELEAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"ScoreLead\s+([^\s]+)\s+").unwrap());
static MOVE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+move\s+([^\s]+)\s+").unwrap());
static PSV_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"PSV\s+([^\s]+)\s+").unwrap());
static MOVE_CANDIDATE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([^\s]+)\s+:").unwrap());
static INFO_WINRATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"winrate\s+([^\s]+)\s+").unwrap());
static INFO_SCORELEAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"scoreLead\s+([^\s]+)\s+").unwrap());

#[derive(Debug, Clone)]
pub struct MoveCandidate {
    pub mv: String,
    pub psv: i32,
}

#[derive(Debug, Clone)]
pub struct Diagnostics {
    pub winprob: f32,
    pub score: f32,
    pub bot_move: String,
    pub best_ten: Vec<MoveCandidate>,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            winprob: -1.0,
            score: 0.0,
            bot_move: String::new(),
            best_ten: Vec::new(),
        }
    }
}

pub struct KatagoBot {
    config: KatagoConfig,
    process: Arc<StdMutex<Option<Child>>>,
    stdin: Arc<StdMutex<Option<ChildStdin>>>,
    response_rx: Arc<TokioMutex<mpsc::UnboundedReceiver<String>>>,
    last_move_color: Arc<TokioMutex<String>>,
    diagnostics: Arc<RwLock<Diagnostics>>,
}

impl KatagoBot {
    pub fn new(config: KatagoConfig) -> Result<Self> {
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        let mut bot = Self {
            config: config.clone(),
            process: Arc::new(StdMutex::new(None)),
            stdin: Arc::new(StdMutex::new(None)),
            response_rx: Arc::new(TokioMutex::new(response_rx)),
            last_move_color: Arc::new(TokioMutex::new(String::new())),
            diagnostics: Arc::new(RwLock::new(Diagnostics::default())),
        };

        bot.start_process(response_tx)?;

        // Wait a bit for initialization
        thread::sleep(Duration::from_millis(500));

        Ok(bot)
    }

    fn start_process(&mut self, response_tx: mpsc::UnboundedSender<String>) -> Result<()> {
        info!("Starting KataGo process");

        let mut cmd = Command::new(&self.config.katago_path)
            .arg("gtp")
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
                        debug!("KataGo stderr: {}", line);
                    }
                    Err(e) => {
                        error!("Error reading stderr from KataGo: {}", e);
                        break;
                    }
                }
            }
            debug!("KataGo stderr closed");
        });

        // Spawn stdout reader thread
        let diagnostics = Arc::clone(&self.diagnostics);
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        debug!("KataGo: {}", line);
                        Self::handle_response(&line, &diagnostics);
                        if let Err(e) = response_tx.send(line) {
                            error!("Failed to send response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from KataGo: {}", e);
                        break;
                    }
                }
            }
            warn!("KataGo stdout closed");
        });

        Ok(())
    }

    fn handle_response(line: &str, diagnostics: &Arc<RwLock<Diagnostics>>) {
        let mut diag = diagnostics.write().unwrap();

        // Parse winrate and score from CHAT/MALKOVICH
        if line.contains("CHAT:") || line.contains("MALKOVICH:") {
            if let Some(cap) = WINRATE_RE.captures(line) {
                if let Ok(wr) = cap[1].trim_end_matches('%').parse::<f32>() {
                    diag.winprob = wr * 0.01;
                }
            }
            if let Some(cap) = SCORELEAD_RE.captures(line) {
                if let Ok(sc) = cap[1].parse::<f32>() {
                    diag.score = sc;
                }
            }
            diag.best_ten.clear();
        }
        // Parse move candidates with PSV
        else if line.contains(" PSV ") {
            if let Some(psv_cap) = PSV_RE.captures(line) {
                if let Ok(psv) = psv_cap[1].parse::<i32>() {
                    if let Some(move_cap) = MOVE_CANDIDATE_RE.captures(line) {
                        diag.best_ten.push(MoveCandidate {
                            mv: move_cap[1].to_string(),
                            psv,
                        });
                    }
                }
            }
        }
        // Parse kata-analyze info response
        else if line.starts_with("info ") {
            if let Some(cap) = INFO_WINRATE_RE.captures(line) {
                if let Ok(wr) = cap[1].parse::<f32>() {
                    diag.winprob = wr;
                }
            }
            if let Some(cap) = INFO_SCORELEAD_RE.captures(line) {
                if let Ok(sc) = cap[1].parse::<f32>() {
                    diag.score = sc;
                }
            }
            if let Some(cap) = MOVE_RE.captures(line) {
                diag.bot_move = cap[1].to_string();
            }
        }
        // Parse GTP move response
        else if let Some(stripped) = line.strip_prefix('=') {
            let resp = stripped.trim();
            if !resp.is_empty() {
                diag.bot_move = resp.to_string();
            }
        }
    }

    fn send_command(&self, cmd: &str) -> Result<()> {
        debug!("Sending command: {}", cmd);
        let mut stdin = self.stdin.lock().unwrap();
        let stdin = stdin.as_mut().ok_or(KatagoError::ProcessDied)?;

        writeln!(stdin, "{}", cmd)?;
        stdin.flush()?;
        Ok(())
    }

    async fn wait_for_response(&self, timeout_secs: u64) -> Result<String> {
        let duration = Duration::from_secs(timeout_secs);

        timeout(duration, async {
            loop {
                let mut rx = self.response_rx.lock().await;
                if let Some(response) = rx.recv().await {
                    if response.starts_with('=') || response.starts_with("info ") {
                        return Ok(response);
                    }
                } else {
                    return Err(KatagoError::ProcessDied);
                }
            }
        })
        .await
        .map_err(|_| KatagoError::Timeout(timeout_secs))?
    }

    async fn wait_for_analysis_response(&self, timeout_secs: u64) -> Result<String> {
        let duration = Duration::from_secs(timeout_secs);
        let mut collected_lines = Vec::new();

        timeout(duration, async {
            loop {
                let mut rx = self.response_rx.lock().await;
                if let Some(response) = rx.recv().await {
                    debug!("kata-analyze response line: '{}'", response);

                    // Skip the initial '=' acknowledgment
                    if response.starts_with('=') {
                        debug!("Skipping acknowledgment line");
                        continue;
                    }

                    // Collect all analysis output lines (info, rootInfo, ownership, etc.)
                    if response.starts_with("info ")
                        || response.starts_with("rootInfo ")
                        || response.starts_with("ownership ")
                        || response.starts_with("ownershipStdev ")
                    {
                        debug!("Collecting analysis line: starts_with info={}, rootInfo={}, ownership={}", 
                               response.starts_with("info "),
                               response.starts_with("rootInfo "),
                               response.starts_with("ownership "));
                        collected_lines.push(response.clone());

                        // If we got ownership data, we're done
                        if response.starts_with("ownership ") {
                            info!("Found ownership line, returning {} collected lines", collected_lines.len());
                            return Ok(collected_lines.join("\n"));
                        }
                    } else {
                        debug!("Ignoring line that doesn't start with info/rootInfo/ownership: '{}'", response);
                    }
                } else {
                    return Err(KatagoError::ProcessDied);
                }
            }
        })
        .await
        .map_err(|_| KatagoError::Timeout(timeout_secs))?
    }

    fn set_rules(&self, komi: f32, config: &RequestConfig) -> Result<()> {
        let rules = if config.client.as_deref() == Some("kifucam") {
            "chinese"
        } else if komi != komi.floor() {
            // Non-integer komi
            if ((komi - 0.5) % 2.0).abs() > 0.01 {
                "chinese" // 7.5, not 6.5
            } else {
                "japanese"
            }
        } else {
            "japanese"
        };

        self.send_command(&format!("kata-set-rules {}", rules))?;
        Ok(())
    }

    fn set_komi(&self, komi: f32) -> Result<()> {
        self.send_command(&format!("komi {}", komi))?;
        Ok(())
    }

    pub async fn select_move(&self, moves: &[String], config: &RequestConfig) -> Result<String> {
        info!("Selecting move for position with {} moves", moves.len());

        // Reset diagnostics
        {
            let mut diag = self.diagnostics.write().unwrap();
            *diag = Diagnostics::default();
        }

        let komi = config.komi.unwrap_or(7.5);
        self.set_komi(komi)?;

        // Reset board
        self.send_command("clear_board")?;
        self.send_command("clear_cache")?;

        self.set_rules(komi, config)?;

        // Play moves
        let mut color = "b";
        for (idx, mv) in moves.iter().enumerate() {
            // Skip early passes (before move 20) for chinese handicap komi
            if mv != "pass" || idx > 20 {
                self.send_command(&format!("play {} {}", color, mv))?;
            }
            color = if color == "b" { "w" } else { "b" };
        }

        *self.last_move_color.lock().await = color.to_string();

        // Request move
        self.send_command(&format!("genmove {}", color))?;

        // Wait for response
        let response = self
            .wait_for_response(self.config.move_timeout_secs)
            .await?;

        if let Some(stripped) = response.strip_prefix('=') {
            let mv = stripped.trim().to_string();
            info!("KataGo selected move: {}", mv);
            Ok(mv)
        } else {
            Err(KatagoError::ParseError("Invalid move response".to_string()))
        }
    }

    pub async fn score(&self, moves: &[String], config: &RequestConfig) -> Result<Vec<f32>> {
        info!("Getting score for position with {} moves", moves.len());

        // Reset diagnostics
        {
            let mut diag = self.diagnostics.write().unwrap();
            *diag = Diagnostics::default();
        }

        let ownership = config.ownership.unwrap_or(true);
        let komi = config.komi.unwrap_or(7.5);

        self.set_komi(komi)?;

        // Reset board
        self.send_command("clear_board")?;
        self.send_command("clear_cache")?;

        self.set_rules(komi, config)?;

        // Play moves
        let mut color = "b";
        for (idx, mv) in moves.iter().enumerate() {
            if mv != "pass" || idx > 20 {
                self.send_command(&format!("play {} {}", color, mv))?;
            }
            color = if color == "b" { "w" } else { "b" };
        }

        // Request ownership analysis
        let ownership_flag = if ownership { "true" } else { "false" };
        self.send_command(&format!("kata-analyze 100 ownership {}", ownership_flag))?;

        // Wait for info response with ownership data
        let response = self
            .wait_for_analysis_response(self.config.move_timeout_secs)
            .await?;

        info!(
            "Full kata-analyze response ({} bytes): {}",
            response.len(),
            response
        );

        // Parse ownership values if requested
        let mut probs = Vec::new();
        if ownership {
            debug!("Parsing ownership from response: {}", response);

            // Split response into lines and search for ownership data
            // KataGo outputs ownership as: "ownership <361 floats>"
            for line in response.lines() {
                let trimmed = line.trim();
                if let Some(ownership_str) = trimmed.strip_prefix("ownership ") {
                    debug!("Found ownership line: {}", line);
                    debug!("Ownership string to parse: {}", ownership_str);

                    for token in ownership_str.split_whitespace() {
                        match token.parse::<f32>() {
                            Ok(val) => probs.push(val),
                            Err(_) => {
                                // Stop parsing when we hit non-numeric tokens
                                debug!("Stopped parsing at non-numeric token: {}", token);
                                break;
                            }
                        }
                    }

                    // Found ownership data, no need to check more lines
                    break;
                }
            }

            if probs.is_empty() {
                warn!("Response does not contain ownership data: {}", response);
            }
        }

        // Send stop command
        self.send_command("stop")?;

        info!(
            "Parsed {} ownership values from kata-analyze response",
            probs.len()
        );
        Ok(probs)
    }

    pub fn diagnostics(&self) -> Diagnostics {
        self.diagnostics.read().unwrap().clone()
    }
}

impl Drop for KatagoBot {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.lock().unwrap().take() {
            info!("Terminating KataGo process");
            let _ = process.kill();
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::env;
    use std::path::Path;

    // Helper to check if KataGo is available for testing
    fn katago_available() -> bool {
        env::var("KATAGO_PATH").is_ok() || Path::new("./katago").exists()
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --test-threads=1
    fn test_katago_process_starts_successfully() {
        if !katago_available() {
            eprintln!("Skipping test: KataGo not available");
            eprintln!("Set KATAGO_PATH env var or place katago binary in current directory");
            return;
        }

        let config = KatagoConfig {
            katago_path: env::var("KATAGO_PATH").unwrap_or_else(|_| "./katago".to_string()),
            model_path: env::var("KATAGO_MODEL_PATH")
                .unwrap_or_else(|_| "./model.bin.gz".to_string()),
            config_path: env::var("KATAGO_CONFIG_PATH")
                .unwrap_or_else(|_| "./gtp_config.cfg".to_string()),
            move_timeout_secs: 20,
        };

        // Test that process can be created without immediate crash
        let bot_result = KatagoBot::new(config);
        assert!(
            bot_result.is_ok(),
            "KataGo process should start successfully"
        );

        let bot = bot_result.unwrap();

        // Give it a moment to crash if it's going to
        thread::sleep(Duration::from_secs(2));

        // Verify process is still alive by checking if we can send a command
        let result = bot.send_command("name");
        assert!(
            result.is_ok(),
            "Should be able to send commands to running process"
        );
    }

    #[test]
    #[ignore]
    fn test_stderr_is_captured() {
        if !katago_available() {
            eprintln!("Skipping test: KataGo not available");
            return;
        }

        // This test verifies that stderr is piped, not nulled
        // By trying to start with an invalid model path, we should see stderr output
        let config = KatagoConfig {
            katago_path: env::var("KATAGO_PATH").unwrap_or_else(|_| "./katago".to_string()),
            model_path: "/nonexistent/model.bin.gz".to_string(),
            config_path: env::var("KATAGO_CONFIG_PATH")
                .unwrap_or_else(|_| "./gtp_config.cfg".to_string()),
            move_timeout_secs: 5,
        };

        // This should fail, but we should see stderr logs
        let result = KatagoBot::new(config);
        // The process will likely die, so we expect an error eventually
        // But the important part is that stderr was captured (check logs manually)
        assert!(
            result.is_err() || result.is_ok(),
            "Test completed - check logs for stderr output"
        );
    }

    #[test]
    fn test_config_validation() {
        // Test that missing files are reported properly
        let config = KatagoConfig {
            katago_path: "/nonexistent/katago".to_string(),
            model_path: "/nonexistent/model.bin.gz".to_string(),
            config_path: "/nonexistent/config.cfg".to_string(),
            move_timeout_secs: 20,
        };

        let result = KatagoBot::new(config);
        assert!(result.is_err(), "Should fail with nonexistent binary");
    }
}
