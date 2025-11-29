# Resilience Module - Quick Reference

## Import Everything

```rust
use llm_research_api::resilience::{
    // Circuit Breaker
    CircuitBreaker, CircuitBreakerConfig, CircuitState,

    // Retry
    retry, ExponentialBackoff, ConstantBackoff, LinearBackoff,
    RetryConfig, JitterStrategy,

    // Timeout
    with_timeout, TimeoutConfig, TimeoutLayer,

    // Shutdown
    ShutdownSignal, ShutdownCoordinator, GracefulShutdown, ConnectionGuard,
};
```

## Circuit Breaker

### Quick Start
```rust
let breaker = CircuitBreaker::new("service", CircuitBreakerConfig::default());
let result = breaker.call(|| async { Ok::<_, ()>(42) }).await?;
```

### Custom Config
```rust
let config = CircuitBreakerConfig {
    failure_threshold: 5,      // Open after 5 failures
    success_threshold: 2,      // Close after 2 successes
    timeout: Duration::from_secs(60),  // Wait 60s before half-open
    half_open_max_requests: 3, // Allow 3 requests when half-open
};
```

### Check State
```rust
let state = breaker.state().await;  // Closed, Open, or HalfOpen
let stats = breaker.metrics();      // Get statistics
```

## Retry

### Exponential Backoff
```rust
let policy = ExponentialBackoff::new(RetryConfig {
    max_attempts: 5,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    multiplier: 2.0,
    jitter: true,
});

retry(policy, || async { Ok::<_, ()>(42) }).await?;
```

### Constant Backoff
```rust
let policy = ConstantBackoff::new(3, Duration::from_millis(500));
```

### Linear Backoff
```rust
let policy = LinearBackoff::new(
    5,                              // max attempts
    Duration::from_millis(100),     // initial delay
    Duration::from_millis(100),     // increment
    Duration::from_secs(5),         // max delay
);
```

### Custom Jitter
```rust
let policy = ExponentialBackoff::with_jitter(
    config,
    JitterStrategy::Equal,  // None, Full, Equal, or Decorrelated
);
```

## Timeout

### Simple Timeout
```rust
let result = with_timeout(
    Duration::from_secs(5),
    async { Ok::<_, ()>(42) }
).await?;
```

### Per-Operation Config
```rust
let config = TimeoutConfig::new(Duration::from_secs(30))
    .with_operation("fast", Duration::from_secs(5))
    .with_operation("slow", Duration::from_secs(120));

let timeout = config.get_timeout("fast");
```

### Tower Middleware
```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .service(my_service);
```

### Axum Middleware
```rust
use axum::middleware;

let app = Router::new()
    .layer(middleware::from_fn(
        timeout::middleware::with_timeout(Duration::from_secs(30))
    ));
```

## Graceful Shutdown

### Basic Setup
```rust
let signal = ShutdownSignal::new();
let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
```

### Register Hooks
```rust
coordinator.on_shutdown(|| async {
    println!("Cleaning up...");
});
```

### Track Connections
```rust
coordinator.connection_opened();
// ... do work ...
coordinator.connection_closed();
```

### Connection Guard
```rust
if let Some(_guard) = ConnectionGuard::new(coordinator.clone()) {
    // Connection tracked automatically
    // Closed when guard drops
}
```

### Wait for Signal
```rust
signal.wait().await;  // Waits for SIGTERM or SIGINT
```

### Manual Trigger
```rust
signal.trigger();
```

### Shutdown
```rust
coordinator.shutdown().await?;  // Graceful
coordinator.force_shutdown().await;  // Immediate
```

### Implement GracefulShutdown
```rust
use async_trait::async_trait;

struct MyService;

#[async_trait]
impl GracefulShutdown for MyService {
    async fn shutdown(&self) -> Result<(), ShutdownError> {
        // Cleanup logic
        Ok(())
    }

    fn name(&self) -> &str {
        "MyService"
    }
}

coordinator.register_component(Arc::new(MyService)).await;
```

## Combining Patterns

