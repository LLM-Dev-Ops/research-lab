# Resilience Module Examples

This document provides practical examples of using the resilience module in real-world scenarios.

## Table of Contents

1. [Basic Examples](#basic-examples)
2. [Advanced Patterns](#advanced-patterns)
3. [Integration Examples](#integration-examples)
4. [Production Patterns](#production-patterns)

## Basic Examples

### 1. Simple Circuit Breaker

```rust
use llm_research_api::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use std::time::Duration;

async fn call_external_service() -> Result<String, reqwest::Error> {
    reqwest::get("https://api.example.com/data")
        .await?
        .text()
        .await
}

#[tokio::main]
async fn main() {
    let breaker = CircuitBreaker::new(
        "example_api",
        CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    );

    match breaker.call(|| call_external_service()).await {
        Ok(data) => println!("Received: {}", data),
        Err(e) => eprintln!("Failed: {}", e),
    }
}
```

### 2. Exponential Backoff Retry

```rust
use llm_research_api::resilience::retry::{retry, ExponentialBackoff, RetryConfig};
use std::time::Duration;

async fn flaky_operation() -> Result<i32, String> {
    // Simulates a flaky operation
    if rand::random::<f64>() > 0.7 {
        Ok(42)
    } else {
        Err("Temporary failure".to_string())
    }
}

#[tokio::main]
async fn main() {
    let policy = ExponentialBackoff::new(RetryConfig {
        max_attempts: 5,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(10),
        multiplier: 2.0,
        jitter: true,
    });

    match retry(policy, flaky_operation).await {
        Ok(result) => println!("Success: {}", result),
        Err(e) => eprintln!("Failed after {} attempts: {}", e.attempts, e.error),
    }
}
```

### 3. Timeout Protection

```rust
use llm_research_api::resilience::timeout::with_timeout;
use std::time::Duration;
use tokio::time::sleep;

async fn slow_operation() -> Result<String, ()> {
    sleep(Duration::from_secs(10)).await;
    Ok("Done".to_string())
}

#[tokio::main]
async fn main() {
    match with_timeout(Duration::from_secs(5), slow_operation()).await {
        Ok(result) => println!("Completed: {}", result),
        Err(e) => eprintln!("Timeout: {}", e),
    }
}
```

### 4. Graceful Shutdown

```rust
use llm_research_api::resilience::shutdown::{ShutdownSignal, ShutdownCoordinator};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let signal = ShutdownSignal::new();
    let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));

    // Register cleanup hooks
    coordinator.on_shutdown(|| async {
        println!("Closing database connections...");
        // Cleanup code here
    });

    // Simulate work
    tokio::spawn({
        let signal = signal.clone();
        async move {
            signal.wait().await;
            println!("Shutdown signal received!");
        }
    });

    // Wait and shutdown
    tokio::time::sleep(Duration::from_secs(5)).await;
    signal.trigger();

    coordinator.shutdown().await.expect("Shutdown failed");
}
```

## Advanced Patterns

### 1. Combining Circuit Breaker and Retry

```rust
use llm_research_api::resilience::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    retry::{retry, ExponentialBackoff, RetryConfig},
};
use std::time::Duration;

async fn resilient_api_call(
    breaker: &CircuitBreaker,
    url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let retry_policy = ExponentialBackoff::new(RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(2),
        multiplier: 2.0,
        jitter: true,
    });

    breaker.call(|| {
        let url = url.to_string();
        async move {
            retry(retry_policy.clone(), || {
                let url = url.clone();
                async move {
                    reqwest::get(&url)
                        .await
                        .map_err(|e| e.to_string())?
                        .text()
                        .await
                        .map_err(|e| e.to_string())
                }
            }).await.map_err(|e| e.error)
        }
    }).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
```

### 2. Full Resilience Stack

```rust
use llm_research_api::resilience::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    retry::{retry, ExponentialBackoff, RetryConfig},
    timeout::with_timeout,
};
use std::time::Duration;
use std::sync::Arc;

struct ResilientService {
    breaker: Arc<CircuitBreaker>,
    retry_policy: ExponentialBackoff,
    timeout: Duration,
}

impl ResilientService {
    fn new() -> Self {
        Self {
            breaker: Arc::new(CircuitBreaker::new(
                "resilient_service",
                CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 2,
                    timeout: Duration::from_secs(60),
                    half_open_max_requests: 3,
                }
            )),
            retry_policy: ExponentialBackoff::new(RetryConfig {
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
                multiplier: 2.0,
                jitter: true,
            }),
            timeout: Duration::from_secs(10),
        }
    }

    async fn call<F, T, E>(&self, f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>> + Clone + Send + Sync,
        E: std::fmt::Display + Send + 'static,
        T: Send + 'static,
    {
        let timeout_duration = self.timeout;
        let retry_policy = self.retry_policy.clone();

        self.breaker.call(|| {
            async move {
                retry(retry_policy.clone(), || {
                    let f = f.clone();
                    async move {
                        with_timeout(timeout_duration, f())
                            .await
                            .map_err(|e| e.to_string())
                    }
                }).await.map_err(|e| e.error)
            }
        }).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
```

### 3. Custom Retry Policy

```rust
use llm_research_api::resilience::retry::RetryPolicy;
use std::time::Duration;

// Retry policy that only retries specific errors
struct SelectiveRetryPolicy {
    max_attempts: usize,
    delay: Duration,
}

impl RetryPolicy for SelectiveRetryPolicy {
    fn next_delay(&self, attempt: usize) -> Option<Duration> {
        if attempt >= self.max_attempts {
            None
        } else {
            Some(self.delay)
        }
    }

    fn is_retryable<E>(&self, error: &E) -> bool
    where
        E: std::fmt::Display,
    {
        let error_msg = error.to_string();
        // Only retry on timeout or connection errors
        error_msg.contains("timeout") || error_msg.contains("connection")
    }

    fn max_attempts(&self) -> usize {
        self.max_attempts
    }
}
```

## Integration Examples

### 1. Axum Handler with Resilience

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use llm_research_api::resilience::{
    circuit_breaker::CircuitBreaker,
    retry::{retry, ExponentialBackoff, RetryConfig},
    timeout::with_timeout,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
struct AppState {
    breaker: Arc<CircuitBreaker>,
}

#[derive(Deserialize)]
struct QueryParams {
    id: String,
}

#[derive(Serialize)]
struct ApiResponse {
    data: String,
}

async fn fetch_data(id: &str) -> Result<String, String> {
    // Simulate external API call
    Ok(format!("Data for {}", id))
}

async fn handler(
    State(state): State<AppState>,
    Json(params): Json<QueryParams>,
) -> Response {
    let retry_policy = ExponentialBackoff::new(RetryConfig::default());

    let result = state.breaker.call(|| {
        let id = params.id.clone();
        async move {
            retry(retry_policy.clone(), || {
                let id = id.clone();
                async move {
                    with_timeout(
                        Duration::from_secs(5),
                        fetch_data(&id)
                    ).await.map_err(|e| e.to_string())
                }
            }).await.map_err(|e| e.error)
        }
    }).await;

    match result {
        Ok(data) => (StatusCode::OK, Json(ApiResponse { data })).into_response(),
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}
```

### 2. Database Queries with Retry

```rust
use llm_research_api::resilience::retry::{retry, ExponentialBackoff, RetryConfig};
use sqlx::{PgPool, Error as SqlxError};
use std::time::Duration;

async fn get_user_resilient(
    pool: &PgPool,
    user_id: i32,
) -> Result<User, SqlxError> {
    let retry_policy = ExponentialBackoff::new(RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(50),
        max_delay: Duration::from_secs(1),
        multiplier: 2.0,
        jitter: true,
    });

    retry(retry_policy, || {
        let pool = pool.clone();
        async move {
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
        }
    }).await.map_err(|e| e.error)
}

#[derive(sqlx::FromRow)]
struct User {
    id: i32,
    name: String,
}
```

### 3. Application with Graceful Shutdown

```rust
use axum::{Router, routing::get};
use llm_research_api::resilience::shutdown::{
    ShutdownSignal, ShutdownCoordinator, ConnectionGuard,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let signal = ShutdownSignal::new();
    let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(30)));

    // Register shutdown hooks
    coordinator.on_shutdown(|| async {
        tracing::info!("Closing database connections...");
    });

    let app = Router::new()
        .route("/", get(|| async { "Hello!" }))
        .layer(axum::middleware::from_fn_with_state(
            coordinator.clone(),
            connection_tracking_middleware,
        ));

    // Spawn server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            signal.wait().await;
        });

    // Run until shutdown
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    // Perform graceful shutdown
    coordinator.shutdown().await.expect("Shutdown failed");
}

async fn connection_tracking_middleware(
    State(coordinator): State<Arc<ShutdownCoordinator>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Some(_guard) = ConnectionGuard::new(coordinator) {
        next.run(request).await
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "Shutting down")
            .into_response()
    }
}
```

## Production Patterns

### 1. Multi-Service Circuit Breakers

```rust
use llm_research_api::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

