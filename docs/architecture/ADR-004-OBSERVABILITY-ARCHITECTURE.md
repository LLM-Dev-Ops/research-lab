# ADR-004: Observability Architecture

## Status
Accepted

## Date
2025-01-15

## Context

Production systems require comprehensive observability to:
- Detect and diagnose issues quickly
- Understand system behavior and performance
- Meet SLA commitments
- Enable capacity planning
- Support post-incident analysis

The three pillars of observability:
1. **Metrics**: Numerical measurements over time
2. **Logs**: Discrete events with context
3. **Traces**: Request flow across services

## Decision

We implement a comprehensive observability stack based on open standards:

| Pillar | Technology | Standard |
|--------|------------|----------|
| Metrics | Prometheus + Grafana | OpenMetrics |
| Logs | Structured JSON + ELK | JSON |
| Traces | OpenTelemetry + Jaeger | OpenTelemetry |
| Alerts | Alertmanager + PagerDuty | - |

### Metrics Architecture

**Prometheus Metrics Types:**

```rust
// Counter - monotonically increasing
static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "path", "status"]
    ).unwrap()
});

// Histogram - distribution of values
static HTTP_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request latency",
        &["method", "path"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap()
});

// Gauge - current value
static DB_CONNECTIONS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "db_pool_connections_active",
        "Active database connections"
    ).unwrap()
});
```

**Key Metrics Categories:**

| Category | Metrics | Purpose |
|----------|---------|---------|
| HTTP | requests_total, duration, size | API performance |
| Database | connections, query_duration, errors | DB health |
| Cache | hits, misses, size, evictions | Cache efficiency |
| Business | experiments_created, runs_completed | Business KPIs |
| System | cpu, memory, goroutines | Resource usage |

### Logging Architecture

**Structured Logging Format:**

```json
{
  "timestamp": "2025-01-15T10:30:00.123Z",
  "level": "INFO",
  "target": "llm_research_api::handlers::experiments",
  "message": "Experiment created",
  "trace_id": "abc123",
  "span_id": "def456",
  "request_id": "req-789",
  "user_id": "user-012",
  "experiment_id": "exp-345",
  "duration_ms": 45,
  "fields": {
    "experiment_name": "GPT-4 Benchmark",
    "model_count": 2
  }
}
```

**Log Levels:**

| Level | Use Case | Example |
|-------|----------|---------|
| ERROR | Failures requiring attention | Database connection failed |
| WARN | Potential issues | Rate limit approaching |
| INFO | Normal operations | Request completed |
| DEBUG | Detailed debugging | SQL query executed |
| TRACE | Verbose tracing | Function entry/exit |

**Sensitive Data Handling:**

```rust
pub struct SensitiveDataRedactor {
    patterns: Vec<Regex>,
}

impl SensitiveDataRedactor {
    pub fn redact(&self, value: &str) -> String {
        let mut result = value.to_string();
        for pattern in &self.patterns {
            result = pattern.replace_all(&result, "[REDACTED]").to_string();
        }
        result
    }
}

// Redact patterns
const SENSITIVE_PATTERNS: &[&str] = &[
    r"(?i)password[=:]\s*\S+",
    r"(?i)api[_-]?key[=:]\s*\S+",
    r"(?i)bearer\s+\S+",
    r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
];
```

### Distributed Tracing Architecture

**OpenTelemetry Integration:**

```rust
pub fn init_tracing(config: &TracingConfig) -> Result<(), TracingError> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp_endpoint),
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::TraceIdRatioBased(config.sample_rate))
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", config.service_name.clone()),
                    KeyValue::new("service.version", config.version.clone()),
                ])),
        )
        .install_batch(runtime::Tokio)?;

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(telemetry)
        .with(fmt::layer().json())
        .init();

    Ok(())
}
```

**Span Attributes:**

```rust
#[instrument(
    name = "create_experiment",
    skip(state, req),
    fields(
        experiment_name = %req.name,
        owner_id = %req.owner_id,
        model_count = req.config.model_ids.len(),
    )
)]
pub async fn create_experiment(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    // Implementation
}
```

### Alerting Architecture

**Alert Severity Levels:**

| Severity | Response Time | Notification | Example |
|----------|---------------|--------------|---------|
| Critical | 5 minutes | PagerDuty + Slack | Service down |
| Warning | 15 minutes | Slack | High error rate |
| Info | Next business day | Email | Approaching quota |

**SLO-Based Alerts:**

```yaml
# Error Budget Alert
- alert: ErrorBudgetBurn
  expr: |
    (
      1 - (
        sum(rate(http_requests_total{status!~"5.."}[1h]))
        /
        sum(rate(http_requests_total[1h]))
      )
    ) > (1 - 0.999) * 2
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Error budget burning too fast"
    description: "Error rate is 2x the allowed budget"

# Latency SLO Alert
- alert: LatencySLOViolation
  expr: |
    histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 0.5
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "P95 latency exceeds SLO"
```

### Health Check Architecture

**Health Check Types:**

```rust
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub latency: Duration,
    pub message: Option<String>,
}

// Liveness: Is the process running?
pub async fn liveness() -> &'static str {
    "OK"
}

// Readiness: Can it serve traffic?
pub async fn readiness(state: &AppState) -> HealthCheckResult {
    let checks = tokio::join!(
        check_database(&state.db_pool),
        check_cache(&state.cache),
        check_storage(&state.s3_client),
    );
    aggregate_health(checks)
}

// Startup: Has it finished initializing?
pub async fn startup(state: &AppState) -> HealthCheckResult {
    // Check migrations, cache warm-up, etc.
}
```

**Kubernetes Probes:**

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 3

startupProbe:
  httpGet:
    path: /health/startup
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 30
```

## Alternatives Considered

### Datadog (All-in-One)
- **Pro**: Single vendor, unified experience
- **Con**: Vendor lock-in, cost at scale
- **Decision**: Deferred; may adopt for convenience

### Loki Instead of ELK
- **Pro**: Native Prometheus integration
- **Con**: Less mature, limited query capabilities
- **Decision**: Consider for future migration

### AWS X-Ray Instead of Jaeger
- **Pro**: AWS native, less infrastructure
- **Con**: AWS lock-in, limited customization
- **Decision**: Rejected; prefer open standards

## Consequences

### Positive
- **Open Standards**: No vendor lock-in
- **Correlation**: Traces, logs, metrics linked by IDs
- **Cost Effective**: Self-hosted, predictable costs
- **Extensible**: Add custom metrics easily

### Negative
- **Operational Overhead**: Multiple systems to manage
- **Learning Curve**: Team needs to learn multiple tools
- **Storage Costs**: High cardinality data is expensive

### Mitigations

**Operational Simplification:**
- Helm charts for deployment
- Terraform for infrastructure
- Automated backup and retention

**Cost Management:**
- Sampling for high-volume traces
- Log retention policies
- Metric cardinality limits

## Observability Dashboard Structure

```
grafana/
├── dashboards/
│   ├── overview.json           # System overview
│   ├── api-performance.json    # API latency and errors
│   ├── database.json           # Database metrics
│   ├── cache.json              # Cache performance
│   ├── business-kpis.json      # Business metrics
│   └── slo-dashboard.json      # SLO tracking
└── alerts/
    ├── infrastructure.yaml     # System alerts
    ├── application.yaml        # App alerts
    └── slo.yaml               # SLO alerts
```

## References
- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Google SRE Book - Monitoring](https://sre.google/sre-book/monitoring-distributed-systems/)
- [Grafana Dashboard Best Practices](https://grafana.com/docs/grafana/latest/best-practices/)
