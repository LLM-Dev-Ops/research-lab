# LLM-Research-Lab Pseudocode - Part 1: Core Data Models & Experiment Tracking

> **SPARC Phase 2: Pseudocode (1 of 3)**
> Part of the LLM DevOps Ecosystem

---

## 1. Core Data Models

### 1.1 Primary Entity Types

```rust
//! Core domain entities for LLM-Research-Lab
//! All types are designed for serialization, validation, and database persistence

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier types with strong typing to prevent mixing IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExperimentId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetVersionId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowRunId(Uuid);

/// Content-addressable hash for artifact integrity
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String); // SHA-256 hex string

impl ContentHash {
    pub fn from_bytes(data: &[u8]) -> Self {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(hex::encode(hasher.finalize()))
    }

    pub fn from_stream<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(Self(hex::encode(hasher.finalize())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Semantic version for metrics, datasets, and artifacts
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

impl SemanticVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build_metadata: None,
        }
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        // Major version must match for compatibility
        self.major == other.major
    }

    pub fn increment_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            prerelease: None,
            build_metadata: None,
        }
    }

    pub fn increment_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            prerelease: None,
            build_metadata: None,
        }
    }

    pub fn increment_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            prerelease: None,
            build_metadata: None,
        }
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.prerelease {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}
```

### 1.2 Experiment Domain

```rust
/// Core experiment entity representing a research initiative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: ExperimentId,
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub owner_id: UserId,
    pub collaborators: Vec<UserId>,
    pub tags: Vec<String>,
    pub status: ExperimentStatus,
    pub config: ExperimentConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Experiment lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Archived,
    Failed,
}

impl ExperimentStatus {
    pub fn can_transition_to(&self, target: ExperimentStatus) -> bool {
        use ExperimentStatus::*;
        match (self, target) {
            (Draft, Active) => true,
            (Draft, Archived) => true,
            (Active, Paused) => true,
            (Active, Completed) => true,
            (Active, Failed) => true,
            (Paused, Active) => true,
            (Paused, Archived) => true,
            (Completed, Archived) => true,
            (Failed, Archived) => true,
            _ => false,
        }
    }
}

/// Configuration for experiment execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub model_configs: Vec<ModelConfig>,
    pub dataset_refs: Vec<DatasetRef>,
    pub metric_configs: Vec<MetricConfig>,
    pub parameters: ExperimentParameters,
    pub resource_requirements: ResourceRequirements,
    pub reproducibility_settings: ReproducibilitySettings,
}

/// Model configuration for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_id: String,
    pub provider: ModelProvider,
    pub variant: Option<String>,
    pub endpoint: Option<String>,
    pub parameters: ModelParameters,
    pub credentials_ref: Option<String>, // Reference to secrets manager
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    OpenAI,
    Anthropic,
    Google,
    Cohere,
    HuggingFace,
    AzureOpenAI,
    AWS Bedrock,
    Custom { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop_sequences: Vec<String>,
    pub seed: Option<u64>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Reference to a versioned dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRef {
    pub dataset_id: DatasetId,
    pub version: DatasetVersionSelector,
    pub split: Option<DataSplit>,
    pub sample_config: Option<SampleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetVersionSelector {
    Latest,
    Specific(DatasetVersionId),
    Tag(String),
    ContentHash(ContentHash),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSplit {
    Train,
    Validation,
    Test,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleConfig {
    pub strategy: SampleStrategy,
    pub size: SampleSize,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleStrategy {
    Random,
    Stratified { column: String },
    Systematic { interval: usize },
    Reservoir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleSize {
    Count(usize),
    Percentage(f64),
    All,
}

/// Configuration for metric evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricConfig {
    pub metric_id: MetricId,
    pub version: SemanticVersion,
    pub parameters: HashMap<String, serde_json::Value>,
    pub weight: Option<f64>, // For composite scores
    pub threshold: Option<MetricThreshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThreshold {
    pub warning: Option<f64>,
    pub critical: Option<f64>,
    pub direction: ThresholdDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdDirection {
    HigherIsBetter,
    LowerIsBetter,
}

/// Experiment-level parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentParameters {
    pub hyperparameters: HashMap<String, ParameterValue>,
    pub search_space: Option<SearchSpace>,
    pub optimization_target: Option<OptimizationTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<ParameterValue>),
    Map(HashMap<String, ParameterValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSpace {
    pub parameters: HashMap<String, ParameterSearchConfig>,
    pub strategy: SearchStrategy,
    pub max_trials: usize,
    pub early_stopping: Option<EarlyStoppingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterSearchConfig {
    Categorical { values: Vec<serde_json::Value> },
    Uniform { low: f64, high: f64 },
    LogUniform { low: f64, high: f64 },
    IntUniform { low: i64, high: i64 },
    Normal { mean: f64, std: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStrategy {
    Grid,
    Random,
    Bayesian,
    Hyperband,
    Population Based,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlyStoppingConfig {
    pub metric: String,
    pub patience: usize,
    pub min_delta: f64,
    pub mode: ThresholdDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationTarget {
    pub metric: String,
    pub direction: ThresholdDirection,
    pub constraints: Vec<OptimizationConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConstraint {
    pub metric: String,
    pub operator: ConstraintOperator,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
}

/// Resource requirements for experiment execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub compute: ComputeRequirements,
    pub storage: StorageRequirements,
    pub timeout: Option<std::time::Duration>,
    pub priority: ExecutionPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequirements {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
    pub gpu: Option<GpuRequirements>,
    pub node_selector: HashMap<String, String>,
    pub tolerations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    pub count: u32,
    pub gpu_type: Option<String>, // e.g., "nvidia-a100", "nvidia-v100"
    pub memory_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    pub scratch_space_mb: u64,
    pub artifact_retention_days: Option<u32>,
    pub checkpoint_frequency: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Reproducibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilitySettings {
    pub capture_environment: bool,
    pub pin_dependencies: bool,
    pub deterministic_mode: bool,
    pub global_seed: Option<u64>,
    pub checkpoint_artifacts: bool,
    pub record_system_info: bool,
}

impl Default for ReproducibilitySettings {
    fn default() -> Self {
        Self {
            capture_environment: true,
            pin_dependencies: true,
            deterministic_mode: true,
            global_seed: None,
            checkpoint_artifacts: true,
            record_system_info: true,
        }
    }
}
```

### 1.3 Experiment Run Domain

