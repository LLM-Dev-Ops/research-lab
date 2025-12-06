//! External Data Source Consumer Adapters
//!
//! This module provides thin adapter layers for consuming data from external
//! LLM-Dev-Ops ecosystem services. These adapters are additive and do not
//! modify any existing research modules, evaluation metrics, scoring systems,
//! or experiment templates.
//!
//! # Architecture
//!
//! Each consumer adapter implements a trait-based interface for consuming
//! specific types of data from upstream services:
//!
//! - `SimulatorConsumer`: Consumes simulation outputs and synthetic evaluation runs
//! - `ObservatoryConsumer`: Consumes telemetry, experiment traces, and research metrics
//! - `BenchmarkExchangeConsumer`: Consumes community benchmarks and standardized scoring sets
//! - `DataVaultConsumer`: Consumes stored datasets and lineage-aware research artifacts
//! - `TestBenchIngester`: Runtime-only file/SDK-based benchmark ingestion (no compile-time dep)
//!
//! # Usage
//!
//! ```rust,ignore
//! use adapters::consumers::{SimulatorConsumer, SimulatorClient};
//!
//! let client = SimulatorClient::new(config);
//! let outputs = client.consume_simulation_outputs(run_id).await?;
//! ```

pub mod simulator;
pub mod observatory;
pub mod benchmark_exchange;
pub mod data_vault;
pub mod test_bench;

pub use simulator::*;
pub use observatory::*;
pub use benchmark_exchange::*;
pub use data_vault::*;
pub use test_bench::*;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

/// Common configuration for external service connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceConfig {
    /// Base URL or endpoint for the service
    pub endpoint: String,
    /// Optional authentication token
    pub auth_token: Option<String>,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for ExternalServiceConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            auth_token: None,
            timeout_ms: 30000,
            max_retries: 3,
        }
    }
}

/// Result type for consumer operations.
pub type ConsumerResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Trait for data source consumers with health check capability.
#[async_trait]
pub trait HealthCheckable: Send + Sync {
    /// Check if the external service is reachable and healthy.
    async fn health_check(&self) -> ConsumerResult<bool>;
}

/// Metadata about consumed data for lineage tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumptionMetadata {
    /// Source service identifier
    pub source: String,
    /// Timestamp of consumption (ISO 8601)
    pub consumed_at: String,
    /// Version or revision of consumed data
    pub version: Option<String>,
    /// Checksum for integrity verification
    pub checksum: Option<String>,
    /// Additional metadata fields
    pub extra: Option<Value>,
}

impl ConsumptionMetadata {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            consumed_at: chrono::Utc::now().to_rfc3339(),
            version: None,
            checksum: None,
            extra: None,
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    pub fn with_checksum(mut self, checksum: &str) -> Self {
        self.checksum = Some(checksum.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ExternalServiceConfig::default();
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_consumption_metadata() {
        let meta = ConsumptionMetadata::new("test-source")
            .with_version("1.0.0")
            .with_checksum("abc123");

        assert_eq!(meta.source, "test-source");
        assert_eq!(meta.version, Some("1.0.0".to_string()));
        assert_eq!(meta.checksum, Some("abc123".to_string()));
    }
}
