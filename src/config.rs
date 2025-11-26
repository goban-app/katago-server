use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
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
            config_path: "./gtp_config.cfg".to_string(),
            move_timeout_secs: 20,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?;
        
        Ok(settings.try_deserialize()?)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("KATAGO"))
            .build()?;
        
        Ok(settings.try_deserialize()?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
