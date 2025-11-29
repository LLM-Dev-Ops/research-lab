//! Timeout utilities for preventing operations from running indefinitely.
//!
//! This module provides timeout mechanisms for async operations and middleware
//! for axum request handlers.
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::resilience::timeout::{with_timeout, TimeoutConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = TimeoutConfig {
//!     default: Duration::from_secs(30),
//!     operation_specific: Default::default(),
//! };
//!
//! let result = with_timeout(
//!     Duration::from_secs(5),
//!     async { Ok::<_, std::io::Error>(42) }
//! ).await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use thiserror::Error;
use tokio::time::{sleep, timeout, Sleep, Timeout};
use tower::{Layer, Service};

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Default timeout for all operations
    pub default: Duration,
    /// Operation-specific timeouts
    pub operation_specific: HashMap<String, Duration>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default: Duration::from_secs(30),
            operation_specific: HashMap::new(),
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout config with default timeout
    pub fn new(default: Duration) -> Self {
        Self {
            default,
            operation_specific: HashMap::new(),
        }
    }

    /// Add an operation-specific timeout
    pub fn with_operation(mut self, operation: impl Into<String>, timeout: Duration) -> Self {
        self.operation_specific.insert(operation.into(), timeout);
        self
    }

    /// Get timeout for a specific operation
    pub fn get_timeout(&self, operation: &str) -> Duration {
        self.operation_specific
            .get(operation)
            .copied()
            .unwrap_or(self.default)
    }
}

/// Timeout errors
#[derive(Debug, Clone, Error)]
pub enum TimeoutError {
    /// Operation timed out
    #[error("Operation timed out after {elapsed:?}")]
    Elapsed { elapsed: Duration },

    /// Inner operation failed
    #[error("Operation failed: {0}")]
    Inner(String),
}

/// Apply a timeout to an async operation
///
/// # Example
///
/// ```no_run
/// use llm_research_api::resilience::timeout::with_timeout;
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = with_timeout(
///     Duration::from_secs(5),
///     async {
///         // Your async operation
///         Ok::<_, std::io::Error>(42)
///     }
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_timeout<F, T, E>(
    duration: Duration,
    future: F,
) -> Result<T, TimeoutError>
where
    F: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match timeout(duration, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(TimeoutError::Inner(e.to_string())),
        Err(_) => Err(TimeoutError::Elapsed { elapsed: duration }),
    }
}

/// Tower layer for adding timeouts to services
#[derive(Debug, Clone)]
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    /// Create a new timeout layer
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimeoutService {
            inner,
            timeout: self.timeout,
        }
    }
}

/// Tower service that applies timeouts
#[derive(Debug, Clone)]
pub struct TimeoutService<S> {
    inner: S,
    timeout: Duration,
}

