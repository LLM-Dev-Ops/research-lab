//! Evaluations resource client
//!
//! This module provides methods for managing evaluations and metrics.

use crate::client::{HttpClient, PaginatedResponse, PaginationParams};
use crate::error::SdkResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Client for evaluation operations
#[derive(Debug, Clone)]
pub struct EvaluationsClient {
    client: Arc<HttpClient>,
}

impl EvaluationsClient {
    /// Create a new evaluations client
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Create a new evaluation
    pub async fn create(&self, request: CreateEvaluationRequest) -> SdkResult<Evaluation> {
        self.client.post("/evaluations", request).await
    }

    /// Get an evaluation by ID
    pub async fn get(&self, id: Uuid) -> SdkResult<Evaluation> {
        self.client.get(&format!("/evaluations/{}", id)).await
    }

    /// List evaluations with optional filtering and pagination
    pub async fn list(
        &self,
        params: Option<ListEvaluationsParams>,
    ) -> SdkResult<PaginatedResponse<Evaluation>> {
        match params {
            Some(p) => self.client.get_with_query("/evaluations", &p).await,
            None => self.client.get("/evaluations").await,
        }
    }

    /// Update an evaluation
    pub async fn update(
        &self,
        id: Uuid,
        request: UpdateEvaluationRequest,
    ) -> SdkResult<Evaluation> {
        self.client
            .put(&format!("/evaluations/{}", id), request)
            .await
    }

    /// Delete an evaluation
    pub async fn delete(&self, id: Uuid) -> SdkResult<()> {
        self.client.delete(&format!("/evaluations/{}", id)).await
    }

    /// Run an evaluation
    pub async fn run(&self, id: Uuid, request: RunEvaluationRequest) -> SdkResult<EvaluationRun> {
        self.client
            .post(&format!("/evaluations/{}/run", id), request)
            .await
    }

    /// Get evaluation run status
    pub async fn get_run(&self, evaluation_id: Uuid, run_id: Uuid) -> SdkResult<EvaluationRun> {
        self.client
            .get(&format!("/evaluations/{}/runs/{}", evaluation_id, run_id))
            .await
    }

    /// List runs for an evaluation
    pub async fn list_runs(
        &self,
        evaluation_id: Uuid,
        pagination: Option<PaginationParams>,
    ) -> SdkResult<PaginatedResponse<EvaluationRun>> {
        let path = format!("/evaluations/{}/runs", evaluation_id);
        match pagination {
            Some(p) => self.client.get_with_query(&path, &p).await,
            None => self.client.get(&path).await,
        }
    }

    /// Get results for an evaluation run
    pub async fn get_results(
        &self,
        evaluation_id: Uuid,
        run_id: Uuid,
    ) -> SdkResult<EvaluationResults> {
        self.client
            .get(&format!(
                "/evaluations/{}/runs/{}/results",
                evaluation_id, run_id
            ))
            .await
    }

    /// Submit metric values for an evaluation
    pub async fn submit_metrics(
        &self,
        evaluation_id: Uuid,
        run_id: Uuid,
        request: SubmitMetricsRequest,
    ) -> SdkResult<()> {
        self.client
            .post::<(), _>(
                &format!(
                    "/evaluations/{}/runs/{}/metrics",
                    evaluation_id, run_id
                ),
                request,
            )
            .await
    }

    /// List available metric types
    pub async fn list_metric_types(&self) -> SdkResult<Vec<MetricType>> {
        self.client.get("/evaluations/metric-types").await
    }

    /// Compare multiple evaluation runs
    pub async fn compare(&self, request: CompareEvaluationsRequest) -> SdkResult<ComparisonResult> {
        self.client.post("/evaluations/compare", request).await
    }
}

