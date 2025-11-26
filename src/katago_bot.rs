use crate::config::{KatagoConfig, RequestConfig};
use crate::error::{KatagoError, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

lazy_static! {
    static ref WINRATE_RE: Regex = Regex::new(r"Winrate\s+([^\s]+)\s+").unwrap();
    static ref SCORELEAD_RE: Regex = Regex::new(r"ScoreLead\s+([^\s]+)\s+").unwrap();
    static ref MOVE_RE: Regex = Regex::new(r"\s+move\s+([^\s]+)\s+").unwrap();
    static ref PSV_RE: Regex = Regex::new(r"PSV\s+([^\s]+)\s+").unwrap();
    static ref MOVE_CANDIDATE_RE: Regex = Regex::new(r"^([^\s]+)\s+:").unwrap();
    static ref INFO_WINRATE_RE: Regex = Regex::new(r"winrate\s+([^\s]+)\s+").unwrap();
    static ref INFO_SCORELEAD_RE: Regex = Regex::new(r"scoreLead\s+([^\s]+)\s+").unwrap();
}

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
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    response_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    last_move_color: Arc<Mutex<String>>,
    diagnostics: Arc<Mutex<Diagnostics>>,
}

impl KatagoBot {
    pub fn new(config: KatagoConfig) -> Result<Self> {
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        
        let mut bot = Self {
            config: config.clone(),
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            response_rx: Arc::new(Mutex::new(response_rx)),
            last_move_color: Arc::new(Mutex::new(String::new())),
            diagnostics: Arc::new(Mutex::new(Diagnostics::default())),
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
            .stderr(Stdio::stdout())
            .spawn()
            .map_err(|e| KatagoError::ProcessStartFailed(e.to_string()))?;

        let stdout = cmd.stdout.take().ok_or(KatagoError::ProcessStartFailed(
            "Failed to capture stdout".to_string(),
        ))?;
        let stdin = cmd.stdin.take().ok_or(KatagoError::ProcessStartFailed(
            "Failed to capture stdin".to_string(),
        ))?;

        *self.stdin.lock().unwrap() = Some(stdin);
        *self.process.lock().unwrap() = Some(cmd);

        // Spawn reader thread
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

    fn handle_response(line: &str, diagnostics: &Arc<Mutex<Diagnostics>>) {
        let mut diag = diagnostics.lock().unwrap();

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
        else if line.starts_with('=') {
            let resp = line[1..].trim();
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
                let mut rx = self.response_rx.lock().unwrap();
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

    pub async fn select_move(
        &self,
        moves: &[String],
        config: &RequestConfig,
    ) -> Result<String> {
        info!("Selecting move for position with {} moves", moves.len());
        
        // Reset diagnostics
        {
            let mut diag = self.diagnostics.lock().unwrap();
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

        *self.last_move_color.lock().unwrap() = color.to_string();

        // Request move
        self.send_command(&format!("genmove {}", color))?;
        
        // Wait for response
        let response = self.wait_for_response(self.config.move_timeout_secs).await?;
        
        if response.starts_with('=') {
            let mv = response[1..].trim().to_string();
            info!("KataGo selected move: {}", mv);
            Ok(mv)
        } else {
            Err(KatagoError::ParseError("Invalid move response".to_string()))
        }
    }

    pub async fn score(
        &self,
        moves: &[String],
        config: &RequestConfig,
    ) -> Result<Vec<f32>> {
        info!("Getting score for position with {} moves", moves.len());
        
        // Reset diagnostics
        {
            let mut diag = self.diagnostics.lock().unwrap();
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
        
        // Wait for info response
        let response = self.wait_for_response(self.config.move_timeout_secs).await?;
        
        // Parse ownership values if requested
        let mut probs = Vec::new();
        if ownership && response.contains("ownership") {
            if let Some(ownership_pos) = response.find("ownership") {
                let ownership_str = &response[ownership_pos + 9..];
                for token in ownership_str.split_whitespace() {
                    if let Ok(val) = token.parse::<f32>() {
                        probs.push(val);
                    }
                }
            }
        }
        
        // Send stop command
        self.send_command("stop")?;
        
        info!("Parsed {} ownership values", probs.len());
        Ok(probs)
    }

    pub fn diagnostics(&self) -> Diagnostics {
        self.diagnostics.lock().unwrap().clone()
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
