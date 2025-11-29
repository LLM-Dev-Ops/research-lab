//! Bulkhead pattern implementation for resource isolation and fault tolerance.
//!
//! The bulkhead pattern isolates different parts of the system to prevent cascade
//! failures. This implementation uses semaphores to limit concurrent requests and
//! provides per-service isolation.
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::reliability::bulkhead::*;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), BulkheadError> {
//! let config = BulkheadConfig {
//!     max_concurrent: 10,
//!     max_queue_size: 100,
//!     timeout: Duration::from_secs(30),
//! };
//!
//! let bulkhead = Bulkhead::new("database", config);
//!
//! // Execute within bulkhead
//! let result = bulkhead.execute(|| async {
//!     // Your async operation here
//!     Ok::<_, BulkheadError>(42)
//! }).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use dashmap::DashMap;
use metrics::{counter, gauge, histogram};
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::{Semaphore, SemaphorePermit};
use tracing::{debug, warn};

/// Bulkhead-related errors
#[derive(Error, Debug, Clone)]
pub enum BulkheadError {
    #[error("Request rejected: bulkhead is full")]
    Rejected,

    #[error("Queue is full, cannot accept more requests")]
    QueueFull,

    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Configuration for a bulkhead
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    /// Maximum number of concurrent requests allowed
    pub max_concurrent: usize,
    /// Maximum size of the waiting queue
    pub max_queue_size: usize,
    /// Timeout for acquiring a permit
    pub timeout: Duration,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 25,
            max_queue_size: 100,
            timeout: Duration::from_secs(30),
        }
    }
}

impl BulkheadConfig {
    /// Creates a config for a small bulkhead (limited resources)
    pub fn small() -> Self {
        Self {
            max_concurrent: 10,
            max_queue_size: 50,
            timeout: Duration::from_secs(10),
        }
    }

    /// Creates a config for a large bulkhead (more resources)
    pub fn large() -> Self {
        Self {
            max_concurrent: 100,
            max_queue_size: 500,
            timeout: Duration::from_secs(60),
        }
    }

    /// Sets the maximum concurrent requests
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Sets the maximum queue size
    pub fn with_max_queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = size;
        self
    }

    /// Sets the timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Request priority for optional priority-based queuing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    /// Low priority requests (can be shed under load)
    Low = 0,
    /// Normal priority requests
    Normal = 1,
    /// High priority requests (preferred during load)
    High = 2,
    /// Critical requests (must be processed)
    Critical = 3,
}

impl Default for RequestPriority {
    fn default() -> Self {
        RequestPriority::Normal
    }
}

/// Metrics for a bulkhead
#[derive(Debug, Default, Clone)]
pub struct BulkheadMetrics {
    /// Number of active requests
    pub active: usize,
    /// Number of queued requests
    pub queued: usize,
    /// Number of rejected requests
    pub rejected: u64,
    /// Number of timed out requests
    pub timeouts: u64,
    /// Number of successful completions
    pub successes: u64,
    /// Number of failures
    pub failures: u64,
}

/// Bulkhead implementation using semaphores for concurrency control
pub struct Bulkhead {
    /// Name of this bulkhead (for metrics and logging)
    name: String,
    /// Configuration
    config: BulkheadConfig,
    /// Semaphore for controlling concurrent access
    semaphore: Arc<Semaphore>,
    /// Metrics tracking
    metrics: Arc<DashMap<String, u64>>,
}

impl Bulkhead {
    /// Creates a new bulkhead with the given configuration
    pub fn new(name: impl Into<String>, config: BulkheadConfig) -> Self {
        let name = name.into();
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

        Self {
            name,
            config,
            semaphore,
            metrics: Arc::new(DashMap::new()),
        }
    }

    /// Returns the name of this bulkhead
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the configuration
    pub fn config(&self) -> &BulkheadConfig {
        &self.config
    }

    /// Returns current metrics
    pub fn metrics(&self) -> BulkheadMetrics {
        let active = self.config.max_concurrent - self.semaphore.available_permits();
        let queued = 0; // Semaphore doesn't expose queue size directly

        BulkheadMetrics {
            active,
            queued,
            rejected: self.get_metric("rejected"),
            timeouts: self.get_metric("timeouts"),
            successes: self.get_metric("successes"),
            failures: self.get_metric("failures"),
        }
    }

