//! Resilience patterns for the LLM Research API.
//!
//! This module provides essential resilience patterns to build fault-tolerant systems:
//!
//! - **Circuit Breaker**: Prevents cascading failures by temporarily blocking requests
//!   to failing services
//! - **Retry**: Handles transient failures with configurable backoff strategies
//! - **Timeout**: Prevents operations from running indefinitely
//! - **Graceful Shutdown**: Coordinates clean shutdown of application components
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::resilience::{
//!     circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
//!     retry::{retry, ExponentialBackoff, RetryConfig},
//!     timeout::with_timeout,
//! };
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Circuit breaker
//! let breaker = CircuitBreaker::new(
//!     "external_api",
//!     CircuitBreakerConfig::default(),
//! );
//!
//! // Retry with exponential backoff
//! let retry_policy = ExponentialBackoff::new(RetryConfig::default());
//!
//! // Combine resilience patterns
//! let result = breaker.call(|| async {
//!     retry(retry_policy.clone(), || async {
//!         with_timeout(
//!             Duration::from_secs(5),
//!             async {
//!                 // Your operation here
//!                 Ok::<_, std::io::Error>(42)
//!             }
//!         ).await
//!     }).await
//! }).await?;
//! # Ok(())
//! # }
//! ```

pub mod circuit_breaker;
pub mod retry;
pub mod shutdown;
pub mod timeout;

// Re-export commonly used types
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState,
};
pub use retry::{
    retry, retry_with_context, ConstantBackoff, ExponentialBackoff, JitterStrategy, LinearBackoff,
    RetryConfig, RetryError, RetryPolicy,
};
pub use shutdown::{
    ConnectionGuard, GracefulShutdown, ShutdownCoordinator, ShutdownError, ShutdownSignal,
};
pub use timeout::{timeout_after, with_timeout, TimeoutConfig, TimeoutError, TimeoutLayer};
