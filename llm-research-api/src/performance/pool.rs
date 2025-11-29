//! Connection pool configuration and management for PostgreSQL and ClickHouse.
//!
//! This module provides optimized connection pool configurations with health monitoring,
//! statistics tracking, and configurable settings for both PostgreSQL and ClickHouse databases.
//!
//! # Examples
//!
//! ```no_run
//! use llm_research_api::performance::pool::{PoolConfig, create_postgres_pool};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = PoolConfig::builder()
//!         .min_connections(5)
//!         .max_connections(20)
//!         .build();
//!
//!     let pool = create_postgres_pool("postgresql://localhost/db", config).await?;
//!     Ok(())
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during pool operations.
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Failed to create connection pool: {0}")]
    CreationError(String),

    #[error("Failed to acquire connection: {0}")]
    AcquireError(String),

    #[error("Connection health check failed: {0}")]
    HealthCheckError(String),

    #[error("Invalid pool configuration: {0}")]
    ConfigurationError(String),

    #[error("Pool statistics unavailable")]
    StatisticsUnavailable,
}

/// Configuration for connection pools.
///
/// Provides fine-grained control over connection pool behavior including
/// connection limits, timeouts, and lifecycle management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain in the pool.
    pub min_connections: u32,

    /// Maximum number of connections allowed in the pool.
    pub max_connections: u32,

    /// Timeout for acquiring a connection from the pool.
    pub acquire_timeout: Duration,

    /// Maximum time a connection can remain idle before being closed.
    pub idle_timeout: Option<Duration>,

    /// Maximum lifetime of a connection before it is closed and replaced.
    pub max_lifetime: Option<Duration>,

    /// Enable test on acquire to verify connection health.
    pub test_on_acquire: bool,

    /// Enable test on release to verify connection health before returning to pool.
    pub test_on_release: bool,

    /// Interval for running background health checks.
    pub health_check_interval: Option<Duration>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            max_lifetime: Some(Duration::from_secs(1800)), // 30 minutes
            test_on_acquire: true,
            test_on_release: false,
            health_check_interval: Some(Duration::from_secs(60)),
        }
    }
}

/// Builder for creating PoolConfig instances.
///
/// Provides a fluent interface for configuring connection pools.
#[derive(Debug, Default)]
pub struct PoolConfigBuilder {
    min_connections: Option<u32>,
    max_connections: Option<u32>,
    acquire_timeout: Option<Duration>,
    idle_timeout: Option<Duration>,
    max_lifetime: Option<Duration>,
    test_on_acquire: Option<bool>,
    test_on_release: Option<bool>,
    health_check_interval: Option<Duration>,
}

impl PoolConfigBuilder {
    /// Creates a new PoolConfigBuilder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum number of connections.
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = Some(min);
        self
    }

    /// Sets the maximum number of connections.
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }

    /// Sets the connection acquisition timeout.
    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = Some(timeout);
        self
    }

    /// Sets the idle timeout for connections.
    pub fn idle_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Sets the maximum lifetime for connections.
    pub fn max_lifetime(mut self, lifetime: Option<Duration>) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Enables or disables test on acquire.
    pub fn test_on_acquire(mut self, enabled: bool) -> Self {
        self.test_on_acquire = Some(enabled);
        self
    }

    /// Enables or disables test on release.
    pub fn test_on_release(mut self, enabled: bool) -> Self {
        self.test_on_release = Some(enabled);
        self
    }

    /// Sets the health check interval.
    pub fn health_check_interval(mut self, interval: Option<Duration>) -> Self {
        self.health_check_interval = interval;
        self
    }

    /// Builds the PoolConfig instance.
    ///
    /// # Errors
    ///
    /// Returns `PoolError::ConfigurationError` if configuration is invalid.
    pub fn build(self) -> Result<PoolConfig, PoolError> {
        let default = PoolConfig::default();

        let min = self.min_connections.unwrap_or(default.min_connections);
        let max = self.max_connections.unwrap_or(default.max_connections);

        if min > max {
            return Err(PoolError::ConfigurationError(
                format!("min_connections ({}) cannot be greater than max_connections ({})", min, max)
            ));
        }

        if max == 0 {
            return Err(PoolError::ConfigurationError(
                "max_connections must be greater than 0".to_string()
            ));
        }

        Ok(PoolConfig {
            min_connections: min,
            max_connections: max,
            acquire_timeout: self.acquire_timeout.unwrap_or(default.acquire_timeout),
            idle_timeout: self.idle_timeout.or(default.idle_timeout),
            max_lifetime: self.max_lifetime.or(default.max_lifetime),
            test_on_acquire: self.test_on_acquire.unwrap_or(default.test_on_acquire),
            test_on_release: self.test_on_release.unwrap_or(default.test_on_release),
            health_check_interval: self.health_check_interval.or(default.health_check_interval),
        })
    }
}