    /// Gets a metric value
    fn get_metric(&self, key: &str) -> u64 {
        self.metrics.get(key).map(|v| *v).unwrap_or(0)
    }

    /// Increments a metric counter
    fn increment_metric(&self, key: &str) {
        self.metrics
            .entry(key.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(1);

        // Also emit to global metrics system
        counter!(format!("bulkhead_{}", key), &[("name", self.name.clone())]).increment(1);
    }

    /// Records current gauge values
    fn record_gauges(&self) {
        let metrics = self.metrics();
        gauge!(format!("bulkhead_active"), &[("name", self.name.clone())])
            .set(metrics.active as f64);
        gauge!(format!("bulkhead_queued"), &[("name", self.name.clone())])
            .set(metrics.queued as f64);
    }

    /// Acquires a permit with timeout
    async fn acquire_permit(&self) -> Result<SemaphorePermit, BulkheadError> {
        // Check if we're over queue limit
        let waiting = self.config.max_concurrent - self.semaphore.available_permits();
        if waiting >= self.config.max_queue_size {
            warn!("Bulkhead '{}' queue is full", self.name);
            self.increment_metric("rejected");
            return Err(BulkheadError::QueueFull);
        }

        debug!("Acquiring permit for bulkhead '{}'", self.name);

        // Try to acquire with timeout
        match tokio::time::timeout(
            self.config.timeout,
            self.semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(permit)) => {
                self.record_gauges();
                Ok(permit)
            }
            Ok(Err(_)) => {
                // Semaphore was closed (shouldn't happen in our case)
                warn!("Bulkhead '{}' semaphore closed", self.name);
                self.increment_metric("rejected");
                Err(BulkheadError::Rejected)
            }
            Err(_) => {
                warn!("Bulkhead '{}' request timed out", self.name);
                self.increment_metric("timeouts");
                Err(BulkheadError::Timeout(self.config.timeout))
            }
        }
    }

    /// Executes an async operation within the bulkhead
    pub async fn execute<F, Fut, T, E>(&self, f: F) -> Result<T, BulkheadError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let start = Instant::now();
        let _permit = self.acquire_permit().await?;

        debug!("Executing operation in bulkhead '{}'", self.name);

        let result = f().await;
        let duration = start.elapsed();

        histogram!(
            format!("bulkhead_duration_seconds"),
            &[("name", self.name.clone())]
        )
        .record(duration.as_secs_f64());

        match result {
            Ok(value) => {
                self.increment_metric("successes");
                self.record_gauges();
                Ok(value)
            }
            Err(e) => {
                warn!("Operation failed in bulkhead '{}': {}", self.name, e);
                self.increment_metric("failures");
                self.record_gauges();
                Err(BulkheadError::OperationFailed(e.to_string()))
            }
        }
    }

    /// Tries to execute without waiting if a permit is immediately available
    pub async fn try_execute<F, Fut, T, E>(&self, f: F) -> Result<T, BulkheadError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let permit = self.semaphore.try_acquire().map_err(|_| {
            self.increment_metric("rejected");
            BulkheadError::Rejected
        })?;

        let start = Instant::now();
        debug!("Executing operation in bulkhead '{}' (no wait)", self.name);

        let result = f().await;
        let duration = start.elapsed();

        drop(permit);

        histogram!(
            format!("bulkhead_duration_seconds"),
            &[("name", self.name.clone())]
        )
        .record(duration.as_secs_f64());

        match result {
            Ok(value) => {
                self.increment_metric("successes");
                self.record_gauges();
                Ok(value)
            }
            Err(e) => {
                warn!("Operation failed in bulkhead '{}': {}", self.name, e);
                self.increment_metric("failures");
                self.record_gauges();
                Err(BulkheadError::OperationFailed(e.to_string()))
            }
        }
    }

    /// Returns the number of available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

impl fmt::Debug for Bulkhead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bulkhead")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("available_permits", &self.available_permits())
            .finish()
    }
}

/// Registry for managing multiple bulkheads
pub struct BulkheadRegistry {
    bulkheads: DashMap<String, Arc<Bulkhead>>,
}