```rust
/// Individual execution of an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRun {
    pub id: RunId,
    pub experiment_id: ExperimentId,
    pub run_number: u64,
    pub name: Option<String>,
    pub status: RunStatus,
    pub parameters: HashMap<String, ParameterValue>,
    pub environment: EnvironmentSnapshot,
    pub metrics: RunMetrics,
    pub artifacts: Vec<ArtifactRef>,
    pub logs: LogSummary,
    pub parent_run_id: Option<RunId>, // For nested/child runs
    pub tags: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub error: Option<RunError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled | RunStatus::TimedOut
        )
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, RunStatus::Completed)
    }
}

/// Complete environment snapshot for reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub os: OsInfo,
    pub hardware: HardwareInfo,
    pub runtime: RuntimeInfo,
    pub dependencies: DependencyManifest,
    pub environment_variables: HashMap<String, String>, // Filtered for safety
    pub git_state: Option<GitState>,
    pub container_info: Option<ContainerInfo>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub kernel_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub memory_total_mb: u64,
    pub gpus: Vec<GpuInfo>,
    pub hostname_hash: String, // Hashed for privacy
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub memory_mb: u64,
    pub driver_version: String,
    pub cuda_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub rust_version: Option<String>,
    pub python_version: Option<String>,
    pub cuda_version: Option<String>,
    pub custom_runtimes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyManifest {
    pub format: DependencyFormat,
    pub content_hash: ContentHash,
    pub packages: Vec<PackageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyFormat {
    CargoLock,
    PipFreeze,
    CondaEnvironment,
    NpmPackageLock,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    pub repository_url: Option<String>,
    pub branch: String,
    pub commit_hash: String,
    pub is_dirty: bool,
    pub uncommitted_changes: Option<String>, // Diff summary
    pub remote_tracking: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub image: String,
    pub image_digest: String,
    pub runtime: String, // docker, containerd, etc.
    pub resource_limits: Option<ContainerResourceLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceLimits {
    pub cpu_limit: Option<f64>,
    pub memory_limit_mb: Option<u64>,
    pub gpu_count: Option<u32>,
}

/// Collected metrics during a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetrics {
    pub scalars: HashMap<String, ScalarMetricSeries>,
    pub distributions: HashMap<String, DistributionMetric>,
    pub confusion_matrices: HashMap<String, ConfusionMatrix>,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarMetricSeries {
    pub values: Vec<ScalarDataPoint>,
    pub aggregations: MetricAggregations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarDataPoint {
    pub value: f64,
    pub step: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricAggregations {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetric {
    pub histogram: Histogram,
    pub summary: DistributionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub bucket_boundaries: Vec<f64>,
    pub bucket_counts: Vec<u64>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionSummary {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
    pub percentiles: HashMap<String, f64>, // "p50", "p90", "p95", "p99"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfusionMatrix {
    pub labels: Vec<String>,
    pub matrix: Vec<Vec<u64>>,
    pub normalized: Option<Vec<Vec<f64>>>,
}

/// Reference to stored artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub name: String,
    pub artifact_type: ArtifactType,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub storage_uri: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Model,
    Checkpoint,
    Dataset,
    Visualization,
    Log,
    Config,
    Report,
    Custom(String),
}

/// Log summary for a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSummary {
    pub total_lines: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub log_uri: String,
    pub highlights: Vec<LogHighlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogHighlight {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub line_number: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

/// Error information for failed runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunError {
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub recoverable: bool,
    pub context: HashMap<String, String>,
}
```

---

## 2. Experiment Tracking System

### 2.1 Experiment Tracker Core

