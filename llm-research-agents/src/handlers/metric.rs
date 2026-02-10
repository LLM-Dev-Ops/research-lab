//! Metric Agent Handler
//!
//! HTTP handler for the Experimental Metric Agent endpoint.
//!
//! # Constitution Compliance
//!
//! Per PROMPT 2 (RUNTIME & INFRASTRUCTURE IMPLEMENTATION):
//!
//! - Handler is stateless
//! - Handler is deterministic
//! - No orchestration logic
//! - No optimization logic
//! - No direct SQL access
//! - Async, non-blocking writes via ruvector-service only
//!
//! # Endpoint
//!
//! `POST /api/v1/agents/metric`
//!
//! # Request Format
//!
//! ```json
//! {
//!   "request_id": "uuid",
//!   "context_id": "experiment-123",
//!   "metrics_requested": [...],
//!   "data": {...},
//!   "config": {...}
//! }
//! ```
//!
//! # Response Format
//!
//! ```json
//! {
//!   "success": true,
//!   "request_id": "uuid",
//!   "output": {...},
//!   "decision_event": {...}
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use validator::Validate;

use crate::agents::{Agent, ExperimentalMetricAgent, METRIC_AGENT_ID, METRIC_AGENT_VERSION};
use crate::clients::{RuVectorClient, RuVectorPersistence};
use crate::contracts::metrics::{MetricsInput, MetricsOutput};
use crate::contracts::DecisionEvent;
use crate::execution::{ExecutionArtifact, ExecutionContext, ExecutionSpan};
use crate::telemetry::TelemetryEmitter;

/// Trace context for distributed tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

/// Request for metric computation.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MetricComputeRequest {
    /// Metrics input
    #[validate(nested)]
    pub input: MetricsInput,

    /// Optional trace context for distributed tracing
    pub trace_context: Option<MetricTraceContext>,

    /// Execution context from the Agentics Core orchestrator.
    /// Required for all externally-invoked operations.
    pub execution_context: Option<ExecutionContext>,
}

/// Response from metric computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComputeResponse {
    /// Request succeeded
    pub success: bool,

    /// Original request ID
    pub request_id: Uuid,

    /// Computation output (if successful)
    pub output: Option<MetricsOutput>,

    /// Decision event (if successful)
    pub decision_event: Option<DecisionEventSummary>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Error code (if failed)
    pub error_code: Option<String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Summary of decision event (without full outputs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEventSummary {
    pub id: Uuid,
    pub agent_id: String,
    pub agent_version: String,
    pub decision_type: String,
    pub confidence: String,
    pub storage_ref: Option<String>,
}

/// Handler for metric computation requests.
///
/// This handler is designed to be deployed as a Google Cloud Edge Function.
/// It is stateless and all persistence is done via ruvector-service.
pub struct MetricHandler {
    agent: ExperimentalMetricAgent,
    ruvector: Option<RuVectorClient>,
    telemetry: TelemetryEmitter,
}

impl MetricHandler {
    /// Create a new metric handler.
    pub fn new() -> Self {
        let ruvector = RuVectorClient::from_env().ok();

        Self {
            agent: ExperimentalMetricAgent::new(),
            ruvector,
            telemetry: TelemetryEmitter::new(),
        }
    }

    /// Create handler with custom configuration.
    pub fn with_config(ruvector: Option<RuVectorClient>, telemetry: TelemetryEmitter) -> Self {
        Self {
            agent: ExperimentalMetricAgent::new(),
            ruvector,
            telemetry,
        }
    }

    /// Handle a metric computation request.
    ///
    /// This is the primary entry point for the Edge Function.
    /// Creates an agent-level execution span, attaches artifacts, and returns
    /// both the response and the span for inclusion in the repo-level span.
    ///
    /// `repo_span_id` is the span_id of the enclosing repo-level span.
    #[instrument(skip(self, request), fields(
        request_id = %request.input.request_id,
        context_id = %request.input.context_id,
        metrics_count = request.input.metrics_requested.len()
    ))]
    pub async fn handle(
        &self,
        request: MetricComputeRequest,
        repo_span_id: Uuid,
    ) -> (MetricComputeResponse, ExecutionSpan) {
        let start_time = Instant::now();
        let request_id = request.input.request_id;
        let mut agent_span = ExecutionSpan::new_agent(repo_span_id, METRIC_AGENT_ID);

        info!("Handling metric computation request");

        // Validate request
        if let Err(e) = request.validate() {
            error!(error = %e, "Request validation failed");
            agent_span.fail(format!("Validation error: {}", e));
            let response = MetricComputeResponse {
                success: false,
                request_id,
                output: None,
                decision_event: None,
                error: Some(format!("Validation error: {}", e)),
                error_code: Some("METRIC_INPUT_INVALID".to_string()),
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            };
            return (response, agent_span);
        }

        // Execute agent
        let (output, event) = match self.agent.invoke(request.input).await {
            Ok(result) => result,
            Err(e) => {
                error!(error = %e, "Metric computation failed");
                agent_span.fail(format!("Computation error: {}", e));
                let response = MetricComputeResponse {
                    success: false,
                    request_id,
                    output: None,
                    decision_event: None,
                    error: Some(format!("Computation error: {}", e)),
                    error_code: Some("METRIC_COMPUTATION_FAILED".to_string()),
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                };
                return (response, agent_span);
            }
        };

        // Persist decision event
        let storage_ref = self.persist_decision_event(&event).await;

        // Attach decision event as artifact to agent span
        agent_span.add_artifact(ExecutionArtifact {
            id: format!("decision-event-{}", event.id),
            uri: storage_ref.clone(),
            hash: Some(event.inputs_hash.clone()),
            filename: None,
            artifact_type: "decision_event".to_string(),
            data: serde_json::json!({
                "event_id": event.id,
                "decision_type": event.decision_type.to_string(),
                "confidence": event.confidence.value.to_string(),
                "agent_id": event.agent_id,
                "agent_version": event.agent_version,
            }),
        });

        // Attach computed metrics as artifact
        agent_span.add_artifact(ExecutionArtifact {
            id: format!("metrics-output-{}", request_id),
            uri: None,
            hash: None,
            filename: None,
            artifact_type: "computed_metrics".to_string(),
            data: serde_json::json!({
                "metrics_count": output.metrics.len(),
                "metric_names": output.metrics.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
                "request_id": request_id,
            }),
        });

        agent_span.complete();

        // Build response
        let decision_summary = DecisionEventSummary {
            id: event.id,
            agent_id: event.agent_id.clone(),
            agent_version: event.agent_version.clone(),
            decision_type: event.decision_type.to_string(),
            confidence: event.confidence.value.to_string(),
            storage_ref,
        };

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            metrics_computed = output.metrics.len(),
            processing_time_ms = processing_time_ms,
            "Metric computation completed"
        );

        let response = MetricComputeResponse {
            success: true,
            request_id,
            output: Some(output),
            decision_event: Some(decision_summary),
            error: None,
            error_code: None,
            processing_time_ms,
        };

        (response, agent_span)
    }

    /// Persist decision event to ruvector-service.
    async fn persist_decision_event(&self, event: &DecisionEvent) -> Option<String> {
        let Some(ref client) = self.ruvector else {
            warn!("RuVector client not configured, skipping persistence");
            return None;
        };

        match client.persist_decision_event(event.clone()).await {
            Ok(persisted) => {
                info!(
                    storage_ref = %persisted.storage_ref,
                    "Decision event persisted"
                );
                Some(persisted.storage_ref)
            }
            Err(e) => {
                error!(error = %e, "Failed to persist decision event");
                None
            }
        }
    }

    /// Get agent identity information.
    pub fn agent_info(&self) -> AgentInfo {
        AgentInfo {
            id: METRIC_AGENT_ID.to_string(),
            version: METRIC_AGENT_VERSION.to_string(),
            classification: "EXPERIMENTAL_METRICS".to_string(),
            endpoint: "/api/v1/agents/metric".to_string(),
        }
    }
}