/// Request to create a new evaluation
#[derive(Debug, Clone, Serialize)]
pub struct CreateEvaluationRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub evaluation_type: EvaluationType,
    pub config: EvaluationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl CreateEvaluationRequest {
    /// Create a new evaluation request
    pub fn new(
        name: impl Into<String>,
        evaluation_type: EvaluationType,
        config: EvaluationConfig,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            evaluation_type,
            config,
            tags: None,
            metadata: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Evaluation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationType {
    /// Automated metric-based evaluation
    Automated,
    /// Human evaluation with judges
    Human,
    /// A/B comparison evaluation
    Comparison,
    /// LLM-as-judge evaluation
    LlmJudge,
    /// Custom evaluation with user-defined metrics
    Custom,
}

impl std::fmt::Display for EvaluationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Automated => write!(f, "automated"),
            Self::Human => write!(f, "human"),
            Self::Comparison => write!(f, "comparison"),
            Self::LlmJudge => write!(f, "llm_judge"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// Evaluation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationConfig {
    #[serde(default)]
    pub metrics: Vec<MetricConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judge_config: Option<JudgeConfig>,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

impl EvaluationConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_metric(mut self, metric: MetricConfig) -> Self {
        self.metrics.push(metric);
        self
    }

    pub fn with_dataset(mut self, dataset_id: Uuid) -> Self {
        self.dataset_id = Some(dataset_id);
        self
    }

    pub fn with_model(mut self, model_id: Uuid) -> Self {
        self.model_ids.get_or_insert_with(Vec::new).push(model_id);
        self
    }

    pub fn with_models(mut self, model_ids: Vec<Uuid>) -> Self {
        self.model_ids = Some(model_ids);
        self
    }

    pub fn with_prompt_template(mut self, prompt_template_id: Uuid) -> Self {
        self.prompt_template_id = Some(prompt_template_id);
        self
    }

    pub fn with_sample_size(mut self, sample_size: u32) -> Self {
        self.sample_size = Some(sample_size);
        self
    }

    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn with_judge(mut self, judge_config: JudgeConfig) -> Self {
        self.judge_config = Some(judge_config);
        self
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }
}

/// Configuration for a specific metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricConfig {
    pub name: String,
    pub metric_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

impl MetricConfig {
    pub fn new(name: impl Into<String>, metric_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metric_type: metric_type.into(),
            weight: None,
            threshold: None,
            parameters: HashMap::new(),
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }
}

/// LLM judge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeConfig {
    pub model_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Vec<JudgeCriterion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<JudgeScale>,
}

impl JudgeConfig {
    pub fn new(model_id: Uuid) -> Self {
        Self {
            model_id,
            prompt_template_id: None,
            criteria: None,
            scale: None,
        }
    }

    pub fn with_prompt_template(mut self, prompt_template_id: Uuid) -> Self {
        self.prompt_template_id = Some(prompt_template_id);
        self
    }

    pub fn with_criteria(mut self, criteria: Vec<JudgeCriterion>) -> Self {
        self.criteria = Some(criteria);
        self
    }

    pub fn with_scale(mut self, scale: JudgeScale) -> Self {
        self.scale = Some(scale);
        self
    }
}

/// Judge evaluation criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeCriterion {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

impl JudgeCriterion {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            weight: None,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }
}

/// Judge rating scale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeScale {
    pub min: i32,
    pub max: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<i32, String>>,
}

impl JudgeScale {
    pub fn new(min: i32, max: i32) -> Self {
        Self {
            min,
            max,
            labels: None,
        }
    }