```rust
//! Experiment tracking system providing full lifecycle management
//! with reproducibility guarantees and artifact persistence

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main experiment tracker service
pub struct ExperimentTracker {
    experiment_store: Arc<dyn ExperimentStore>,
    run_store: Arc<dyn RunStore>,
    artifact_store: Arc<dyn ArtifactStore>,
    metric_store: Arc<dyn MetricStore>,
    environment_capture: Arc<EnvironmentCapture>,
    lineage_tracker: Arc<LineageTracker>,
    event_publisher: Arc<dyn EventPublisher>,
    config: TrackerConfig,
}

#[derive(Debug, Clone)]
pub struct TrackerConfig {
    pub auto_capture_environment: bool,
    pub auto_checkpoint: bool,
    pub checkpoint_interval: std::time::Duration,
    pub metric_buffer_size: usize,
    pub max_artifact_size_bytes: u64,
    pub compression_enabled: bool,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            auto_capture_environment: true,
            auto_checkpoint: true,
            checkpoint_interval: std::time::Duration::from_secs(300),
            metric_buffer_size: 1000,
            max_artifact_size_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            compression_enabled: true,
        }
    }
}

impl ExperimentTracker {
    /// Create a new experiment tracker with the given dependencies
    pub fn new(
        experiment_store: Arc<dyn ExperimentStore>,
        run_store: Arc<dyn RunStore>,
        artifact_store: Arc<dyn ArtifactStore>,
        metric_store: Arc<dyn MetricStore>,
        environment_capture: Arc<EnvironmentCapture>,
        lineage_tracker: Arc<LineageTracker>,
        event_publisher: Arc<dyn EventPublisher>,
        config: TrackerConfig,
    ) -> Self {
        Self {
            experiment_store,
            run_store,
            artifact_store,
            metric_store,
            environment_capture,
            lineage_tracker,
            event_publisher,
            config,
        }
    }

    /// Create a new experiment
    pub async fn create_experiment(
        &self,
        request: CreateExperimentRequest,
    ) -> Result<Experiment, TrackerError> {
        // Validate request
        self.validate_experiment_request(&request)?;

        // Generate unique ID
        let id = ExperimentId(Uuid::new_v4());
        let now = Utc::now();

        // Build experiment entity
        let experiment = Experiment {
            id,
            name: request.name,
            description: request.description,
            hypothesis: request.hypothesis,
            owner_id: request.owner_id,
            collaborators: request.collaborators.unwrap_or_default(),
            tags: request.tags.unwrap_or_default(),
            status: ExperimentStatus::Draft,
            config: request.config,
            created_at: now,
            updated_at: now,
            archived_at: None,
            metadata: request.metadata.unwrap_or_default(),
        };

        // Persist experiment
        self.experiment_store.save(&experiment).await?;

        // Record lineage
        self.lineage_tracker
            .record_experiment_creation(id, &experiment)
            .await?;

        // Publish event
        self.event_publisher
            .publish(Event::ExperimentCreated {
                experiment_id: id,
                owner_id: experiment.owner_id,
                timestamp: now,
            })
            .await?;

        Ok(experiment)
    }

    /// Start a new run for an experiment
    pub async fn start_run(
        &self,
        experiment_id: ExperimentId,
        request: StartRunRequest,
    ) -> Result<ExperimentRun, TrackerError> {
        // Verify experiment exists and is active
        let experiment = self.experiment_store.get(experiment_id).await?;
        if experiment.status != ExperimentStatus::Active {
            return Err(TrackerError::InvalidState {
                message: format!(
                    "Experiment must be active to start runs, current status: {:?}",
                    experiment.status
                ),
            });
        }

        // Generate run ID and number
        let run_id = RunId(Uuid::new_v4());
        let run_number = self.run_store.get_next_run_number(experiment_id).await?;
        let now = Utc::now();

        // Capture environment if enabled
        let environment = if self.config.auto_capture_environment {
            self.environment_capture.capture().await?
        } else {
            request.environment.unwrap_or_else(|| {
                EnvironmentSnapshot::minimal()
            })
        };

        // Build run entity
        let run = ExperimentRun {
            id: run_id,
            experiment_id,
            run_number,
            name: request.name,
            status: RunStatus::Running,
            parameters: request.parameters,
            environment,
            metrics: RunMetrics::default(),
            artifacts: Vec::new(),
            logs: LogSummary::default(),
            parent_run_id: request.parent_run_id,
            tags: request.tags.unwrap_or_default(),
            started_at: Some(now),
            ended_at: None,
            created_at: now,
            created_by: request.user_id,
            error: None,
        };

        // Persist run
        self.run_store.save(&run).await?;

        // Record lineage
        self.lineage_tracker
            .record_run_start(run_id, experiment_id, &run)
            .await?;

        // Publish event
        self.event_publisher
            .publish(Event::RunStarted {
                run_id,
                experiment_id,
                timestamp: now,
            })
            .await?;

        Ok(run)
    }

    /// Log scalar metrics for a run
    pub async fn log_metrics(
        &self,
        run_id: RunId,
        metrics: Vec<MetricEntry>,
    ) -> Result<(), TrackerError> {
        // Validate run is active
        let run = self.run_store.get(run_id).await?;
        if run.status != RunStatus::Running {
            return Err(TrackerError::InvalidState {
                message: "Cannot log metrics to a non-running run".to_string(),
            });
        }

        let timestamp = Utc::now();

        // Process each metric
        for entry in metrics {
            let data_point = ScalarDataPoint {
                value: entry.value,
                step: entry.step,
                timestamp,
                context: entry.context.unwrap_or_default(),
            };

            self.metric_store
                .append_scalar(run_id, &entry.name, data_point)
                .await?;
        }

        // Publish metrics event (batched)
        self.event_publisher
            .publish(Event::MetricsLogged {
                run_id,
                count: metrics.len(),
                timestamp,
            })
            .await?;

        Ok(())
    }

    /// Log an artifact for a run
    pub async fn log_artifact(
        &self,
        run_id: RunId,
        request: LogArtifactRequest,
    ) -> Result<ArtifactRef, TrackerError> {
        // Validate run is active
        let run = self.run_store.get(run_id).await?;
        if run.status != RunStatus::Running {
            return Err(TrackerError::InvalidState {
                message: "Cannot log artifacts to a non-running run".to_string(),
            });
        }

        // Validate artifact size
        if request.data.len() as u64 > self.config.max_artifact_size_bytes {
            return Err(TrackerError::ArtifactTooLarge {
                size: request.data.len() as u64,
                max_size: self.config.max_artifact_size_bytes,
            });
        }

        // Compute content hash
        let content_hash = ContentHash::from_bytes(&request.data);

        // Compress if enabled
        let (storage_data, compressed) = if self.config.compression_enabled {
            (compress_data(&request.data)?, true)
        } else {
            (request.data.clone(), false)
        };

        // Store artifact
        let artifact_id = ArtifactId(Uuid::new_v4());
        let storage_uri = self.artifact_store
            .store(artifact_id, &storage_data, compressed)
            .await?;

        let now = Utc::now();
        let artifact_ref = ArtifactRef {
            id: artifact_id,
            name: request.name,
            artifact_type: request.artifact_type,
            content_hash,
            size_bytes: request.data.len() as u64,
            storage_uri,
            metadata: request.metadata.unwrap_or_default(),
            created_at: now,
        };

        // Update run with artifact reference
        self.run_store
            .add_artifact(run_id, artifact_ref.clone())
            .await?;

        // Record lineage
        self.lineage_tracker
            .record_artifact(run_id, &artifact_ref)
            .await?;

        // Publish event
        self.event_publisher
            .publish(Event::ArtifactLogged {
                run_id,
                artifact_id,
                artifact_type: artifact_ref.artifact_type.clone(),
                timestamp: now,
            })
            .await?;

        Ok(artifact_ref)
    }

    /// Complete a run successfully
    pub async fn complete_run(
        &self,
        run_id: RunId,
        final_metrics: Option<HashMap<String, f64>>,
    ) -> Result<ExperimentRun, TrackerError> {
        let mut run = self.run_store.get(run_id).await?;

        if run.status != RunStatus::Running {
            return Err(TrackerError::InvalidState {
                message: format!(
                    "Run must be running to complete, current status: {:?}",
                    run.status
                ),
            });
        }

        let now = Utc::now();
        run.status = RunStatus::Completed;
        run.ended_at = Some(now);

        // Log final metrics if provided
        if let Some(metrics) = final_metrics {
            let entries: Vec<_> = metrics
                .into_iter()
                .map(|(name, value)| MetricEntry {
                    name,
                    value,
                    step: None,
                    context: None,
                })
                .collect();

            // Don't fail completion if metric logging fails
            if let Err(e) = self.log_metrics_internal(run_id, entries).await {
                tracing::warn!("Failed to log final metrics: {}", e);
            }
        }

        // Compute final aggregations
        run.metrics = self.metric_store.compute_aggregations(run_id).await?;

        // Persist updated run
        self.run_store.save(&run).await?;

        // Record lineage
        self.lineage_tracker.record_run_completion(run_id, &run).await?;

        // Publish event
        self.event_publisher
            .publish(Event::RunCompleted {
                run_id,
                experiment_id: run.experiment_id,
                status: RunStatus::Completed,
                duration: run.ended_at.unwrap() - run.started_at.unwrap(),
                timestamp: now,
            })
            .await?;

        Ok(run)
    }

    /// Fail a run with error information
    pub async fn fail_run(
        &self,
        run_id: RunId,
        error: RunError,
    ) -> Result<ExperimentRun, TrackerError> {
        let mut run = self.run_store.get(run_id).await?;

        if run.status.is_terminal() {
            return Err(TrackerError::InvalidState {
                message: format!("Run is already in terminal state: {:?}", run.status),
            });
        }

        let now = Utc::now();
        run.status = RunStatus::Failed;
        run.ended_at = Some(now);
        run.error = Some(error);

        // Persist updated run
        self.run_store.save(&run).await?;

        // Record lineage
        self.lineage_tracker.record_run_failure(run_id, &run).await?;

        // Publish event
        self.event_publisher
            .publish(Event::RunFailed {
                run_id,
                experiment_id: run.experiment_id,
                error_type: run.error.as_ref().map(|e| e.error_type.clone()),
                timestamp: now,
            })
            .await?;

        Ok(run)
    }

    /// Compare multiple runs
    pub async fn compare_runs(
        &self,
        run_ids: Vec<RunId>,
        comparison_config: ComparisonConfig,
    ) -> Result<RunComparison, TrackerError> {
        if run_ids.len() < 2 {
            return Err(TrackerError::ValidationError {
                message: "At least 2 runs required for comparison".to_string(),
            });
        }

        // Load all runs
        let mut runs = Vec::new();
        for run_id in &run_ids {
            let run = self.run_store.get(*run_id).await?;
            runs.push(run);
        }

        // Verify all runs belong to the same experiment if required
        if comparison_config.same_experiment_only {
            let experiment_id = runs[0].experiment_id;
            for run in &runs[1..] {
                if run.experiment_id != experiment_id {
                    return Err(TrackerError::ValidationError {
                        message: "All runs must belong to the same experiment".to_string(),
                    });
                }
            }
        }

        // Build comparison
        let mut comparison = RunComparison {
            run_ids: run_ids.clone(),
            metric_comparisons: HashMap::new(),
            parameter_diff: HashMap::new(),
            environment_diff: None,
            statistical_tests: HashMap::new(),
            generated_at: Utc::now(),
        };

        // Compare metrics
        let metric_names: std::collections::HashSet<_> = runs
            .iter()
            .flat_map(|r| r.metrics.scalars.keys())
            .collect();

        for metric_name in metric_names {
            let values: Vec<_> = runs
                .iter()
                .map(|r| {
                    r.metrics
                        .scalars
                        .get(metric_name)
                        .map(|s| s.aggregations.mean)
                })
                .collect();

            comparison.metric_comparisons.insert(
                metric_name.clone(),
                MetricComparison {
                    metric_name: metric_name.clone(),
                    values: values.clone(),
                    best_run_index: find_best_index(&values, comparison_config.higher_is_better),
                    improvement_percentages: calculate_improvements(&values),
                },
            );

            // Run statistical test if enough data
            if comparison_config.run_statistical_tests && values.iter().all(|v| v.is_some()) {
                if let Some(test_result) = run_statistical_test(
                    &runs,
                    metric_name,
                    &comparison_config,
                ).await? {
                    comparison.statistical_tests.insert(metric_name.clone(), test_result);
                }
            }
        }

        // Compare parameters
        comparison.parameter_diff = compute_parameter_diff(&runs);

        // Compare environments if requested
        if comparison_config.compare_environments {
            comparison.environment_diff = Some(compute_environment_diff(&runs));
        }

        Ok(comparison)
    }

    /// Query runs with filtering and pagination
    pub async fn query_runs(
        &self,
        query: RunQuery,
    ) -> Result<QueryResult<ExperimentRun>, TrackerError> {
        self.run_store.query(query).await
    }

    /// Get experiment lineage graph
    pub async fn get_lineage(
        &self,
        experiment_id: ExperimentId,
    ) -> Result<LineageGraph, TrackerError> {
        self.lineage_tracker.get_experiment_lineage(experiment_id).await
    }

    // Private helper methods

    fn validate_experiment_request(
        &self,
        request: &CreateExperimentRequest,
    ) -> Result<(), TrackerError> {
        if request.name.is_empty() {
            return Err(TrackerError::ValidationError {
                message: "Experiment name cannot be empty".to_string(),
            });
        }

        if request.name.len() > 256 {
            return Err(TrackerError::ValidationError {
                message: "Experiment name cannot exceed 256 characters".to_string(),
            });
        }

        // Validate model configs
        for model_config in &request.config.model_configs {
            if model_config.model_id.is_empty() {
                return Err(TrackerError::ValidationError {
                    message: "Model ID cannot be empty".to_string(),
                });
            }
        }

        // Validate dataset refs
        for dataset_ref in &request.config.dataset_refs {
            // Additional validation could check if dataset exists
        }

        Ok(())
    }

    async fn log_metrics_internal(
        &self,
        run_id: RunId,
        metrics: Vec<MetricEntry>,
    ) -> Result<(), TrackerError> {
        let timestamp = Utc::now();
        for entry in metrics {
            let data_point = ScalarDataPoint {
                value: entry.value,
                step: entry.step,
                timestamp,
                context: entry.context.unwrap_or_default(),
            };
            self.metric_store
                .append_scalar(run_id, &entry.name, data_point)
                .await?;
        }
        Ok(())
    }
}

/// Request types for experiment tracker operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExperimentRequest {
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub owner_id: UserId,
    pub collaborators: Option<Vec<UserId>>,
    pub tags: Option<Vec<String>>,
    pub config: ExperimentConfig,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunRequest {
    pub user_id: UserId,
    pub name: Option<String>,
    pub parameters: HashMap<String, ParameterValue>,
    pub tags: Option<Vec<String>>,
    pub parent_run_id: Option<RunId>,
    pub environment: Option<EnvironmentSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEntry {
    pub name: String,
    pub value: f64,
    pub step: Option<u64>,
    pub context: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogArtifactRequest {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub data: Vec<u8>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonConfig {
    pub same_experiment_only: bool,
    pub higher_is_better: bool,
    pub compare_environments: bool,
    pub run_statistical_tests: bool,
    pub significance_level: f64,
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            same_experiment_only: true,
            higher_is_better: true,
            compare_environments: false,
            run_statistical_tests: true,
            significance_level: 0.05,
        }
    }
}

/// Comparison result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunComparison {
    pub run_ids: Vec<RunId>,
    pub metric_comparisons: HashMap<String, MetricComparison>,
    pub parameter_diff: HashMap<String, ParameterDiff>,
    pub environment_diff: Option<EnvironmentDiff>,
    pub statistical_tests: HashMap<String, StatisticalTestResult>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    pub metric_name: String,
    pub values: Vec<Option<f64>>,
    pub best_run_index: Option<usize>,
    pub improvement_percentages: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDiff {
    pub parameter_name: String,
    pub values: Vec<Option<ParameterValue>>,
    pub is_different: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDiff {
    pub os_differences: Vec<String>,
    pub hardware_differences: Vec<String>,
    pub dependency_differences: Vec<DependencyDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDiff {
    pub package: String,
    pub versions: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTestResult {
    pub test_type: StatisticalTest,
    pub statistic: f64,
    pub p_value: f64,
    pub significant: bool,
    pub effect_size: Option<f64>,
    pub confidence_interval: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticalTest {
    TTest,
    WilcoxonRankSum,
    Anova,
    KruskalWallis,
    PairedTTest,
    BootstrapComparison,
}
```