impl BulkheadRegistry {
    /// Creates a new registry
    pub fn new() -> Self {
        Self {
            bulkheads: DashMap::new(),
        }
    }

    /// Registers a new bulkhead
    pub fn register(&self, name: impl Into<String>, config: BulkheadConfig) -> Arc<Bulkhead> {
        let name = name.into();
        let bulkhead = Arc::new(Bulkhead::new(name.clone(), config));
        self.bulkheads.insert(name, Arc::clone(&bulkhead));
        bulkhead
    }

    /// Gets a bulkhead by name
    pub fn get(&self, name: &str) -> Option<Arc<Bulkhead>> {
        self.bulkheads.get(name).map(|b| Arc::clone(&*b))
    }

    /// Gets or creates a bulkhead with default config
    pub fn get_or_create(&self, name: impl Into<String>) -> Arc<Bulkhead> {
        let name = name.into();
        if let Some(bulkhead) = self.get(&name) {
            bulkhead
        } else {
            self.register(name, BulkheadConfig::default())
        }
    }

    /// Lists all registered bulkheads
    pub fn list(&self) -> Vec<String> {
        self.bulkheads.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Gets metrics for all bulkheads
    pub fn all_metrics(&self) -> Vec<(String, BulkheadMetrics)> {
        self.bulkheads
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().metrics()))
            .collect()
    }
}

