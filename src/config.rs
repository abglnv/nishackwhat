use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub port: u16,
    pub redis_url: String,
    pub key_prefix: String,
    pub scan_interval_secs: u64,
    pub heartbeat_ttl_secs: u64,
    pub banned_sites: Vec<String>,
    pub banned_apps: Vec<String>,
    #[serde(default)]
    pub sau_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 8080,
            redis_url: "redis://127.0.0.1:6379".to_string(),
            key_prefix: "nishack".to_string(),
            scan_interval_secs: 30,
            heartbeat_ttl_secs: 90,
            banned_sites: vec![],
            banned_apps: vec![],
            sau_mode: false,
        }
    }
}

pub fn load_config() -> Config {
    match fs::read_to_string("config.toml") {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config.toml: {e}, using defaults");
            Config::default()
        }),
        Err(_) => {
            tracing::info!("No config.toml found, using defaults");
            Config::default()
        }
    }
}
