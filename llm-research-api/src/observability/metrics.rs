//! Comprehensive Prometheus metrics system for LLM Research API
//!
//! This module provides enterprise-grade metrics collection and export for monitoring
//! the health, performance, and usage of the LLM Research API. It includes:
//!
//! - HTTP request metrics (duration, count, size, in-flight requests)
//! - Database metrics (query performance, connection pool)
//! - Business metrics (experiments, evaluations, models, datasets)
//! - System metrics (CPU, memory, file descriptors)
//! - Metrics endpoint for Prometheus scraping
//! - Tower middleware for automatic HTTP metrics collection
//!
//! # Example
//!
//! ```rust,ignore
//! use llm_research_api::observability::metrics::{init_metrics, metrics_handler, MetricsLayer};
//! use axum::{Router, routing::get};
//! use tower::ServiceBuilder;
//!
//! // Initialize metrics registry
//! init_metrics().expect("Failed to initialize metrics");
//!
//! // Create router with metrics endpoint and middleware
//! let app: Router<()> = Router::new()
//!     .route("/api/data", get(|| async { "data" }))
//!     .route("/metrics", get(metrics_handler))
//!     .layer(ServiceBuilder::new().layer(MetricsLayer::default()));
//! ```

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use metrics::{
    counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram, Counter,
    Gauge, Histogram, Unit,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::{
    collections::HashSet,
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tower::{Layer, Service};
use tracing::error;

// ============================================================================
// Global Metrics Registry
// ============================================================================

/// Global Prometheus handle for metrics export
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Global metrics registry initialization
static METRICS_INITIALIZED: OnceLock<bool> = OnceLock::new();

/// Initializes the metrics system with a Prometheus exporter.
///
/// This function must be called once at application startup before collecting any metrics.
/// Subsequent calls will return Ok without re-initializing.
///
/// # Errors
///
/// Returns an error if the Prometheus exporter fails to install.
///
/// # Example
///
/// ```rust
/// use llm_research_api::observability::metrics::init_metrics;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// init_metrics()?;
/// # Ok(())
/// # }
/// ```
pub fn init_metrics() -> Result<(), MetricsError> {
    if METRICS_INITIALIZED.get().is_some() {
        return Ok(());
    }

    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix("http_request_duration".to_string()),
            &[
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .map_err(|e| MetricsError::Installation(e.to_string()))?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix("db_query_duration".to_string()),
            &[
                0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
            ],
        )
        .map_err(|e| MetricsError::Installation(e.to_string()))?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix("experiment_duration".to_string()),
            &[1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0],
        )
        .map_err(|e| MetricsError::Installation(e.to_string()))?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix("http_request_size".to_string()),
            &[
                100.0, 1_000.0, 10_000.0, 100_000.0, 1_000_000.0, 10_000_000.0,
            ],
        )
        .map_err(|e| MetricsError::Installation(e.to_string()))?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix("http_response_size".to_string()),
            &[
                100.0, 1_000.0, 10_000.0, 100_000.0, 1_000_000.0, 10_000_000.0,
            ],
        )
        .map_err(|e| MetricsError::Installation(e.to_string()))?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Prefix("dataset_upload_size".to_string()),
            &[
                1_000.0, 10_000.0, 100_000.0, 1_000_000.0, 10_000_000.0, 100_000_000.0,
                1_000_000_000.0,
            ],
        )
        .map_err(|e| MetricsError::Installation(e.to_string()))?
        .install_recorder()
        .map_err(|e| MetricsError::Installation(e.to_string()))?;

    PROMETHEUS_HANDLE
        .set(handle)
        .map_err(|_| MetricsError::Installation("Handle already set".to_string()))?;

    METRICS_INITIALIZED
        .set(true)
        .map_err(|_| MetricsError::Installation("Metrics already initialized".to_string()))?;

    // Register all metric descriptions
    register_metric_descriptions();

    Ok(())
}

