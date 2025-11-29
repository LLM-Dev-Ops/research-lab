# Health Checks - Quick Reference

## Endpoints

| Endpoint | Purpose | K8s Probe | Status Codes |
|----------|---------|-----------|--------------|
| `/health/live` | Process alive? | Liveness | 200 |
| `/health/ready` | Ready for traffic? | Readiness | 200, 503 |
| `/health` | Detailed diagnostics | - | 200, 503 |

## Quick Setup

```rust
use llm_research_api::*;
use std::sync::Arc;

// 1. Create health checks
let postgres = Arc::new(PostgresHealthCheck::new(db_pool));
let s3 = Arc::new(S3HealthCheck::new(s3_client, bucket));

// 2. Create registry
let registry = Arc::new(
    HealthCheckRegistry::new("1.0.0")
        .register(postgres)
        .register(s3)
);

// 3. Add routes
let app = Router::new()
    .route("/health/live", get(liveness_handler))
    .route("/health/ready", get(readiness_handler))
    .route("/health", get(health_handler))
    .with_state(HealthCheckState::new(registry));
```

## Configuration Examples

```rust
// Critical component (affects readiness)
HealthCheckConfig::critical()
    .with_timeout(Duration::from_secs(3))
    .with_cache_ttl(Duration::from_secs(30))

// Non-critical component (won't fail readiness)
HealthCheckConfig::non_critical()
    .with_timeout(Duration::from_secs(5))
    .with_cache_ttl(Duration::from_secs(60))
```

## Kubernetes Configuration

```yaml
# Liveness Probe
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3

# Readiness Probe
readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 2
```

## Health Check Components

| Component | Type | Default Timeout | Default Cache | Critical |
|-----------|------|-----------------|---------------|----------|
| PostgresHealthCheck | Database | 3s | 30s | Yes |
| ClickHouseHealthCheck | Database | 3s | 30s | No |
| S3HealthCheck | Storage | 5s | 30s | Yes |

## Response Examples

### Healthy Response
```json
{
  "status": "healthy",
  "components": {
    "postgres": {
      "status": "healthy",
      "latency_ms": 15
    }
  },
  "timestamp": "2024-11-28T23:45:00Z",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

### Degraded Response
```json
{
  "status": "degraded",
  "components": {
    "postgres": {
      "status": "healthy",
      "latency_ms": 15
    },
    "clickhouse": {
      "status": "degraded",
      "message": "Connection timeout"
    }
  }
}
```

### Unhealthy Response (503)
```json
{
  "status": "unhealthy",
  "components": {
    "postgres": {
      "status": "unhealthy",
      "message": "Database error: connection refused"
    }
  }
}
```

## Custom Health Check

```rust
use async_trait::async_trait;

pub struct CustomHealthCheck {
    config: HealthCheckConfig,
}

#[async_trait]
impl HealthCheck for CustomHealthCheck {
    async fn check(&self) -> ComponentHealth {
        // Perform check with timeout
        match tokio::time::timeout(
            self.config.check_timeout,
            perform_check()
        ).await {
            Ok(Ok(_)) => ComponentHealth::healthy("custom"),
            Ok(Err(e)) => ComponentHealth::unhealthy("custom", e.to_string()),
            Err(_) => ComponentHealth::unhealthy("custom", "Timeout"),
        }
    }

    fn name(&self) -> &str { "custom" }
    fn config(&self) -> &HealthCheckConfig { &self.config }
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Readiness always failing | Increase timeout or make component non-critical |
| High database load | Increase cache TTL to 60s+ |
| Slow health checks | Reduce timeout, check network latency |
| Pod never ready | Check logs, verify database connectivity |

## Performance

- **Caching**: Results cached for 30-60s
- **Concurrency**: All checks run in parallel
- **Timeouts**: 2-5s depending on component
- **Response time**: ~2ms (cached), ~150ms (uncached)

## Examples

```bash
# Run examples
cargo run --example health_check_example
cargo run --example kubernetes_health_integration

# Test endpoints
curl http://localhost:8080/health/live
curl http://localhost:8080/health/ready
curl http://localhost:8080/health
```
