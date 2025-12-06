//! LLM-Simulator Consumption Adapter
//!
//! Thin adapter for consuming simulation outputs and synthetic evaluation runs
//! from the LLM-Simulator service. This adapter does not modify any existing
//! research logic or evaluation modules.
//!
//! # Consumed Data Types
//!
//! - Simulation outputs (model behavior simulations)
//! - Synthetic evaluation runs (generated test scenarios)
//! - Simulation configuration snapshots
//! - Performance profiles from simulated workloads

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ConsumerResult, ConsumptionMetadata, ExternalServiceConfig, HealthCheckable};

/// Configuration specific to LLM-Simulator consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorConfig {
    /// Base service configuration
    #[serde(flatten)]
    pub base: ExternalServiceConfig,
    /// Simulation namespace/project
    pub namespace: Option<String>,
    /// Whether to include raw simulation traces
    pub include_traces: bool,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            base: ExternalServiceConfig {
                endpoint: "https://api.llm-simulator.local".to_string(),
                ..Default::default()
            },
            namespace: None,
            include_traces: false,
        }
    }
}

/// Represents a simulation output from LLM-Simulator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutput {
    /// Unique simulation run identifier
    pub simulation_id: Uuid,
    /// Model configuration used in simulation
    pub model_config: Value,
    /// Simulated response outputs
    pub outputs: Vec<SimulatedResponse>,
    /// Performance metrics from simulation
    pub performance_metrics: SimulationPerformanceMetrics,
    /// Consumption metadata for lineage
    pub metadata: ConsumptionMetadata,
}

/// A single simulated response from the simulator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedResponse {
    /// Input prompt used
    pub prompt: String,
    /// Generated response
    pub response: String,
    /// Token count
    pub token_count: u32,
    /// Simulated latency in milliseconds
    pub latency_ms: f64,
    /// Confidence/quality score
    pub quality_score: Option<f64>,
}

/// Performance metrics from a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationPerformanceMetrics {
    /// Total tokens processed
    pub total_tokens: u64,
    /// Average tokens per second
    pub tokens_per_second: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// P95 latency
    pub p95_latency_ms: f64,
    /// Memory usage in bytes
    pub memory_bytes: Option<u64>,
}

/// Represents a synthetic evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticEvaluationRun {
    /// Evaluation run identifier
    pub evaluation_id: Uuid,
    /// Test scenarios included
    pub scenarios: Vec<SyntheticScenario>,
    /// Aggregate results
    pub aggregate_results: Value,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// A synthetic test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticScenario {
    /// Scenario identifier
    pub scenario_id: String,
    /// Scenario type/category
    pub category: String,
    /// Input data
    pub input: Value,
    /// Expected output (if available)
    pub expected_output: Option<Value>,
    /// Actual simulated output
    pub simulated_output: Value,
    /// Pass/fail status
    pub passed: bool,
}

/// Trait for consuming simulation data from LLM-Simulator.
#[async_trait]
pub trait SimulatorConsumer: HealthCheckable {
    /// Consume simulation outputs for a specific run.
    async fn consume_simulation_outputs(
        &self,
        simulation_id: Uuid,
    ) -> ConsumerResult<SimulationOutput>;

    /// Consume simulation outputs within a time range.
    async fn consume_simulation_outputs_range(
        &self,
        start_time: &str,
        end_time: &str,
        limit: Option<usize>,
    ) -> ConsumerResult<Vec<SimulationOutput>>;

    /// Consume synthetic evaluation runs.
    async fn consume_synthetic_evaluations(
        &self,
        evaluation_ids: &[Uuid],
    ) -> ConsumerResult<Vec<SyntheticEvaluationRun>>;

    /// Consume the latest simulation outputs for a model configuration.
    async fn consume_latest_for_model(
        &self,
        model_identifier: &str,
        count: usize,
    ) -> ConsumerResult<Vec<SimulationOutput>>;