### Full Stack
```rust
let breaker = CircuitBreaker::new("api", CircuitBreakerConfig::default());
let retry_policy = ExponentialBackoff::new(RetryConfig::default());

breaker.call(|| async {
    retry(retry_policy.clone(), || async {
        with_timeout(Duration::from_secs(5), async {
            api_call().await
        }).await
    }).await
}).await?
```

### Error Handling
```rust
match result {
    Ok(value) => { /* success */ },
    Err(CircuitBreakerError::Open { name }) => { /* circuit open */ },
    Err(CircuitBreakerError::Rejected { name }) => { /* rejected */ },
    Err(CircuitBreakerError::ExecutionFailed(e)) => { /* operation failed */ },
}
```

## Default Configurations

### CircuitBreakerConfig
- failure_threshold: 5
- success_threshold: 2
- timeout: 60s
- half_open_max_requests: 3

### RetryConfig
- max_attempts: 3
- initial_delay: 100ms
- max_delay: 30s
- multiplier: 2.0
- jitter: true

### TimeoutConfig
- default: 30s
- operation_specific: empty

### ShutdownCoordinator
- timeout: (must be specified)

## Common Patterns

### Database Query
```rust
retry(ExponentialBackoff::new(RetryConfig::default()), || async {
    sqlx::query("SELECT * FROM users")
        .fetch_all(&pool)
        .await
}).await?
```

### External API Call
```rust
let breaker = CircuitBreaker::new("external_api", CircuitBreakerConfig::default());

breaker.call(|| async {
    with_timeout(Duration::from_secs(10), async {
        reqwest::get("https://api.example.com")
            .await?
            .json()
            .await
    }).await
}).await?
```

### Axum Handler
```rust
async fn handler(State(breaker): State<Arc<CircuitBreaker>>) -> Response {
    match breaker.call(|| async { fetch_data().await }).await {
        Ok(data) => Json(data).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
```

## Testing

### Circuit Breaker Test
```rust
#[tokio::test]
async fn test_circuit_opens() {
    let breaker = CircuitBreaker::new("test", config);
    for _ in 0..5 {
        let _ = breaker.call(|| async { Err::<(), _>("error") }).await;
    }
    assert_eq!(breaker.state().await, CircuitState::Open);
}
```

### Retry Test
```rust
#[tokio::test]
async fn test_retry_succeeds() {
    let counter = Arc::new(AtomicUsize::new(0));
    let policy = ExponentialBackoff::new(config);

    let result = retry(policy, || {
        let c = counter.clone();
        async move {
            if c.fetch_add(1, Ordering::Relaxed) < 2 {
                Err("not yet")
            } else {
                Ok(42)
            }
        }
    }).await;

    assert_eq!(result.unwrap(), 42);
}
```

## Performance Tips

1. **Circuit Breaker**: Use atomic operations for minimal overhead
2. **Retry**: Enable jitter to prevent thundering herd
3. **Timeout**: Set slightly higher than expected latency
4. **Shutdown**: Register hooks early, set appropriate timeout

## Common Errors

### CircuitBreakerError
- `Open`: Circuit is open, request rejected
- `Rejected`: Too many requests in half-open state
- `ExecutionFailed(E)`: Operation failed with error E

### RetryError
- Contains underlying error and attempt count

### TimeoutError
- `Elapsed`: Operation timed out
- `Inner`: Operation failed before timeout

### ShutdownError
- `Timeout`: Shutdown took too long
- `ComponentFailed`: Component shutdown failed
- `AlreadyShuttingDown`: Shutdown in progress

## File Locations

```
src/resilience/
├── circuit_breaker.rs  # Circuit breaker implementation
├── retry.rs           # Retry policies
├── timeout.rs         # Timeout utilities
├── shutdown.rs        # Graceful shutdown
├── mod.rs            # Module exports
├── README.md         # Full documentation
├── EXAMPLES.md       # Detailed examples
└── QUICK_REFERENCE.md # This file
```

## Running Tests

```bash
# All tests
cargo test --package llm-research-api --lib resilience

# Specific module
cargo test circuit_breaker
cargo test retry
cargo test timeout
cargo test shutdown
```