### 2.2 Storage Traits

```rust
/// Storage trait for experiments
#[async_trait]
pub trait ExperimentStore: Send + Sync {
    async fn save(&self, experiment: &Experiment) -> Result<(), TrackerError>;
    async fn get(&self, id: ExperimentId) -> Result<Experiment, TrackerError>;
    async fn update(&self, experiment: &Experiment) -> Result<(), TrackerError>;
    async fn delete(&self, id: ExperimentId) -> Result<(), TrackerError>;
    async fn query(&self, query: ExperimentQuery) -> Result<QueryResult<Experiment>, TrackerError>;
    async fn exists(&self, id: ExperimentId) -> Result<bool, TrackerError>;
}

/// Storage trait for experiment runs
#[async_trait]
pub trait RunStore: Send + Sync {
    async fn save(&self, run: &ExperimentRun) -> Result<(), TrackerError>;
    async fn get(&self, id: RunId) -> Result<ExperimentRun, TrackerError>;
    async fn update(&self, run: &ExperimentRun) -> Result<(), TrackerError>;
    async fn delete(&self, id: RunId) -> Result<(), TrackerError>;
    async fn query(&self, query: RunQuery) -> Result<QueryResult<ExperimentRun>, TrackerError>;
    async fn get_next_run_number(&self, experiment_id: ExperimentId) -> Result<u64, TrackerError>;
    async fn add_artifact(&self, run_id: RunId, artifact: ArtifactRef) -> Result<(), TrackerError>;
    async fn get_runs_for_experiment(
        &self,
        experiment_id: ExperimentId,
    ) -> Result<Vec<ExperimentRun>, TrackerError>;
}

/// Storage trait for artifacts
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    async fn store(
        &self,
        id: ArtifactId,
        data: &[u8],
        compressed: bool,
    ) -> Result<String, TrackerError>; // Returns storage URI

    async fn retrieve(&self, uri: &str) -> Result<Vec<u8>, TrackerError>;
    async fn delete(&self, uri: &str) -> Result<(), TrackerError>;
    async fn exists(&self, uri: &str) -> Result<bool, TrackerError>;
    async fn get_metadata(&self, uri: &str) -> Result<ArtifactMetadata, TrackerError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub size_bytes: u64,
    pub content_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub checksum: String,
}

/// Storage trait for metrics time series
#[async_trait]
pub trait MetricStore: Send + Sync {
    async fn append_scalar(
        &self,
        run_id: RunId,
        metric_name: &str,
        data_point: ScalarDataPoint,
    ) -> Result<(), TrackerError>;

    async fn get_scalar_series(
        &self,
        run_id: RunId,
        metric_name: &str,
    ) -> Result<ScalarMetricSeries, TrackerError>;

    async fn compute_aggregations(&self, run_id: RunId) -> Result<RunMetrics, TrackerError>;

    async fn query_metrics(
        &self,
        run_id: RunId,
        metric_names: Option<Vec<String>>,
        time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> Result<HashMap<String, ScalarMetricSeries>, TrackerError>;
}

/// Query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentQuery {
    pub owner_id: Option<UserId>,
    pub status: Option<Vec<ExperimentStatus>>,
    pub tags: Option<Vec<String>>,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub pagination: Pagination,
    pub sort: Sort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunQuery {
    pub experiment_id: Option<ExperimentId>,
    pub status: Option<Vec<RunStatus>>,
    pub tags: Option<Vec<String>>,
    pub created_by: Option<UserId>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
    pub parameter_filters: HashMap<String, ParameterFilter>,
    pub metric_filters: HashMap<String, MetricFilter>,
    pub pagination: Pagination,
    pub sort: Sort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: u64,
    pub limit: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sort {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterFilter {
    Equals(ParameterValue),
    In(Vec<ParameterValue>),
    Range { min: Option<f64>, max: Option<f64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricFilter {
    GreaterThan(f64),
    LessThan(f64),
    Between { min: f64, max: f64 },
    Exists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    pub items: Vec<T>,
    pub total_count: u64,
    pub has_more: bool,
    pub next_offset: Option<u64>,
}
```

