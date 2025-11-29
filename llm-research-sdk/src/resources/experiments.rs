//! Experiments resource client
//!
//! This module provides methods for managing experiments.

use crate::client::{HttpClient, PaginatedResponse, PaginationParams};
use crate::error::{SdkError, SdkResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Client for experiment operations
#[derive(Debug, Clone)]
pub struct ExperimentsClient {
    client: Arc<HttpClient>,
}

impl ExperimentsClient {
    /// Create a new experiments client
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Create a new experiment
    pub async fn create(&self, request: CreateExperimentRequest) -> SdkResult<Experiment> {
        self.client.post("/experiments", request).await
    }

    /// Get an experiment by ID
    pub async fn get(&self, id: Uuid) -> SdkResult<Experiment> {
        self.client.get(&format!("/experiments/{}", id)).await
    }

    /// List experiments with optional filtering and pagination
    pub async fn list(
        &self,
        params: Option<ListExperimentsParams>,
    ) -> SdkResult<PaginatedResponse<Experiment>> {
        match params {
            Some(p) => self.client.get_with_query("/experiments", &p).await,
            None => self.client.get("/experiments").await,
        }
    }

    /// Update an experiment
    pub async fn update(
        &self,
        id: Uuid,
        request: UpdateExperimentRequest,
    ) -> SdkResult<Experiment> {
        self.client
            .put(&format!("/experiments/{}", id), request)
            .await
    }

    /// Delete an experiment
    pub async fn delete(&self, id: Uuid) -> SdkResult<()> {
        self.client.delete(&format!("/experiments/{}", id)).await
    }

    /// Start an experiment
    pub async fn start(&self, id: Uuid) -> SdkResult<Experiment> {
        self.client
            .post(&format!("/experiments/{}/start", id), ())
            .await
    }

    /// Create a run for an experiment
    pub async fn create_run(
        &self,
        experiment_id: Uuid,
        request: CreateRunRequest,
    ) -> SdkResult<ExperimentRun> {
        self.client
            .post(&format!("/experiments/{}/runs", experiment_id), request)
            .await
    }

    /// List runs for an experiment
    pub async fn list_runs(
        &self,
        experiment_id: Uuid,
        pagination: Option<PaginationParams>,
    ) -> SdkResult<PaginatedResponse<ExperimentRun>> {
        let path = format!("/experiments/{}/runs", experiment_id);
        match pagination {
            Some(p) => self.client.get_with_query(&path, &p).await,
            None => self.client.get(&path).await,
        }
    }

    /// Complete a run
    pub async fn complete_run(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
    ) -> SdkResult<ExperimentRun> {
        self.client
            .post(
                &format!("/experiments/{}/runs/{}/complete", experiment_id, run_id),
                (),
            )
            .await
    }

    /// Fail a run
    pub async fn fail_run(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        error: impl Into<String>,
    ) -> SdkResult<ExperimentRun> {
        self.client
            .post(
                &format!("/experiments/{}/runs/{}/fail", experiment_id, run_id),
                FailRunRequest {
                    error: error.into(),
                },
            )
            .await
    }

    /// Get metrics for an experiment
    pub async fn get_metrics(&self, experiment_id: Uuid) -> SdkResult<ExperimentMetrics> {
        self.client
            .get(&format!("/experiments/{}/metrics", experiment_id))
            .await
    }
}

/// Request to create a new experiment
#[derive(Debug, Clone, Serialize)]
pub struct CreateExperimentRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis: Option<String>,
    pub owner_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collaborators: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub config: ExperimentConfig,
}

impl CreateExperimentRequest {
    /// Create a new experiment request with minimum required fields
    pub fn new(name: impl Into<String>, owner_id: Uuid, config: ExperimentConfig) -> Self {
        Self {
            name: name.into(),
            description: None,
            hypothesis: None,
            owner_id,
            collaborators: None,
            tags: None,
            config,
        }
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a hypothesis
    pub fn with_hypothesis(mut self, hypothesis: impl Into<String>) -> Self {
        self.hypothesis = Some(hypothesis.into());
        self
    }

    /// Add collaborators
    pub fn with_collaborators(mut self, collaborators: Vec<Uuid>) -> Self {
        self.collaborators = Some(collaborators);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
}

/// Experiment configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentConfig {
    #[serde(default)]
    pub model_ids: Vec<Uuid>,
    #[serde(default)]
    pub dataset_ids: Vec<Uuid>,
    #[serde(default)]
    pub prompt_template_ids: Vec<Uuid>,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub evaluation_metrics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_samples: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_seed: Option<u64>,
}

impl ExperimentConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model_id: Uuid) -> Self {
        self.model_ids.push(model_id);
        self
    }

    pub fn with_dataset(mut self, dataset_id: Uuid) -> Self {
        self.dataset_ids.push(dataset_id);
        self
    }

    pub fn with_prompt_template(mut self, template_id: Uuid) -> Self {
        self.prompt_template_ids.push(template_id);
        self
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    pub fn with_metric(mut self, metric: impl Into<String>) -> Self {
        self.evaluation_metrics.push(metric.into());
        self
    }
}