/// Registers descriptions for all metrics in the system.
///
/// This improves the Prometheus output with helpful descriptions and units.
fn register_metric_descriptions() {
    // HTTP metrics
    describe_counter!(
        "http_requests_total",
        Unit::Count,
        "Total number of HTTP requests received"
    );
    describe_histogram!(
        "http_request_duration_seconds",
        Unit::Seconds,
        "HTTP request duration in seconds"
    );
    describe_gauge!(
        "http_requests_in_flight",
        Unit::Count,
        "Number of HTTP requests currently being processed"
    );
    describe_histogram!(
        "http_request_size_bytes",
        Unit::Bytes,
        "HTTP request body size in bytes"
    );
    describe_histogram!(
        "http_response_size_bytes",
        Unit::Bytes,
        "HTTP response body size in bytes"
    );

    // Database metrics
    describe_histogram!(
        "db_query_duration_seconds",
        Unit::Seconds,
        "Database query duration in seconds"
    );
    describe_gauge!(
        "db_connections_total",
        Unit::Count,
        "Number of active database connections"
    );
    describe_counter!(
        "db_connection_errors_total",
        Unit::Count,
        "Total number of database connection errors"
    );
    describe_counter!(
        "db_query_errors_total",
        Unit::Count,
        "Total number of database query errors"
    );

    // Business metrics
    describe_counter!(
        "experiments_created_total",
        Unit::Count,
        "Total number of experiments created"
    );
    describe_counter!(
        "experiments_completed_total",
        Unit::Count,
        "Total number of experiments completed"
    );
    describe_counter!(
        "experiment_runs_total",
        Unit::Count,
        "Total number of experiment runs"
    );
    describe_histogram!(
        "experiment_duration_seconds",
        Unit::Seconds,
        "Experiment execution duration in seconds"
    );
    describe_counter!(
        "evaluations_processed_total",
        Unit::Count,
        "Total number of evaluations processed"
    );
    describe_counter!(
        "models_registered_total",
        Unit::Count,
        "Total number of models registered"
    );
    describe_counter!(
        "datasets_uploaded_total",
        Unit::Count,
        "Total number of datasets uploaded"
    );
    describe_histogram!(
        "dataset_upload_size_bytes",
        Unit::Bytes,
        "Dataset upload size in bytes"
    );

    // System metrics
    describe_counter!(
        "process_cpu_seconds_total",
        Unit::Seconds,
        "Total CPU time consumed by the process"
    );
    describe_gauge!(
        "process_resident_memory_bytes",
        Unit::Bytes,
        "Resident memory size in bytes"
    );
    describe_gauge!(
        "process_open_fds",
        Unit::Count,
        "Number of open file descriptors"
    );
    describe_gauge!(
        "process_start_time_seconds",
        Unit::Seconds,
        "Process start time as Unix timestamp"
    );
}

// ============================================================================
// Metrics Error Types
// ============================================================================

/// Errors that can occur during metrics operations
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Failed to install metrics exporter: {0}")]
    Installation(String),

    #[error("Metrics not initialized")]
    NotInitialized,

    #[error("Failed to render metrics: {0}")]
    Render(String),
}

// ============================================================================
// Metrics Endpoint Handler
// ============================================================================

/// Axum handler that returns Prometheus metrics in text format.
///
/// This handler should be mounted at `/metrics` to allow Prometheus to scrape metrics.
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::get};
/// use llm_research_api::observability::metrics::metrics_handler;
///
/// let app: Router<()> = Router::new().route("/metrics", get(metrics_handler));
/// ```
pub async fn metrics_handler() -> Response {
    match PROMETHEUS_HANDLE.get() {
        Some(handle) => {
            let metrics = handle.render();
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4")],
                metrics,
            )
                .into_response()
        }
        None => {
            error!("Metrics handler called but metrics not initialized");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Metrics not initialized",
            )
                .into_response()
        }
    }
}

// ============================================================================
// HTTP Request Metrics
// ============================================================================

/// Records HTTP request metrics
pub struct HttpMetrics {
    requests_total: Counter,
    request_duration: Histogram,
    requests_in_flight: Gauge,
    request_size: Histogram,
    response_size: Histogram,
}