### 2.3 Environment Capture

```rust
/// Environment capture service for reproducibility
pub struct EnvironmentCapture {
    config: EnvironmentCaptureConfig,
}

#[derive(Debug, Clone)]
pub struct EnvironmentCaptureConfig {
    pub capture_env_vars: bool,
    pub env_var_allowlist: Option<Vec<String>>,
    pub env_var_blocklist: Vec<String>,
    pub capture_git: bool,
    pub capture_dependencies: bool,
    pub capture_hardware: bool,
}

impl Default for EnvironmentCaptureConfig {
    fn default() -> Self {
        Self {
            capture_env_vars: true,
            env_var_allowlist: None,
            env_var_blocklist: vec![
                "API_KEY".to_string(),
                "SECRET".to_string(),
                "PASSWORD".to_string(),
                "TOKEN".to_string(),
                "CREDENTIAL".to_string(),
                "AWS_".to_string(),
                "AZURE_".to_string(),
                "GCP_".to_string(),
            ],
            capture_git: true,
            capture_dependencies: true,
            capture_hardware: true,
        }
    }
}

impl EnvironmentCapture {
    pub fn new(config: EnvironmentCaptureConfig) -> Self {
        Self { config }
    }

    /// Capture complete environment snapshot
    pub async fn capture(&self) -> Result<EnvironmentSnapshot, TrackerError> {
        let os = self.capture_os_info()?;
        let hardware = if self.config.capture_hardware {
            self.capture_hardware_info().await?
        } else {
            HardwareInfo::minimal()
        };
        let runtime = self.capture_runtime_info()?;
        let dependencies = if self.config.capture_dependencies {
            self.capture_dependencies().await?
        } else {
            DependencyManifest::empty()
        };
        let environment_variables = if self.config.capture_env_vars {
            self.capture_env_vars()?
        } else {
            HashMap::new()
        };
        let git_state = if self.config.capture_git {
            self.capture_git_state().await.ok()
        } else {
            None
        };
        let container_info = self.detect_container_info().await.ok();

        Ok(EnvironmentSnapshot {
            os,
            hardware,
            runtime,
            dependencies,
            environment_variables,
            git_state,
            container_info,
            captured_at: Utc::now(),
        })
    }

    fn capture_os_info(&self) -> Result<OsInfo, TrackerError> {
        Ok(OsInfo {
            name: std::env::consts::OS.to_string(),
            version: get_os_version()?,
            architecture: std::env::consts::ARCH.to_string(),
            kernel_version: get_kernel_version().ok(),
        })
    }

    async fn capture_hardware_info(&self) -> Result<HardwareInfo, TrackerError> {
        let cpu_info = get_cpu_info()?;
        let memory_info = get_memory_info()?;
        let gpu_info = detect_gpus().await?;
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        Ok(HardwareInfo {
            cpu_model: cpu_info.model,
            cpu_cores: cpu_info.cores,
            cpu_threads: cpu_info.threads,
            memory_total_mb: memory_info.total_mb,
            gpus: gpu_info,
            hostname_hash: hash_string(&hostname),
        })
    }

    fn capture_runtime_info(&self) -> Result<RuntimeInfo, TrackerError> {
        Ok(RuntimeInfo {
            rust_version: option_env!("RUSTC_VERSION").map(String::from),
            python_version: detect_python_version().ok(),
            cuda_version: detect_cuda_version().ok(),
            custom_runtimes: detect_custom_runtimes(),
        })
    }

    async fn capture_dependencies(&self) -> Result<DependencyManifest, TrackerError> {
        // Try multiple dependency formats in order of preference
        if let Ok(manifest) = self.capture_cargo_lock().await {
            return Ok(manifest);
        }
        if let Ok(manifest) = self.capture_pip_freeze().await {
            return Ok(manifest);
        }
        if let Ok(manifest) = self.capture_conda_env().await {
            return Ok(manifest);
        }

        Ok(DependencyManifest::empty())
    }

    async fn capture_cargo_lock(&self) -> Result<DependencyManifest, TrackerError> {
        let cargo_lock_path = std::path::Path::new("Cargo.lock");
        if !cargo_lock_path.exists() {
            return Err(TrackerError::NotFound {
                message: "Cargo.lock not found".to_string(),
            });
        }

        let content = tokio::fs::read_to_string(cargo_lock_path).await?;
        let lockfile: toml::Value = toml::from_str(&content)?;

        let packages = lockfile
            .get("package")
            .and_then(|p| p.as_array())
            .map(|packages| {
                packages
                    .iter()
                    .filter_map(|p| {
                        Some(PackageInfo {
                            name: p.get("name")?.as_str()?.to_string(),
                            version: p.get("version")?.as_str()?.to_string(),
                            source: p.get("source").and_then(|s| s.as_str()).map(String::from),
                            checksum: p.get("checksum").and_then(|s| s.as_str()).map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(DependencyManifest {
            format: DependencyFormat::CargoLock,
            content_hash: ContentHash::from_bytes(content.as_bytes()),
            packages,
        })
    }

    async fn capture_pip_freeze(&self) -> Result<DependencyManifest, TrackerError> {
        let output = tokio::process::Command::new("pip")
            .args(["freeze", "--all"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(TrackerError::CommandFailed {
                command: "pip freeze".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let content = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<PackageInfo> = content
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split("==").collect();
                if parts.len() == 2 {
                    Some(PackageInfo {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        source: None,
                        checksum: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(DependencyManifest {
            format: DependencyFormat::PipFreeze,
            content_hash: ContentHash::from_bytes(content.as_bytes()),
            packages,
        })
    }

    async fn capture_conda_env(&self) -> Result<DependencyManifest, TrackerError> {
        let output = tokio::process::Command::new("conda")
            .args(["list", "--export"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(TrackerError::CommandFailed {
                command: "conda list".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let content = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<PackageInfo> = content
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    Some(PackageInfo {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        source: parts.get(2).map(|s| s.to_string()),
                        checksum: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(DependencyManifest {
            format: DependencyFormat::CondaEnvironment,
            content_hash: ContentHash::from_bytes(content.as_bytes()),
            packages,
        })
    }

    fn capture_env_vars(&self) -> Result<HashMap<String, String>, TrackerError> {
        let mut vars = HashMap::new();

        for (key, value) in std::env::vars() {
            // Check blocklist
            let blocked = self.config.env_var_blocklist.iter().any(|pattern| {
                key.to_uppercase().contains(&pattern.to_uppercase())
            });

            if blocked {
                continue;
            }

            // Check allowlist if specified
            if let Some(ref allowlist) = self.config.env_var_allowlist {
                if !allowlist.iter().any(|pattern| {
                    key.to_uppercase().contains(&pattern.to_uppercase())
                }) {
                    continue;
                }
            }

            vars.insert(key, value);
        }

        Ok(vars)
    }

    async fn capture_git_state(&self) -> Result<GitState, TrackerError> {
        // Get current branch
        let branch_output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .await?;

        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Get commit hash
        let commit_output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .await?;

        let commit_hash = String::from_utf8_lossy(&commit_output.stdout)
            .trim()
            .to_string();

        // Check if dirty
        let status_output = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .await?;

        let is_dirty = !status_output.stdout.is_empty();

        // Get uncommitted changes summary if dirty
        let uncommitted_changes = if is_dirty {
            let diff_output = tokio::process::Command::new("git")
                .args(["diff", "--stat"])
                .output()
                .await?;
            Some(String::from_utf8_lossy(&diff_output.stdout).to_string())
        } else {
            None
        };

        // Get remote URL
        let remote_output = tokio::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
            .await
            .ok();

        let repository_url = remote_output.and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

        Ok(GitState {
            repository_url,
            branch,
            commit_hash,
            is_dirty,
            uncommitted_changes,
            remote_tracking: None,
        })
    }

    async fn detect_container_info(&self) -> Result<ContainerInfo, TrackerError> {
        // Check if running in container
        if !std::path::Path::new("/.dockerenv").exists()
            && !std::path::Path::new("/run/.containerenv").exists()
        {
            return Err(TrackerError::NotFound {
                message: "Not running in container".to_string(),
            });
        }

        // Try to get image info from environment
        let image = std::env::var("CONTAINER_IMAGE")
            .or_else(|_| std::env::var("IMAGE_NAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        let image_digest = std::env::var("CONTAINER_DIGEST")
            .or_else(|_| std::env::var("IMAGE_DIGEST"))
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(ContainerInfo {
            image,
            image_digest,
            runtime: detect_container_runtime(),
            resource_limits: detect_container_limits().await.ok(),
        })
    }
}

impl EnvironmentSnapshot {
    pub fn minimal() -> Self {
        Self {
            os: OsInfo {
                name: std::env::consts::OS.to_string(),
                version: "unknown".to_string(),
                architecture: std::env::consts::ARCH.to_string(),
                kernel_version: None,
            },
            hardware: HardwareInfo::minimal(),
            runtime: RuntimeInfo {
                rust_version: None,
                python_version: None,
                cuda_version: None,
                custom_runtimes: HashMap::new(),
            },
            dependencies: DependencyManifest::empty(),
            environment_variables: HashMap::new(),
            git_state: None,
            container_info: None,
            captured_at: Utc::now(),
        }
    }
}

impl HardwareInfo {
    pub fn minimal() -> Self {
        Self {
            cpu_model: "unknown".to_string(),
            cpu_cores: 0,
            cpu_threads: 0,
            memory_total_mb: 0,
            gpus: Vec::new(),
            hostname_hash: "unknown".to_string(),
        }
    }
}

impl DependencyManifest {
    pub fn empty() -> Self {
        Self {
            format: DependencyFormat::Custom("none".to_string()),
            content_hash: ContentHash("empty".to_string()),
            packages: Vec::new(),
        }
    }
}

impl Default for RunMetrics {
    fn default() -> Self {
        Self {
            scalars: HashMap::new(),
            distributions: HashMap::new(),
            confusion_matrices: HashMap::new(),
            custom: HashMap::new(),
        }
    }
}

impl Default for LogSummary {
    fn default() -> Self {
        Self {
            total_lines: 0,
            error_count: 0,
            warning_count: 0,
            log_uri: String::new(),
            highlights: Vec::new(),
        }
    }
}

// Helper functions (implementations would be platform-specific)
fn get_os_version() -> Result<String, TrackerError> {
    // Platform-specific implementation
    Ok("unknown".to_string())
}

fn get_kernel_version() -> Result<String, TrackerError> {
    // Platform-specific implementation
    Ok("unknown".to_string())
}

fn get_cpu_info() -> Result<CpuInfo, TrackerError> {
    // Platform-specific implementation
    Ok(CpuInfo {
        model: "unknown".to_string(),
        cores: num_cpus::get_physical() as u32,
        threads: num_cpus::get() as u32,
    })
}

struct CpuInfo {
    model: String,
    cores: u32,
    threads: u32,
}

fn get_memory_info() -> Result<MemoryInfo, TrackerError> {
    // Platform-specific implementation using sysinfo crate
    Ok(MemoryInfo { total_mb: 0 })
}

struct MemoryInfo {
    total_mb: u64,
}

async fn detect_gpus() -> Result<Vec<GpuInfo>, TrackerError> {
    // Use nvidia-smi or similar
    Ok(Vec::new())
}

fn detect_python_version() -> Result<String, TrackerError> {
    // Run python --version
    Ok("unknown".to_string())
}

fn detect_cuda_version() -> Result<String, TrackerError> {
    // Check CUDA installation
    Ok("unknown".to_string())
}

fn detect_custom_runtimes() -> HashMap<String, String> {
    HashMap::new()
}

fn hash_string(s: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}

fn detect_container_runtime() -> String {
    if std::path::Path::new("/.dockerenv").exists() {
        "docker".to_string()
    } else if std::path::Path::new("/run/.containerenv").exists() {
        "podman".to_string()
    } else {
        "unknown".to_string()
    }
}

async fn detect_container_limits() -> Result<ContainerResourceLimits, TrackerError> {
    // Read from cgroup files
    Ok(ContainerResourceLimits {
        cpu_limit: None,
        memory_limit_mb: None,
        gpu_count: None,
    })
}

fn compress_data(data: &[u8]) -> Result<Vec<u8>, TrackerError> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}
```