impl<S, Request> Service<Request> for TimeoutService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let timeout_duration = self.timeout;
        let fut = self.inner.call(request);

        Box::pin(async move {
            match timeout(timeout_duration, fut).await {
                Ok(result) => result.map_err(Into::into),
                Err(_elapsed) => Err(Box::new(TimeoutError::Elapsed {
                    elapsed: timeout_duration,
                }) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }
}

/// Middleware for request timeouts in axum
pub mod middleware {
    use super::*;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use axum::middleware::Next;
    use axum::response::{IntoResponse, Response};

    /// Apply timeout to axum request handlers
    ///
    /// # Example
    ///
    /// ```no_run
    /// use axum::{Router, routing::get};
    /// use axum::middleware;
    /// use llm_research_api::resilience::timeout::middleware::request_timeout;
    /// use std::time::Duration;
    ///
    /// # async fn handler() -> &'static str { "Hello" }
    /// # fn example() {
    /// let app = Router::new()
    ///     .route("/", get(handler))
    ///     .layer(middleware::from_fn(|req, next| {
    ///         request_timeout(req, next, Duration::from_secs(30))
    ///     }));
    /// # }
    /// ```
    pub async fn request_timeout(
        request: Request,
        next: Next,
        timeout_duration: Duration,
    ) -> Response {
        match timeout(timeout_duration, next.run(request)).await {
            Ok(response) => response,
            Err(_) => (
                StatusCode::REQUEST_TIMEOUT,
                format!("Request timed out after {:?}", timeout_duration),
            )
                .into_response(),
        }
    }

    /// Create a timeout middleware with a specific duration
    pub fn with_timeout(
        timeout_duration: Duration,
    ) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone {
        move |req, next| {
            let timeout_duration = timeout_duration;
            Box::pin(request_timeout(req, next, timeout_duration))
        }
    }
}

/// Helper to create a timeout future
pub fn timeout_after(duration: Duration) -> TimeoutFuture {
    TimeoutFuture {
        sleep: Box::pin(sleep(duration)),
    }
}

/// A future that completes after a timeout
pub struct TimeoutFuture {
    sleep: Pin<Box<Sleep>>,
}

impl Future for TimeoutFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.sleep.as_mut().poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_with_timeout_succeeds() {
        let result = with_timeout(Duration::from_secs(1), async {
            sleep(Duration::from_millis(100)).await;
            Ok::<_, std::io::Error>(42)
        })
        .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_times_out() {
        let result = with_timeout(Duration::from_millis(100), async {
            sleep(Duration::from_secs(10)).await;
            Ok::<_, std::io::Error>(42)
        })
        .await;

        assert!(matches!(result, Err(TimeoutError::Elapsed { .. })));
    }

    #[tokio::test]
    async fn test_with_timeout_propagates_error() {
        let result = with_timeout(Duration::from_secs(1), async {
            Err::<i32, _>("test error")
        })
        .await;

        assert!(matches!(result, Err(TimeoutError::Inner(_))));
    }

    #[tokio::test]
    async fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_timeout_config_with_operation() {
        let config = TimeoutConfig::new(Duration::from_secs(30))
            .with_operation("fast", Duration::from_secs(5))
            .with_operation("slow", Duration::from_secs(120));

        assert_eq!(config.get_timeout("fast"), Duration::from_secs(5));
        assert_eq!(config.get_timeout("slow"), Duration::from_secs(120));
        assert_eq!(config.get_timeout("unknown"), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_timeout_layer() {
        use tower::ServiceExt;

        let layer = TimeoutLayer::new(Duration::from_millis(100));
        let service = tower::service_fn(|_: ()| async {
            sleep(Duration::from_secs(10)).await;
            Ok::<_, std::io::Error>(42)
        });

        let mut service = layer.layer(service);

        let result = service.ready().await.unwrap().call(()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_timeout_layer_succeeds() {
        use tower::ServiceExt;

        let layer = TimeoutLayer::new(Duration::from_secs(1));
        let service = tower::service_fn(|_: ()| async {
            sleep(Duration::from_millis(100)).await;
            Ok::<_, std::io::Error>(42)
        });

        let mut service = layer.layer(service);

        let result = service.ready().await.unwrap().call(()).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_timeout_future() {
        let start = std::time::Instant::now();
        timeout_after(Duration::from_millis(100)).await;
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn test_timeout_error_display() {
        let error = TimeoutError::Elapsed {
            elapsed: Duration::from_secs(5),
        };
        assert!(format!("{}", error).contains("5s"));

        let error = TimeoutError::Inner("test".to_string());
        assert!(format!("{}", error).contains("test"));
    }

    #[tokio::test]
    async fn test_multiple_timeout_operations() {
        let config = TimeoutConfig::new(Duration::from_secs(1))
            .with_operation("op1", Duration::from_millis(100))
            .with_operation("op2", Duration::from_millis(200));

        let result1 = with_timeout(config.get_timeout("op1"), async {
            sleep(Duration::from_millis(50)).await;
            Ok::<_, std::io::Error>("op1")
        })
        .await;

        let result2 = with_timeout(config.get_timeout("op2"), async {
            sleep(Duration::from_millis(150)).await;
            Ok::<_, std::io::Error>("op2")
        })
        .await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_timeout_cleanup() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let result = with_timeout(Duration::from_millis(100), async move {
            sleep(Duration::from_secs(10)).await;
            flag_clone.store(true, Ordering::Relaxed);
            Ok::<_, std::io::Error>(42)
        })
        .await;

        assert!(result.is_err());
        // Give some time for cleanup
        sleep(Duration::from_millis(50)).await;
        // Flag should not be set since we timed out
        assert!(!flag.load(Ordering::Relaxed));
    }
}