struct ServiceRegistry {
    breakers: HashMap<String, Arc<CircuitBreaker>>,
}

impl ServiceRegistry {
    fn new() -> Self {
        let mut breakers = HashMap::new();

        // Different configurations for different services
        breakers.insert(
            "auth_service".to_string(),
            Arc::new(CircuitBreaker::new(
                "auth_service",
                CircuitBreakerConfig {
                    failure_threshold: 3,
                    success_threshold: 2,
                    timeout: Duration::from_secs(30),
                    half_open_max_requests: 2,
                }
            ))
        );

        breakers.insert(
            "data_service".to_string(),
            Arc::new(CircuitBreaker::new(
                "data_service",
                CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 3,
                    timeout: Duration::from_secs(60),
                    half_open_max_requests: 5,
                }
            ))
        );

        Self { breakers }
    }

    fn get(&self, service: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.get(service).cloned()
    }
}
```

### 2. Adaptive Retry with Backpressure

```rust
use llm_research_api::resilience::retry::{RetryPolicy, retry};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct AdaptiveRetryPolicy {
    base_config: RetryConfig,
    load_factor: Arc<AtomicUsize>,
}

impl AdaptiveRetryPolicy {
    fn new(base_config: RetryConfig, load_factor: Arc<AtomicUsize>) -> Self {
        Self { base_config, load_factor }
    }
}

