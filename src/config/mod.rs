pub mod spam_rules;

use serde::Deserialize;
use std::env;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub max_buffer_size: usize,
    pub flush_interval_seconds: u64,
    pub geoip_db_path: String,
    pub ua_regexes_path: String,
    pub rate_limit_per_minute: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::new();
        
        // Load .env file if it exists
        dotenv::dotenv().ok();

        // Set defaults
        cfg.set_default("max_buffer_size", 10000)?;
        cfg.set_default("flush_interval_seconds", 10)?;
        cfg.set_default("rate_limit_per_minute", 60)?;
        cfg.set_default("ua_regexes_path", "config/regexes.yaml")?;

        // Load from environment
        cfg.merge(config::Environment::new())?;

        cfg.try_into()
    }
}

pub fn get_config() -> Config {
    Config::from_env().expect("Failed to load configuration")
} 