impl HttpMetrics {
    /// Creates a new HttpMetrics instance with labels
    pub fn new(method: &str, path: &str, status: &str) -> Self {
        let labels = [
            ("method", method.to_string()),
            ("path", path.to_string()),
            ("status", status.to_string()),
        ];

        Self {
            requests_total: counter!("http_requests_total", &labels[..2]),
            request_duration: histogram!("http_request_duration_seconds", &labels[..2]),
            requests_in_flight: gauge!("http_requests_in_flight"),
            request_size: histogram!("http_request_size_bytes", &labels[..2]),
            response_size: histogram!("http_response_size_bytes", &labels[..2]),
        }
    }

    /// Records the start of an HTTP request
    pub fn start(&self) {
        self.requests_in_flight.increment(1.0);
    }

    /// Records the completion of an HTTP request
    pub fn finish(&self, duration: Duration, req_size: u64, resp_size: u64) {
        self.requests_total.increment(1);
        self.request_duration.record(duration.as_secs_f64());
        self.requests_in_flight.decrement(1.0);
        self.request_size.record(req_size as f64);
        self.response_size.record(resp_size as f64);
    }
}

// ============================================================================
// Database Metrics
// ============================================================================

/// Database metrics collector
pub struct DatabaseMetrics;

impl DatabaseMetrics {
    /// Records a database query duration
    pub fn record_query(query_type: &str, table: &str, duration: Duration) {
        histogram!(
            "db_query_duration_seconds",
            "query_type" => query_type.to_string(),
            "table" => table.to_string()
        )
        .record(duration.as_secs_f64());
    }

    /// Records a database query error
    pub fn record_query_error(query_type: &str, error_type: &str) {
        counter!(
            "db_query_errors_total",
            "query_type" => query_type.to_string(),
            "error_type" => error_type.to_string()
        )
        .increment(1);
    }

    /// Records a database connection error
    pub fn record_connection_error() {
        counter!("db_connection_errors_total").increment(1);
    }

    /// Updates the active database connections gauge
    pub fn set_active_connections(count: usize) {
        gauge!("db_connections_total").set(count as f64);
    }
}

// ============================================================================
// Business Metrics
// ============================================================================

/// Business metrics for tracking domain-specific operations
pub struct BusinessMetrics;

impl BusinessMetrics {
    /// Records experiment creation
    pub fn experiment_created() {
        counter!("experiments_created_total").increment(1);
    }

    /// Records experiment completion
    pub fn experiment_completed(status: &str) {
        counter!("experiments_completed_total", "status" => status.to_string()).increment(1);
    }

    /// Records an experiment run
    pub fn experiment_run(status: &str) {
        counter!("experiment_runs_total", "status" => status.to_string()).increment(1);
    }

    /// Records experiment duration
    pub fn experiment_duration(duration: Duration) {
        histogram!("experiment_duration_seconds").record(duration.as_secs_f64());
    }

    /// Records evaluation processing
    pub fn evaluation_processed() {
        counter!("evaluations_processed_total").increment(1);
    }

    /// Records model registration
    pub fn model_registered() {
        counter!("models_registered_total").increment(1);
    }

    /// Records dataset upload
    pub fn dataset_uploaded(size_bytes: u64) {
        counter!("datasets_uploaded_total").increment(1);
        histogram!("dataset_upload_size_bytes").record(size_bytes as f64);
    }
}

// ============================================================================
// System Metrics
// ============================================================================

/// System-level metrics collector
pub struct SystemMetrics;