    /// List available simulation runs.
    async fn list_available_simulations(
        &self,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> ConsumerResult<Vec<Uuid>>;
}

/// Client implementation for consuming from LLM-Simulator.
pub struct SimulatorClient {
    config: SimulatorConfig,
}

impl SimulatorClient {
    /// Create a new simulator client with the given configuration.
    pub fn new(config: SimulatorConfig) -> Self {
        Self { config }
    }

    /// Create a client with default configuration.
    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            config: SimulatorConfig {
                base: ExternalServiceConfig {
                    endpoint: endpoint.to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &SimulatorConfig {
        &self.config
    }
}

#[async_trait]
impl HealthCheckable for SimulatorClient {
    async fn health_check(&self) -> ConsumerResult<bool> {
        // In production, this would make an HTTP request to the health endpoint
        // For now, return true to indicate the adapter is properly configured
        Ok(!self.config.base.endpoint.is_empty())
    }
}

#[async_trait]
impl SimulatorConsumer for SimulatorClient {
    async fn consume_simulation_outputs(
        &self,
        simulation_id: Uuid,
    ) -> ConsumerResult<SimulationOutput> {
        // This is a thin adapter - actual HTTP calls would use the workspace's
        // reqwest dependency and the llm-simulator SDK
        // The implementation connects to: self.config.base.endpoint

        // Placeholder structure showing the expected data flow
        Ok(SimulationOutput {
            simulation_id,
            model_config: serde_json::json!({}),
            outputs: vec![],
            performance_metrics: SimulationPerformanceMetrics {
                total_tokens: 0,
                tokens_per_second: 0.0,
                avg_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                memory_bytes: None,
            },
            metadata: ConsumptionMetadata::new("llm-simulator"),
        })
    }

    async fn consume_simulation_outputs_range(
        &self,
        _start_time: &str,
        _end_time: &str,
        _limit: Option<usize>,
    ) -> ConsumerResult<Vec<SimulationOutput>> {
        // Implementation would query the simulator API with time range filters
        Ok(vec![])
    }

    async fn consume_synthetic_evaluations(
        &self,
        evaluation_ids: &[Uuid],
    ) -> ConsumerResult<Vec<SyntheticEvaluationRun>> {
        // Implementation would batch-fetch evaluation runs
        Ok(evaluation_ids
            .iter()
            .map(|id| SyntheticEvaluationRun {
                evaluation_id: *id,
                scenarios: vec![],
                aggregate_results: serde_json::json!({}),
                metadata: ConsumptionMetadata::new("llm-simulator"),
            })
            .collect())
    }

    async fn consume_latest_for_model(
        &self,
        _model_identifier: &str,
        _count: usize,
    ) -> ConsumerResult<Vec<SimulationOutput>> {
        // Implementation would filter by model identifier
        Ok(vec![])
    }

    async fn list_available_simulations(
        &self,
        _namespace: Option<&str>,
        _limit: Option<usize>,
    ) -> ConsumerResult<Vec<Uuid>> {
        // Implementation would list available simulation IDs
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_config_default() {
        let config = SimulatorConfig::default();
        assert!(!config.include_traces);
        assert!(config.namespace.is_none());
    }

    #[test]
    fn test_client_creation() {
        let client = SimulatorClient::with_endpoint("https://simulator.example.com");
        assert_eq!(client.config().base.endpoint, "https://simulator.example.com");
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = SimulatorClient::with_endpoint("https://simulator.example.com");
        let result = client.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_simulation_output_serialization() {
        let output = SimulationOutput {
            simulation_id: Uuid::new_v4(),
            model_config: serde_json::json!({"model": "test"}),
            outputs: vec![],
            performance_metrics: SimulationPerformanceMetrics {
                total_tokens: 1000,
                tokens_per_second: 100.0,
                avg_latency_ms: 50.0,
                p95_latency_ms: 100.0,
                memory_bytes: Some(1024),
            },
            metadata: ConsumptionMetadata::new("llm-simulator"),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }
}