impl RetryPolicy for AdaptiveRetryPolicy {
    fn next_delay(&self, attempt: usize) -> Option<Duration> {
        let load = self.load_factor.load(Ordering::Relaxed);

        // Reduce retries under high load
        let max_attempts = if load > 80 {
            self.base_config.max_attempts / 2
        } else {
            self.base_config.max_attempts
        };

        if attempt >= max_attempts {
            return None;
        }

        let base_delay = self.base_config.initial_delay.as_secs_f64()
            * self.base_config.multiplier.powi(attempt as i32);

        Some(Duration::from_secs_f64(
            base_delay.min(self.base_config.max_delay.as_secs_f64())
        ))
    }

    fn max_attempts(&self) -> usize {
        self.base_config.max_attempts
    }
}
```

### 3. Comprehensive Health Monitoring

```rust
use llm_research_api::resilience::circuit_breaker::CircuitBreaker;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct ServiceHealth {
    name: String,
    circuit_state: String,
    failures: u64,
    successes: u64,
    rejection_rate: f64,
}

async fn get_service_health(breaker: &Arc<CircuitBreaker>) -> ServiceHealth {
    let stats = breaker.metrics();
    let state = breaker.state().await;

    let total_requests = stats.failures + stats.successes;
    let rejection_rate = if total_requests > 0 {
        stats.rejected_count as f64 / total_requests as f64
    } else {
        0.0
    };

    ServiceHealth {
        name: "example_service".to_string(),
        circuit_state: state.to_string(),
        failures: stats.failures,
        successes: stats.successes,
        rejection_rate,
    }
}
```

### 4. Cascading Shutdown

```rust
use llm_research_api::resilience::shutdown::{
    GracefulShutdown, ShutdownCoordinator, ShutdownError,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

struct WebServer;
struct DatabasePool;
struct CacheService;

#[async_trait]
impl GracefulShutdown for WebServer {
    async fn shutdown(&self) -> Result<(), ShutdownError> {
        tracing::info!("Shutting down web server...");
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    fn name(&self) -> &str {
        "WebServer"
    }
}

#[async_trait]
impl GracefulShutdown for DatabasePool {
    async fn shutdown(&self) -> Result<(), ShutdownError> {
        tracing::info!("Closing database connections...");
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    fn name(&self) -> &str {
        "DatabasePool"
    }
}

#[async_trait]
impl GracefulShutdown for CacheService {
    async fn shutdown(&self) -> Result<(), ShutdownError> {
        tracing::info!("Flushing cache...");
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    fn name(&self) -> &str {
        "CacheService"
    }
}

async fn setup_shutdown() {
    let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));

    // Register components in shutdown order
    coordinator.register_component(Arc::new(WebServer)).await;
    coordinator.register_component(Arc::new(DatabasePool)).await;
    coordinator.register_component(Arc::new(CacheService)).await;

    // Shutdown will happen in registration order
    coordinator.shutdown().await.expect("Shutdown failed");
}
```

## Testing Examples

### 1. Testing Circuit Breaker Behavior

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let breaker = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 3,
                ..Default::default()
            }
        );

        // Trigger failures
        for _ in 0..3 {
            let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);
    }
}
```

### 2. Testing Retry Logic

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let policy = ExponentialBackoff::new(RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        });

        let result = retry(policy, || {
            let c = counter_clone.clone();
            async move {
                let count = c.fetch_add(1, Ordering::Relaxed);
                if count < 3 {
                    Err("not yet")
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }
}
```

## Summary

These examples demonstrate how to use the resilience module in various scenarios:

- **Basic patterns** for getting started
- **Advanced combinations** for robust error handling
- **Integration examples** for real-world applications
- **Production patterns** for scalable systems

Each pattern can be used independently or combined for comprehensive fault tolerance.