impl Default for BulkheadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper function to execute code within a bulkhead
pub async fn with_bulkhead<F, Fut, T, E>(
    bulkhead: &Bulkhead,
    f: F,
) -> Result<T, BulkheadError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    bulkhead.execute(f).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_bulkhead_config_default() {
        let config = BulkheadConfig::default();
        assert_eq!(config.max_concurrent, 25);
        assert_eq!(config.max_queue_size, 100);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_bulkhead_config_small() {
        let config = BulkheadConfig::small();
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.max_queue_size, 50);
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_bulkhead_config_large() {
        let config = BulkheadConfig::large();
        assert_eq!(config.max_concurrent, 100);
        assert_eq!(config.max_queue_size, 500);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_bulkhead_config_builders() {
        let config = BulkheadConfig::default()
            .with_max_concurrent(50)
            .with_max_queue_size(200)
            .with_timeout(Duration::from_secs(45));

        assert_eq!(config.max_concurrent, 50);
        assert_eq!(config.max_queue_size, 200);
        assert_eq!(config.timeout, Duration::from_secs(45));
    }

    #[test]
    fn test_request_priority_ordering() {
        assert!(RequestPriority::Critical > RequestPriority::High);
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
    }

    #[test]
    fn test_bulkhead_new() {
        let config = BulkheadConfig::small();
        let bulkhead = Bulkhead::new("test", config);

        assert_eq!(bulkhead.name(), "test");
        assert_eq!(bulkhead.available_permits(), 10);
    }

    #[tokio::test]
    async fn test_bulkhead_execute_success() {
        let config = BulkheadConfig::small();
        let bulkhead = Bulkhead::new("test", config);

        let result = bulkhead
            .execute(|| async { Ok::<_, std::io::Error>(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let metrics = bulkhead.metrics();
        assert_eq!(metrics.successes, 1);
        assert_eq!(metrics.failures, 0);
    }

    #[tokio::test]
    async fn test_bulkhead_execute_failure() {
        let config = BulkheadConfig::small();
        let bulkhead = Bulkhead::new("test", config);

        let result = bulkhead
            .execute(|| async {
                Err::<i32, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "test error",
                ))
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(BulkheadError::OperationFailed(_))));

        let metrics = bulkhead.metrics();
        assert_eq!(metrics.failures, 1);
        assert_eq!(metrics.successes, 0);
    }

    #[tokio::test]
    async fn test_bulkhead_concurrency_limit() {
        let config = BulkheadConfig {
            max_concurrent: 2,
            max_queue_size: 0,
            timeout: Duration::from_millis(500),
        };
        let bulkhead = Arc::new(Bulkhead::new("test", config));

        let counter = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(AtomicUsize::new(0));

        // Start 2 operations that hold the permits
        let c1 = Arc::clone(&counter);
        let s1 = Arc::clone(&started);
        let b1 = Arc::clone(&bulkhead);
        let h1 = tokio::spawn(async move {
            b1.execute(|| async {
                s1.fetch_add(1, Ordering::SeqCst);
                c1.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(300)).await;
                c1.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(())
            })
            .await
        });

        let c2 = Arc::clone(&counter);
        let s2 = Arc::clone(&started);
        let b2 = Arc::clone(&bulkhead);
        let h2 = tokio::spawn(async move {
            b2.execute(|| async {
                s2.fetch_add(1, Ordering::SeqCst);
                c2.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(300)).await;
                c2.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(())
            })
            .await
        });

        // Wait until both operations have started (acquired permits)
        while started.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Third request should be rejected because all permits are taken
        let result = bulkhead
            .try_execute(|| async { Ok::<_, std::io::Error>(()) })
            .await;

        assert!(matches!(result, Err(BulkheadError::Rejected)));

        // Wait for original operations to complete
        let _ = h1.await;
        let _ = h2.await;

        // Now it should work
        let result = bulkhead
            .try_execute(|| async { Ok::<_, std::io::Error>(()) })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulkhead_metrics() {
        let config = BulkheadConfig::small();
        let bulkhead = Bulkhead::new("test", config);

        // Success
        let _ = bulkhead
            .execute(|| async { Ok::<_, std::io::Error>(1) })
            .await;

        // Failure
        let _ = bulkhead
            .execute(|| async {
                Err::<i32, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "error",
                ))
            })
            .await;

        let metrics = bulkhead.metrics();
        assert_eq!(metrics.successes, 1);
        assert_eq!(metrics.failures, 1);
        assert_eq!(metrics.active, 0);
    }

    #[tokio::test]
    async fn test_bulkhead_timeout() {
        let config = BulkheadConfig {
            max_concurrent: 1,
            max_queue_size: 10, // Allow queuing
            timeout: Duration::from_millis(100),
        };
        let bulkhead = Arc::new(Bulkhead::new("test", config));

        let started = Arc::new(AtomicUsize::new(0));

        // Hold the permit
        let s1 = Arc::clone(&started);
        let b1 = Arc::clone(&bulkhead);
        let h1 = tokio::spawn(async move {
            b1.execute(|| async {
                s1.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok::<_, std::io::Error>(())
            })
            .await
        });

        // Wait until the first operation has acquired the permit
        while started.load(Ordering::SeqCst) < 1 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // This should timeout waiting for the permit (100ms timeout, first op holds for 500ms)
        let result = bulkhead
            .execute(|| async { Ok::<_, std::io::Error>(()) })
            .await;

        assert!(matches!(result, Err(BulkheadError::Timeout(_))));

        let _ = h1.await;
    }

    #[test]
    fn test_bulkhead_registry_new() {
        let registry = BulkheadRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_bulkhead_registry_register() {
        let registry = BulkheadRegistry::new();
        let bulkhead = registry.register("test", BulkheadConfig::default());

        assert_eq!(bulkhead.name(), "test");
        assert_eq!(registry.list().len(), 1);
        assert!(registry.list().contains(&"test".to_string()));
    }

    #[test]
    fn test_bulkhead_registry_get() {
        let registry = BulkheadRegistry::new();
        registry.register("test", BulkheadConfig::default());

        let bulkhead = registry.get("test");
        assert!(bulkhead.is_some());
        assert_eq!(bulkhead.unwrap().name(), "test");

        let missing = registry.get("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_bulkhead_registry_get_or_create() {
        let registry = BulkheadRegistry::new();

        let bulkhead1 = registry.get_or_create("test");
        assert_eq!(bulkhead1.name(), "test");

        let bulkhead2 = registry.get_or_create("test");
        assert_eq!(bulkhead2.name(), "test");

        // Should be the same instance
        assert_eq!(registry.list().len(), 1);
    }

    #[tokio::test]
    async fn test_with_bulkhead_wrapper() {
        let bulkhead = Bulkhead::new("test", BulkheadConfig::default());

        let result = with_bulkhead(&bulkhead, || async { Ok::<_, std::io::Error>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