### 2.4 Lineage Tracker

```rust
/// Lineage tracker for experiment provenance
pub struct LineageTracker {
    store: Arc<dyn LineageStore>,
}

#[async_trait]
pub trait LineageStore: Send + Sync {
    async fn record_node(&self, node: LineageNode) -> Result<(), TrackerError>;
    async fn record_edge(&self, edge: LineageEdge) -> Result<(), TrackerError>;
    async fn get_node(&self, id: &str) -> Result<LineageNode, TrackerError>;
    async fn get_ancestors(&self, id: &str, depth: Option<u32>) -> Result<Vec<LineageNode>, TrackerError>;
    async fn get_descendants(&self, id: &str, depth: Option<u32>) -> Result<Vec<LineageNode>, TrackerError>;
    async fn get_graph(&self, root_id: &str) -> Result<LineageGraph, TrackerError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: String,
    pub node_type: LineageNodeType,
    pub name: String,
    pub version: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageNodeType {
    Experiment,
    Run,
    Dataset,
    DatasetVersion,
    Model,
    Artifact,
    Metric,
    Transformation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: LineageEdgeType,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageEdgeType {
    Contains,      // Experiment contains Run
    UsesDataset,   // Run uses Dataset
    UsesModel,     // Run uses Model
    Produces,      // Run produces Artifact
    DerivedFrom,   // Dataset derived from another Dataset
    Evaluates,     // Run evaluates with Metric
    Transforms,    // Transformation applied
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
    pub root_id: String,
}

impl LineageTracker {
    pub fn new(store: Arc<dyn LineageStore>) -> Self {
        Self { store }
    }

    pub async fn record_experiment_creation(
        &self,
        experiment_id: ExperimentId,
        experiment: &Experiment,
    ) -> Result<(), TrackerError> {
        let node = LineageNode {
            id: format!("experiment:{}", experiment_id.0),
            node_type: LineageNodeType::Experiment,
            name: experiment.name.clone(),
            version: None,
            metadata: serde_json::to_value(&experiment.metadata)
                .ok()
                .and_then(|v| v.as_object().cloned())
                .map(|m| m.into_iter().collect())
                .unwrap_or_default(),
            created_at: experiment.created_at,
        };

        self.store.record_node(node).await
    }

    pub async fn record_run_start(
        &self,
        run_id: RunId,
        experiment_id: ExperimentId,
        run: &ExperimentRun,
    ) -> Result<(), TrackerError> {
        // Create run node
        let run_node = LineageNode {
            id: format!("run:{}", run_id.0),
            node_type: LineageNodeType::Run,
            name: run.name.clone().unwrap_or_else(|| format!("Run #{}", run.run_number)),
            version: Some(run.run_number.to_string()),
            metadata: HashMap::new(),
            created_at: run.created_at,
        };
        self.store.record_node(run_node).await?;

        // Create edge from experiment to run
        let edge = LineageEdge {
            source_id: format!("experiment:{}", experiment_id.0),
            target_id: format!("run:{}", run_id.0),
            edge_type: LineageEdgeType::Contains,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        self.store.record_edge(edge).await
    }

    pub async fn record_run_completion(
        &self,
        run_id: RunId,
        run: &ExperimentRun,
    ) -> Result<(), TrackerError> {
        // Update run node with completion metadata
        let mut metadata = HashMap::new();
        metadata.insert(
            "status".to_string(),
            serde_json::Value::String(format!("{:?}", run.status)),
        );
        if let (Some(start), Some(end)) = (run.started_at, run.ended_at) {
            let duration = (end - start).num_seconds();
            metadata.insert(
                "duration_seconds".to_string(),
                serde_json::Value::Number(duration.into()),
            );
        }

        let node = LineageNode {
            id: format!("run:{}", run_id.0),
            node_type: LineageNodeType::Run,
            name: run.name.clone().unwrap_or_else(|| format!("Run #{}", run.run_number)),
            version: Some(run.run_number.to_string()),
            metadata,
            created_at: run.created_at,
        };

        self.store.record_node(node).await
    }

    pub async fn record_run_failure(
        &self,
        run_id: RunId,
        run: &ExperimentRun,
    ) -> Result<(), TrackerError> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "status".to_string(),
            serde_json::Value::String("failed".to_string()),
        );
        if let Some(ref error) = run.error {
            metadata.insert(
                "error_type".to_string(),
                serde_json::Value::String(error.error_type.clone()),
            );
            metadata.insert(
                "error_message".to_string(),
                serde_json::Value::String(error.message.clone()),
            );
        }

        let node = LineageNode {
            id: format!("run:{}", run_id.0),
            node_type: LineageNodeType::Run,
            name: run.name.clone().unwrap_or_else(|| format!("Run #{}", run.run_number)),
            version: Some(run.run_number.to_string()),
            metadata,
            created_at: run.created_at,
        };

        self.store.record_node(node).await
    }

    pub async fn record_artifact(
        &self,
        run_id: RunId,
        artifact: &ArtifactRef,
    ) -> Result<(), TrackerError> {
        // Create artifact node
        let artifact_node = LineageNode {
            id: format!("artifact:{}", artifact.id.0),
            node_type: LineageNodeType::Artifact,
            name: artifact.name.clone(),
            version: None,
            metadata: artifact
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect(),
            created_at: artifact.created_at,
        };
        self.store.record_node(artifact_node).await?;

        // Create edge from run to artifact
        let edge = LineageEdge {
            source_id: format!("run:{}", run_id.0),
            target_id: format!("artifact:{}", artifact.id.0),
            edge_type: LineageEdgeType::Produces,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        self.store.record_edge(edge).await
    }

    pub async fn record_dataset_usage(
        &self,
        run_id: RunId,
        dataset_ref: &DatasetRef,
    ) -> Result<(), TrackerError> {
        let edge = LineageEdge {
            source_id: format!("run:{}", run_id.0),
            target_id: format!("dataset:{}", dataset_ref.dataset_id.0),
            edge_type: LineageEdgeType::UsesDataset,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "version_selector".to_string(),
                    serde_json::to_value(&dataset_ref.version).unwrap_or_default(),
                );
                if let Some(ref split) = dataset_ref.split {
                    m.insert(
                        "split".to_string(),
                        serde_json::Value::String(format!("{:?}", split)),
                    );
                }
                m
            },
            created_at: Utc::now(),
        };
        self.store.record_edge(edge).await
    }

    pub async fn get_experiment_lineage(
        &self,
        experiment_id: ExperimentId,
    ) -> Result<LineageGraph, TrackerError> {
        self.store
            .get_graph(&format!("experiment:{}", experiment_id.0))
            .await
    }
}
```