impl PoolConfig {
    /// Creates a new PoolConfigBuilder.
    pub fn builder() -> PoolConfigBuilder {
        PoolConfigBuilder::new()
    }

    /// Creates a configuration optimized for high-throughput workloads.
    pub fn high_throughput() -> Self {
        Self {
            min_connections: 10,
            max_connections: 50,
            acquire_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(1200)),
            test_on_acquire: true,
            test_on_release: false,
            health_check_interval: Some(Duration::from_secs(30)),
        }
    }

    /// Creates a configuration optimized for low-latency workloads.
    pub fn low_latency() -> Self {
        Self {
            min_connections: 15,
            max_connections: 30,
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(180)),
            max_lifetime: Some(Duration::from_secs(900)),
            test_on_acquire: true,
            test_on_release: false,
            health_check_interval: Some(Duration::from_secs(20)),
        }
    }

    /// Creates a configuration optimized for development environments.
    pub fn development() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            acquire_timeout: Duration::from_secs(60),
            idle_timeout: Some(Duration::from_secs(1200)),
            max_lifetime: None,
            test_on_acquire: false,
            test_on_release: false,
            health_check_interval: Some(Duration::from_secs(120)),
        }
    }
}

/// Statistics for a connection pool.
///
/// Provides insights into pool health and performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    /// Number of connections currently in use.
    pub active_connections: u32,

    /// Number of idle connections in the pool.
    pub idle_connections: u32,

    /// Number of requests waiting for a connection.
    pub waiting_connections: u32,

    /// Total number of connections (active + idle).
    pub total_connections: u32,

    /// Maximum number of connections allowed.
    pub max_connections: u32,

    /// Timestamp when statistics were collected.
    pub collected_at: DateTime<Utc>,

    /// Average time to acquire a connection (milliseconds).
    pub avg_acquire_time_ms: Option<f64>,

    /// Total number of connections created since pool creation.
    pub total_created: u64,

    /// Total number of connections closed since pool creation.
    pub total_closed: u64,
}

impl PoolStatistics {
    /// Calculates the pool utilization percentage.
    pub fn utilization_percent(&self) -> f64 {
        if self.max_connections == 0 {
            0.0
        } else {
            (self.active_connections as f64 / self.max_connections as f64) * 100.0
        }
    }

    /// Checks if the pool is under pressure (high utilization with waiting connections).
    pub fn is_under_pressure(&self) -> bool {
        self.utilization_percent() > 80.0 && self.waiting_connections > 0
    }

    /// Checks if the pool is healthy.
    pub fn is_healthy(&self) -> bool {
        self.total_connections <= self.max_connections && !self.is_under_pressure()
    }
}

/// Health status of a connection pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PoolHealth {
    /// Pool is operating normally.
    Healthy,

    /// Pool is degraded but functional.
    Degraded,

    /// Pool is unhealthy and may not function properly.
    Unhealthy,
}

/// Connection pool monitor for tracking health and statistics.
pub struct PoolMonitor {
    statistics_history: Arc<RwLock<Vec<PoolStatistics>>>,
    max_history_size: usize,
}

impl PoolMonitor {
    /// Creates a new PoolMonitor.
    pub fn new(max_history_size: usize) -> Self {
        Self {
            statistics_history: Arc::new(RwLock::new(Vec::with_capacity(max_history_size))),
            max_history_size,
        }
    }

    /// Records pool statistics.
    pub async fn record_statistics(&self, stats: PoolStatistics) {
        let mut history = self.statistics_history.write().await;
        history.push(stats);

        // Keep only the most recent statistics
        if history.len() > self.max_history_size {
            let excess = history.len() - self.max_history_size;
            history.drain(0..excess);
        }
    }

    /// Gets the most recent statistics.
    pub async fn get_latest_statistics(&self) -> Option<PoolStatistics> {
        let history = self.statistics_history.read().await;
        history.last().cloned()
    }

    /// Gets all statistics history.
    pub async fn get_statistics_history(&self) -> Vec<PoolStatistics> {
        let history = self.statistics_history.read().await;
        history.clone()
    }

    /// Determines the health status of the pool based on recent statistics.
    pub async fn get_health_status(&self) -> PoolHealth {
        let stats = match self.get_latest_statistics().await {
            Some(s) => s,
            None => return PoolHealth::Unhealthy,
        };

        if !stats.is_healthy() {
            return PoolHealth::Unhealthy;
        }

        if stats.utilization_percent() > 70.0 {
            return PoolHealth::Degraded;
        }

        PoolHealth::Healthy
    }

