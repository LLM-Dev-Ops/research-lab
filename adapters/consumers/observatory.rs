//! LLM-Observatory Consumption Adapter
//!
//! Thin adapter for consuming telemetry, experiment traces, and research metrics
//! from the LLM-Observatory service. This adapter does not modify any existing
//! research logic, evaluation metrics, or scoring systems.
//!
//! # Consumed Data Types
//!
//! - Telemetry data (request/response traces, timing data)
//! - Experiment traces (execution logs, state transitions)
//! - Research metrics (custom metrics, aggregated statistics)
//! - Observability events (alerts, anomalies, thresholds)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ConsumerResult, ConsumptionMetadata, ExternalServiceConfig, HealthCheckable};

/// Configuration specific to LLM-Observatory consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryConfig {
    /// Base service configuration
    #[serde(flatten)]
    pub base: ExternalServiceConfig,
    /// Project/workspace identifier
    pub project_id: Option<String>,
    /// Default time window for queries (in seconds)
    pub default_time_window_secs: u64,
    /// Whether to include raw span data
    pub include_raw_spans: bool,
}

impl Default for ObservatoryConfig {
    fn default() -> Self {
        Self {
            base: ExternalServiceConfig {
                endpoint: "https://api.llm-observatory.local".to_string(),
                ..Default::default()
            },
            project_id: None,
            default_time_window_secs: 3600, // 1 hour
            include_raw_spans: false,
        }
    }
}

/// Telemetry data from Observatory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryData {
    /// Trace identifier
    pub trace_id: String,
    /// Span identifier
    pub span_id: String,
    /// Parent span (if any)
    pub parent_span_id: Option<String>,
    /// Operation name
    pub operation_name: String,
    /// Start timestamp (Unix millis)
    pub start_time_ms: u64,
    /// Duration in milliseconds
    pub duration_ms: f64,
    /// Status code
    pub status: TelemetryStatus,
    /// Attributes/tags
    pub attributes: Value,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Status of a telemetry span.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryStatus {
    Ok,
    Error,
    Timeout,
    Cancelled,
}

/// Experiment trace from Observatory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentTrace {
    /// Experiment identifier
    pub experiment_id: Uuid,
    /// Run identifier
    pub run_id: Uuid,
    /// Trace events in chronological order
    pub events: Vec<TraceEvent>,
    /// Aggregate timing metrics
    pub timing: TraceTiming,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// A single trace event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Event timestamp (Unix millis)
    pub timestamp_ms: u64,
    /// Event type/name
    pub event_type: String,
    /// Event severity
    pub severity: EventSeverity,
    /// Event message
    pub message: String,
    /// Additional context
    pub context: Option<Value>,
}

/// Event severity levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Timing information for a trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceTiming {
    /// Total duration in milliseconds
    pub total_duration_ms: f64,
    /// Time spent in each phase
    pub phase_timings: Value,
    /// Bottleneck identification
    pub bottlenecks: Vec<String>,
}

/// Research metrics from Observatory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchMetrics {
    /// Metric name
    pub name: String,
    /// Metric type (counter, gauge, histogram)
    pub metric_type: MetricType,
    /// Data points
    pub data_points: Vec<MetricDataPoint>,
    /// Aggregations
    pub aggregations: MetricAggregations,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Type of metric.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// A single metric data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    /// Timestamp (Unix millis)
    pub timestamp_ms: u64,
    /// Value
    pub value: f64,
    /// Labels/dimensions
    pub labels: Value,
}

/// Aggregated metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricAggregations {
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Average value
    pub avg: f64,
    /// Sum of values
    pub sum: f64,
    /// Count of data points
    pub count: u64,
    /// Percentiles (p50, p90, p95, p99)
    pub percentiles: Option<Value>,
}

/// Query parameters for Observatory consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryQuery {
    /// Start time (ISO 8601 or Unix millis)
    pub start_time: String,
    /// End time (ISO 8601 or Unix millis)
    pub end_time: String,
    /// Filter by experiment IDs
    pub experiment_ids: Option<Vec<Uuid>>,
    /// Filter by metric names
    pub metric_names: Option<Vec<String>>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Aggregation interval (e.g., "1m", "5m", "1h")
    pub aggregation_interval: Option<String>,
}

/// Trait for consuming telemetry and metrics from LLM-Observatory.
#[async_trait]
pub trait ObservatoryConsumer: HealthCheckable {
    /// Consume telemetry data for a specific trace.
    async fn consume_telemetry(&self, trace_id: &str) -> ConsumerResult<Vec<TelemetryData>>;

    /// Consume telemetry data within a time range.
    async fn consume_telemetry_range(
        &self,
        query: &ObservatoryQuery,
    ) -> ConsumerResult<Vec<TelemetryData>>;

