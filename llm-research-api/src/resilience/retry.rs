//! Retry mechanisms with exponential backoff and jitter.
//!
//! This module provides flexible retry policies for handling transient failures.
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::resilience::retry::{retry, RetryConfig, ExponentialBackoff};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = RetryConfig {
//!     max_attempts: 5,
//!     initial_delay: Duration::from_millis(100),
//!     max_delay: Duration::from_secs(30),
//!     multiplier: 2.0,
//!     jitter: true,
//! };
//!
//! let policy = ExponentialBackoff::new(config);
//!
//! let result = retry(policy, || async {
//!     // Your operation here
//!     Ok::<_, std::io::Error>(42)
//! }).await?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Whether to add jitter to delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Jitter strategy for retry delays
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitterStrategy {
    /// No jitter
    None,
    /// Full jitter: random value between 0 and calculated delay
    Full,
    /// Equal jitter: half delay + random half
    Equal,
    /// Decorrelated jitter: exponentially weighted moving average
    Decorrelated,
}

impl Default for JitterStrategy {
    fn default() -> Self {
        Self::Full
    }
}

/// Trait for retry policies
pub trait RetryPolicy: Send + Sync {
    /// Calculate the delay before the next retry attempt
    ///
    /// Returns `None` if no more retries should be attempted
    fn next_delay(&self, attempt: usize) -> Option<Duration>;

    /// Check if an error is retryable
    fn is_retryable<E>(&self, _error: &E) -> bool {
        true // By default, all errors are retryable
    }

    /// Maximum number of attempts
    fn max_attempts(&self) -> usize;
}

/// Exponential backoff retry policy
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    config: RetryConfig,
    jitter_strategy: JitterStrategy,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff policy
    pub fn new(config: RetryConfig) -> Self {
        let jitter_strategy = if config.jitter {
            JitterStrategy::Full
        } else {
            JitterStrategy::None
        };

        Self {
            config,
            jitter_strategy,
        }
    }

    /// Create with specific jitter strategy
    pub fn with_jitter(config: RetryConfig, jitter_strategy: JitterStrategy) -> Self {
        Self {
            config,
            jitter_strategy,
        }
    }

    fn apply_jitter(&self, delay: Duration) -> Duration {
        match self.jitter_strategy {
            JitterStrategy::None => delay,
            JitterStrategy::Full => {
                let jitter = rand::random::<f64>();
                Duration::from_secs_f64(delay.as_secs_f64() * jitter)
            }
            JitterStrategy::Equal => {
                let jitter = rand::random::<f64>();
                let base = delay.as_secs_f64() / 2.0;
                Duration::from_secs_f64(base + (base * jitter))
            }
            JitterStrategy::Decorrelated => {
                let jitter = rand::random::<f64>();
                let base = self.config.initial_delay.as_secs_f64();
                Duration::from_secs_f64(base + (delay.as_secs_f64() * 3.0 * jitter))
            }
        }
    }
}

impl RetryPolicy for ExponentialBackoff {
    fn next_delay(&self, attempt: usize) -> Option<Duration> {
        if attempt >= self.config.max_attempts {
            return None;
        }

        let base_delay = self.config.initial_delay.as_secs_f64()
            * self.config.multiplier.powi(attempt as i32);

        let delay = Duration::from_secs_f64(base_delay.min(self.config.max_delay.as_secs_f64()));

        Some(self.apply_jitter(delay))
    }

    fn max_attempts(&self) -> usize {
        self.config.max_attempts
    }
}

/// Constant backoff retry policy (fixed delay between retries)
#[derive(Debug, Clone)]
pub struct ConstantBackoff {
    max_attempts: usize,
    delay: Duration,
    jitter_strategy: JitterStrategy,
}

impl ConstantBackoff {
    /// Create a new constant backoff policy
    pub fn new(max_attempts: usize, delay: Duration) -> Self {
        Self {
            max_attempts,
            delay,
            jitter_strategy: JitterStrategy::None,
        }
    }

    /// Create with jitter
    pub fn with_jitter(
        max_attempts: usize,
        delay: Duration,
        jitter_strategy: JitterStrategy,
    ) -> Self {
        Self {
            max_attempts,
            delay,
            jitter_strategy,
        }
    }

