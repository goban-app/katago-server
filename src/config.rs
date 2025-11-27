use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KatagoConfig {
    pub katago_path: String,
    pub model_path: String,
    pub config_path: String,
    pub move_timeout_secs: u64,
}

impl Default for KatagoConfig {
    fn default() -> Self {
        Self {
            katago_path: "./katago".to_string(),
            model_path: "./model.bin.gz".to_string(),
            config_path: "./analysis_config.cfg".to_string(),
            move_timeout_secs: 20,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 2718,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub katago: KatagoConfig,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Config::default();

        if let Ok(host) = std::env::var("KATAGO_SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("KATAGO_SERVER_PORT") {
            config.server.port = port.parse()?;
        }
        if let Ok(path) = std::env::var("KATAGO_KATAGO_PATH") {
            config.katago.katago_path = path;
        }
        if let Ok(path) = std::env::var("KATAGO_MODEL_PATH") {
            config.katago.model_path = path;
        }
        if let Ok(path) = std::env::var("KATAGO_CONFIG_PATH") {
            config.katago.config_path = path;
        }
        if let Ok(timeout) = std::env::var("KATAGO_MOVE_TIMEOUT_SECS") {
            config.katago.move_timeout_secs = timeout.parse()?;
        }

        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)] // Kept for potential future GTP mode support
pub struct RequestConfig {
    #[serde(default)]
    pub komi: Option<f32>,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub ownership: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 2718);
        assert_eq!(config.katago.katago_path, "./katago");
        assert_eq!(config.katago.model_path, "./model.bin.gz");
        assert_eq!(config.katago.config_path, "./gtp_config.cfg");
        assert_eq!(config.katago.move_timeout_secs, 20);
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 2718);
    }

    #[test]
    fn test_katago_config_default() {
        let config = KatagoConfig::default();
        assert_eq!(config.katago_path, "./katago");
        assert_eq!(config.model_path, "./model.bin.gz");
        assert_eq!(config.config_path, "./gtp_config.cfg");
        assert_eq!(config.move_timeout_secs, 20);
    }

    #[test]
    fn test_request_config_default() {
        let config = RequestConfig::default();
        assert!(config.komi.is_none());
        assert!(config.client.is_none());
        assert!(config.request_id.is_none());
        assert!(config.ownership.is_none());
    }

    #[test]
    fn test_request_config_deserialization() {
        let json = r#"{"komi": 7.5, "client": "test-client"}"#;
        let config: RequestConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.komi, Some(7.5));
        assert_eq!(config.client, Some("test-client".to_string()));
        assert!(config.request_id.is_none());
        assert!(config.ownership.is_none());
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("KATAGO_SERVER_HOST", "127.0.0.1");
        std::env::set_var("KATAGO_SERVER_PORT", "3000");
        std::env::set_var("KATAGO_KATAGO_PATH", "/usr/bin/katago");
        std::env::set_var("KATAGO_MODEL_PATH", "/models/best.bin.gz");
        std::env::set_var("KATAGO_CONFIG_PATH", "/config/gtp.cfg");
        std::env::set_var("KATAGO_MOVE_TIMEOUT_SECS", "30");

        let config = Config::from_env().unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.katago.katago_path, "/usr/bin/katago");
        assert_eq!(config.katago.model_path, "/models/best.bin.gz");
        assert_eq!(config.katago.config_path, "/config/gtp.cfg");
        assert_eq!(config.katago.move_timeout_secs, 30);

        // Cleanup
        std::env::remove_var("KATAGO_SERVER_HOST");
        std::env::remove_var("KATAGO_SERVER_PORT");
        std::env::remove_var("KATAGO_KATAGO_PATH");
        std::env::remove_var("KATAGO_MODEL_PATH");
        std::env::remove_var("KATAGO_CONFIG_PATH");
        std::env::remove_var("KATAGO_MOVE_TIMEOUT_SECS");
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
[server]
host = "localhost"
port = 8080

[katago]
katago_path = "/custom/katago"
model_path = "/custom/model.bin.gz"
config_path = "/custom/config.cfg"
move_timeout_secs = 15
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.katago.katago_path, "/custom/katago");
        assert_eq!(config.katago.model_path, "/custom/model.bin.gz");
        assert_eq!(config.katago.config_path, "/custom/config.cfg");
        assert_eq!(config.katago.move_timeout_secs, 15);
    }

    #[test]
    fn test_partial_toml_with_defaults() {
        let toml_str = r#"
[katago]
model_path = "/custom/model.bin.gz"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.host, "0.0.0.0"); // default
        assert_eq!(config.server.port, 2718); // default
        assert_eq!(config.katago.model_path, "/custom/model.bin.gz");
        assert_eq!(config.katago.katago_path, "./katago"); // default
    }
}