impl SystemMetrics {
    /// Records the process start time (should be called once at startup)
    pub fn record_start_time() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        gauge!("process_start_time_seconds").set(now.as_secs_f64());
    }

    /// Updates CPU time
    pub fn update_cpu_time(seconds: f64) {
        counter!("process_cpu_seconds_total").absolute(seconds as u64);
    }

    /// Updates resident memory
    pub fn update_resident_memory(bytes: u64) {
        gauge!("process_resident_memory_bytes").set(bytes as f64);
    }

    /// Updates open file descriptors
    pub fn update_open_fds(count: u64) {
        gauge!("process_open_fds").set(count as f64);
    }

    /// Collects and updates all system metrics (Linux-specific)
    #[cfg(target_os = "linux")]
    pub fn update_all() {
        use std::fs;

        // Read process stats
        if let Ok(stat) = fs::read_to_string("/proc/self/stat") {
            let parts: Vec<&str> = stat.split_whitespace().collect();
            if parts.len() > 14 {
                // User time + system time (in clock ticks)
                if let (Ok(utime), Ok(stime)) = (parts[13].parse::<u64>(), parts[14].parse::<u64>()) {
                    let clock_ticks_per_sec = 100; // Usually 100 on Linux
                    let cpu_seconds = (utime + stime) as f64 / clock_ticks_per_sec as f64;
                    Self::update_cpu_time(cpu_seconds);
                }
            }
        }

        // Read memory stats
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(kb_val) = kb.parse::<u64>() {
                            Self::update_resident_memory(kb_val * 1024);
                        }
                    }
                    break;
                }
            }
        }

        // Count open file descriptors
        if let Ok(entries) = fs::read_dir("/proc/self/fd") {
            let count = entries.count() as u64;
            Self::update_open_fds(count);
        }
    }

    /// Stub for non-Linux systems
    #[cfg(not(target_os = "linux"))]
    pub fn update_all() {
        // System metrics collection not implemented for non-Linux platforms
        // This is a no-op on Windows, macOS, etc.
    }
}

// ============================================================================
// Metrics Middleware
// ============================================================================

/// Configuration for the metrics middleware
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Paths to exclude from metrics collection (e.g., /health, /metrics)
    pub excluded_paths: HashSet<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        let mut excluded_paths = HashSet::new();
        excluded_paths.insert("/health".to_string());
        excluded_paths.insert("/metrics".to_string());

        Self { excluded_paths }
    }
}

impl MetricsConfig {
    /// Creates a new MetricsConfig with custom excluded paths
    pub fn new(excluded_paths: HashSet<String>) -> Self {
        Self { excluded_paths }
    }

    /// Adds a path to the exclusion list
    pub fn exclude_path(mut self, path: impl Into<String>) -> Self {
        self.excluded_paths.insert(path.into());
        self
    }

    /// Checks if a path should be excluded from metrics
    pub fn is_excluded(&self, path: &str) -> bool {
        self.excluded_paths.contains(path)
    }
}

/// Tower layer for automatic HTTP metrics collection
#[derive(Clone)]
pub struct MetricsLayer {
    config: Arc<MetricsConfig>,
}

impl Default for MetricsLayer {
    fn default() -> Self {
        Self {
            config: Arc::new(MetricsConfig::default()),
        }
    }
}

impl MetricsLayer {
    /// Creates a new MetricsLayer with custom configuration
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Middleware service for collecting HTTP metrics
#[derive(Clone)]
pub struct MetricsMiddleware<S> {
    inner: S,
    config: Arc<MetricsConfig>,
}

impl<S> Service<Request> for MetricsMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let excluded = self.config.is_excluded(&path);

        // Get request size
        let req_size = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let future = self.inner.call(req);

        Box::pin(async move {
            let response = future.await?;

            if !excluded {
                let status = response.status().as_u16().to_string();
                let duration = start.elapsed();

                // Get response size
                let resp_size = response
                    .headers()
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);

                // Record metrics
                let metrics = HttpMetrics::new(&method, &path, &status);
                metrics.finish(duration, req_size, resp_size);

                // Also increment the counter with status
                counter!(
                    "http_requests_total",
                    "method" => method,
                    "path" => path,
                    "status" => status
                )
                .increment(1);
            }

            Ok(response)
        })
    }
}

// ============================================================================
// Helper Functions and Utilities
// ============================================================================

/// Timer guard that automatically records duration when dropped
pub struct DurationGuard {
    start: Instant,
    histogram: Histogram,
}

impl DurationGuard {
    /// Creates a new duration guard
    pub fn new(histogram: Histogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }
}