    fn apply_jitter(&self, delay: Duration) -> Duration {
        match self.jitter_strategy {
            JitterStrategy::None => delay,
            JitterStrategy::Full => {
                let jitter = rand::random::<f64>();
                Duration::from_secs_f64(delay.as_secs_f64() * jitter)
            }
            JitterStrategy::Equal => {
                let jitter = rand::random::<f64>();
                let base = delay.as_secs_f64() / 2.0;
                Duration::from_secs_f64(base + (base * jitter))
            }
            JitterStrategy::Decorrelated => {
                let jitter = rand::random::<f64>();
                Duration::from_secs_f64(delay.as_secs_f64() * jitter)
            }
        }
    }
}

impl RetryPolicy for ConstantBackoff {
    fn next_delay(&self, attempt: usize) -> Option<Duration> {
        if attempt >= self.max_attempts {
            None
        } else {
            Some(self.apply_jitter(self.delay))
        }
    }

    fn max_attempts(&self) -> usize {
        self.max_attempts
    }
}

/// Linear backoff retry policy
#[derive(Debug, Clone)]
pub struct LinearBackoff {
    max_attempts: usize,
    initial_delay: Duration,
    max_delay: Duration,
    increment: Duration,
    jitter_strategy: JitterStrategy,
}

impl LinearBackoff {
    /// Create a new linear backoff policy
    pub fn new(
        max_attempts: usize,
        initial_delay: Duration,
        increment: Duration,
        max_delay: Duration,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay,
            increment,
            jitter_strategy: JitterStrategy::None,
        }
    }

    /// Create with jitter
    pub fn with_jitter(
        max_attempts: usize,
        initial_delay: Duration,
        increment: Duration,
        max_delay: Duration,
        jitter_strategy: JitterStrategy,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay,
            increment,
            jitter_strategy,
        }
    }

    fn apply_jitter(&self, delay: Duration) -> Duration {
        match self.jitter_strategy {
            JitterStrategy::None => delay,
            JitterStrategy::Full => {
                let jitter = rand::random::<f64>();
                Duration::from_secs_f64(delay.as_secs_f64() * jitter)
            }
            JitterStrategy::Equal => {
                let jitter = rand::random::<f64>();
                let base = delay.as_secs_f64() / 2.0;
                Duration::from_secs_f64(base + (base * jitter))
            }
            JitterStrategy::Decorrelated => {
                let jitter = rand::random::<f64>();
                Duration::from_secs_f64(delay.as_secs_f64() * jitter)
            }
        }
    }
}

impl RetryPolicy for LinearBackoff {
    fn next_delay(&self, attempt: usize) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }

        let delay = self.initial_delay + self.increment * attempt as u32;
        let delay = delay.min(self.max_delay);

        Some(self.apply_jitter(delay))
    }

    fn max_attempts(&self) -> usize {
        self.max_attempts
    }
}

/// Error wrapper that includes retry attempt information
#[derive(Debug)]
pub struct RetryError<E> {
    /// The underlying error
    pub error: E,
    /// Number of attempts made
    pub attempts: usize,
}

impl<E: fmt::Display> fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Operation failed after {} attempts: {}",
            self.attempts, self.error
        )
    }
}

impl<E: std::error::Error> std::error::Error for RetryError<E> {}

/// Retry an operation with the given policy
///
/// # Example
///
/// ```no_run
/// use llm_research_api::resilience::retry::{retry, ExponentialBackoff, RetryConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let policy = ExponentialBackoff::new(RetryConfig::default());
/// let result = retry(policy, || async {
///     Ok::<_, std::io::Error>(42)
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn retry<F, Fut, T, E, P>(policy: P, mut f: F) -> Result<T, RetryError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: RetryPolicy,
{
    let mut attempt = 0;
    let mut last_error = None;

    loop {
        debug!("Retry attempt {}/{}", attempt + 1, policy.max_attempts());

        match f().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("Operation succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if !policy.is_retryable(&e) {
                    warn!("Error is not retryable, giving up");
                    return Err(RetryError {
                        error: e,
                        attempts: attempt + 1,
                    });
                }

                if let Some(delay) = policy.next_delay(attempt) {
                    debug!("Retrying after {:?}", delay);
                    sleep(delay).await;
                    attempt += 1;
                    last_error = Some(e);
                } else {
                    warn!("Max retry attempts reached");
                    return Err(RetryError {
                        error: e,
                        attempts: attempt + 1,
                    });
                }
            }
        }
    }
}

