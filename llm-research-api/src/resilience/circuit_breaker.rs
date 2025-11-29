//! Circuit Breaker implementation for preventing cascading failures.
//!
//! A circuit breaker monitors for failures and temporarily blocks requests when failures
//! exceed a threshold, allowing the system time to recover.
//!
//! # States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Too many failures detected, requests are rejected
//! - **HalfOpen**: Testing if the system has recovered
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     success_threshold: 2,
//!     timeout: Duration::from_secs(60),
//!     half_open_max_requests: 3,
//! };
//!
//! let breaker = CircuitBreaker::new("my_service", config);
//!
//! match breaker.call(|| async { Ok::<_, std::io::Error>(42) }).await {
//!     Ok(result) => println!("Success: {}", result),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation, requests pass through
    Closed,
    /// Too many failures, requests are rejected
    Open,
    /// Testing if the system has recovered
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// Configuration for circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: usize,
    /// Number of consecutive successes in half-open state to close circuit
    pub success_threshold: usize,
    /// Time to wait before transitioning from open to half-open
    pub timeout: Duration,
    /// Maximum number of requests allowed in half-open state
    pub half_open_max_requests: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

/// Circuit breaker errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected
    #[error("Circuit breaker is open for {name}")]
    Open { name: String },

    /// Request rejected (too many in-flight in half-open state)
    #[error("Circuit breaker rejected request for {name}")]
    Rejected { name: String },

    /// Execution failed with underlying error
    #[error("Execution failed: {0}")]
    ExecutionFailed(E),
}

/// Metrics for circuit breaker
#[derive(Debug, Default)]
struct CircuitBreakerMetrics {
    /// Total number of failures
    failures: AtomicU64,
    /// Total number of successes
    successes: AtomicU64,
    /// Number of times circuit opened
    opened_count: AtomicU64,
    /// Number of times circuit closed
    closed_count: AtomicU64,
    /// Number of rejected requests
    rejected_count: AtomicU64,
}