### 2.5 Event System

```rust
/// Event types for experiment tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    ExperimentCreated {
        experiment_id: ExperimentId,
        owner_id: UserId,
        timestamp: DateTime<Utc>,
    },
    ExperimentUpdated {
        experiment_id: ExperimentId,
        updated_fields: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    ExperimentArchived {
        experiment_id: ExperimentId,
        timestamp: DateTime<Utc>,
    },
    RunStarted {
        run_id: RunId,
        experiment_id: ExperimentId,
        timestamp: DateTime<Utc>,
    },
    RunCompleted {
        run_id: RunId,
        experiment_id: ExperimentId,
        status: RunStatus,
        duration: chrono::Duration,
        timestamp: DateTime<Utc>,
    },
    RunFailed {
        run_id: RunId,
        experiment_id: ExperimentId,
        error_type: Option<String>,
        timestamp: DateTime<Utc>,
    },
    MetricsLogged {
        run_id: RunId,
        count: usize,
        timestamp: DateTime<Utc>,
    },
    ArtifactLogged {
        run_id: RunId,
        artifact_id: ArtifactId,
        artifact_type: ArtifactType,
        timestamp: DateTime<Utc>,
    },
}

/// Event publisher trait
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: Event) -> Result<(), TrackerError>;
    async fn publish_batch(&self, events: Vec<Event>) -> Result<(), TrackerError>;
}

/// Event subscriber trait
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn subscribe(&self, filter: EventFilter) -> Result<EventStream, TrackerError>;
}

pub type EventStream = tokio::sync::mpsc::Receiver<Event>;

#[derive(Debug, Clone)]
pub struct EventFilter {
    pub event_types: Option<Vec<String>>,
    pub experiment_ids: Option<Vec<ExperimentId>>,
    pub run_ids: Option<Vec<RunId>>,
    pub since: Option<DateTime<Utc>>,
}
```