/// Retry with context information
///
/// Allows adding context to errors while retrying
pub async fn retry_with_context<F, Fut, T, E, P, C>(
    policy: P,
    mut f: F,
    context: C,
) -> Result<T, RetryError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: RetryPolicy,
    C: fmt::Display,
    E: fmt::Display,
{
    let result = retry(policy, f).await;

    if let Err(ref e) = result {
        warn!("Retry failed for {}: {}", context, e);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_immediately() {
        let policy = ExponentialBackoff::new(RetryConfig::default());
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(policy, || {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                Ok::<_, ()>(42)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let policy = ExponentialBackoff::new(RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(policy, || {
            let c = counter_clone.clone();
            async move {
                let count = c.fetch_add(1, Ordering::Relaxed);
                if count < 3 {
                    Err("temporary error")
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        let policy = ExponentialBackoff::new(RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(policy, || {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>("permanent error")
            }
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        // With max_attempts=3, we make attempts 0, 1, 2, 3 = 4 total attempts
        // (attempt 3 fails and next_delay(3) returns None since 3 >= max_attempts)
        assert_eq!(err.attempts, 4);
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[tokio::test]
    async fn test_exponential_backoff_delays() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: false,
        };

        let policy = ExponentialBackoff::new(config);

        assert_eq!(policy.next_delay(0), Some(Duration::from_millis(100)));
        assert_eq!(policy.next_delay(1), Some(Duration::from_millis(200)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_millis(400)));
        assert_eq!(policy.next_delay(5), None);
    }

    #[tokio::test]
    async fn test_exponential_backoff_max_delay() {
        let config = RetryConfig {
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
            jitter: false,
        };

        let policy = ExponentialBackoff::new(config);

        // Should cap at max_delay
        let delay = policy.next_delay(5).unwrap();
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_constant_backoff() {
        let policy = ConstantBackoff::new(3, Duration::from_millis(100));

        assert_eq!(policy.next_delay(0), Some(Duration::from_millis(100)));
        assert_eq!(policy.next_delay(1), Some(Duration::from_millis(100)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_millis(100)));
        assert_eq!(policy.next_delay(3), None);
    }

    #[tokio::test]
    async fn test_linear_backoff() {
        let policy = LinearBackoff::new(
            5,
            Duration::from_millis(100),
            Duration::from_millis(50),
            Duration::from_secs(1),
        );

        assert_eq!(policy.next_delay(0), Some(Duration::from_millis(100)));
        assert_eq!(policy.next_delay(1), Some(Duration::from_millis(150)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_millis(200)));
        assert_eq!(policy.next_delay(5), None);
    }

    #[tokio::test]
    async fn test_linear_backoff_max_delay() {
        let policy = LinearBackoff::new(
            10,
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(500),
        );

        // Should cap at max_delay
        let delay = policy.next_delay(5).unwrap();
        assert_eq!(delay, Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_jitter_adds_randomness() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: true,
        };

        let policy = ExponentialBackoff::new(config);

        // Get multiple delays for the same attempt
        let delays: Vec<_> = (0..10)
            .map(|_| policy.next_delay(1).unwrap())
            .collect();

        // Check that we have different values (with high probability)
        let all_same = delays.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same, "Jitter should produce different delays");

        // All delays should be <= base delay without jitter
        let base_delay = Duration::from_millis(200);
        for delay in delays {
            assert!(delay <= base_delay);
        }
    }

    #[tokio::test]
    async fn test_retry_with_context() {
        let policy = ExponentialBackoff::new(RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        });

        let result = retry_with_context(
            policy,
            || async { Err::<(), _>("error") },
            "test operation",
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_error_display() {
        let error = RetryError {
            error: "test error",
            attempts: 3,
        };

        let display = format!("{}", error);
        assert!(display.contains("3 attempts"));
        assert!(display.contains("test error"));
    }
}