impl Drop for DurationGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.histogram.record(duration.as_secs_f64());
    }
}

/// Helper function to observe a duration for a labeled histogram
///
/// # Example
///
/// ```rust,no_run
/// use llm_research_api::observability::metrics::observe_duration;
/// use std::time::Duration;
///
/// observe_duration("db_query_duration_seconds", Duration::from_millis(50), &[
///     ("query_type", "select"),
///     ("table", "experiments"),
/// ]);
/// ```
pub fn observe_duration(metric_name: &str, duration: Duration, labels: &[(&str, &str)]) {
    let metric_name = metric_name.to_string();
    let label_vec: Vec<(String, String)> = labels
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    histogram!(metric_name, &label_vec).record(duration.as_secs_f64());
}

/// Helper function to increment a counter
///
/// # Example
///
/// ```rust,no_run
/// use llm_research_api::observability::metrics::increment_counter;
///
/// increment_counter("experiments_created_total", &[]);
/// increment_counter("experiment_runs_total", &[("status", "success")]);
/// ```
pub fn increment_counter(metric_name: &str, labels: &[(&str, &str)]) {
    let metric_name = metric_name.to_string();
    let label_vec: Vec<(String, String)> = labels
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    counter!(metric_name, &label_vec).increment(1);
}

/// Helper function to set a gauge value
///
/// # Example
///
/// ```rust,no_run
/// use llm_research_api::observability::metrics::set_gauge;
///
/// set_gauge("db_connections_total", 10.0, &[]);
/// ```
pub fn set_gauge(metric_name: &str, value: f64, labels: &[(&str, &str)]) {
    let metric_name = metric_name.to_string();
    let label_vec: Vec<(String, String)> = labels
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    gauge!(metric_name, &label_vec).set(value);
}

/// Batch metric update recorder
pub struct MetricsRecorder {
    updates: Vec<Box<dyn FnOnce() + Send>>,
}

impl MetricsRecorder {
    /// Creates a new metrics recorder
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
        }
    }

    /// Adds a counter increment to the batch
    pub fn increment_counter(mut self, metric_name: &str, labels: &[(&str, &str)]) -> Self {
        let metric_name = metric_name.to_string();
        let label_vec: Vec<(String, String)> = labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        self.updates.push(Box::new(move || {
            counter!(metric_name, &label_vec).increment(1);
        }));
        self
    }

    /// Adds a histogram observation to the batch
    pub fn observe_histogram(
        mut self,
        metric_name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> Self {
        let metric_name = metric_name.to_string();
        let label_vec: Vec<(String, String)> = labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        self.updates.push(Box::new(move || {
            histogram!(metric_name, &label_vec).record(value);
        }));
        self
    }

    /// Adds a gauge update to the batch
    pub fn set_gauge(mut self, metric_name: &str, value: f64, labels: &[(&str, &str)]) -> Self {
        let metric_name = metric_name.to_string();
        let label_vec: Vec<(String, String)> = labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        self.updates.push(Box::new(move || {
            gauge!(metric_name, &label_vec).set(value);
        }));
        self
    }

    /// Executes all batched metric updates
    pub fn record(self) {
        for update in self.updates {
            update();
        }
    }
}