/// Request to update an experiment
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateExperimentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ExperimentConfig>,
}

impl UpdateExperimentRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_hypothesis(mut self, hypothesis: impl Into<String>) -> Self {
        self.hypothesis = Some(hypothesis.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_config(mut self, config: ExperimentConfig) -> Self {
        self.config = Some(config);
        self
    }
}

/// Parameters for listing experiments
#[derive(Debug, Clone, Serialize, Default)]
pub struct ListExperimentsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

impl ListExperimentsParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_owner(mut self, owner_id: Uuid) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags.join(","));
        self
    }
}

/// Request to create a run
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateRunRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_overrides: Option<serde_json::Value>,
}

impl CreateRunRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_overrides(mut self, overrides: serde_json::Value) -> Self {
        self.config_overrides = Some(overrides);
        self
    }
}

/// Request to fail a run
#[derive(Debug, Clone, Serialize)]
pub struct FailRunRequest {
    pub error: String,
}

/// Experiment entity
#[derive(Debug, Clone, Deserialize)]
pub struct Experiment {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub owner_id: Uuid,
    pub collaborators: Vec<Uuid>,
    pub tags: Vec<String>,
    pub status: ExperimentStatus,
    pub config: ExperimentConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Experiment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Running,
    Completed,
    Failed,
    Archived,
}

impl std::fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

/// Experiment run
#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentRun {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub status: String,
    pub config: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Experiment metrics
#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentMetrics {
    pub experiment_id: Uuid,
    pub aggregated_metrics: HashMap<String, MetricSummary>,
    pub runs: Vec<RunMetrics>,
}

/// Summary statistics for a metric
#[derive(Debug, Clone, Deserialize)]
pub struct MetricSummary {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub count: u64,
}

/// Metrics for a single run
#[derive(Debug, Clone, Deserialize)]
pub struct RunMetrics {
    pub run_id: Uuid,
    pub metrics: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_experiment_request_builder() {
        let owner_id = Uuid::new_v4();
        let config = ExperimentConfig::new()
            .with_metric("accuracy")
            .with_parameter("temperature", serde_json::json!(0.7));

        let request = CreateExperimentRequest::new("Test Experiment", owner_id, config)
            .with_description("A test experiment")
            .with_tags(vec!["test".to_string(), "benchmark".to_string()]);

        assert_eq!(request.name, "Test Experiment");
        assert_eq!(request.description, Some("A test experiment".to_string()));
        assert_eq!(request.tags, Some(vec!["test".to_string(), "benchmark".to_string()]));
    }

    #[test]
    fn test_experiment_config_builder() {
        let model_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();

        let config = ExperimentConfig::new()
            .with_model(model_id)
            .with_dataset(dataset_id)
            .with_metric("accuracy")
            .with_metric("latency")
            .with_parameter("temperature", serde_json::json!(0.5));

        assert_eq!(config.model_ids.len(), 1);
        assert_eq!(config.dataset_ids.len(), 1);
        assert_eq!(config.evaluation_metrics.len(), 2);
        assert!(config.parameters.contains_key("temperature"));
    }

    #[test]
    fn test_list_params_builder() {
        let owner_id = Uuid::new_v4();
        let params = ListExperimentsParams::new()
            .with_limit(10)
            .with_offset(20)
            .with_status("running")
            .with_owner(owner_id)
            .with_tags(vec!["test".to_string(), "prod".to_string()]);

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(20));
        assert_eq!(params.status, Some("running".to_string()));
        assert_eq!(params.owner_id, Some(owner_id));
        assert_eq!(params.tags, Some("test,prod".to_string()));
    }
}