### 2.6 Error Types

```rust
/// Tracker error types
#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Invalid state: {message}")]
    InvalidState { message: String },

    #[error("Storage error: {source}")]
    Storage {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Artifact too large: {size} bytes exceeds max {max_size} bytes")]
    ArtifactTooLarge { size: u64, max_size: u64 },

    #[error("Command failed: {command} - {stderr}")]
    CommandFailed { command: String, stderr: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl TrackerError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TrackerError::Storage { .. } | TrackerError::Internal { .. }
        )
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            TrackerError::NotFound { .. } => "NOT_FOUND",
            TrackerError::ValidationError { .. } => "VALIDATION_ERROR",
            TrackerError::InvalidState { .. } => "INVALID_STATE",
            TrackerError::Storage { .. } => "STORAGE_ERROR",
            TrackerError::ArtifactTooLarge { .. } => "ARTIFACT_TOO_LARGE",
            TrackerError::CommandFailed { .. } => "COMMAND_FAILED",
            TrackerError::Serialization(_) => "SERIALIZATION_ERROR",
            TrackerError::Io(_) => "IO_ERROR",
            TrackerError::TomlParse(_) => "TOML_PARSE_ERROR",
            TrackerError::Unauthorized { .. } => "UNAUTHORIZED",
            TrackerError::Conflict { .. } => "CONFLICT",
            TrackerError::Internal { .. } => "INTERNAL_ERROR",
        }
    }
}
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Draft |
| **SPARC Phase** | Pseudocode (Part 1 of 3) |
| **Created** | 2025-11-28 |
| **Ecosystem** | LLM DevOps |
| **Next Part** | Pseudocode Part 2: Metrics & Datasets |

---

*This pseudocode document is part of the SPARC methodology. Part 2 covers Metric Benchmarking and Dataset Versioning. Part 3 covers Integration APIs, Reproducibility Engine, and Workflow Orchestration.*