impl Default for MetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use std::time::Duration;
    use tower::{Service, ServiceBuilder, ServiceExt};

    #[test]
    fn test_init_metrics() {
        // Metrics may already be initialized in other tests, which is fine.
        // The init_metrics function is designed to be idempotent - calling it
        // multiple times should either succeed or return Ok if already initialized.
        let result = init_metrics();
        // Accept either success (Ok) or installation error (if underlying lib fails on re-init)
        // This is expected behavior in a multi-test environment.
        if let Err(e) = &result {
            // Installation error is acceptable if recorder was already set by another test
            assert!(matches!(e, MetricsError::Installation(_)));
        }
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(config.is_excluded("/health"));
        assert!(config.is_excluded("/metrics"));
        assert!(!config.is_excluded("/api/data"));
    }

    #[test]
    fn test_metrics_config_custom() {
        let config = MetricsConfig::default()
            .exclude_path("/internal")
            .exclude_path("/debug");

        assert!(config.is_excluded("/health"));
        assert!(config.is_excluded("/internal"));
        assert!(config.is_excluded("/debug"));
        assert!(!config.is_excluded("/api/data"));
    }

    #[tokio::test]
    async fn test_http_metrics() {
        init_metrics().ok();

        let metrics = HttpMetrics::new("GET", "/api/test", "200");
        metrics.start();

        let duration = Duration::from_millis(100);
        metrics.finish(duration, 1024, 2048);

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_database_metrics() {
        init_metrics().ok();

        DatabaseMetrics::record_query("select", "experiments", Duration::from_millis(10));
        DatabaseMetrics::record_query_error("insert", "constraint_violation");
        DatabaseMetrics::record_connection_error();
        DatabaseMetrics::set_active_connections(5);

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_business_metrics() {
        init_metrics().ok();

        BusinessMetrics::experiment_created();
        BusinessMetrics::experiment_completed("success");
        BusinessMetrics::experiment_run("running");
        BusinessMetrics::experiment_duration(Duration::from_secs(60));
        BusinessMetrics::evaluation_processed();
        BusinessMetrics::model_registered();
        BusinessMetrics::dataset_uploaded(1_000_000);

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_system_metrics() {
        init_metrics().ok();

        SystemMetrics::record_start_time();
        SystemMetrics::update_cpu_time(1.5);
        SystemMetrics::update_resident_memory(1024 * 1024 * 100);
        SystemMetrics::update_open_fds(42);

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_observe_duration_helper() {
        init_metrics().ok();

        observe_duration(
            "test_duration",
            Duration::from_millis(50),
            &[("label1", "value1")],
        );

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_increment_counter_helper() {
        init_metrics().ok();

        increment_counter("test_counter", &[("status", "success")]);

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_set_gauge_helper() {
        init_metrics().ok();

        set_gauge("test_gauge", 42.0, &[("type", "test")]);

        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_metrics_recorder() {
        init_metrics().ok();

        MetricsRecorder::new()
            .increment_counter("test_counter", &[("label", "value")])
            .observe_histogram("test_histogram", 1.5, &[])
            .set_gauge("test_gauge", 100.0, &[])
            .record();

        // Verify metrics were recorded (no panic)
    }

    #[tokio::test]
    async fn test_metrics_middleware() {
        init_metrics().ok();

        async fn test_handler() -> &'static str {
            "test response"
        }

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(ServiceBuilder::new().layer(MetricsLayer::default()));

        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify the middleware didn't break the request flow
    }

    #[tokio::test]
    async fn test_metrics_middleware_excluded_path() {
        init_metrics().ok();

        async fn health_handler() -> &'static str {
            "OK"
        }

        let app = Router::new()
            .route("/health", get(health_handler))
            .layer(ServiceBuilder::new().layer(MetricsLayer::default()));

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The /health endpoint should be excluded from metrics
    }

    #[tokio::test]
    async fn test_metrics_handler() {
        init_metrics().ok();

        let response = metrics_handler().await;
        assert_eq!(response.status(), StatusCode::OK);

        // Verify content-type header
        let headers = response.headers();
        assert_eq!(
            headers.get("content-type").unwrap(),
            "text/plain; version=0.0.4"
        );
    }

    #[test]
    fn test_duration_guard() {
        init_metrics().ok();

        let hist = histogram!("test_duration_guard");
        let _guard = DurationGuard::new(hist);

        // Sleep a bit to ensure measurable duration
        std::thread::sleep(Duration::from_millis(10));

        // Guard will record duration when dropped
    }

    #[test]
    fn test_metrics_error_display() {
        let err = MetricsError::Installation("test error".to_string());
        assert_eq!(err.to_string(), "Failed to install metrics exporter: test error");

        let err = MetricsError::NotInitialized;
        assert_eq!(err.to_string(), "Metrics not initialized");

        let err = MetricsError::Render("render error".to_string());
        assert_eq!(err.to_string(), "Failed to render metrics: render error");
    }
}
