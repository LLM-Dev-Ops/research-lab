# Resilience Module

A comprehensive resilience module for the LLM Research API, providing essential fault-tolerance patterns to build reliable distributed systems.

## Overview

This module implements four key resilience patterns:

1. **Circuit Breaker** - Prevents cascading failures by temporarily blocking requests to failing services
2. **Retry with Exponential Backoff** - Handles transient failures with configurable retry strategies
3. **Timeout** - Prevents operations from running indefinitely
4. **Graceful Shutdown** - Coordinates clean shutdown of application components

## Components

### Circuit Breaker

The circuit breaker pattern prevents cascading failures by monitoring for errors and temporarily blocking requests when a threshold is exceeded.

#### Features

- Three states: Closed, Open, and HalfOpen
- Configurable failure and success thresholds
- Automatic state transitions with timing
- Thread-safe implementation using atomics
- Comprehensive metrics tracking

#### Example

```rust
use llm_research_api::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use std::time::Duration;

let config = CircuitBreakerConfig {
    failure_threshold: 5,
    success_threshold: 2,
    timeout: Duration::from_secs(60),
    half_open_max_requests: 3,
};

let breaker = CircuitBreaker::new("external_api", config);

// Use the circuit breaker
match breaker.call(|| async {
    // Your operation here
    call_external_api().await
}).await {
    Ok(result) => println!("Success: {:?}", result),
    Err(e) => eprintln!("Error: {}", e),
}

// Check metrics
let stats = breaker.metrics();
println!("Failures: {}, Successes: {}", stats.failures, stats.successes);
```

#### Configuration

- `failure_threshold`: Number of consecutive failures before opening circuit (default: 5)
- `success_threshold`: Number of consecutive successes in half-open state to close circuit (default: 2)
- `timeout`: Time to wait before transitioning from open to half-open (default: 60s)
- `half_open_max_requests`: Maximum concurrent requests in half-open state (default: 3)

### Retry with Exponential Backoff

Implements multiple retry strategies for handling transient failures.

#### Features

- Multiple backoff strategies: Exponential, Constant, Linear
- Configurable jitter: Full, Equal, Decorrelated
- Custom retry policies via trait
- Attempt tracking and error context

#### Example

```rust
use llm_research_api::resilience::retry::{retry, ExponentialBackoff, RetryConfig};
use std::time::Duration;

let config = RetryConfig {
    max_attempts: 5,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    multiplier: 2.0,
    jitter: true,
};

let policy = ExponentialBackoff::new(config);

let result = retry(policy, || async {
    // Your operation here
    fetch_data().await
}).await?;
```

#### Backoff Strategies

1. **Exponential Backoff**: Delay increases exponentially (100ms, 200ms, 400ms, 800ms, ...)
2. **Constant Backoff**: Fixed delay between retries
3. **Linear Backoff**: Delay increases linearly (100ms, 200ms, 300ms, ...)

#### Jitter Strategies

- **None**: No randomization
- **Full**: Random value between 0 and calculated delay
- **Equal**: Half delay + random half
- **Decorrelated**: Exponentially weighted moving average

### Timeout

Provides timeout mechanisms for async operations and axum middleware.

#### Features

- Simple async timeout wrapper
- Tower middleware for service integration
- Axum request timeout middleware
- Operation-specific timeout configuration

#### Example

```rust
use llm_research_api::resilience::timeout::{with_timeout, TimeoutConfig};
use std::time::Duration;

// Simple timeout
let result = with_timeout(
    Duration::from_secs(5),
    async {
        slow_operation().await
    }
).await?;

// Configuration with operation-specific timeouts
let config = TimeoutConfig::new(Duration::from_secs(30))
    .with_operation("fast_query", Duration::from_secs(5))
    .with_operation("slow_batch", Duration::from_secs(300));

let timeout_duration = config.get_timeout("fast_query");
```

#### Axum Middleware

```rust
use axum::{Router, routing::get};
use axum::middleware;
use llm_research_api::resilience::timeout::middleware::with_timeout;
use std::time::Duration;

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn(with_timeout(Duration::from_secs(30))));
```

### Graceful Shutdown

Coordinates graceful shutdown across application components with connection draining.

#### Features

- Signal handling (SIGTERM, SIGINT)
- Shutdown hook registration
- Component shutdown coordination
- Connection tracking and draining
- Forceful shutdown timeout

#### Example