    /// Consume experiment traces.
    async fn consume_experiment_traces(
        &self,
        experiment_id: Uuid,
    ) -> ConsumerResult<Vec<ExperimentTrace>>;

    /// Consume research metrics.
    async fn consume_research_metrics(
        &self,
        query: &ObservatoryQuery,
    ) -> ConsumerResult<Vec<ResearchMetrics>>;

    /// Consume specific metrics by name.
    async fn consume_metrics_by_name(
        &self,
        metric_names: &[String],
        query: &ObservatoryQuery,
    ) -> ConsumerResult<Vec<ResearchMetrics>>;

    /// Get available metric names.
    async fn list_available_metrics(&self) -> ConsumerResult<Vec<String>>;
}

/// Client implementation for consuming from LLM-Observatory.
pub struct ObservatoryClient {
    config: ObservatoryConfig,
}

impl ObservatoryClient {
    /// Create a new observatory client with the given configuration.
    pub fn new(config: ObservatoryConfig) -> Self {
        Self { config }
    }

    /// Create a client with default configuration and custom endpoint.
    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            config: ObservatoryConfig {
                base: ExternalServiceConfig {
                    endpoint: endpoint.to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ObservatoryConfig {
        &self.config
    }
}

#[async_trait]
impl HealthCheckable for ObservatoryClient {
    async fn health_check(&self) -> ConsumerResult<bool> {
        Ok(!self.config.base.endpoint.is_empty())
    }
}

#[async_trait]
impl ObservatoryConsumer for ObservatoryClient {
    async fn consume_telemetry(&self, trace_id: &str) -> ConsumerResult<Vec<TelemetryData>> {
        // Implementation would fetch telemetry spans for the given trace
        Ok(vec![TelemetryData {
            trace_id: trace_id.to_string(),
            span_id: "root".to_string(),
            parent_span_id: None,
            operation_name: "placeholder".to_string(),
            start_time_ms: 0,
            duration_ms: 0.0,
            status: TelemetryStatus::Ok,
            attributes: serde_json::json!({}),
            metadata: ConsumptionMetadata::new("llm-observatory"),
        }])
    }

    async fn consume_telemetry_range(
        &self,
        _query: &ObservatoryQuery,
    ) -> ConsumerResult<Vec<TelemetryData>> {
        // Implementation would query Observatory with time range
        Ok(vec![])
    }

    async fn consume_experiment_traces(
        &self,
        experiment_id: Uuid,
    ) -> ConsumerResult<Vec<ExperimentTrace>> {
        // Implementation would fetch experiment traces
        Ok(vec![ExperimentTrace {
            experiment_id,
            run_id: Uuid::new_v4(),
            events: vec![],
            timing: TraceTiming {
                total_duration_ms: 0.0,
                phase_timings: serde_json::json!({}),
                bottlenecks: vec![],
            },
            metadata: ConsumptionMetadata::new("llm-observatory"),
        }])
    }

    async fn consume_research_metrics(
        &self,
        _query: &ObservatoryQuery,
    ) -> ConsumerResult<Vec<ResearchMetrics>> {
        // Implementation would fetch research metrics
        Ok(vec![])
    }

    async fn consume_metrics_by_name(
        &self,
        metric_names: &[String],
        _query: &ObservatoryQuery,
    ) -> ConsumerResult<Vec<ResearchMetrics>> {
        // Implementation would fetch specific metrics by name
        Ok(metric_names
            .iter()
            .map(|name| ResearchMetrics {
                name: name.clone(),
                metric_type: MetricType::Gauge,
                data_points: vec![],
                aggregations: MetricAggregations {
                    min: 0.0,
                    max: 0.0,
                    avg: 0.0,
                    sum: 0.0,
                    count: 0,
                    percentiles: None,
                },
                metadata: ConsumptionMetadata::new("llm-observatory"),
            })
            .collect())
    }

    async fn list_available_metrics(&self) -> ConsumerResult<Vec<String>> {
        // Implementation would list metric names from Observatory
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observatory_config_default() {
        let config = ObservatoryConfig::default();
        assert_eq!(config.default_time_window_secs, 3600);
        assert!(!config.include_raw_spans);
    }

    #[test]
    fn test_client_creation() {
        let client = ObservatoryClient::with_endpoint("https://observatory.example.com");
        assert_eq!(
            client.config().base.endpoint,
            "https://observatory.example.com"
        );
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = ObservatoryClient::with_endpoint("https://observatory.example.com");
        let result = client.health_check().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_telemetry_status_serialization() {
        let status = TelemetryStatus::Ok;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ok\"");
    }

    #[test]
    fn test_metric_type_serialization() {
        let metric_type = MetricType::Histogram;
        let json = serde_json::to_string(&metric_type).unwrap();
        assert_eq!(json, "\"histogram\"");
    }
}