    pub fn with_labels(mut self, labels: HashMap<i32, String>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Create a 1-5 Likert scale
    pub fn likert_5() -> Self {
        let mut labels = HashMap::new();
        labels.insert(1, "Very Poor".to_string());
        labels.insert(2, "Poor".to_string());
        labels.insert(3, "Average".to_string());
        labels.insert(4, "Good".to_string());
        labels.insert(5, "Excellent".to_string());
        Self::new(1, 5).with_labels(labels)
    }
}

/// Request to update an evaluation
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateEvaluationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<EvaluationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl UpdateEvaluationRequest {
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

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_config(mut self, config: EvaluationConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Parameters for listing evaluations
#[derive(Debug, Clone, Serialize, Default)]
pub struct ListEvaluationsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

impl ListEvaluationsParams {
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

    pub fn with_type(mut self, evaluation_type: EvaluationType) -> Self {
        self.evaluation_type = Some(evaluation_type.to_string());
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags.join(","));
        self
    }
}

/// Request to run an evaluation
#[derive(Debug, Clone, Serialize, Default)]
pub struct RunEvaluationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_overrides: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub async_execution: Option<bool>,
}

impl RunEvaluationRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_overrides(mut self, overrides: serde_json::Value) -> Self {
        self.config_overrides = Some(overrides);
        self
    }

    pub fn async_execution(mut self, async_exec: bool) -> Self {
        self.async_execution = Some(async_exec);
        self
    }
}

/// Request to submit metrics
#[derive(Debug, Clone, Serialize)]
pub struct SubmitMetricsRequest {
    pub metrics: Vec<MetricValue>,
}

impl SubmitMetricsRequest {
    pub fn new(metrics: Vec<MetricValue>) -> Self {
        Self { metrics }
    }
}