```rust
use llm_research_api::resilience::shutdown::{ShutdownSignal, ShutdownCoordinator};
use std::time::Duration;

// Create signal handler
let signal = ShutdownSignal::new();
let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));

// Register shutdown hooks
coordinator.on_shutdown(|| async {
    println!("Cleaning up resources...");
});

// Track connections
coordinator.connection_opened();
// ... do work ...
coordinator.connection_closed();

// Wait for shutdown signal
signal.wait().await;

// Perform graceful shutdown
coordinator.shutdown().await?;
```

#### Connection Guards

```rust
use std::sync::Arc;

let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(30)));

// Connection guard automatically tracks connections
if let Some(_guard) = ConnectionGuard::new(coordinator.clone()) {
    // Process request
    // Connection is automatically closed when guard drops
} else {
    // Shutdown in progress, reject request
}
```

## Combining Patterns

Resilience patterns are most effective when combined:

```rust
use llm_research_api::resilience::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    retry::{retry, ExponentialBackoff, RetryConfig},
    timeout::with_timeout,
};
use std::time::Duration;

// Create circuit breaker
let breaker = CircuitBreaker::new("service", CircuitBreakerConfig::default());

// Create retry policy
let retry_policy = ExponentialBackoff::new(RetryConfig {
    max_attempts: 3,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(5),
    multiplier: 2.0,
    jitter: true,
});

// Combine all patterns
let result = breaker.call(|| async {
    retry(retry_policy.clone(), || async {
        with_timeout(
            Duration::from_secs(5),
            async {
                // Your operation
                external_api_call().await
            }
        ).await
    }).await
}).await?;
```

## Testing

Each module includes comprehensive tests (8-13 tests per file, 45 total tests):

```bash
# Run all resilience tests
cargo test --package llm-research-api --lib resilience

# Run specific module tests
cargo test --package llm-research-api circuit_breaker
cargo test --package llm-research-api retry
cargo test --package llm-research-api timeout
cargo test --package llm-research-api shutdown
```

## Metrics and Monitoring

### Circuit Breaker Metrics

```rust
let stats = breaker.metrics();
println!("Total failures: {}", stats.failures);
println!("Total successes: {}", stats.successes);
println!("Times opened: {}", stats.opened_count);
println!("Times closed: {}", stats.closed_count);
println!("Rejected requests: {}", stats.rejected_count);
```

### Shutdown Coordination

```rust
println!("Active connections: {}", coordinator.active_connections());
println!("Shutting down: {}", coordinator.is_shutting_down());
```

## Best Practices

1. **Circuit Breaker**
   - Set failure threshold based on your SLA
   - Use shorter timeouts for critical services
   - Monitor state transitions for early warning

2. **Retry**
   - Always use jitter to avoid thundering herd
   - Set max_delay to prevent excessive waiting
   - Consider idempotency of operations

3. **Timeout**
   - Set timeouts slightly higher than expected latency
   - Use operation-specific timeouts for different workloads
   - Include timeout in error messages for debugging

4. **Shutdown**
   - Register shutdown hooks early in application lifecycle
   - Set graceful timeout based on longest operation
   - Reject new connections when shutting down

## Performance Considerations

- Circuit breaker uses atomic operations for minimal overhead
- Retry adds delay only on failures
- Timeout uses tokio's efficient timer implementation
- Shutdown coordination uses RwLock for read-heavy operations

## Thread Safety

All components are thread-safe and can be shared across async tasks:

- Circuit breaker uses `Arc<RwLock>` for state management
- Retry is stateless and can be cloned
- Timeout wraps futures without additional synchronization
- Shutdown coordinator uses atomic operations and locks

## Error Handling

Each pattern has specific error types:

- `CircuitBreakerError<E>`: Open, Rejected, ExecutionFailed(E)
- `RetryError<E>`: Wraps underlying error with attempt count
- `TimeoutError`: Elapsed, Inner
- `ShutdownError`: Timeout, ComponentFailed, AlreadyShuttingDown

## File Structure

```
resilience/
├── mod.rs                 # Module exports (63 lines)
├── circuit_breaker.rs     # Circuit breaker implementation (612 lines, 10 tests)
├── retry.rs              # Retry policies (628 lines, 11 tests)
├── timeout.rs            # Timeout utilities (394 lines, 11 tests)
├── shutdown.rs           # Graceful shutdown (558 lines, 13 tests)
└── README.md             # This file
```

## Dependencies

Required dependencies (already in Cargo.toml):

- `tokio` - Async runtime
- `async-trait` - Async trait support
- `thiserror` - Error handling
- `tracing` - Logging and diagnostics
- `tower` - Service middleware
- `axum` - Web framework integration

## License

This module is part of the LLM Research Lab project and follows the same license.
