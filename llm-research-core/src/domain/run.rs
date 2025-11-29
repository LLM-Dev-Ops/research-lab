use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use super::ids::{ArtifactId, ExperimentId, RunId, UserId};
use super::config::ParameterValue;

// ===== Run Status =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Completed
                | RunStatus::Failed
                | RunStatus::Cancelled
                | RunStatus::TimedOut
        )
    }

    pub fn is_running(&self) -> bool {
        matches!(self, RunStatus::Running)
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, RunStatus::Completed)
    }
}

// ===== Environment Snapshot for Reproducibility =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HardwareInfo {
    pub cpu_model: Option<String>,
    pub cpu_cores: Option<u32>,
    pub memory_total_gb: Option<u32>,
    pub gpu_model: Option<String>,
    pub gpu_count: Option<u32>,
    pub gpu_memory_gb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeInfo {
    pub python_version: Option<String>,
    pub cuda_version: Option<String>,
    pub pytorch_version: Option<String>,
    pub tensorflow_version: Option<String>,
    pub transformers_version: Option<String>,
    pub additional: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DependencyManifest {
    pub manifest_type: String, // e.g., "pip", "conda", "cargo", "npm"
    pub content: String,        // requirements.txt, environment.yml, Cargo.lock, etc.
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GitState {
    pub repository_url: Option<String>,
    pub branch: Option<String>,
    pub commit_hash: Option<String>,
    pub is_dirty: bool,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContainerInfo {
    pub image_name: String,
    pub image_tag: String,
    pub image_digest: Option<String>,
    pub dockerfile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentSnapshot {
    pub os: OsInfo,
    pub hardware: HardwareInfo,
    pub runtime: RuntimeInfo,
    pub dependencies: Vec<DependencyManifest>,
    pub git_state: Option<GitState>,
    pub container: Option<ContainerInfo>,
    pub environment_variables: HashMap<String, String>,
    pub captured_at: DateTime<Utc>,
}

// ===== Run Metrics =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScalarMetricPoint {
    pub step: u64,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScalarMetricSeries {
    pub name: String,
    pub points: Vec<ScalarMetricPoint>,
    pub unit: Option<String>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionMetric {
    pub name: String,
    pub step: u64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub stddev: f64,
    pub percentile_25: f64,
    pub percentile_75: f64,
    pub percentile_95: f64,
    pub percentile_99: f64,
    pub count: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConfusionMatrix {
    pub labels: Vec<String>,
    pub matrix: Vec<Vec<u64>>, // 2D matrix: matrix[actual][predicted]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunMetrics {
    pub scalars: Vec<ScalarMetricSeries>,
    pub distributions: Vec<DistributionMetric>,
    pub confusion_matrices: Vec<ConfusionMatrix>,
    pub custom_metrics: HashMap<String, serde_json::Value>,
}

impl Default for RunMetrics {
    fn default() -> Self {
        Self {
            scalars: Vec::new(),
            distributions: Vec::new(),
            confusion_matrices: Vec::new(),
            custom_metrics: HashMap::new(),
        }
    }
}

// ===== Artifact Reference =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub artifact_type: String, // e.g., "model", "dataset", "plot", "logs", "checkpoint"
    pub path: String,
    pub size_bytes: Option<u64>,
    pub checksum: Option<String>,
    pub mime_type: Option<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// ===== Log Summary =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LogHighlight {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LogSummary {
    pub total_lines: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub highlights: Vec<LogHighlight>,
    pub log_artifact_id: Option<ArtifactId>,
}

impl Default for LogSummary {
    fn default() -> Self {
        Self {
            total_lines: 0,
            error_count: 0,
            warning_count: 0,
            highlights: Vec::new(),
            log_artifact_id: None,
        }
    }
}

// ===== Run Error =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunError {
    pub error_type: String,
    pub message: String,
    pub stacktrace: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub is_retryable: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

// ===== Experiment Run =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct ExperimentRun {
    pub id: RunId,
    pub experiment_id: ExperimentId,
    pub run_number: u32,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub status: RunStatus,
    pub parameters: HashMap<String, ParameterValue>,
    pub environment: Option<EnvironmentSnapshot>,
    pub metrics: RunMetrics,
    pub artifacts: Vec<ArtifactRef>,
    pub logs: LogSummary,
    pub parent_run_id: Option<RunId>,
    pub tags: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub error: Option<RunError>,
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExperimentRun {
    pub fn new(
        experiment_id: ExperimentId,
        run_number: u32,
        name: String,
        created_by: UserId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: RunId::new(),
            experiment_id,
            run_number,
            name,
            status: RunStatus::Pending,
            parameters: HashMap::new(),
            environment: None,
            metrics: RunMetrics::default(),
            artifacts: Vec::new(),
            logs: LogSummary::default(),
            parent_run_id: None,
            tags: Vec::new(),
            started_at: None,
            ended_at: None,
            created_at: now,
            created_by,
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_parameters(mut self, parameters: HashMap<String, ParameterValue>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_parent(mut self, parent_id: RunId) -> Self {
        self.parent_run_id = Some(parent_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn start(&mut self) {
        self.status = RunStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn queue(&mut self) {
        self.status = RunStatus::Queued;
    }

    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.ended_at = Some(Utc::now());
    }

    pub fn fail(&mut self, error: RunError) {
        self.status = RunStatus::Failed;
        self.ended_at = Some(Utc::now());
        self.error = Some(error);
    }

    pub fn cancel(&mut self) {
        self.status = RunStatus::Cancelled;
        self.ended_at = Some(Utc::now());
    }

    pub fn timeout(&mut self) {
        self.status = RunStatus::TimedOut;
        self.ended_at = Some(Utc::now());
    }

    pub fn duration_seconds(&self) -> Option<i64> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some((end - start).num_seconds()),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    pub fn is_running(&self) -> bool {
        self.status.is_running()
    }

    pub fn is_successful(&self) -> bool {
        self.status.is_successful()
    }

    pub fn add_artifact(&mut self, artifact: ArtifactRef) {
        self.artifacts.push(artifact);
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn capture_environment(&mut self, environment: EnvironmentSnapshot) {
        self.environment = Some(environment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_status_is_terminal() {
        assert!(RunStatus::Completed.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::Cancelled.is_terminal());
        assert!(RunStatus::TimedOut.is_terminal());
        assert!(!RunStatus::Pending.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
    }

    #[test]
    fn test_run_lifecycle() {
        let experiment_id = ExperimentId::new();
        let user_id = UserId::new();
        let mut run = ExperimentRun::new(
            experiment_id,
            1,
            "Test Run".to_string(),
            user_id,
        );

        assert_eq!(run.status, RunStatus::Pending);
        assert!(!run.is_terminal());

        run.queue();
        assert_eq!(run.status, RunStatus::Queued);

        run.start();
        assert_eq!(run.status, RunStatus::Running);
        assert!(run.started_at.is_some());
        assert!(run.is_running());

        run.complete();
        assert_eq!(run.status, RunStatus::Completed);
        assert!(run.ended_at.is_some());
        assert!(run.is_terminal());
        assert!(run.is_successful());
    }

    #[test]
    fn test_run_with_builder() {
        let experiment_id = ExperimentId::new();
        let user_id = UserId::new();
        let parent_id = RunId::new();

        let mut params = HashMap::new();
        params.insert("learning_rate".to_string(), ParameterValue::from(0.001f64));

        let run = ExperimentRun::new(
            experiment_id,
            1,
            "Test Run".to_string(),
            user_id,
        )
        .with_parameters(params.clone())
        .with_parent(parent_id)
        .with_tags(vec!["test".to_string(), "experiment".to_string()]);

        assert_eq!(run.parameters.len(), 1);
        assert_eq!(run.parent_run_id, Some(parent_id));
        assert_eq!(run.tags.len(), 2);
    }

    #[test]
    fn test_run_duration() {
        let experiment_id = ExperimentId::new();
        let user_id = UserId::new();
        let mut run = ExperimentRun::new(
            experiment_id,
            1,
            "Test Run".to_string(),
            user_id,
        );

        assert!(run.duration_seconds().is_none());

        run.start();
        assert!(run.duration_seconds().is_none()); // No end time yet

        run.complete();
        let duration = run.duration_seconds();
        assert!(duration.is_some());
        assert!(duration.unwrap() >= 0);
    }
}
