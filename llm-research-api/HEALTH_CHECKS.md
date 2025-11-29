# Health Check System

Comprehensive health check implementation for the LLM Research API, designed for production Kubernetes deployments.

## Overview

The health check system provides three distinct endpoints optimized for different use cases:

- **`/health/live`** - Liveness probe (Kubernetes)
- **`/health/ready`** - Readiness probe (Kubernetes)
- **`/health`** - Detailed diagnostics

## Architecture

### Components

```
HealthCheckRegistry
├── PostgresHealthCheck (critical)
├── ClickHouseHealthCheck (non-critical)
└── S3HealthCheck (critical)
```

### Key Features

- **Concurrent Execution**: All health checks run in parallel for fast response times
- **Smart Caching**: Results cached with configurable TTL to avoid overwhelming dependencies
- **Timeout Protection**: Individual timeouts per check prevent cascading failures
- **Critical/Non-Critical**: Flexible component classification affects readiness status
- **Extensible**: Implement `HealthCheck` trait for custom checks

## Endpoints

### Liveness Probe: `/health/live`

**Purpose**: Indicates if the application process is running and not deadlocked.

**Kubernetes Usage**: Determines if the pod should be restarted.

**Response**:
```json
{
  "status": "healthy",
  "components": {
    "application": {
      "name": "application",
      "status": "healthy",
      "last_check": "2024-11-28T23:45:00Z"
    }
  },
  "timestamp": "2024-11-28T23:45:00Z",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

**Status Codes**:
- `200 OK` - Application is alive

**Configuration**:
```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3
```

### Readiness Probe: `/health/ready`

**Purpose**: Indicates if the application is ready to receive traffic.

**Kubernetes Usage**: Determines if the pod should receive requests from the service.

**Behavior**:
- Checks **only critical** components (PostgreSQL, S3)
- Returns `503` if any critical component is unhealthy
- Non-critical components (ClickHouse) are ignored

**Response** (healthy):
```json
{
  "status": "healthy",
  "components": {
    "postgres": {
      "name": "postgres",
      "status": "healthy",
      "latency_ms": 15,
      "last_check": "2024-11-28T23:45:00Z"
    },
    "s3": {
      "name": "s3",
      "status": "healthy",
      "latency_ms": 87,
      "last_check": "2024-11-28T23:45:00Z"
    }
  },
  "timestamp": "2024-11-28T23:45:00Z"
}
```

**Response** (unhealthy):
```json
{
  "status": "unhealthy",
  "components": {
    "postgres": {
      "name": "postgres",
      "status": "unhealthy",
      "message": "Database error: connection refused",
      "last_check": "2024-11-28T23:45:00Z"
    },
    "s3": {
      "name": "s3",
      "status": "healthy",
      "latency_ms": 87,
      "last_check": "2024-11-28T23:45:00Z"
    }
  },
  "timestamp": "2024-11-28T23:45:00Z"
}
```

**Status Codes**:
- `200 OK` - All critical dependencies healthy
- `503 Service Unavailable` - One or more critical dependencies unhealthy

**Configuration**:
```yaml
readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 2
```

### Detailed Health: `/health`

**Purpose**: Comprehensive diagnostics including all components, latencies, and application metadata.

**Use Cases**:
- Operational monitoring
- Debugging
- Metrics collection
- Status dashboards

**Response**:
```json
{
  "status": "degraded",
  "components": {
    "postgres": {
      "name": "postgres",
      "status": "healthy",
      "latency_ms": 12,
      "last_check": "2024-11-28T23:45:00Z"
    },
    "clickhouse": {
      "name": "clickhouse",
      "status": "degraded",
      "message": "Database error: connection timeout",
      "latency_ms": 5001,
      "last_check": "2024-11-28T23:45:00Z"
    },
    "s3": {
      "name": "s3",
      "status": "healthy",
      "latency_ms": 92,
      "last_check": "2024-11-28T23:45:00Z"
    }
  },
  "timestamp": "2024-11-28T23:45:00Z",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

**Status Codes**:
- `200 OK` - Overall status is healthy or degraded
- `503 Service Unavailable` - Overall status is unhealthy

## Health Check Types

### Built-in Health Checks

#### PostgresHealthCheck

Verifies PostgreSQL database connectivity with a simple `SELECT 1` query.

**Configuration**:
```rust
let check = PostgresHealthCheck::with_config(
    db_pool,
    HealthCheckConfig::critical()
        .with_timeout(Duration::from_secs(3))
        .with_cache_ttl(Duration::from_secs(30))
);
```

**Health States**:
- `Healthy`: Query succeeded within timeout
- `Unhealthy`: Connection failed or query timed out

#### ClickHouseHealthCheck

Verifies ClickHouse connectivity with a ping query.

**Configuration**:
```rust
let check = ClickHouseHealthCheck::with_config(
    client,
    HealthCheckConfig::non_critical()  // Won't fail readiness
        .with_timeout(Duration::from_secs(5))
        .with_cache_ttl(Duration::from_secs(60))
);
```

**Health States**:
- `Healthy`: Query succeeded
- `Degraded`: Query failed (if non-critical)
- `Unhealthy`: Query failed (if critical)

#### S3HealthCheck

Verifies S3 bucket access with a HEAD bucket operation.

**Configuration**:
```rust
let check = S3HealthCheck::with_config(
    s3_client,
    bucket_name,
    HealthCheckConfig::critical()
        .with_timeout(Duration::from_secs(5))
        .with_cache_ttl(Duration::from_secs(45))
);
```

**Health States**:
- `Healthy`: Bucket accessible
- `Unhealthy`: Bucket inaccessible or operation timed out

### Custom Health Checks

Implement the `HealthCheck` trait for custom checks:

```rust
use async_trait::async_trait;
use llm_research_api::{HealthCheck, ComponentHealth, HealthCheckConfig};

pub struct RedisHealthCheck {
    client: redis::Client,
    config: HealthCheckConfig,
}

#[async_trait]
impl HealthCheck for RedisHealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = std::time::Instant::now();

        match tokio::time::timeout(
            self.config.check_timeout,
            self.client.get_async_connection(),
        ).await {
            Ok(Ok(_)) => {
                ComponentHealth::healthy("redis")
                    .with_latency(start.elapsed())
            }
            Ok(Err(e)) => {
                ComponentHealth::unhealthy("redis", format!("Redis error: {}", e))
            }
            Err(_) => {
                ComponentHealth::unhealthy("redis", "Health check timed out")
            }
        }
    }

    fn name(&self) -> &str {
        "redis"
    }

    fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}
```

## Configuration

### HealthCheckConfig

Controls timeout, caching, and criticality for individual health checks.

```rust
// Critical component (affects readiness)
let config = HealthCheckConfig::critical()
    .with_timeout(Duration::from_secs(3))
    .with_cache_ttl(Duration::from_secs(30));

// Non-critical component (degraded, not unhealthy)
let config = HealthCheckConfig::non_critical()
    .with_timeout(Duration::from_secs(5))
    .with_cache_ttl(Duration::from_secs(60));
```

**Parameters**:
- `check_timeout`: Maximum time for health check execution (default: 5s)
- `cache_ttl`: How long to cache results (default: 30s)
- `is_critical`: Whether component affects readiness (default: true)

### Recommended Settings

| Component | Critical | Timeout | Cache TTL | Rationale |
|-----------|----------|---------|-----------|-----------|
| PostgreSQL | Yes | 2-3s | 30s | Primary datastore, fast queries |
| ClickHouse | No | 5s | 60s | Analytics, can tolerate degradation |
| S3 | Yes | 5s | 45s | Storage backend, network latency |
| Redis | Maybe | 2s | 30s | Cache, may be non-critical |
| External API | No | 10s | 120s | Third-party, should not block readiness |

## Integration

### Basic Setup

```rust
use llm_research_api::{
    HealthCheckRegistry, HealthCheckState,
    PostgresHealthCheck, S3HealthCheck,
    liveness_handler, readiness_handler, health_handler,
};
use axum::{routing::get, Router};
use std::sync::Arc;

// Create health checks
let postgres_check = Arc::new(PostgresHealthCheck::new(db_pool));
let s3_check = Arc::new(S3HealthCheck::new(s3_client, bucket));

// Create registry
let registry = Arc::new(
    HealthCheckRegistry::new("1.0.0")
        .register(postgres_check)
        .register(s3_check)
);

let health_state = HealthCheckState::new(registry);

// Add routes
let app = Router::new()
    .route("/health/live", get(liveness_handler))
    .route("/health/ready", get(readiness_handler))
    .route("/health", get(health_handler))
    .with_state(health_state);
```

### Advanced Integration with Application State

```rust
#[derive(Clone)]
struct AppState {
    db_pool: PgPool,
    s3_client: S3Client,
    health: HealthCheckState,
}

let app = Router::new()
    // Health endpoints
    .route("/health/live", get(liveness_handler))
    .route("/health/ready", get(readiness_handler))
    .route("/health", get(health_handler))
    // API endpoints
    .route("/api/users", get(get_users))
    .with_state(app_state);
```

## Performance Considerations

### Caching Strategy

Health check results are cached to prevent overwhelming dependencies with frequent health checks:

1. **First Request**: Executes actual health check
2. **Subsequent Requests**: Returns cached result if TTL not expired
3. **Cache Expiry**: Executes new health check, updates cache

**Benefits**:
- Reduces load on critical infrastructure
- Faster response times
- Protects against health check storms

**Example Cache Behavior**:
```
t=0s:  /health/ready → Execute checks → 150ms response → Cache results
t=5s:  /health/ready → Return cached → 2ms response
t=10s: /health/ready → Return cached → 2ms response
t=30s: /health/ready → TTL expired → Execute checks → 150ms response
```

### Concurrent Execution

All health checks run concurrently using `futures::future::join_all`:

```rust
// Sequential would take: 150ms + 200ms + 100ms = 450ms
// Concurrent takes: max(150ms, 200ms, 100ms) = 200ms
```

**Timeout Protection**: Individual timeouts prevent slow checks from blocking others.

## Monitoring and Alerting

### Prometheus Metrics

The health check system can be integrated with Prometheus for monitoring:

```rust
// Export health status as gauge
health_status{component="postgres"} 1.0  // 1.0 = healthy, 0.5 = degraded, 0.0 = unhealthy
health_status{component="clickhouse"} 0.5
health_status{component="s3"} 1.0

// Export latency as histogram
health_check_duration_seconds{component="postgres"} 0.015
health_check_duration_seconds{component="s3"} 0.087
```

### Recommended Alerts

```yaml
# Alert if readiness probe failing
- alert: ServiceNotReady
  expr: health_status{critical="true"} < 1.0
  for: 2m
  annotations:
    summary: "Service not ready due to {{ $labels.component }}"

# Alert if health check latency high
- alert: HealthCheckSlow
  expr: health_check_duration_seconds > 3.0
  for: 5m
  annotations:
    summary: "Health check for {{ $labels.component }} is slow"
```

## Testing

The module includes comprehensive tests:

```bash
# Run health check tests
cargo test --package llm-research-api observability::health

# Run with output
cargo test --package llm-research-api observability::health -- --nocapture
```

### Test Coverage

- ✅ Health status aggregation
- ✅ Component health constructors
- ✅ HTTP status code mapping
- ✅ Timeout handling
- ✅ Cache expiration
- ✅ Critical vs non-critical components
- ✅ Concurrent execution
- ✅ Response format validation

## Examples

### Running Examples

```bash
# Basic health check example
cargo run --example health_check_example

# Kubernetes integration example
cargo run --example kubernetes_health_integration
```

### Example Output

```
🏥 Kubernetes Health Check Integration Example
============================================================

1️⃣  Initializing dependencies...
   ✓ PostgreSQL connected
   ✓ ClickHouse configured
   ✓ S3 configured

2️⃣  Configuring health checks...
   ✓ PostgreSQL health check (critical)
   ✓ ClickHouse health check (non-critical)
   ✓ S3 health check (critical)

3️⃣  Creating health check registry...
   ✓ Registry created with 3 health checks
   ✓ Critical checks: PostgreSQL, S3
   ✓ Non-critical checks: ClickHouse

🚀 Server running on http://0.0.0.0:8080

Health Check Endpoints:
  ├─ http://0.0.0.0:8080/health/live  (Kubernetes liveness probe)
  ├─ http://0.0.0.0:8080/health/ready (Kubernetes readiness probe)
  └─ http://0.0.0.0:8080/health       (Detailed diagnostics)
```

## Troubleshooting

### Common Issues

#### 1. Readiness probe keeps failing

**Symptoms**: Pod never becomes ready, no traffic received

**Causes**:
- Database connection pool exhausted
- S3 credentials invalid
- Network policy blocking access
- Timeout too aggressive

**Solutions**:
```rust
// Increase timeout
HealthCheckConfig::critical()
    .with_timeout(Duration::from_secs(10))

// Make component non-critical temporarily
HealthCheckConfig::non_critical()

// Check connection pool size
PgPoolOptions::new()
    .max_connections(20)  // Increase pool size
```

#### 2. Health checks causing database load

**Symptoms**: Database CPU spikes during health checks

**Solutions**:
```rust
// Increase cache TTL
HealthCheckConfig::critical()
    .with_cache_ttl(Duration::from_secs(60))  // Cache for 1 minute

// Use connection pooling
let db_pool = PgPoolOptions::new()
    .max_connections(10)  // Dedicated connections for health checks
```

#### 3. Intermittent health check failures

**Symptoms**: Occasional 503 responses, logs show timeouts

**Solutions**:
```rust
// Increase timeout for unreliable networks
HealthCheckConfig::critical()
    .with_timeout(Duration::from_secs(10))

// Add retry logic in custom health check
for attempt in 0..3 {
    match check().await {
        Ok(result) => return result,
        Err(_) if attempt < 2 => tokio::time::sleep(Duration::from_millis(100)).await,
        Err(e) => return ComponentHealth::unhealthy("component", e.to_string()),
    }
}
```

## Best Practices

1. **Keep liveness simple**: Never include external dependencies in liveness checks
2. **Mark analytics non-critical**: Components like ClickHouse should degrade gracefully
3. **Use appropriate timeouts**: Balance responsiveness vs reliability
4. **Cache aggressively**: High-frequency health checks should use 30s+ cache TTL
5. **Monitor health check latency**: Alert if checks are consistently slow
6. **Test failure scenarios**: Verify behavior when dependencies are unavailable
7. **Document critical dependencies**: Make it clear what's required for readiness

## Security Considerations

- Health endpoints typically **don't require authentication** (Kubernetes needs access)
- Avoid exposing sensitive information in health check messages
- Consider rate limiting health endpoints to prevent DoS
- Use internal service mesh for health checks when possible

## License

This implementation is part of the LLM Research Lab project.