impl CircuitBreakerMetrics {
    fn record_failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
    }

    fn record_success(&self) {
        self.successes.fetch_add(1, Ordering::Relaxed);
    }

    fn record_opened(&self) {
        self.opened_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_closed(&self) {
        self.closed_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_rejected(&self) {
        self.rejected_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Internal state of the circuit breaker
struct CircuitBreakerState {
    state: CircuitState,
    consecutive_failures: usize,
    consecutive_successes: usize,
    last_failure_time: Option<Instant>,
    half_open_requests: usize,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure_time: None,
            half_open_requests: 0,
        }
    }
}

/// Circuit breaker implementation
///
/// Generic over the result type to support any operation.
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
    metrics: Arc<CircuitBreakerMetrics>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let name = name.into();
        info!("Creating circuit breaker: {}", name);

        Self {
            name,
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState::new())),
            metrics: Arc::new(CircuitBreakerMetrics::default()),
        }
    }

    /// Get the current state of the circuit breaker
    pub async fn state(&self) -> CircuitState {
        self.state.read().await.state
    }

    /// Get metrics
    pub fn metrics(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            failures: self.metrics.failures.load(Ordering::Relaxed),
            successes: self.metrics.successes.load(Ordering::Relaxed),
            opened_count: self.metrics.opened_count.load(Ordering::Relaxed),
            closed_count: self.metrics.closed_count.load(Ordering::Relaxed),
            rejected_count: self.metrics.rejected_count.load(Ordering::Relaxed),
        }
    }

    /// Call a function with circuit breaker protection
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        // Check if we should allow the request
        self.before_call().await?;

        // Execute the function
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_error().await;
                Err(CircuitBreakerError::ExecutionFailed(e))
            }
        }
    }

    /// Check state before allowing a call
    async fn before_call<E>(&self) -> Result<(), CircuitBreakerError<E>> {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::Closed => {
                // Allow the request
                Ok(())
            }
            CircuitState::Open => {
                // Check if we should transition to half-open
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() >= self.config.timeout {
                        info!("Circuit breaker {} transitioning to half-open", self.name);
                        state.state = CircuitState::HalfOpen;
                        state.half_open_requests = 1;
                        state.consecutive_successes = 0;
                        Ok(())
                    } else {
                        self.metrics.record_rejected();
                        Err(CircuitBreakerError::Open {
                            name: self.name.clone(),
                        })
                    }
                } else {
                    self.metrics.record_rejected();
                    Err(CircuitBreakerError::Open {
                        name: self.name.clone(),
                    })
                }
            }
            CircuitState::HalfOpen => {
                // Check if we've exceeded max requests
                if state.half_open_requests >= self.config.half_open_max_requests {
                    self.metrics.record_rejected();
                    Err(CircuitBreakerError::Rejected {
                        name: self.name.clone(),
                    })
                } else {
                    state.half_open_requests += 1;
                    Ok(())
                }
            }
        }
    }

    /// Handle successful execution
    async fn on_success(&self) {
        let mut state = self.state.write().await;
        self.metrics.record_success();

        match state.state {
            CircuitState::Closed => {
                state.consecutive_failures = 0;
            }
            CircuitState::HalfOpen => {
                state.consecutive_successes += 1;
                state.consecutive_failures = 0;

                if state.consecutive_successes >= self.config.success_threshold {
                    info!("Circuit breaker {} closing after {} successes",
                          self.name, state.consecutive_successes);
                    state.state = CircuitState::Closed;
                    state.consecutive_successes = 0;
                    state.half_open_requests = 0;
                    self.metrics.record_closed();
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                state.consecutive_failures = 0;
            }
        }
    }

    /// Handle failed execution
    async fn on_error(&self) {
        let mut state = self.state.write().await;
        self.metrics.record_failure();

        state.consecutive_failures += 1;
        state.consecutive_successes = 0;
        state.last_failure_time = Some(Instant::now());

        match state.state {
            CircuitState::Closed => {
                if state.consecutive_failures >= self.config.failure_threshold {
                    warn!("Circuit breaker {} opening after {} failures",
                          self.name, state.consecutive_failures);
                    state.state = CircuitState::Open;
                    self.metrics.record_opened();
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker {} re-opening due to failure in half-open state",
                      self.name);
                state.state = CircuitState::Open;
                state.half_open_requests = 0;
                self.metrics.record_opened();
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }

    /// Reset the circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        info!("Manually resetting circuit breaker: {}", self.name);
        state.state = CircuitState::Closed;
        state.consecutive_failures = 0;
        state.consecutive_successes = 0;
        state.half_open_requests = 0;
        state.last_failure_time = None;
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub failures: u64,
    pub successes: u64,
    pub opened_count: u64,
    pub closed_count: u64,
    pub rejected_count: u64,
}

/// Convenience function to wrap an operation with a circuit breaker
pub async fn with_circuit_breaker<F, Fut, T, E>(
    breaker: &CircuitBreaker,
    f: F,
) -> Result<T, CircuitBreakerError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    breaker.call(f).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_starts_closed() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test", config);

        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_successful_calls_keep_circuit_closed() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test", config);

        for _ in 0..10 {
            let result = breaker.call(|| async { Ok::<_, ()>(42) }).await;
            assert!(result.is_ok());
            assert_eq!(breaker.state().await, CircuitState::Closed);
        }

        let stats = breaker.metrics();
        assert_eq!(stats.successes, 10);
        assert_eq!(stats.failures, 0);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_threshold_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new("test", config);

        // Fail 3 times
        for _ in 0..3 {
            let result = breaker.call(|| async { Err::<(), _>("error") }).await;
            assert!(matches!(result, Err(CircuitBreakerError::ExecutionFailed(_))));
        }

        // Circuit should now be open
        assert_eq!(breaker.state().await, CircuitState::Open);

        let stats = breaker.metrics();
        assert_eq!(stats.failures, 3);
        assert_eq!(stats.opened_count, 1);
    }

    #[tokio::test]
    async fn test_open_circuit_rejects_requests() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_secs(10),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);

        // Next request should be rejected
        let result = breaker.call(|| async { Ok::<_, ()>(42) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open { .. })));

        let stats = breaker.metrics();
        assert!(stats.rejected_count > 0);
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Next call should transition to half-open
        let _ = breaker.call(|| async { Ok::<_, ()>(42) }).await;

        let state = breaker.state().await;
        assert!(state == CircuitState::HalfOpen || state == CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_closes_after_success_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_max_requests: 5,
        };
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Succeed twice to close
        for _ in 0..2 {
            let result = breaker.call(|| async { Ok::<_, ()>(42) }).await;
            assert!(result.is_ok());
        }

        assert_eq!(breaker.state().await, CircuitState::Closed);

        let stats = breaker.metrics();
        assert_eq!(stats.closed_count, 1);
    }

    #[tokio::test]
    async fn test_half_open_reopens_on_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Fail in half-open state
        let _ = breaker.call(|| async { Err::<(), _>("error") }).await;

        assert_eq!(breaker.state().await, CircuitState::Open);

        let stats = breaker.metrics();
        assert_eq!(stats.opened_count, 2);
    }

    #[tokio::test]
    async fn test_half_open_limits_concurrent_requests() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_max_requests: 2,
            ..Default::default()
        };
        let breaker = Arc::new(CircuitBreaker::new("test", config));

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Start two concurrent requests (should be allowed)
        let breaker1 = breaker.clone();
        let breaker2 = breaker.clone();
        let breaker3 = breaker.clone();

        let should_block = Arc::new(AtomicBool::new(true));
        let should_block_clone = should_block.clone();

        let handle1 = tokio::spawn(async move {
            breaker1.call(|| async move {
                while should_block_clone.load(Ordering::Relaxed) {
                    sleep(Duration::from_millis(10)).await;
                }
                Ok::<_, ()>(1)
            }).await
        });

        let handle2 = tokio::spawn(async move {
            sleep(Duration::from_millis(20)).await;
            breaker2.call(|| async { Ok::<_, ()>(2) }).await
        });

        // Third request should be rejected
        sleep(Duration::from_millis(50)).await;
        let result3 = breaker3.call(|| async { Ok::<_, ()>(3) }).await;

        // Unblock the first request
        should_block.store(false, Ordering::Relaxed);

        let _ = handle1.await;
        let _ = handle2.await;

        assert!(matches!(result3, Err(CircuitBreakerError::Rejected { .. })));
    }

    #[tokio::test]
    async fn test_manual_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new("test", config);

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);

        // Reset manually
        breaker.reset().await;

        assert_eq!(breaker.state().await, CircuitState::Closed);

        // Should work now
        let result = breaker.call(|| async { Ok::<_, ()>(42) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_circuit_breaker_helper() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test", config);

        let result = with_circuit_breaker(&breaker, || async {
            Ok::<_, ()>(42)
        }).await;

        assert_eq!(result.unwrap(), 42);
    }
}