impl Default for MetricHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub version: String,
    pub classification: String,
    pub endpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::metrics::*;
    use rust_decimal_macros::dec;

    fn create_test_request() -> MetricComputeRequest {
        let records: Vec<serde_json::Value> = (0..50)
            .map(|i| {
                serde_json::json!({
                    "accuracy": 0.5 + (i as f64) * 0.01,
                    "latency": 100.0 + (i as f64) * 5.0,
                })
            })
            .collect();

        MetricComputeRequest {
            input: MetricsInput {
                request_id: Uuid::new_v4(),
                context_id: "test-experiment".to_string(),
                metrics_requested: vec![
                    MetricRequest {
                        name: "mean_accuracy".to_string(),
                        metric_type: MetricType::CentralTendency,
                        variable: "accuracy".to_string(),
                        group_by: None,
                        params: None,
                    },
                ],
                data: MetricsData {
                    source: "test".to_string(),
                    records,
                    schema: None,
                },
                config: MetricsConfig {
                    handle_missing: MissingValueStrategy::Skip,
                    precision: 4,
                    include_ci: true,
                    ci_level: Some(dec!(0.95)),
                },
            },
            trace_context: None,
            execution_context: Some(ExecutionContext {
                execution_id: Uuid::new_v4(),
                parent_span_id: Uuid::new_v4(),
            }),
        }
    }

    #[tokio::test]
    async fn test_metric_handler_success() {
        let handler = MetricHandler::new();
        let request = create_test_request();
        let repo_span_id = Uuid::new_v4();

        let (response, agent_span) = handler.handle(request, repo_span_id).await;

        assert!(response.success);
        assert!(response.output.is_some());
        assert!(response.decision_event.is_some());
        assert!(response.error.is_none());

        let output = response.output.unwrap();
        assert_eq!(output.metrics.len(), 1);
        assert_eq!(output.metrics[0].name, "mean_accuracy");

        // Verify agent span
        assert_eq!(agent_span.parent_span_id, repo_span_id);
        assert_eq!(agent_span.span_type, crate::execution::SpanType::Agent);
        assert_eq!(agent_span.status, crate::execution::SpanStatus::Completed);
        assert_eq!(agent_span.agent_name.as_deref(), Some(METRIC_AGENT_ID));
        assert!(!agent_span.artifacts.is_empty());
    }

    #[tokio::test]
    async fn test_metric_handler_empty_data() {
        let handler = MetricHandler::new();
        let mut request = create_test_request();
        request.input.data.records.clear();
        let repo_span_id = Uuid::new_v4();

        let (response, agent_span) = handler.handle(request, repo_span_id).await;

        assert!(!response.success);
        assert!(response.error.is_some());
        assert_eq!(response.error_code, Some("METRIC_COMPUTATION_FAILED".to_string()));

        // Verify agent span records failure
        assert_eq!(agent_span.status, crate::execution::SpanStatus::Failed);
        assert!(agent_span.failure_reason.is_some());
    }

    #[test]
    fn test_agent_info() {
        let handler = MetricHandler::new();
        let info = handler.agent_info();

        assert_eq!(info.id, METRIC_AGENT_ID);
        assert_eq!(info.classification, "EXPERIMENTAL_METRICS");
    }
}