/// A metric value submission
#[derive(Debug, Clone, Serialize)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl MetricValue {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            sample_id: None,
            metadata: None,
        }
    }

    pub fn with_sample_id(mut self, sample_id: impl Into<String>) -> Self {
        self.sample_id = Some(sample_id.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Request to compare evaluations
#[derive(Debug, Clone, Serialize)]
pub struct CompareEvaluationsRequest {
    pub run_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistical_tests: Option<Vec<String>>,
}

impl CompareEvaluationsRequest {
    pub fn new(run_ids: Vec<Uuid>) -> Self {
        Self {
            run_ids,
            metrics: None,
            statistical_tests: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Vec<String>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn with_statistical_tests(mut self, tests: Vec<String>) -> Self {
        self.statistical_tests = Some(tests);
        self
    }
}

/// Evaluation entity
#[derive(Debug, Clone, Deserialize)]
pub struct Evaluation {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub evaluation_type: EvaluationType,
    pub config: EvaluationConfig,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub run_count: u32,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Evaluation run entity
#[derive(Debug, Clone, Deserialize)]
pub struct EvaluationRun {
    pub id: Uuid,
    pub evaluation_id: Uuid,
    pub status: RunStatus,
    pub config: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub progress: Option<RunProgress>,
}

/// Run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Run progress information
#[derive(Debug, Clone, Deserialize)]
pub struct RunProgress {
    pub total_samples: u32,
    pub processed_samples: u32,
    pub current_step: Option<String>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// Evaluation results
#[derive(Debug, Clone, Deserialize)]
pub struct EvaluationResults {
    pub run_id: Uuid,
    pub evaluation_id: Uuid,
    pub summary: ResultsSummary,
    pub metrics: HashMap<String, MetricResult>,
    pub samples: Option<Vec<SampleResult>>,
}

/// Summary of evaluation results
#[derive(Debug, Clone, Deserialize)]
pub struct ResultsSummary {
    pub total_samples: u32,
    pub passed_samples: u32,
    pub failed_samples: u32,
    pub overall_score: Option<f64>,
    pub pass_rate: f64,
}

/// Result for a specific metric
#[derive(Debug, Clone, Deserialize)]
pub struct MetricResult {
    pub name: String,
    pub value: f64,
    pub std_dev: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub percentiles: Option<HashMap<String, f64>>,
    pub passed: Option<bool>,
}

/// Result for a single sample
#[derive(Debug, Clone, Deserialize)]
pub struct SampleResult {
    pub sample_id: String,
    pub input: serde_json::Value,
    pub expected_output: Option<serde_json::Value>,
    pub actual_output: serde_json::Value,
    pub metrics: HashMap<String, f64>,
    pub passed: bool,
    pub error: Option<String>,
}

/// Comparison result between evaluation runs
#[derive(Debug, Clone, Deserialize)]
pub struct ComparisonResult {
    pub run_ids: Vec<Uuid>,
    pub metrics_comparison: HashMap<String, MetricComparison>,
    pub statistical_tests: Option<HashMap<String, StatisticalTestResult>>,
    pub winner: Option<Uuid>,
}

/// Comparison of a single metric across runs
#[derive(Debug, Clone, Deserialize)]
pub struct MetricComparison {
    pub values: HashMap<String, f64>, // run_id -> value
    pub best_run_id: Uuid,
    pub improvement: Option<f64>,
}

/// Result of a statistical test
#[derive(Debug, Clone, Deserialize)]
pub struct StatisticalTestResult {
    pub test_name: String,
    pub p_value: f64,
    pub significant: bool,
    pub effect_size: Option<f64>,
}

/// Available metric type
#[derive(Debug, Clone, Deserialize)]
pub struct MetricType {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub parameters: Vec<MetricParameter>,
}

/// Metric parameter definition
#[derive(Debug, Clone, Deserialize)]
pub struct MetricParameter {
    pub name: String,
    pub parameter_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_evaluation_request_builder() {
        let config = EvaluationConfig::new()
            .with_metric(MetricConfig::new("accuracy", "classification_accuracy"))
            .with_metric(
                MetricConfig::new("f1_score", "f1")
                    .with_weight(0.5)
                    .with_threshold(0.8),
            )
            .with_sample_size(1000);

        let request = CreateEvaluationRequest::new("Model Accuracy Test", EvaluationType::Automated, config)
            .with_description("Evaluate model classification accuracy")
            .with_tags(vec!["accuracy".to_string(), "classification".to_string()]);

        assert_eq!(request.name, "Model Accuracy Test");
        assert_eq!(request.evaluation_type, EvaluationType::Automated);
        assert_eq!(request.config.metrics.len(), 2);
    }

    #[test]
    fn test_evaluation_config_builder() {
        let model_id = Uuid::new_v4();
        let dataset_id = Uuid::new_v4();
        let judge_model_id = Uuid::new_v4();

        let config = EvaluationConfig::new()
            .with_dataset(dataset_id)
            .with_model(model_id)
            .with_sample_size(500)
            .with_random_seed(42)
            .with_judge(JudgeConfig::new(judge_model_id).with_scale(JudgeScale::likert_5()))
            .with_parameter("temperature", serde_json::json!(0.7));

        assert_eq!(config.dataset_id, Some(dataset_id));
        assert_eq!(config.model_ids.as_ref().unwrap().len(), 1);
        assert_eq!(config.sample_size, Some(500));
        assert!(config.judge_config.is_some());
    }

    #[test]
    fn test_metric_config_builder() {
        let metric = MetricConfig::new("bleu", "bleu_score")
            .with_weight(0.3)
            .with_threshold(0.7)
            .with_parameter("n_gram", serde_json::json!(4));

        assert_eq!(metric.name, "bleu");
        assert_eq!(metric.weight, Some(0.3));
        assert!(metric.parameters.contains_key("n_gram"));
    }

    #[test]
    fn test_judge_scale_likert() {
        let scale = JudgeScale::likert_5();
        assert_eq!(scale.min, 1);
        assert_eq!(scale.max, 5);
        assert!(scale.labels.is_some());
        assert_eq!(scale.labels.as_ref().unwrap().len(), 5);
    }

    #[test]
    fn test_compare_evaluations_request() {
        let run_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let request = CompareEvaluationsRequest::new(run_ids.clone())
            .with_metrics(vec!["accuracy".to_string(), "f1_score".to_string()])
            .with_statistical_tests(vec!["t_test".to_string(), "wilcoxon".to_string()]);

        assert_eq!(request.run_ids.len(), 2);
        assert_eq!(request.metrics.as_ref().unwrap().len(), 2);
        assert_eq!(request.statistical_tests.as_ref().unwrap().len(), 2);
    }
}
