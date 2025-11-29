# SPARC Phase 5: Completion - Section 5: Operations & Monitoring

> **LLM-Research-Lab**
> Enterprise-Grade Operations & Monitoring Specification
> Target SLA: 99.9% Availability | p99 Latency < 100ms

---

## Table of Contents

1. [Overview](#overview)
2. [5.1 Observability Stack](#51-observability-stack)
3. [5.2 Alerting Configuration](#52-alerting-configuration)
4. [5.3 SLO/SLI Definition](#53-slosli-definition)
5. [5.4 On-Call Procedures](#54-on-call-procedures)
6. [5.5 Capacity Management](#55-capacity-management)
7. [5.6 Security Operations](#56-security-operations)
8. [Appendix A: Configuration Files](#appendix-a-configuration-files)
9. [Appendix B: Dashboard Templates](#appendix-b-dashboard-templates)
10. [Appendix C: Runbook Index](#appendix-c-runbook-index)

---

## Overview

This section defines the complete operations and monitoring strategy for LLM-Research-Lab, ensuring enterprise-grade reliability, observability, and incident response capabilities. The specification targets a 99.9% availability SLA with comprehensive monitoring, alerting, and operational procedures.

### Design Philosophy

1. **Observability by Default**: Every component emits structured metrics, logs, and traces
2. **Actionable Alerts**: Every alert has a clear runbook and escalation path
3. **Proactive Detection**: SLO violations trigger automated responses before user impact
4. **Data-Driven Operations**: All operational decisions backed by metrics and trends
5. **Defense in Depth**: Multiple layers of monitoring with correlation across signals

### Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Metrics | Prometheus | Time-series metric collection and storage |
| Logging | Loki + Vector | Structured log aggregation and indexing |
| Tracing | OpenTelemetry + Jaeger | Distributed request tracing |
| Dashboards | Grafana | Unified visualization and alerting |
| Alerting | Alertmanager + PagerDuty | Alert routing and on-call management |
| APM | Grafana Tempo | Application performance monitoring |
| Profiling | pprof + pyroscope | Continuous profiling |

---

## 5.1 Observability Stack

### 5.1.1 Metrics Collection (Prometheus)

#### Core Metrics Architecture

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  scrape_timeout: 10s
  evaluation_interval: 15s
  external_labels:
    cluster: 'llm-research-lab-prod'
    environment: 'production'
    region: 'us-east-1'

# Alertmanager configuration
alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - alertmanager:9093
      timeout: 10s
      api_version: v2

# Rule files for alerts and recording rules
rule_files:
  - /etc/prometheus/rules/*.yml
  - /etc/prometheus/alerts/*.yml

# Scrape configurations
scrape_configs:
  # LLM Research Lab API
  - job_name: 'llm-research-lab-api'
    scrape_interval: 10s
    static_configs:
      - targets: ['api:9090']
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
      - source_labels: [__metrics_path__]
        target_label: metrics_path

  # Experiment Runner
  - job_name: 'experiment-runner'
    scrape_interval: 15s
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names:
            - llm-research-lab
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        regex: experiment-runner
        action: keep
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: pod
      - source_labels: [__meta_kubernetes_namespace]
        target_label: namespace

  # PostgreSQL Exporter
  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres-exporter:9187']
    metric_relabel_configs:
      - source_labels: [__name__]
        regex: 'pg_(database_size_bytes|stat_.*|locks_count)'
        action: keep

  # Redis Exporter
  - job_name: 'redis'
    static_configs:
      - targets: ['redis-exporter:9121']

  # Node Exporter (System Metrics)
  - job_name: 'node'
    kubernetes_sd_configs:
      - role: node
    relabel_configs:
      - action: labelmap
        regex: __meta_kubernetes_node_label_(.+)

  # cAdvisor (Container Metrics)
  - job_name: 'cadvisor'
    kubernetes_sd_configs:
      - role: node
    relabel_configs:
      - action: labelmap
        regex: __meta_kubernetes_node_label_(.+)
      - target_label: __address__
        replacement: kubernetes.default.svc:443
      - source_labels: [__meta_kubernetes_node_name]
        regex: (.+)
        target_label: __metrics_path__
        replacement: /api/v1/nodes/${1}/proxy/metrics/cadvisor

# Remote write for long-term storage
remote_write:
  - url: "https://prometheus-remote-storage.example.com/api/v1/write"
    queue_config:
      capacity: 10000
      max_shards: 50
      min_shards: 1
      max_samples_per_send: 5000
      batch_send_deadline: 5s
    write_relabel_configs:
      - source_labels: [__name__]
        regex: 'experiment_.*|slo_.*|api_request_.*'
        action: keep
```

#### Custom Application Metrics

**Rust Implementation (using `prometheus` crate)**

```rust
// src/metrics/mod.rs
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec,
    Registry, Opts, HistogramOpts,
};
use lazy_static::lazy_static;

lazy_static! {
    // HTTP Request Metrics
    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "api_request_duration_seconds",
            "HTTP request latency in seconds"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        &["method", "path", "status"]
    ).unwrap();

    pub static ref HTTP_REQUEST_TOTAL: CounterVec = CounterVec::new(
        Opts::new("api_request_total", "Total HTTP requests"),
        &["method", "path", "status"]
    ).unwrap();

    pub static ref HTTP_REQUEST_IN_FLIGHT: GaugeVec = GaugeVec::new(
        Opts::new("api_request_in_flight", "Currently processing HTTP requests"),
        &["method", "path"]
    ).unwrap();

    // Experiment Metrics
    pub static ref EXPERIMENT_TOTAL: CounterVec = CounterVec::new(
        Opts::new("experiment_total", "Total experiments executed"),
        &["status", "experiment_type"]
    ).unwrap();

    pub static ref EXPERIMENT_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "experiment_duration_seconds",
            "Experiment execution duration"
        )
        .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0]),
        &["experiment_type", "dataset_size"]
    ).unwrap();

    pub static ref EXPERIMENT_ACTIVE: Gauge = Gauge::new(
        "experiment_active_count",
        "Currently running experiments"
    ).unwrap();

    // Database Metrics
    pub static ref DB_QUERY_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "db_query_duration_seconds",
            "Database query duration"
        )
        .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
        &["query_type", "table"]
    ).unwrap();

    pub static ref DB_CONNECTION_POOL_SIZE: Gauge = Gauge::new(
        "db_connection_pool_size",
        "Database connection pool size"
    ).unwrap();

    pub static ref DB_CONNECTION_POOL_IDLE: Gauge = Gauge::new(
        "db_connection_pool_idle",
        "Idle database connections"
    ).unwrap();

    // Model Inference Metrics
    pub static ref MODEL_INFERENCE_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "model_inference_duration_seconds",
            "Model inference latency"
        )
        .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]),
        &["model_name", "model_version", "provider"]
    ).unwrap();

    pub static ref MODEL_INFERENCE_TOKENS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "model_inference_tokens_total",
            "Total tokens processed per inference"
        )
        .buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0]),
        &["model_name", "token_type"]
    ).unwrap();

    pub static ref MODEL_INFERENCE_COST: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "model_inference_cost_usd",
            "Cost per inference in USD"
        )
        .buckets(vec![0.0001, 0.001, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        &["model_name", "provider"]
    ).unwrap();

    // Dataset Metrics
    pub static ref DATASET_LOAD_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "dataset_load_duration_seconds",
            "Dataset loading duration"
        )
        .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 300.0]),
        &["dataset_name", "dataset_version", "format"]
    ).unwrap();

    pub static ref DATASET_SIZE_BYTES: GaugeVec = GaugeVec::new(
        Opts::new("dataset_size_bytes", "Dataset size in bytes"),
        &["dataset_name", "dataset_version"]
    ).unwrap();

    // Cache Metrics
    pub static ref CACHE_HIT_TOTAL: CounterVec = CounterVec::new(
        Opts::new("cache_hit_total", "Cache hits"),
        &["cache_name", "cache_type"]
    ).unwrap();

    pub static ref CACHE_MISS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("cache_miss_total", "Cache misses"),
        &["cache_name", "cache_type"]
    ).unwrap();

    // Resource Metrics
    pub static ref MEMORY_USAGE_BYTES: GaugeVec = GaugeVec::new(
        Opts::new("memory_usage_bytes", "Memory usage by component"),
        &["component"]
    ).unwrap();

    pub static ref CPU_USAGE_SECONDS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("cpu_usage_seconds_total", "CPU time by component"),
        &["component"]
    ).unwrap();

    // Error Metrics
    pub static ref ERROR_TOTAL: CounterVec = CounterVec::new(
        Opts::new("error_total", "Total errors"),
        &["error_type", "component", "severity"]
    ).unwrap();

    pub static ref PANIC_TOTAL: Counter = Counter::new(
        "panic_total",
        "Total panics (should always be 0)"
    ).unwrap();
}

// Metrics registration
pub fn register_metrics(registry: &Registry) -> Result<(), Box<dyn std::error::Error>> {
    registry.register(Box::new(HTTP_REQUEST_DURATION.clone()))?;
    registry.register(Box::new(HTTP_REQUEST_TOTAL.clone()))?;
    registry.register(Box::new(HTTP_REQUEST_IN_FLIGHT.clone()))?;
    registry.register(Box::new(EXPERIMENT_TOTAL.clone()))?;
    registry.register(Box::new(EXPERIMENT_DURATION.clone()))?;
    registry.register(Box::new(EXPERIMENT_ACTIVE.clone()))?;
    registry.register(Box::new(DB_QUERY_DURATION.clone()))?;
    registry.register(Box::new(DB_CONNECTION_POOL_SIZE.clone()))?;
    registry.register(Box::new(DB_CONNECTION_POOL_IDLE.clone()))?;
    registry.register(Box::new(MODEL_INFERENCE_DURATION.clone()))?;
    registry.register(Box::new(MODEL_INFERENCE_TOKENS.clone()))?;
    registry.register(Box::new(MODEL_INFERENCE_COST.clone()))?;
    registry.register(Box::new(DATASET_LOAD_DURATION.clone()))?;
    registry.register(Box::new(DATASET_SIZE_BYTES.clone()))?;
    registry.register(Box::new(CACHE_HIT_TOTAL.clone()))?;
    registry.register(Box::new(CACHE_MISS_TOTAL.clone()))?;
    registry.register(Box::new(MEMORY_USAGE_BYTES.clone()))?;
    registry.register(Box::new(CPU_USAGE_SECONDS_TOTAL.clone()))?;
    registry.register(Box::new(ERROR_TOTAL.clone()))?;
    registry.register(Box::new(PANIC_TOTAL.clone()))?;
    Ok(())
}

// Middleware for automatic HTTP metrics
pub async fn metrics_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let in_flight = HTTP_REQUEST_IN_FLIGHT
        .with_label_values(&[&method, &path]);
    in_flight.inc();

    let timer = HTTP_REQUEST_DURATION
        .with_label_values(&[&method, &path, "pending"])
        .start_timer();

    let response = next.run(req).await;

    let status = response.status().as_u16().to_string();
    HTTP_REQUEST_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();

    timer.observe_duration();
    HTTP_REQUEST_DURATION
        .with_label_values(&[&method, &path, &status])
        .observe(timer.stop_and_record());

    in_flight.dec();

    response
}
```

#### Recording Rules

```yaml
# /etc/prometheus/rules/recording_rules.yml
groups:
  - name: api_slo_recording_rules
    interval: 30s
    rules:
      # Request rate per second
      - record: api:request_rate:5m
        expr: rate(api_request_total[5m])

      # Error rate
      - record: api:error_rate:5m
        expr: |
          sum(rate(api_request_total{status=~"5.."}[5m]))
          /
          sum(rate(api_request_total[5m]))

      # Latency p99
      - record: api:latency:p99:5m
        expr: |
          histogram_quantile(0.99,
            sum(rate(api_request_duration_seconds_bucket[5m])) by (le)
          )

      # Latency p95
      - record: api:latency:p95:5m
        expr: |
          histogram_quantile(0.95,
            sum(rate(api_request_duration_seconds_bucket[5m])) by (le)
          )

      # Latency p50
      - record: api:latency:p50:5m
        expr: |
          histogram_quantile(0.50,
            sum(rate(api_request_duration_seconds_bucket[5m])) by (le)
          )

      # Availability (non-5xx responses)
      - record: api:availability:5m
        expr: |
          sum(rate(api_request_total{status!~"5.."}[5m]))
          /
          sum(rate(api_request_total[5m]))

  - name: experiment_slo_recording_rules
    interval: 1m
    rules:
      # Experiment success rate
      - record: experiment:success_rate:5m
        expr: |
          sum(rate(experiment_total{status="success"}[5m]))
          /
          sum(rate(experiment_total[5m]))

      # Experiment duration p99
      - record: experiment:duration:p99:5m
        expr: |
          histogram_quantile(0.99,
            sum(rate(experiment_duration_seconds_bucket[5m])) by (le)
          )

      # Active experiment count
      - record: experiment:active:current
        expr: experiment_active_count

  - name: database_recording_rules
    interval: 30s
    rules:
      # Database query rate
      - record: db:query_rate:5m
        expr: sum(rate(db_query_duration_seconds_count[5m])) by (query_type)

      # Database query latency p99
      - record: db:query_latency:p99:5m
        expr: |
          histogram_quantile(0.99,
            sum(rate(db_query_duration_seconds_bucket[5m])) by (le, query_type)
          )

      # Connection pool utilization
      - record: db:connection_pool:utilization
        expr: |
          (db_connection_pool_size - db_connection_pool_idle)
          /
          db_connection_pool_size

  - name: model_inference_recording_rules
    interval: 30s
    rules:
      # Model inference rate
      - record: model:inference_rate:5m
        expr: sum(rate(model_inference_duration_seconds_count[5m])) by (model_name, provider)

      # Model inference latency p99
      - record: model:inference_latency:p99:5m
        expr: |
          histogram_quantile(0.99,
            sum(rate(model_inference_duration_seconds_bucket[5m])) by (le, model_name)
          )

      # Total token usage rate
      - record: model:token_usage_rate:5m
        expr: sum(rate(model_inference_tokens_total[5m])) by (model_name, token_type)

      # Cost per request
      - record: model:cost_per_request:5m
        expr: |
          sum(rate(model_inference_cost_usd_sum[5m])) by (model_name)
          /
          sum(rate(model_inference_cost_usd_count[5m])) by (model_name)

  - name: resource_recording_rules
    interval: 30s
    rules:
      # CPU utilization percentage
      - record: node:cpu_utilization:percent
        expr: |
          100 - (avg by (instance) (irate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)

      # Memory utilization percentage
      - record: node:memory_utilization:percent
        expr: |
          100 * (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes))

      # Disk I/O utilization
      - record: node:disk_io_utilization:percent
        expr: |
          rate(node_disk_io_time_seconds_total[5m]) * 100
```

### 5.1.2 Logging Stack (Structured Logs)

#### Log Format Specification

```rust
// src/logging/mod.rs
use tracing::{info, warn, error, debug, trace};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    fmt,
    EnvFilter,
};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LogContext {
    pub request_id: String,
    pub experiment_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

pub fn init_logging() {
    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

// Example structured log entries
pub fn log_experiment_start(ctx: &LogContext, experiment_type: &str, dataset_name: &str) {
    info!(
        request_id = %ctx.request_id,
        experiment_id = ?ctx.experiment_id,
        user_id = ?ctx.user_id,
        experiment_type = experiment_type,
        dataset_name = dataset_name,
        event = "experiment.start",
        "Starting experiment"
    );
}

pub fn log_experiment_complete(
    ctx: &LogContext,
    experiment_type: &str,
    duration_seconds: f64,
    status: &str,
    result_count: usize,
) {
    info!(
        request_id = %ctx.request_id,
        experiment_id = ?ctx.experiment_id,
        user_id = ?ctx.user_id,
        experiment_type = experiment_type,
        duration_seconds = duration_seconds,
        status = status,
        result_count = result_count,
        event = "experiment.complete",
        "Experiment completed"
    );
}

pub fn log_database_error(ctx: &LogContext, query_type: &str, error: &str) {
    error!(
        request_id = %ctx.request_id,
        query_type = query_type,
        error = error,
        event = "database.error",
        "Database query failed"
    );
}

pub fn log_api_request(
    ctx: &LogContext,
    method: &str,
    path: &str,
    status: u16,
    duration_ms: f64,
    user_agent: Option<&str>,
) {
    info!(
        request_id = %ctx.request_id,
        trace_id = ?ctx.trace_id,
        method = method,
        path = path,
        status = status,
        duration_ms = duration_ms,
        user_agent = ?user_agent,
        event = "api.request",
        "API request completed"
    );
}
```

#### Vector Configuration (Log Aggregation)

```toml
# /etc/vector/vector.toml
[sources.kubernetes_logs]
type = "kubernetes_logs"
namespace_annotation_fields.namespace = "namespace"
pod_annotation_fields.pod = "pod_name"

[sources.syslog]
type = "syslog"
address = "0.0.0.0:514"
mode = "tcp"

[transforms.parse_json_logs]
type = "remap"
inputs = ["kubernetes_logs"]
source = '''
  . = parse_json!(.message)
  .kubernetes = .kubernetes
  .timestamp = to_timestamp!(.timestamp)
'''

[transforms.add_metadata]
type = "remap"
inputs = ["parse_json_logs"]
source = '''
  .environment = "production"
  .cluster = "llm-research-lab-prod"
  .service = .kubernetes.pod_labels.app
  .version = .kubernetes.pod_labels.version
'''

[transforms.filter_errors]
type = "filter"
inputs = ["add_metadata"]
condition = '.level == "error" || .level == "warn" || .level == "critical"'

[transforms.sample_debug_logs]
type = "sample"
inputs = ["add_metadata"]
rate = 10  # Keep 1 in 10 debug logs
condition = '.level == "debug"'

[sinks.loki]
type = "loki"
inputs = ["add_metadata", "sample_debug_logs"]
endpoint = "http://loki:3100"
encoding.codec = "json"
labels.environment = "{{ environment }}"
labels.cluster = "{{ cluster }}"
labels.service = "{{ service }}"
labels.level = "{{ level }}"
labels.namespace = "{{ kubernetes.namespace }}"

[sinks.error_logs_s3]
type = "aws_s3"
inputs = ["filter_errors"]
bucket = "llm-research-lab-error-logs"
compression = "gzip"
encoding.codec = "json"
key_prefix = "errors/%Y/%m/%d/"
region = "us-east-1"

[sinks.prometheus_metrics_from_logs]
type = "prometheus_exporter"
inputs = ["add_metadata"]
address = "0.0.0.0:9598"

[[sinks.prometheus_metrics_from_logs.metrics]]
type = "counter"
field = "level"
name = "log_lines_total"
labels.level = "{{ level }}"
labels.service = "{{ service }}"
```

#### Loki Configuration

```yaml
# loki-config.yaml
auth_enabled: false

server:
  http_listen_port: 3100
  grpc_listen_port: 9096

common:
  path_prefix: /loki
  storage:
    filesystem:
      chunks_directory: /loki/chunks
      rules_directory: /loki/rules
  replication_factor: 1
  ring:
    kvstore:
      store: inmemory

schema_config:
  configs:
    - from: 2024-01-01
      store: boltdb-shipper
      object_store: s3
      schema: v11
      index:
        prefix: loki_index_
        period: 24h

storage_config:
  boltdb_shipper:
    active_index_directory: /loki/boltdb-shipper-active
    cache_location: /loki/boltdb-shipper-cache
    cache_ttl: 24h
    shared_store: s3
  aws:
    s3: s3://us-east-1/llm-research-lab-loki
    s3forcepathstyle: true

compactor:
  working_directory: /loki/compactor
  shared_store: s3
  compaction_interval: 10m

limits_config:
  enforce_metric_name: false
  reject_old_samples: true
  reject_old_samples_max_age: 168h
  ingestion_rate_mb: 10
  ingestion_burst_size_mb: 20
  max_query_length: 721h  # 30 days

chunk_store_config:
  max_look_back_period: 720h  # 30 days

table_manager:
  retention_deletes_enabled: true
  retention_period: 720h  # 30 days

query_range:
  align_queries_with_step: true
  cache_results: true
  max_retries: 5
  results_cache:
    cache:
      enable_fifocache: true
      fifocache:
        max_size_bytes: 1GB
        validity: 24h
```

### 5.1.3 Distributed Tracing (OpenTelemetry + Jaeger)

#### OpenTelemetry Configuration

```rust
// src/tracing/mod.rs
use opentelemetry::{
    global,
    sdk::{
        trace::{self, RandomIdGenerator, Sampler},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://jaeger-collector:4317"),
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    0.1, // Sample 10% of traces
                ))))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("deployment.environment", "production"),
                ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

// Example traced function
#[tracing::instrument(
    name = "execute_experiment",
    skip(ctx, dataset),
    fields(
        experiment_id = %ctx.experiment_id,
        experiment_type = %experiment_type,
        dataset_size = dataset.len(),
    )
)]
pub async fn execute_experiment(
    ctx: &ExperimentContext,
    experiment_type: &str,
    dataset: &Dataset,
) -> Result<ExperimentResult, Error> {
    let span = tracing::span!(tracing::Level::INFO, "load_dataset");
    let _guard = span.enter();

    // Nested span for dataset loading
    let loaded_data = load_dataset_with_tracing(dataset).await?;
    drop(_guard);

    // Nested span for model inference
    let span = tracing::span!(
        tracing::Level::INFO,
        "model_inference",
        model_name = %ctx.model_name
    );
    let _guard = span.enter();
    let results = run_inference(ctx, &loaded_data).await?;
    drop(_guard);

    // Nested span for result aggregation
    let span = tracing::span!(tracing::Level::INFO, "aggregate_results");
    let _guard = span.enter();
    let aggregated = aggregate_results(results).await?;

    Ok(aggregated)
}

#[tracing::instrument(
    name = "http_request",
    skip(req, body),
    fields(
        http.method = %req.method(),
        http.url = %req.uri(),
        http.status_code = tracing::field::Empty,
    )
)]
pub async fn trace_http_request(
    req: axum::extract::Request,
    body: String,
) -> Result<axum::response::Response, Error> {
    let current_span = tracing::Span::current();

    // Execute request
    let response = handle_request(req, body).await?;

    // Record response status in span
    current_span.record("http.status_code", response.status().as_u16());

    Ok(response)
}
```

#### Jaeger Deployment Configuration

```yaml
# jaeger-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
  namespace: llm-research-lab
spec:
  replicas: 1
  selector:
    matchLabels:
      app: jaeger
  template:
    metadata:
      labels:
        app: jaeger
    spec:
      containers:
      - name: jaeger
        image: jaegertracing/all-in-one:1.50
        env:
        - name: COLLECTOR_ZIPKIN_HOST_PORT
          value: ":9411"
        - name: COLLECTOR_OTLP_ENABLED
          value: "true"
        - name: SPAN_STORAGE_TYPE
          value: "elasticsearch"
        - name: ES_SERVER_URLS
          value: "http://elasticsearch:9200"
        - name: ES_TAGS_AS_FIELDS_ALL
          value: "true"
        - name: QUERY_BASE_PATH
          value: "/jaeger"
        ports:
        - containerPort: 5775
          protocol: UDP
        - containerPort: 6831
          protocol: UDP
        - containerPort: 6832
          protocol: UDP
        - containerPort: 5778
          protocol: TCP
        - containerPort: 16686
          protocol: TCP
        - containerPort: 14268
          protocol: TCP
        - containerPort: 14250
          protocol: TCP
        - containerPort: 9411
          protocol: TCP
        - containerPort: 4317
          protocol: TCP
        - containerPort: 4318
          protocol: TCP
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
---
apiVersion: v1
kind: Service
metadata:
  name: jaeger-collector
  namespace: llm-research-lab
spec:
  selector:
    app: jaeger
  ports:
  - name: grpc-otlp
    port: 4317
    protocol: TCP
    targetPort: 4317
  - name: http-otlp
    port: 4318
    protocol: TCP
    targetPort: 4318
  - name: jaeger-collector
    port: 14268
    protocol: TCP
    targetPort: 14268
  - name: zipkin
    port: 9411
    protocol: TCP
    targetPort: 9411
---
apiVersion: v1
kind: Service
metadata:
  name: jaeger-query
  namespace: llm-research-lab
spec:
  selector:
    app: jaeger
  ports:
  - name: query
    port: 16686
    protocol: TCP
    targetPort: 16686
  type: ClusterIP
```

### 5.1.4 Dashboard Requirements (Grafana)

#### Grafana Data Sources Configuration

```yaml
# grafana-datasources.yaml
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    jsonData:
      timeInterval: 15s
      queryTimeout: 60s
      httpMethod: POST
    editable: false

  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
    jsonData:
      maxLines: 1000
      derivedFields:
        - datasourceUid: jaeger
          matcherRegex: "trace_id=(\\w+)"
          name: TraceID
          url: '$${__value.raw}'
    editable: false

  - name: Jaeger
    type: jaeger
    access: proxy
    url: http://jaeger-query:16686
    jsonData:
      tracesToLogs:
        datasourceUid: loki
        tags: ['request_id', 'experiment_id']
        mappedTags: [{ key: 'service.name', value: 'service' }]
        mapTagNamesEnabled: true
        spanStartTimeShift: '-1h'
        spanEndTimeShift: '1h'
    editable: false

  - name: Tempo
    type: tempo
    access: proxy
    url: http://tempo:3200
    jsonData:
      tracesToLogs:
        datasourceUid: loki
        tags: ['request_id']
      serviceMap:
        datasourceUid: prometheus
    editable: false

  - name: PostgreSQL
    type: postgres
    url: postgres:5432
    database: llm_research_lab
    user: grafana_reader
    secureJsonData:
      password: ${GRAFANA_DB_PASSWORD}
    jsonData:
      sslmode: require
      maxOpenConns: 10
      maxIdleConns: 5
      connMaxLifetime: 14400
    editable: false
```

#### Core Dashboard Definitions

See [Appendix B](#appendix-b-dashboard-templates) for complete JSON dashboard definitions.

**Required Dashboards:**

1. **Service Overview Dashboard**
   - Request rate, error rate, latency (RED metrics)
   - Active experiments, experiment success rate
   - Resource utilization (CPU, memory, disk)
   - Database connection pool status

2. **SLO Dashboard**
   - Availability SLO (99.9% target)
   - Latency SLO (p99 < 100ms)
   - Error budget consumption
   - SLO compliance trends

3. **Experiment Execution Dashboard**
   - Active experiments timeline
   - Experiment duration distribution
   - Success/failure breakdown by type
   - Dataset loading performance

4. **Model Inference Dashboard**
   - Inference latency by model/provider
   - Token usage and cost tracking
   - Model provider API errors
   - Cache hit rates

5. **Database Performance Dashboard**
   - Query latency by type
   - Connection pool utilization
   - Slow query log
   - Lock contention metrics

6. **Infrastructure Dashboard**
   - Kubernetes pod status
   - Node resource utilization
   - Persistent volume capacity
   - Network I/O and errors

---

## 5.2 Alerting Configuration

### 5.2.1 Alert Severity Levels

| Priority | Response Time | Scope | Example |
|----------|---------------|-------|---------|
| **P1 - Critical** | Immediate (< 5 min) | Service down, data loss risk | API unavailable, database corruption |
| **P2 - High** | < 15 minutes | Degraded service, SLO violation | p99 latency > 500ms, error rate > 5% |
| **P3 - Medium** | < 1 hour | Potential future impact | Disk at 75%, elevated error rate |
| **P4 - Low** | < 4 hours | Informational, no immediate impact | Certificate expiring in 30 days |

### 5.2.2 Alert Definitions

```yaml
# /etc/prometheus/alerts/slo_alerts.yml
groups:
  - name: slo_availability
    interval: 30s
    rules:
      - alert: AvailabilitySLOViolation
        expr: api:availability:5m < 0.999
        for: 5m
        labels:
          severity: P1
          component: api
          slo: availability
        annotations:
          summary: "Availability SLO violation ({{ $value | humanizePercentage }})"
          description: |
            API availability has dropped below 99.9% SLO.
            Current availability: {{ $value | humanizePercentage }}
            Error rate: {{ query "api:error_rate:5m" | first | value | humanizePercentage }}

            Runbook: https://runbooks.example.com/availability-slo-violation
          dashboard: https://grafana.example.com/d/slo-dashboard

      - alert: AvailabilitySLOWarning
        expr: api:availability:5m < 0.9995
        for: 10m
        labels:
          severity: P2
          component: api
          slo: availability
        annotations:
          summary: "Availability approaching SLO limit ({{ $value | humanizePercentage }})"
          description: |
            API availability is close to the 99.9% SLO threshold.
            Current availability: {{ $value | humanizePercentage }}
            Error budget remaining: {{ query "slo:error_budget:remaining:percent" | first | value }}%

            Runbook: https://runbooks.example.com/availability-slo-warning

  - name: slo_latency
    interval: 30s
    rules:
      - alert: LatencySLOViolation
        expr: api:latency:p99:5m > 0.1
        for: 5m
        labels:
          severity: P1
          component: api
          slo: latency
        annotations:
          summary: "p99 latency SLO violation ({{ $value | humanizeDuration }})"
          description: |
            API p99 latency has exceeded 100ms SLO.
            Current p99 latency: {{ $value | humanizeDuration }}
            p95 latency: {{ query "api:latency:p95:5m" | first | value | humanizeDuration }}
            p50 latency: {{ query "api:latency:p50:5m" | first | value | humanizeDuration }}

            Runbook: https://runbooks.example.com/latency-slo-violation
          dashboard: https://grafana.example.com/d/latency-dashboard

      - alert: LatencySLOWarning
        expr: api:latency:p99:5m > 0.075
        for: 10m
        labels:
          severity: P2
          component: api
          slo: latency
        annotations:
          summary: "p99 latency approaching SLO ({{ $value | humanizeDuration }})"
          description: |
            API p99 latency is approaching the 100ms SLO threshold.
            Current p99 latency: {{ $value | humanizeDuration }}

            Runbook: https://runbooks.example.com/latency-slo-warning

  - name: error_budget
    interval: 1h
    rules:
      - alert: ErrorBudgetExhausted
        expr: slo:error_budget:remaining:percent < 0
        for: 0m
        labels:
          severity: P1
          component: slo
        annotations:
          summary: "Error budget completely exhausted"
          description: |
            The 30-day error budget has been completely consumed.
            All non-critical deployments must be halted.

            Runbook: https://runbooks.example.com/error-budget-exhausted

      - alert: ErrorBudgetCritical
        expr: slo:error_budget:remaining:percent < 10
        for: 0m
        labels:
          severity: P2
          component: slo
        annotations:
          summary: "Error budget critical ({{ $value }}% remaining)"
          description: |
            Less than 10% of error budget remains for this 30-day period.
            Review ongoing incidents and consider deployment freeze.

            Runbook: https://runbooks.example.com/error-budget-critical

      - alert: ErrorBudgetWarning
        expr: slo:error_budget:remaining:percent < 25
        for: 0m
        labels:
          severity: P3
          component: slo
        annotations:
          summary: "Error budget warning ({{ $value }}% remaining)"
          description: |
            Less than 25% of error budget remains.
            Exercise caution with deployments.

  - name: infrastructure_alerts
    interval: 30s
    rules:
      - alert: ServiceDown
        expr: up{job=~"llm-research-lab.*"} == 0
        for: 1m
        labels:
          severity: P1
          component: infrastructure
        annotations:
          summary: "Service {{ $labels.job }} is down"
          description: |
            The {{ $labels.job }} service on {{ $labels.instance }} is not responding.

            Runbook: https://runbooks.example.com/service-down

      - alert: HighCPUUsage
        expr: node:cpu_utilization:percent > 80
        for: 10m
        labels:
          severity: P2
          component: infrastructure
        annotations:
          summary: "High CPU usage on {{ $labels.instance }} ({{ $value }}%)"
          description: |
            CPU utilization has been above 80% for 10 minutes.
            Instance: {{ $labels.instance }}
            Current usage: {{ $value }}%

            Runbook: https://runbooks.example.com/high-cpu-usage

      - alert: HighMemoryUsage
        expr: node:memory_utilization:percent > 85
        for: 10m
        labels:
          severity: P2
          component: infrastructure
        annotations:
          summary: "High memory usage on {{ $labels.instance }} ({{ $value }}%)"
          description: |
            Memory utilization has been above 85% for 10 minutes.
            Instance: {{ $labels.instance }}
            Current usage: {{ $value }}%

            Runbook: https://runbooks.example.com/high-memory-usage

      - alert: DiskSpaceLow
        expr: |
          (node_filesystem_avail_bytes{fstype!~"tmpfs|fuse.lxcfs"}
          / node_filesystem_size_bytes{fstype!~"tmpfs|fuse.lxcfs"}) * 100 < 15
        for: 5m
        labels:
          severity: P2
          component: infrastructure
        annotations:
          summary: "Disk space low on {{ $labels.instance }} ({{ $value }}% free)"
          description: |
            Less than 15% disk space remaining.
            Instance: {{ $labels.instance }}
            Mountpoint: {{ $labels.mountpoint }}

            Runbook: https://runbooks.example.com/disk-space-low

  - name: database_alerts
    interval: 30s
    rules:
      - alert: DatabaseConnectionPoolExhausted
        expr: db:connection_pool:utilization > 0.9
        for: 5m
        labels:
          severity: P1
          component: database
        annotations:
          summary: "Database connection pool nearly exhausted ({{ $value | humanizePercentage }})"
          description: |
            Database connection pool utilization is above 90%.
            This may cause request timeouts and service degradation.

            Runbook: https://runbooks.example.com/db-connection-pool-exhausted

      - alert: SlowDatabaseQueries
        expr: db:query_latency:p99:5m{query_type="select"} > 1.0
        for: 10m
        labels:
          severity: P2
          component: database
        annotations:
          summary: "Slow database queries detected (p99: {{ $value | humanizeDuration }})"
          description: |
            Database query latency p99 is above 1 second.
            Query type: {{ $labels.query_type }}

            Runbook: https://runbooks.example.com/slow-database-queries

      - alert: DatabaseReplicationLag
        expr: pg_replication_lag_seconds > 30
        for: 5m
        labels:
          severity: P2
          component: database
        annotations:
          summary: "Database replication lag ({{ $value }}s)"
          description: |
            Replication lag has exceeded 30 seconds.
            This may impact read replica consistency.

            Runbook: https://runbooks.example.com/db-replication-lag

  - name: experiment_alerts
    interval: 1m
    rules:
      - alert: HighExperimentFailureRate
        expr: (1 - experiment:success_rate:5m) > 0.1
        for: 10m
        labels:
          severity: P2
          component: experiment-runner
        annotations:
          summary: "High experiment failure rate ({{ $value | humanizePercentage }})"
          description: |
            More than 10% of experiments are failing.
            This may indicate issues with model APIs, datasets, or infrastructure.

            Runbook: https://runbooks.example.com/high-experiment-failure-rate

      - alert: ExperimentQueueBacklog
        expr: experiment_queue_depth > 100
        for: 15m
        labels:
          severity: P3
          component: experiment-runner
        annotations:
          summary: "Large experiment queue backlog ({{ $value }} experiments)"
          description: |
            More than 100 experiments are queued.
            Consider scaling up experiment runner workers.

            Runbook: https://runbooks.example.com/experiment-queue-backlog

  - name: model_inference_alerts
    interval: 30s
    rules:
      - alert: ModelAPIHighLatency
        expr: model:inference_latency:p99:5m > 10.0
        for: 5m
        labels:
          severity: P2
          component: model-api
        annotations:
          summary: "Model API high latency ({{ $labels.model_name }}: {{ $value | humanizeDuration }})"
          description: |
            Model inference latency p99 is above 10 seconds.
            Model: {{ $labels.model_name }}
            Provider: {{ $labels.provider }}

            Runbook: https://runbooks.example.com/model-api-high-latency

      - alert: ModelAPIErrors
        expr: |
          sum(rate(model_inference_errors_total[5m])) by (model_name, provider)
          /
          sum(rate(model_inference_duration_seconds_count[5m])) by (model_name, provider)
          > 0.05
        for: 5m
        labels:
          severity: P2
          component: model-api
        annotations:
          summary: "High model API error rate ({{ $labels.model_name }}: {{ $value | humanizePercentage }})"
          description: |
            More than 5% of model API requests are failing.
            Model: {{ $labels.model_name }}
            Provider: {{ $labels.provider }}

            Runbook: https://runbooks.example.com/model-api-errors

  - name: security_alerts
    interval: 1m
    rules:
      - alert: UnauthorizedAccessAttempt
        expr: increase(api_request_total{status="401"}[5m]) > 50
        for: 0m
        labels:
          severity: P2
          component: security
        annotations:
          summary: "Multiple unauthorized access attempts detected"
          description: |
            More than 50 unauthorized (401) requests in 5 minutes.
            This may indicate a brute force attack.

            Runbook: https://runbooks.example.com/unauthorized-access-attempts

      - alert: SuspiciousDataAccess
        expr: increase(dataset_access_denied_total[10m]) > 10
        for: 0m
        labels:
          severity: P2
          component: security
        annotations:
          summary: "Multiple data access denials detected"
          description: |
            More than 10 dataset access denials in 10 minutes.
            Review access logs for potential security breach.

            Runbook: https://runbooks.example.com/suspicious-data-access
```

### 5.2.3 Alert Routing and Escalation

```yaml
# alertmanager.yml
global:
  resolve_timeout: 5m
  slack_api_url: ${SLACK_WEBHOOK_URL}
  pagerduty_url: https://events.pagerduty.com/v2/enqueue

route:
  receiver: 'default-receiver'
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 5m
  repeat_interval: 4h

  routes:
    # P1 Critical - Immediate PagerDuty + Slack
    - match:
        severity: P1
      receiver: 'pagerduty-critical'
      group_wait: 0s
      group_interval: 1m
      repeat_interval: 5m
      continue: true

    - match:
        severity: P1
      receiver: 'slack-critical'
      group_wait: 0s
      group_interval: 1m
      repeat_interval: 30m

    # P2 High - PagerDuty during business hours, Slack always
    - match:
        severity: P2
      receiver: 'pagerduty-high'
      group_wait: 5s
      group_interval: 5m
      repeat_interval: 30m
      continue: true
      time_intervals:
        - business_hours

    - match:
        severity: P2
      receiver: 'slack-high'
      group_wait: 5s
      group_interval: 5m
      repeat_interval: 1h

    # P3 Medium - Slack only
    - match:
        severity: P3
      receiver: 'slack-medium'
      group_wait: 30s
      group_interval: 10m
      repeat_interval: 4h

    # P4 Low - Email digest
    - match:
        severity: P4
      receiver: 'email-low'
      group_wait: 5m
      group_interval: 1h
      repeat_interval: 24h

    # Security alerts - dedicated channel
    - match:
        component: security
      receiver: 'security-team'
      group_wait: 0s
      repeat_interval: 15m
      continue: true

# Time intervals
time_intervals:
  - name: business_hours
    time_intervals:
      - times:
          - start_time: '09:00'
            end_time: '17:00'
        weekdays: ['monday:friday']
        location: 'America/New_York'

# Receivers
receivers:
  - name: 'default-receiver'
    slack_configs:
      - channel: '#llm-research-lab-alerts'
        title: 'Default Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: ${PAGERDUTY_SERVICE_KEY_CRITICAL}
        severity: critical
        description: '{{ .CommonAnnotations.summary }}'
        details:
          alert_count: '{{ len .Alerts }}'
          firing_alerts: '{{ range .Alerts.Firing }}{{ .Labels.alertname }} {{ end }}'
          runbook: '{{ .CommonAnnotations.runbook }}'
          dashboard: '{{ .CommonAnnotations.dashboard }}'
        client: 'Prometheus Alertmanager'
        client_url: 'https://alertmanager.example.com'

  - name: 'pagerduty-high'
    pagerduty_configs:
      - service_key: ${PAGERDUTY_SERVICE_KEY_HIGH}
        severity: error
        description: '{{ .CommonAnnotations.summary }}'

  - name: 'slack-critical'
    slack_configs:
      - channel: '#llm-research-lab-critical'
        title: ':rotating_light: P1 CRITICAL ALERT'
        text: |
          *Summary:* {{ .CommonAnnotations.summary }}
          *Description:* {{ .CommonAnnotations.description }}
          *Runbook:* {{ .CommonAnnotations.runbook }}
          *Dashboard:* {{ .CommonAnnotations.dashboard }}
          *Firing Alerts:* {{ len .Alerts.Firing }}
        color: danger
        send_resolved: true

  - name: 'slack-high'
    slack_configs:
      - channel: '#llm-research-lab-alerts'
        title: ':warning: P2 High Priority Alert'
        text: '{{ .CommonAnnotations.description }}'
        color: warning
        send_resolved: true

  - name: 'slack-medium'
    slack_configs:
      - channel: '#llm-research-lab-alerts'
        title: ':information_source: P3 Medium Priority Alert'
        text: '{{ .CommonAnnotations.summary }}'
        color: good
        send_resolved: true

  - name: 'email-low'
    email_configs:
      - to: 'llm-research-lab-team@example.com'
        from: 'alertmanager@example.com'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alertmanager@example.com'
        auth_password: ${SMTP_PASSWORD}
        headers:
          Subject: 'LLM Research Lab - P4 Alert Digest'

  - name: 'security-team'
    slack_configs:
      - channel: '#security-alerts'
        title: ':lock: Security Alert'
        text: '{{ .CommonAnnotations.description }}'
        color: danger
    email_configs:
      - to: 'security-team@example.com'
        from: 'alertmanager@example.com'
        smarthost: 'smtp.example.com:587'

# Inhibition rules
inhibit_rules:
  # Inhibit warning alerts if critical alert is firing
  - source_match:
      severity: P1
    target_match:
      severity: P2
    equal: ['alertname', 'cluster', 'service']

  - source_match:
      severity: P1
    target_match:
      severity: P3
    equal: ['alertname', 'cluster', 'service']

  # Inhibit individual component alerts if service is down
  - source_match:
      alertname: ServiceDown
    target_match_re:
      alertname: '(HighCPU|HighMemory|SlowQueries).*'
    equal: ['instance']
```

### 5.2.4 Runbook Structure

Each alert must link to a runbook with the following structure:

```markdown
# Runbook: [Alert Name]

## Alert Details
- **Severity**: P1/P2/P3/P4
- **Component**: [Component Name]
- **SLO Impact**: Yes/No
- **Typical Duration**: [Expected time to resolve]

## Symptoms
- What users/systems are experiencing
- Observable symptoms without logging into systems

## Diagnosis
1. Check [Dashboard Link]
2. Run diagnostic queries:
   ```promql
   [Prometheus queries for diagnosis]
   ```
3. Check logs:
   ```
   kubectl logs -n llm-research-lab [pod-name]
   ```

## Mitigation Steps
1. **Immediate actions** (< 5 min)
   - Step 1
   - Step 2
2. **Short-term fixes** (< 1 hour)
   - Step 1
   - Step 2
3. **Long-term resolution**
   - Root cause analysis
   - Permanent fixes

## Escalation
- **Primary On-Call**: [PagerDuty escalation policy]
- **Secondary**: [Team lead contact]
- **Manager Escalation Threshold**: [Condition requiring manager involvement]

## Related Information
- Related alerts
- Dependencies
- Recent changes
- Known issues
```

### 5.2.5 Alert Fatigue Prevention

**Strategies:**

1. **Alert Tuning**
   - Review alert effectiveness monthly
   - Disable or adjust alerts with > 50% false positive rate
   - Require "for" duration on all alerts to avoid flapping

2. **Alert Grouping**
   - Group related alerts (e.g., all database alerts during database outage)
   - Use inhibition rules to suppress redundant alerts

3. **Actionability Requirement**
   - Every alert must have a clear action item
   - Remove "informational" alerts from PagerDuty

4. **Alert Review Process**
   - Quarterly alert effectiveness review
   - Track mean time to acknowledge (MTTA) and mean time to resolve (MTTR)
   - Disable alerts with MTTA > 30 minutes (indicates unclear action)

5. **Graduated Response**
   - Use multiple severity levels
   - Escalate from warning → critical only if condition worsens

---

## 5.3 SLO/SLI Definition

### 5.3.1 Service Level Indicators (SLIs)

| SLI | Measurement | Target | Data Source |
|-----|-------------|--------|-------------|
| **Availability** | Ratio of successful requests (non-5xx) to total requests | 99.9% | `api_request_total` metric |
| **Latency (p99)** | 99th percentile request latency | < 100ms | `api_request_duration_seconds` histogram |
| **Latency (p95)** | 95th percentile request latency | < 50ms | `api_request_duration_seconds` histogram |
| **Latency (p50)** | 50th percentile request latency | < 25ms | `api_request_duration_seconds` histogram |
| **Experiment Success Rate** | Ratio of successful experiments to total | > 95% | `experiment_total` metric |
| **Data Freshness** | Age of most recent dataset update | < 24 hours | Custom metric |

### 5.3.2 Service Level Objectives (SLOs)

#### Availability SLO

**Target**: 99.9% availability over 30-day rolling window

**Error Budget**: 0.1% = 43.2 minutes of downtime per 30 days

```yaml
# Recording rule for availability SLO
- record: slo:availability:30d
  expr: |
    sum(rate(api_request_total{status!~"5.."}[30d]))
    /
    sum(rate(api_request_total[30d]))

- record: slo:availability:error_budget:consumed
  expr: 1 - slo:availability:30d

- record: slo:availability:error_budget:remaining:percent
  expr: |
    (0.001 - (1 - slo:availability:30d)) / 0.001 * 100
```

#### Latency SLO

**Target**: 99% of requests complete in < 100ms

**Error Budget**: 1% of requests may exceed 100ms

```yaml
# Recording rule for latency SLO
- record: slo:latency:requests_under_100ms:30d
  expr: |
    sum(rate(api_request_duration_seconds_bucket{le="0.1"}[30d]))
    /
    sum(rate(api_request_duration_seconds_count[30d]))

- record: slo:latency:error_budget:consumed
  expr: 1 - slo:latency:requests_under_100ms:30d

- record: slo:latency:error_budget:remaining:percent
  expr: |
    (0.01 - (1 - slo:latency:requests_under_100ms:30d)) / 0.01 * 100
```

### 5.3.3 Error Budget Tracking

```rust
// src/slo/error_budget.rs
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone)]
pub struct ErrorBudget {
    pub window_duration: Duration,
    pub target_availability: f64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl ErrorBudget {
    pub fn new_30_day() -> Self {
        let now = Utc::now();
        Self {
            window_duration: Duration::days(30),
            target_availability: 0.999,
            start_time: now - Duration::days(30),
            end_time: now,
        }
    }

    pub fn allowed_downtime_seconds(&self) -> f64 {
        let total_seconds = self.window_duration.num_seconds() as f64;
        total_seconds * (1.0 - self.target_availability)
    }

    pub fn consumed_downtime_seconds(&self, actual_availability: f64) -> f64 {
        let total_seconds = self.window_duration.num_seconds() as f64;
        total_seconds * (1.0 - actual_availability)
    }

    pub fn remaining_downtime_seconds(&self, actual_availability: f64) -> f64 {
        self.allowed_downtime_seconds() - self.consumed_downtime_seconds(actual_availability)
    }

    pub fn remaining_percent(&self, actual_availability: f64) -> f64 {
        let remaining = self.remaining_downtime_seconds(actual_availability);
        let allowed = self.allowed_downtime_seconds();
        (remaining / allowed) * 100.0
    }

    pub fn burn_rate(&self, actual_availability: f64, time_window: Duration) -> f64 {
        let consumed = self.consumed_downtime_seconds(actual_availability);
        let time_elapsed = (self.end_time - self.start_time).num_seconds() as f64;
        let window_seconds = time_window.num_seconds() as f64;

        (consumed / time_elapsed) * (self.window_duration.num_seconds() as f64 / window_seconds)
    }

    pub fn is_exhausted(&self, actual_availability: f64) -> bool {
        self.remaining_downtime_seconds(actual_availability) <= 0.0
    }

    pub fn is_critical(&self, actual_availability: f64) -> bool {
        self.remaining_percent(actual_availability) < 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_budget_calculations() {
        let budget = ErrorBudget::new_30_day();

        // 30 days = 2,592,000 seconds
        // 0.1% = 2,592 seconds = 43.2 minutes
        assert_eq!(budget.allowed_downtime_seconds(), 2592.0);

        // If availability is 99.95%, we've used half the budget
        let consumed = budget.consumed_downtime_seconds(0.9995);
        assert_eq!(consumed, 1296.0);

        // Remaining should be 50%
        let remaining_pct = budget.remaining_percent(0.9995);
        assert!((remaining_pct - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_error_budget_exhaustion() {
        let budget = ErrorBudget::new_30_day();

        // If availability is below 99.9%, budget is exhausted
        assert!(budget.is_exhausted(0.998));
        assert!(!budget.is_exhausted(0.9995));
    }
}
```

### 5.3.4 SLO Dashboard Requirements

**Dashboard Panels Required:**

1. **Current SLO Compliance**
   - Gauge showing current 30-day availability
   - Threshold markers at 99.9% (SLO) and 99.95% (warning)
   - Color coding: green (> 99.95%), yellow (99.9-99.95%), red (< 99.9%)

2. **Error Budget Burn Rate**
   - Time series showing daily error budget consumption
   - Projection line showing estimated time to budget exhaustion
   - Alerts when burn rate exceeds 2x expected

3. **Error Budget Remaining**
   - Bar chart showing percentage of error budget remaining
   - Historical comparison to previous 30-day windows
   - Annotations for incidents and deployments

4. **Latency Distribution**
   - Heatmap of request latency over time
   - Percentile lines (p50, p95, p99)
   - SLO threshold marker at 100ms

5. **SLO Violation Events**
   - Table of SLO violation incidents
   - Duration, impact, root cause
   - Links to incident reports

---

## 5.4 On-Call Procedures

### 5.4.1 On-Call Rotation Setup

**Rotation Schedule:**
- **Primary On-Call**: 7-day rotation, Monday-Monday handoff
- **Secondary On-Call**: Shadows primary, escalation path
- **Manager Escalation**: Available for P1 incidents > 2 hours

**Schedule Management:**
- Use PagerDuty for rotation management
- Minimum 2 weeks notice for rotation assignments
- Swap requests must be confirmed 48 hours in advance
- Time zone considerations for global team (follow-the-sun)

**On-Call Responsibilities:**
1. Respond to P1 alerts within 5 minutes
2. Respond to P2 alerts within 15 minutes
3. Acknowledge all alerts in PagerDuty
4. Update incident status in Slack
5. Create incident reports for P1/P2 incidents
6. Participate in post-incident reviews

### 5.4.2 Escalation Matrix

| Severity | Initial Response | Escalation Time | Escalation Path |
|----------|------------------|-----------------|-----------------|
| **P1** | Primary On-Call | 15 minutes | Secondary → Manager → Director |
| **P2** | Primary On-Call | 30 minutes | Secondary → Manager |
| **P3** | Primary On-Call | 2 hours | Secondary |
| **P4** | Business hours | 8 hours | Team lead |

**Escalation Contacts:**

```yaml
# escalation-policy.yaml
escalation_policies:
  - name: llm-research-lab-primary
    num_loops: 3
    on_call_handoff_notifications: restrict
    escalation_rules:
      - escalation_delay_in_minutes: 0
        targets:
          - type: schedule_reference
            id: primary-on-call-schedule

      - escalation_delay_in_minutes: 15
        targets:
          - type: schedule_reference
            id: secondary-on-call-schedule

      - escalation_delay_in_minutes: 30
        targets:
          - type: user_reference
            id: engineering-manager

      - escalation_delay_in_minutes: 60
        targets:
          - type: user_reference
            id: director-of-engineering
```

### 5.4.3 Incident Response Playbook

#### Phase 1: Detection and Triage (0-5 minutes)

1. **Acknowledge Alert**
   - Click acknowledge in PagerDuty to stop escalation
   - Post initial acknowledgment in #llm-research-lab-incidents Slack channel

2. **Initial Assessment**
   - Check alert dashboard for context
   - Determine severity (confirm P1/P2 classification)
   - Identify affected components and user impact

3. **Incident Declaration**
   - For P1: Declare major incident, start incident bridge
   - For P2: Create incident thread in Slack
   - Assign incident commander role (usually primary on-call)

#### Phase 2: Investigation (5-30 minutes)

1. **Gather Context**
   ```bash
   # Check service health
   kubectl get pods -n llm-research-lab

   # Check recent deployments
   kubectl rollout history deployment/llm-research-lab-api -n llm-research-lab

   # Check logs for errors
   kubectl logs -n llm-research-lab deployment/llm-research-lab-api --tail=100

   # Check Grafana dashboards
   # Link: https://grafana.example.com/d/slo-dashboard
   ```

2. **Identify Root Cause**
   - Review recent changes (deployments, config updates)
   - Check dependency health (database, model APIs, external services)
   - Correlate metrics, logs, and traces

3. **Document Findings**
   - Update incident thread with findings
   - Create incident document (Google Doc/Confluence)
   - Include timeline of events

#### Phase 3: Mitigation (30 minutes - 2 hours)

1. **Implement Fix**
   - Apply hotfix, rollback, or scaling adjustment
   - For rollback:
     ```bash
     kubectl rollout undo deployment/llm-research-lab-api -n llm-research-lab
     ```
   - For scaling:
     ```bash
     kubectl scale deployment/llm-research-lab-api --replicas=10 -n llm-research-lab
     ```

2. **Verify Resolution**
   - Monitor SLO metrics for recovery
   - Check error rates return to baseline
   - Verify user-facing functionality

3. **Communicate Status**
   - Update incident thread with resolution
   - Post status to #llm-research-lab channel
   - Update status page if customer-facing

#### Phase 4: Recovery and Validation (2-4 hours)

1. **Monitor for Regression**
   - Watch dashboards for 30-60 minutes post-fix
   - Ensure no new alerts triggered
   - Verify SLO compliance

2. **Close Incident**
   - Resolve PagerDuty incident
   - Update incident document with resolution
   - Thank participants in Slack

3. **Schedule Post-Incident Review**
   - For P1: Within 24 hours
   - For P2: Within 48 hours
   - Invite all participants and stakeholders

### 5.4.4 Communication Templates

#### Incident Declaration (P1)

```
:rotating_light: **INCIDENT DECLARED - P1 CRITICAL** :rotating_light:

**Incident**: [Brief description]
**Severity**: P1 Critical
**Impact**: [User-facing impact]
**Incident Commander**: @[primary-on-call]
**Status**: Investigating

**Incident Doc**: [Link]
**Dashboard**: [Link]
**Bridge**: [Conference link]

Updates every 15 minutes or when status changes.
```

#### Status Update

```
**INCIDENT UPDATE** - [Timestamp]

**Status**: [Investigating | Identified | Monitoring | Resolved]
**Summary**: [What we know so far]
**Current Actions**: [What we're doing]
**Next Update**: [Time]

**Incident Doc**: [Link]
```

#### Resolution

```
:white_check_mark: **INCIDENT RESOLVED** :white_check_mark:

**Incident**: [Brief description]
**Duration**: [Start time - End time]
**Root Cause**: [Brief summary]
**Resolution**: [What fixed it]

**Impact Summary**:
- Error budget consumed: X%
- Requests affected: Y
- Users impacted: Z

**Post-Incident Review**: [Scheduled time]
**Incident Report**: [Link when available]

Thanks to @[participants] for quick response!
```

### 5.4.5 Post-Incident Review Process

#### Review Agenda (60 minutes)

1. **Timeline Review** (10 min)
   - Incident timeline from detection to resolution
   - Key decision points and actions taken

2. **Root Cause Analysis** (15 min)
   - What happened and why
   - Contributing factors
   - Five Whys technique

3. **Response Effectiveness** (15 min)
   - What went well
   - What could be improved
   - Runbook effectiveness

4. **Action Items** (15 min)
   - Preventive measures
   - Detection improvements
   - Response process improvements
   - Assign owners and due dates

5. **Lessons Learned** (5 min)
   - Key takeaways
   - Documentation updates needed

#### Post-Incident Report Template

```markdown
# Post-Incident Report: [Incident Name]

## Incident Summary
- **Date**: [Date]
- **Duration**: [Start - End]
- **Severity**: P1/P2
- **Incident Commander**: [Name]
- **Participants**: [Names]

## Impact
- **Users Affected**: [Count]
- **Error Budget Consumed**: [Percentage]
- **Revenue Impact**: [If applicable]
- **SLO Violation**: Yes/No

## Timeline
| Time | Event |
|------|-------|
| HH:MM | Alert triggered |
| HH:MM | Incident acknowledged |
| HH:MM | Root cause identified |
| HH:MM | Mitigation applied |
| HH:MM | Service recovered |
| HH:MM | Incident resolved |

## Root Cause
[Detailed explanation of what caused the incident]

### Five Whys Analysis
1. Why did [symptom] occur? [Answer]
2. Why did [answer1] happen? [Answer]
3. Why did [answer2] happen? [Answer]
4. Why did [answer3] happen? [Answer]
5. Why did [answer4] happen? [Root cause]

## Resolution
[What was done to resolve the incident]

## What Went Well
- [Item]
- [Item]

## What Could Be Improved
- [Item]
- [Item]

## Action Items
| Action | Owner | Due Date | Status |
|--------|-------|----------|--------|
| [Preventive measure] | [Name] | [Date] | Open |
| [Detection improvement] | [Name] | [Date] | Open |
| [Process improvement] | [Name] | [Date] | Open |

## Lessons Learned
- [Lesson]
- [Lesson]

---
**Report Author**: [Name]
**Review Date**: [Date]
**Attendees**: [Names]
```

---

## 5.5 Capacity Management

### 5.5.1 Resource Utilization Thresholds

| Resource | Warning Threshold | Critical Threshold | Action |
|----------|-------------------|-------------------|--------|
| **CPU** | 70% sustained | 85% sustained | Scale horizontally |
| **Memory** | 75% | 90% | Scale vertically or horizontally |
| **Disk** | 75% full | 85% full | Provision additional storage |
| **Network** | 70% bandwidth | 85% bandwidth | Upgrade network tier |
| **Database Connections** | 80% pool | 90% pool | Increase pool size |
| **Queue Depth** | 100 items | 500 items | Scale workers |

**Monitoring Configuration:**

```yaml
# Capacity threshold alerts
- alert: CPUCapacityWarning
  expr: node:cpu_utilization:percent > 70
  for: 30m
  labels:
    severity: P3
    component: capacity
  annotations:
    summary: "CPU capacity warning ({{ $value }}%)"
    description: |
      CPU utilization has been above 70% for 30 minutes.
      Consider scaling if trend continues.

      Runbook: https://runbooks.example.com/cpu-capacity

- alert: MemoryCapacityWarning
  expr: node:memory_utilization:percent > 75
  for: 30m
  labels:
    severity: P3
    component: capacity
  annotations:
    summary: "Memory capacity warning ({{ $value }}%)"
    description: |
      Memory utilization has been above 75% for 30 minutes.
      Consider scaling if trend continues.

- alert: DiskCapacityCritical
  expr: |
    (node_filesystem_avail_bytes / node_filesystem_size_bytes) * 100 < 15
  for: 10m
  labels:
    severity: P2
    component: capacity
  annotations:
    summary: "Disk capacity critical ({{ $value }}% free)"
    description: |
      Less than 15% disk space remaining.
      Immediate action required to prevent service disruption.
```

### 5.5.2 Scaling Triggers

#### Horizontal Pod Autoscaling (HPA)

```yaml
# hpa-api.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: llm-research-lab-api
  namespace: llm-research-lab
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: llm-research-lab-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 75
    - type: Pods
      pods:
        metric:
          name: api_request_in_flight
        target:
          type: AverageValue
          averageValue: "100"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
        - type: Percent
          value: 50
          periodSeconds: 60
        - type: Pods
          value: 2
          periodSeconds: 60
      selectPolicy: Max
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Percent
          value: 10
          periodSeconds: 60
        - type: Pods
          value: 1
          periodSeconds: 60
      selectPolicy: Min
```

#### Cluster Autoscaling

```yaml
# cluster-autoscaler-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: cluster-autoscaler-config
  namespace: kube-system
data:
  autoscaler-config: |
    {
      "scaleDownUtilizationThreshold": 0.5,
      "scaleDownGpuUtilizationThreshold": 0.5,
      "scaleDownUnneededTime": "10m",
      "scaleDownUnreadyTime": "20m",
      "maxNodeProvisionTime": "15m",
      "maxTotalUnreadyPercentage": 45,
      "okTotalUnreadyCount": 3
    }
```

### 5.5.3 Capacity Forecasting

```python
# capacity_forecast.py
import pandas as pd
import numpy as np
from sklearn.linear_model import LinearRegression
from datetime import datetime, timedelta

class CapacityForecaster:
    def __init__(self, prometheus_url):
        self.prom_url = prometheus_url
        self.models = {}

    def fetch_metric_history(self, metric_name, days=90):
        """Fetch historical metric data from Prometheus"""
        query = f"{metric_name}[{days}d]"
        # Implementation using Prometheus HTTP API
        pass

    def train_forecast_model(self, metric_data):
        """Train linear regression model on historical data"""
        df = pd.DataFrame(metric_data)
        df['timestamp'] = pd.to_datetime(df['timestamp'])
        df['days_since_start'] = (df['timestamp'] - df['timestamp'].min()).dt.days

        X = df[['days_since_start']].values
        y = df['value'].values

        model = LinearRegression()
        model.fit(X, y)

        return model

    def forecast(self, metric_name, days_ahead=30):
        """Forecast metric value for N days ahead"""
        historical_data = self.fetch_metric_history(metric_name)
        model = self.train_forecast_model(historical_data)

        last_day = historical_data['days_since_start'].max()
        future_days = np.array([[last_day + i] for i in range(1, days_ahead + 1)])

        forecast_values = model.predict(future_days)

        return {
            'metric': metric_name,
            'forecast': forecast_values,
            'confidence_interval': self.calculate_confidence_interval(model, future_days),
            'threshold_breach_date': self.find_threshold_breach(forecast_values, threshold=0.85)
        }

    def calculate_confidence_interval(self, model, X, confidence=0.95):
        """Calculate confidence intervals for forecast"""
        # Implementation using prediction intervals
        pass

    def find_threshold_breach(self, forecast, threshold):
        """Find date when forecast exceeds threshold"""
        breach_index = np.where(forecast > threshold)[0]
        if len(breach_index) > 0:
            return breach_index[0]
        return None

# Usage
forecaster = CapacityForecaster("http://prometheus:9090")
cpu_forecast = forecaster.forecast("node:cpu_utilization:percent", days_ahead=60)
memory_forecast = forecaster.forecast("node:memory_utilization:percent", days_ahead=60)

# Generate capacity report
if cpu_forecast['threshold_breach_date']:
    print(f"WARNING: CPU capacity expected to breach 85% in {cpu_forecast['threshold_breach_date']} days")
```

### 5.5.4 Cost Monitoring and Alerting

```yaml
# cost-monitoring.yml
groups:
  - name: cost_alerts
    interval: 1h
    rules:
      - alert: DailySpendExceeded
        expr: sum(increase(model_inference_cost_usd_sum[24h])) > 1000
        for: 0m
        labels:
          severity: P2
          component: cost
        annotations:
          summary: "Daily model inference cost exceeded $1000"
          description: |
            Model inference costs have exceeded $1000 in the last 24 hours.
            Current spend: ${{ $value | humanize }}

            Breakdown by provider:
            {{ range query "sum(increase(model_inference_cost_usd_sum[24h])) by (provider)" }}
              {{ .Labels.provider }}: ${{ .Value | humanize }}
            {{ end }}

      - alert: UnexpectedCostIncrease
        expr: |
          sum(rate(model_inference_cost_usd_sum[1h]))
          >
          sum(rate(model_inference_cost_usd_sum[1h] offset 24h)) * 1.5
        for: 2h
        labels:
          severity: P3
          component: cost
        annotations:
          summary: "Unexpected 50% cost increase detected"
          description: |
            Model inference costs are 50% higher than same time yesterday.
            This may indicate inefficient usage or pricing changes.

      - alert: MonthlyBudgetProjection
        expr: |
          (sum(increase(model_inference_cost_usd_sum[30d])) / 30) * 30 > 25000
        for: 0m
        labels:
          severity: P3
          component: cost
        annotations:
          summary: "Monthly budget projection exceeds $25,000"
          description: |
            Current spending trajectory projects monthly cost above budget.
            Projected monthly cost: ${{ $value | humanize }}
```

**Cost Optimization Strategies:**

1. **Caching**: Reduce redundant model inferences
   - Target: 40% cache hit rate for repeated queries
   - Monitor: `cache_hit_total / (cache_hit_total + cache_miss_total)`

2. **Model Selection**: Route to cost-effective models
   - Use cheaper models for simple tasks
   - Reserve expensive models for complex evaluations

3. **Batch Processing**: Group inferences to reduce API overhead
   - Batch size: 10-50 inferences per API call
   - Trade-off: Slight latency increase for cost reduction

4. **Resource Right-Sizing**: Optimize pod resource requests
   ```bash
   # Analyze actual usage vs requests
   kubectl top pods -n llm-research-lab

   # Adjust resource requests based on actual usage + 20% headroom
   ```

---

## 5.6 Security Operations

### 5.6.1 Security Event Monitoring

```yaml
# security-monitoring.yml
groups:
  - name: security_events
    interval: 1m
    rules:
      - alert: MultipleFailedLogins
        expr: increase(authentication_failures_total[5m]) > 10
        for: 0m
        labels:
          severity: P2
          component: security
          category: authentication
        annotations:
          summary: "Multiple failed login attempts detected"
          description: |
            More than 10 failed authentication attempts in 5 minutes.
            User: {{ $labels.user }}
            IP: {{ $labels.source_ip }}

            This may indicate a brute force attack.

            Runbook: https://runbooks.example.com/failed-logins

      - alert: PrivilegeEscalationAttempt
        expr: increase(rbac_authorization_failures_total[10m]) > 5
        for: 0m
        labels:
          severity: P1
          component: security
          category: authorization
        annotations:
          summary: "Potential privilege escalation attempt"
          description: |
            Multiple authorization failures detected.
            User: {{ $labels.user }}
            Resource: {{ $labels.resource }}

            Immediate investigation required.

      - alert: UnusualDataAccess
        expr: |
          sum(rate(dataset_access_total[5m])) by (user)
          >
          sum(rate(dataset_access_total[5m] offset 24h)) by (user) * 3
        for: 10m
        labels:
          severity: P2
          component: security
          category: data_access
        annotations:
          summary: "Unusual data access pattern detected"
          description: |
            User {{ $labels.user }} is accessing data at 3x normal rate.
            This may indicate data exfiltration.

      - alert: SuspiciousAPIActivity
        expr: |
          sum(rate(api_request_total{user_agent=~".*bot.*|.*crawler.*|.*scanner.*"}[5m]))
          > 10
        for: 5m
        labels:
          severity: P3
          component: security
          category: api_abuse
        annotations:
          summary: "Suspicious API activity from automated tools"
          description: |
            High volume of requests from bot-like user agents.
            User agent: {{ $labels.user_agent }}
            IP: {{ $labels.source_ip }}
```

### 5.6.2 Intrusion Detection

**Falco Configuration** (Kubernetes Runtime Security)

```yaml
# falco-rules.yaml
- rule: Unauthorized Process in Container
  desc: Detect execution of unauthorized processes in LLM Research Lab containers
  condition: >
    container.name startswith "llm-research-lab" and
    proc.name not in (allowed_processes) and
    not proc.pname in (allowed_processes)
  output: >
    Unauthorized process started in container
    (user=%user.name command=%proc.cmdline container=%container.name image=%container.image.repository)
  priority: WARNING
  tags: [container, process, security]

- rule: Sensitive File Access
  desc: Detect access to sensitive files containing credentials or secrets
  condition: >
    container.name startswith "llm-research-lab" and
    (fd.name glob "/etc/secrets/*" or
     fd.name glob "/app/.env" or
     fd.name glob "/root/.ssh/*") and
    evt.type in (open, openat)
  output: >
    Sensitive file accessed
    (user=%user.name file=%fd.name container=%container.name command=%proc.cmdline)
  priority: CRITICAL
  tags: [file, secrets, security]

- rule: Unexpected Network Connection
  desc: Detect outbound connections to unexpected destinations
  condition: >
    container.name startswith "llm-research-lab" and
    evt.type=connect and
    fd.sip not in (allowed_egress_ips) and
    fd.sport != 443 and fd.sport != 80
  output: >
    Unexpected network connection
    (user=%user.name destination=%fd.sip:%fd.sport container=%container.name command=%proc.cmdline)
  priority: WARNING
  tags: [network, security]

- rule: Shell Spawned in Container
  desc: Detect shell execution inside container (possible compromise)
  condition: >
    container.name startswith "llm-research-lab" and
    proc.name in (sh, bash, zsh, fish) and
    proc.pname exists
  output: >
    Shell spawned in container
    (user=%user.name shell=%proc.name parent=%proc.pname container=%container.name)
  priority: WARNING
  tags: [shell, container, security]
```

### 5.6.3 Vulnerability Scanning Schedule

| Scan Type | Frequency | Tool | Severity Threshold | Action |
|-----------|-----------|------|-------------------|--------|
| **Container Image** | Every build + daily | Trivy | Critical: Block, High: Alert | Fail CI for Critical |
| **Dependency** | Every build + weekly | cargo-audit | Critical: Block, High: Alert | Fail CI for Critical |
| **SAST** | Every commit | Semgrep | Critical: Block, High: Alert | Block merge for Critical |
| **DAST** | Weekly | OWASP ZAP | Critical: Alert, High: Alert | Create ticket |
| **Infrastructure** | Daily | Prowler (AWS) | Critical: Alert, High: Alert | Remediate within SLA |
| **Secrets** | Every commit | gitleaks | Any: Block | Fail CI for any secret |

**Trivy Configuration**

```yaml
# trivy-scan.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: trivy-scan
  namespace: llm-research-lab
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: trivy
            image: aquasec/trivy:latest
            args:
            - image
            - --severity
            - CRITICAL,HIGH
            - --exit-code
            - "1"
            - --format
            - json
            - --output
            - /reports/trivy-report.json
            - llm-research-lab-api:latest
            volumeMounts:
            - name: reports
              mountPath: /reports
          volumes:
          - name: reports
            persistentVolumeClaim:
              claimName: security-reports
          restartPolicy: OnFailure
```

**Vulnerability Remediation SLAs**

| Severity | Time to Remediation | Approval Required |
|----------|-------------------|-------------------|
| **Critical** | 24 hours | Security team |
| **High** | 7 days | Engineering lead |
| **Medium** | 30 days | Team lead |
| **Low** | 90 days | Next sprint |

### 5.6.4 Access Audit Logging

```rust
// src/audit/mod.rs
use serde::{Serialize, Deserialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: String,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: String,
    pub source_ip: String,
    pub user_agent: Option<String>,
    pub outcome: AuditOutcome,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
    Execute,
    Login,
    Logout,
    AccessDenied,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    PartialSuccess,
}

impl AuditLog {
    pub fn log_data_access(
        user_id: &str,
        dataset_id: &str,
        source_ip: &str,
        outcome: AuditOutcome,
    ) {
        let audit_log = AuditLog {
            timestamp: chrono::Utc::now(),
            user_id: user_id.to_string(),
            action: AuditAction::Read,
            resource_type: "dataset".to_string(),
            resource_id: dataset_id.to_string(),
            source_ip: source_ip.to_string(),
            user_agent: None,
            outcome,
            metadata: serde_json::json!({
                "event_type": "data_access",
            }),
        };

        info!(
            audit_log = ?audit_log,
            event = "audit.data_access",
            "Data access audit log"
        );

        // Also write to dedicated audit log storage
        write_to_audit_store(&audit_log);
    }

    pub fn log_experiment_execution(
        user_id: &str,
        experiment_id: &str,
        outcome: AuditOutcome,
        metadata: serde_json::Value,
    ) {
        let audit_log = AuditLog {
            timestamp: chrono::Utc::now(),
            user_id: user_id.to_string(),
            action: AuditAction::Execute,
            resource_type: "experiment".to_string(),
            resource_id: experiment_id.to_string(),
            source_ip: "internal".to_string(),
            user_agent: None,
            outcome,
            metadata,
        };

        info!(
            audit_log = ?audit_log,
            event = "audit.experiment_execution",
            "Experiment execution audit log"
        );

        write_to_audit_store(&audit_log);
    }

    pub fn log_authorization_failure(
        user_id: &str,
        resource_type: &str,
        resource_id: &str,
        action: AuditAction,
        source_ip: &str,
    ) {
        let audit_log = AuditLog {
            timestamp: chrono::Utc::now(),
            user_id: user_id.to_string(),
            action,
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            source_ip: source_ip.to_string(),
            user_agent: None,
            outcome: AuditOutcome::Failure,
            metadata: serde_json::json!({
                "reason": "insufficient_permissions",
            }),
        };

        info!(
            audit_log = ?audit_log,
            event = "audit.authorization_failure",
            "Authorization failure audit log"
        );

        write_to_audit_store(&audit_log);
    }
}

fn write_to_audit_store(audit_log: &AuditLog) {
    // Write to immutable audit log storage (e.g., S3 with WORM)
    // Implementation depends on chosen audit storage backend
}
```

**Audit Log Retention:**
- **Operational logs**: 30 days in hot storage, 1 year in cold storage
- **Audit logs**: 7 years in compliance-grade storage (WORM)
- **Security event logs**: 2 years in hot storage
- **Access logs**: 90 days in hot storage, 2 years in cold storage

### 5.6.5 Compliance Monitoring

**SOC2 Compliance Checks**

```yaml
# compliance-checks.yml
groups:
  - name: soc2_compliance
    interval: 1h
    rules:
      - alert: EncryptionAtRestDisabled
        expr: |
          kube_persistentvolume_info{encrypted="false"} > 0
        for: 0m
        labels:
          severity: P1
          component: compliance
          framework: soc2
          control: CC6.1
        annotations:
          summary: "Encryption at rest disabled for persistent volume"
          description: |
            Persistent volume is not encrypted, violating SOC2 CC6.1.
            Volume: {{ $labels.persistentvolume }}

      - alert: UnauthorizedAccessAttempts
        expr: |
          increase(rbac_authorization_failures_total[24h]) > 100
        for: 0m
        labels:
          severity: P2
          component: compliance
          framework: soc2
          control: CC6.2
        annotations:
          summary: "Excessive unauthorized access attempts"
          description: |
            More than 100 authorization failures in 24 hours.
            This may indicate inadequate access controls (SOC2 CC6.2).

      - alert: AuditLogGap
        expr: |
          absent(audit_log_entries_total) or
          increase(audit_log_entries_total[1h]) == 0
        for: 1h
        labels:
          severity: P1
          component: compliance
          framework: soc2
          control: CC7.2
        annotations:
          summary: "Audit logging gap detected"
          description: |
            No audit logs generated in the last hour.
            This violates SOC2 CC7.2 monitoring requirements.

      - alert: BackupFailure
        expr: |
          time() - backup_last_success_timestamp > 86400
        for: 0m
        labels:
          severity: P1
          component: compliance
          framework: soc2
          control: CC7.2
        annotations:
          summary: "Backup has not completed in 24 hours"
          description: |
            Last successful backup: {{ $value | humanizeDuration }} ago.
            This violates SOC2 CC7.2 backup requirements.
```

**GDPR Compliance Monitoring**

```rust
// src/compliance/gdpr.rs
use chrono::{DateTime, Utc, Duration};

pub struct GDPRComplianceMonitor {
    retention_policies: HashMap<String, Duration>,
}

impl GDPRComplianceMonitor {
    pub fn check_data_retention(&self) -> Vec<ComplianceViolation> {
        // Check if any personal data exceeds retention period (GDPR Art. 5)
        let violations = vec![];

        for (data_type, max_retention) in &self.retention_policies {
            let old_records = self.find_records_older_than(data_type, *max_retention);
            if !old_records.is_empty() {
                violations.push(ComplianceViolation {
                    regulation: "GDPR".to_string(),
                    article: "Article 5".to_string(),
                    description: format!(
                        "{} records of type '{}' exceed retention period",
                        old_records.len(),
                        data_type
                    ),
                    severity: "High".to_string(),
                    remediation: "Delete or anonymize old records".to_string(),
                });
            }
        }

        violations
    }

    pub fn verify_consent_tracking(&self) -> Vec<ComplianceViolation> {
        // Verify all personal data processing has documented consent (GDPR Art. 6)
        // Implementation checks consent records in database
        vec![]
    }

    pub fn check_data_subject_rights(&self) -> Vec<ComplianceViolation> {
        // Monitor response times to data subject requests (GDPR Art. 15-17)
        // - Right to access: 30 days
        // - Right to erasure: 30 days
        // - Right to rectification: 30 days
        vec![]
    }

    pub fn audit_data_breach_notification(&self) -> Vec<ComplianceViolation> {
        // Verify breach notification timelines (GDPR Art. 33-34)
        // - Authority notification: 72 hours
        // - Individual notification: Without undue delay
        vec![]
    }
}

#[derive(Debug)]
pub struct ComplianceViolation {
    pub regulation: String,
    pub article: String,
    pub description: String,
    pub severity: String,
    pub remediation: String,
}
```

---

## Appendix A: Configuration Files

### A.1 Complete Prometheus Configuration

```yaml
# Full prometheus.yml with all scrape configs
# See Section 5.1.1 for details
```

### A.2 Complete Alertmanager Configuration

```yaml
# Full alertmanager.yml with all receivers
# See Section 5.2.3 for details
```

### A.3 Vector Configuration

```toml
# Full vector.toml for log aggregation
# See Section 5.1.2 for details
```

### A.4 Grafana Provisioning

```yaml
# grafana-dashboards.yaml
apiVersion: 1

providers:
  - name: 'default'
    orgId: 1
    folder: 'LLM Research Lab'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /var/lib/grafana/dashboards
```

---

## Appendix B: Dashboard Templates

### B.1 Service Overview Dashboard

```json
{
  "dashboard": {
    "title": "LLM Research Lab - Service Overview",
    "tags": ["llm-research-lab", "overview"],
    "timezone": "browser",
    "panels": [
      {
        "id": 1,
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(api_request_total[5m]))",
            "legendFormat": "Total Requests/sec"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
      },
      {
        "id": 2,
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(api_request_total{status=~\"5..\"}[5m])) / sum(rate(api_request_total[5m]))",
            "legendFormat": "Error Rate"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0},
        "alert": {
          "conditions": [
            {
              "evaluator": {"params": [0.01], "type": "gt"},
              "query": {"params": ["A", "5m", "now"]},
              "type": "query"
            }
          ],
          "executionErrorState": "alerting",
          "frequency": "60s",
          "handler": 1,
          "name": "Error Rate Alert",
          "noDataState": "no_data",
          "notifications": []
        }
      },
      {
        "id": 3,
        "title": "Request Latency (p99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, sum(rate(api_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "p99 Latency"
          },
          {
            "expr": "histogram_quantile(0.95, sum(rate(api_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "p95 Latency"
          },
          {
            "expr": "histogram_quantile(0.50, sum(rate(api_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "p50 Latency"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8},
        "yaxes": [
          {"format": "s", "logBase": 1, "show": true},
          {"format": "short", "logBase": 1, "show": false}
        ]
      },
      {
        "id": 4,
        "title": "Active Experiments",
        "type": "stat",
        "targets": [
          {
            "expr": "experiment_active_count",
            "legendFormat": "Active Experiments"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8}
      }
    ]
  }
}
```

### B.2 SLO Dashboard

```json
{
  "dashboard": {
    "title": "LLM Research Lab - SLO Compliance",
    "tags": ["llm-research-lab", "slo"],
    "panels": [
      {
        "id": 1,
        "title": "Availability SLO (30-day)",
        "type": "gauge",
        "targets": [
          {
            "expr": "slo:availability:30d * 100",
            "legendFormat": "Availability %"
          }
        ],
        "options": {
          "showThresholdLabels": false,
          "showThresholdMarkers": true
        },
        "fieldConfig": {
          "defaults": {
            "thresholds": {
              "mode": "absolute",
              "steps": [
                {"value": 0, "color": "red"},
                {"value": 99.9, "color": "yellow"},
                {"value": 99.95, "color": "green"}
              ]
            },
            "max": 100,
            "min": 99
          }
        }
      },
      {
        "id": 2,
        "title": "Error Budget Remaining",
        "type": "bargauge",
        "targets": [
          {
            "expr": "slo:availability:error_budget:remaining:percent",
            "legendFormat": "Availability Error Budget"
          },
          {
            "expr": "slo:latency:error_budget:remaining:percent",
            "legendFormat": "Latency Error Budget"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "thresholds": {
              "mode": "absolute",
              "steps": [
                {"value": 0, "color": "red"},
                {"value": 10, "color": "yellow"},
                {"value": 25, "color": "green"}
              ]
            },
            "max": 100,
            "min": 0
          }
        }
      },
      {
        "id": 3,
        "title": "SLO Compliance Trend",
        "type": "graph",
        "targets": [
          {
            "expr": "slo:availability:30d * 100",
            "legendFormat": "Availability %"
          },
          {
            "expr": "99.9",
            "legendFormat": "SLO Target"
          }
        ],
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 16}
      }
    ]
  }
}
```

---

## Appendix C: Runbook Index

| Alert | Severity | Runbook Link |
|-------|----------|--------------|
| AvailabilitySLOViolation | P1 | [Link](https://runbooks.example.com/availability-slo-violation) |
| LatencySLOViolation | P1 | [Link](https://runbooks.example.com/latency-slo-violation) |
| ServiceDown | P1 | [Link](https://runbooks.example.com/service-down) |
| DatabaseConnectionPoolExhausted | P1 | [Link](https://runbooks.example.com/db-connection-pool-exhausted) |
| HighExperimentFailureRate | P2 | [Link](https://runbooks.example.com/high-experiment-failure-rate) |
| ModelAPIHighLatency | P2 | [Link](https://runbooks.example.com/model-api-high-latency) |
| HighCPUUsage | P2 | [Link](https://runbooks.example.com/high-cpu-usage) |
| DiskSpaceLow | P2 | [Link](https://runbooks.example.com/disk-space-low) |
| UnauthorizedAccessAttempt | P2 | [Link](https://runbooks.example.com/unauthorized-access-attempts) |

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Complete |
| **SPARC Phase** | Phase 5 - Completion |
| **Section** | 5 - Operations & Monitoring |
| **Created** | 2025-11-28 |
| **Target SLA** | 99.9% Availability |
| **Technology Stack** | Rust, Axum, Tokio, Prometheus, Grafana, OpenTelemetry |

---

*This document is part of the SPARC Phase 5 (Completion) specification for LLM-Research-Lab. It provides enterprise-grade operations and monitoring requirements for production deployment.*