    /// Clears the statistics history.
    pub async fn clear_history(&self) {
        let mut history = self.statistics_history.write().await;
        history.clear();
    }
}

/// Creates a PostgreSQL connection pool with the given configuration.
///
/// # Arguments
///
/// * `database_url` - The PostgreSQL connection URL
/// * `config` - Pool configuration settings
///
/// # Errors
///
/// Returns `PoolError::CreationError` if the pool cannot be created.
///
/// # Examples
///
/// ```no_run
/// use llm_research_api::performance::pool::{PoolConfig, create_postgres_pool};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = PoolConfig::default();
///     let pool = create_postgres_pool("postgresql://localhost/db", config).await?;
///     Ok(())
/// }
/// ```
pub async fn create_postgres_pool(
    database_url: &str,
    config: PoolConfig,
) -> Result<PgPool, PoolError> {
    let mut options = PgPoolOptions::new()
        .min_connections(config.min_connections)
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .test_before_acquire(config.test_on_acquire);

    if let Some(idle_timeout) = config.idle_timeout {
        options = options.idle_timeout(idle_timeout);
    }

    if let Some(max_lifetime) = config.max_lifetime {
        options = options.max_lifetime(max_lifetime);
    }

    options
        .connect(database_url)
        .await
        .map_err(|e| PoolError::CreationError(e.to_string()))
}

/// Gets statistics from a PostgreSQL pool.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL pool
///
/// # Returns
///
/// Pool statistics including active, idle, and waiting connections.
pub fn get_postgres_pool_stats(pool: &PgPool) -> PoolStatistics {
    let size = pool.size();
    let num_idle = pool.num_idle() as u32;

    PoolStatistics {
        active_connections: size.saturating_sub(num_idle),
        idle_connections: num_idle,
        waiting_connections: 0, // Not available in sqlx
        total_connections: size,
        max_connections: pool.options().get_max_connections(),
        collected_at: Utc::now(),
        avg_acquire_time_ms: None,
        total_created: 0,
        total_closed: 0,
    }
}

