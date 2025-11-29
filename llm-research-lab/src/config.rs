use anyhow::Result;
use config::{Config as ConfigLoader, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub clickhouse_url: String,
    pub s3_bucket: String,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config = ConfigLoader::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("LLM_RESEARCH"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3000,
            database_url: "postgres://postgres:postgres@localhost/llm_research".to_string(),
            clickhouse_url: "http://localhost:8123".to_string(),
            s3_bucket: "llm-research-artifacts".to_string(),
            log_level: "info".to_string(),
        }
    }
}