/// Creates a ClickHouse connection pool configuration.
///
/// # Arguments
///
/// * `url` - The ClickHouse connection URL
/// * `config` - Pool configuration settings
///
/// # Returns
///
/// A configured ClickHouse client.
///
/// # Errors
///
/// Returns `PoolError::CreationError` if the client cannot be created.
pub fn create_clickhouse_pool(
    url: &str,
    config: PoolConfig,
) -> Result<clickhouse::Client, PoolError> {
    Ok(clickhouse::Client::default()
        .with_url(url)
        .with_compression(clickhouse::Compression::Lz4)
        .with_option("max_connections", config.max_connections.to_string())
        .with_option("min_connections", config.min_connections.to_string())
        .with_option("connection_timeout", format!("{}s", config.acquire_timeout.as_secs())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
        assert!(config.test_on_acquire);
        assert!(!config.test_on_release);
    }

    #[test]
    fn test_pool_config_builder_basic() {
        let config = PoolConfig::builder()
            .min_connections(10)
            .max_connections(50)
            .build()
            .unwrap();

        assert_eq!(config.min_connections, 10);
        assert_eq!(config.max_connections, 50);
    }

    #[test]
    fn test_pool_config_builder_all_options() {
        let config = PoolConfig::builder()
            .min_connections(5)
            .max_connections(25)
            .acquire_timeout(Duration::from_secs(15))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(900)))
            .test_on_acquire(false)
            .test_on_release(true)
            .health_check_interval(Some(Duration::from_secs(45)))
            .build()
            .unwrap();

        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 25);
        assert_eq!(config.acquire_timeout, Duration::from_secs(15));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(300)));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(900)));
        assert!(!config.test_on_acquire);
        assert!(config.test_on_release);
        assert_eq!(config.health_check_interval, Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_pool_config_builder_validation_min_greater_than_max() {
        let result = PoolConfig::builder()
            .min_connections(30)
            .max_connections(20)
            .build();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PoolError::ConfigurationError(_)));
    }

    #[test]
    fn test_pool_config_builder_validation_zero_max() {
        let result = PoolConfig::builder()
            .max_connections(0)
            .build();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PoolError::ConfigurationError(_)));
    }

    #[test]
    fn test_pool_config_high_throughput() {
        let config = PoolConfig::high_throughput();
        assert_eq!(config.min_connections, 10);
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.acquire_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_pool_config_low_latency() {
        let config = PoolConfig::low_latency();
        assert_eq!(config.min_connections, 15);
        assert_eq!(config.max_connections, 30);
        assert_eq!(config.acquire_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_pool_config_development() {
        let config = PoolConfig::development();
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.max_lifetime, None);
    }

    #[test]
    fn test_pool_statistics_utilization_percent() {
        let stats = PoolStatistics {
            active_connections: 8,
            idle_connections: 2,
            waiting_connections: 0,
            total_connections: 10,
            max_connections: 10,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 10,
            total_closed: 0,
        };

        assert_eq!(stats.utilization_percent(), 80.0);
    }

    #[test]
    fn test_pool_statistics_utilization_percent_zero_max() {
        let stats = PoolStatistics {
            active_connections: 0,
            idle_connections: 0,
            waiting_connections: 0,
            total_connections: 0,
            max_connections: 0,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 0,
            total_closed: 0,
        };

        assert_eq!(stats.utilization_percent(), 0.0);
    }

    #[test]
    fn test_pool_statistics_is_under_pressure() {
        let stats = PoolStatistics {
            active_connections: 18,
            idle_connections: 0,
            waiting_connections: 5,
            total_connections: 18,
            max_connections: 20,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 18,
            total_closed: 0,
        };

        assert!(stats.is_under_pressure());
    }

    #[test]
    fn test_pool_statistics_is_healthy() {
        let stats = PoolStatistics {
            active_connections: 5,
            idle_connections: 5,
            waiting_connections: 0,
            total_connections: 10,
            max_connections: 20,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 10,
            total_closed: 0,
        };

        assert!(stats.is_healthy());
    }

    #[tokio::test]
    async fn test_pool_monitor_new() {
        let monitor = PoolMonitor::new(100);
        assert_eq!(monitor.max_history_size, 100);
        assert!(monitor.get_latest_statistics().await.is_none());
    }

    #[tokio::test]
    async fn test_pool_monitor_record_statistics() {
        let monitor = PoolMonitor::new(100);
        let stats = PoolStatistics {
            active_connections: 5,
            idle_connections: 5,
            waiting_connections: 0,
            total_connections: 10,
            max_connections: 20,
            collected_at: Utc::now(),
            avg_acquire_time_ms: Some(5.0),
            total_created: 10,
            total_closed: 0,
        };

        monitor.record_statistics(stats.clone()).await;

        let latest = monitor.get_latest_statistics().await.unwrap();
        assert_eq!(latest.active_connections, 5);
    }

    #[tokio::test]
    async fn test_pool_monitor_history_limit() {
        let monitor = PoolMonitor::new(3);

        for i in 0..5 {
            let stats = PoolStatistics {
                active_connections: i,
                idle_connections: 0,
                waiting_connections: 0,
                total_connections: i,
                max_connections: 20,
                collected_at: Utc::now(),
                avg_acquire_time_ms: None,
                total_created: i as u64,
                total_closed: 0,
            };
            monitor.record_statistics(stats).await;
        }

        let history = monitor.get_statistics_history().await;
        assert_eq!(history.len(), 3);
        assert_eq!(history.first().unwrap().active_connections, 2);
        assert_eq!(history.last().unwrap().active_connections, 4);
    }

    #[tokio::test]
    async fn test_pool_monitor_get_health_status_healthy() {
        let monitor = PoolMonitor::new(100);
        let stats = PoolStatistics {
            active_connections: 5,
            idle_connections: 15,
            waiting_connections: 0,
            total_connections: 20,
            max_connections: 50,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 20,
            total_closed: 0,
        };

        monitor.record_statistics(stats).await;
        assert_eq!(monitor.get_health_status().await, PoolHealth::Healthy);
    }

    #[tokio::test]
    async fn test_pool_monitor_get_health_status_degraded() {
        let monitor = PoolMonitor::new(100);
        let stats = PoolStatistics {
            active_connections: 36,
            idle_connections: 4,
            waiting_connections: 0,
            total_connections: 40,
            max_connections: 50,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 40,
            total_closed: 0,
        };

        monitor.record_statistics(stats).await;
        assert_eq!(monitor.get_health_status().await, PoolHealth::Degraded);
    }

    #[tokio::test]
    async fn test_pool_monitor_clear_history() {
        let monitor = PoolMonitor::new(100);
        let stats = PoolStatistics {
            active_connections: 5,
            idle_connections: 5,
            waiting_connections: 0,
            total_connections: 10,
            max_connections: 20,
            collected_at: Utc::now(),
            avg_acquire_time_ms: None,
            total_created: 10,
            total_closed: 0,
        };

        monitor.record_statistics(stats).await;
        assert!(monitor.get_latest_statistics().await.is_some());

        monitor.clear_history().await;
        assert!(monitor.get_latest_statistics().await.is_none());
    }
}
