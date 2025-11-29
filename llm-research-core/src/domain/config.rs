use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use super::ids::{DatasetId, DatasetVersionId, SemanticVersion};

// ===== Model Configuration =====

// Re-export ModelProvider from model module to avoid duplication
pub use super::model::ModelProvider;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelParameters {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop_sequences: Option<Vec<String>>,
    pub seed: Option<u64>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: Some(1.0),
            max_tokens: Some(1024),
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            seed: None,
            additional: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    #[validate(length(min = 1, max = 255))]
    pub model_name: String,
    pub model_version: Option<String>,
    pub parameters: ModelParameters,
    pub system_prompt: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

// ===== Dataset Configuration =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DatasetVersionSelector {
    Latest,
    Specific(DatasetVersionId),
    Tag(String),
    SemanticVersion(SemanticVersion),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DataSplit {
    Train,
    Validation,
    Test,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SampleStrategy {
    Random,
    Sequential,
    Stratified,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SampleSize {
    All,
    Count(usize),
    Percentage(u8), // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SampleConfig {
    pub strategy: SampleStrategy,
    pub size: SampleSize,
    pub seed: Option<u64>,
    pub stratify_by: Option<String>,
}

impl Default for SampleConfig {
    fn default() -> Self {
        Self {
            strategy: SampleStrategy::Sequential,
            size: SampleSize::All,
            seed: None,
            stratify_by: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate)]
pub struct DatasetRef {
    pub dataset_id: DatasetId,
    pub version: DatasetVersionSelector,
    pub split: Option<DataSplit>,
    pub sample: Option<SampleConfig>,
    pub filters: HashMap<String, serde_json::Value>,
}

// ===== Metric Configuration =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ThresholdDirection {
    Above,
    Below,
    Equal,
    Between,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricThreshold {
    pub direction: ThresholdDirection,
    pub value: f64,
    pub max_value: Option<f64>, // For 'Between' direction
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct MetricConfig {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub metric_type: String, // e.g., "accuracy", "f1_score", "bleu", "rouge"
    pub parameters: HashMap<String, serde_json::Value>,
    pub threshold: Option<MetricThreshold>,
    pub weight: Option<f64>,
    pub tags: Vec<String>,
}

// ===== Experiment Parameters =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ParameterValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ParameterValue>),
    Object(HashMap<String, ParameterValue>),
}

impl From<String> for ParameterValue {
    fn from(s: String) -> Self {
        ParameterValue::String(s)
    }
}

impl From<i64> for ParameterValue {
    fn from(i: i64) -> Self {
        ParameterValue::Integer(i)
    }
}

impl From<f64> for ParameterValue {
    fn from(f: f64) -> Self {
        ParameterValue::Float(f)
    }
}

impl From<bool> for ParameterValue {
    fn from(b: bool) -> Self {
        ParameterValue::Boolean(b)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SearchStrategy {
    Grid,
    Random,
    Bayesian,
    Hyperband,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchSpace {
    pub parameter_name: String,
    pub values: Vec<ParameterValue>,
    pub distribution: Option<String>, // e.g., "uniform", "log-uniform", "normal"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperimentParameters {
    pub fixed: HashMap<String, ParameterValue>,
    pub search_spaces: Vec<SearchSpace>,
    pub search_strategy: Option<SearchStrategy>,
    pub max_trials: Option<u32>,
    pub concurrent_trials: Option<u32>,
}

impl Default for ExperimentParameters {
    fn default() -> Self {
        Self {
            fixed: HashMap::new(),
            search_spaces: Vec::new(),
            search_strategy: None,
            max_trials: None,
            concurrent_trials: Some(1),
        }
    }
}

// ===== Resource Requirements =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GpuType {
    T4,
    V100,
    A100,
    H100,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GpuRequirements {
    pub gpu_type: Option<GpuType>,
    pub gpu_count: u32,
    pub gpu_memory_gb: Option<u32>,
}

impl Default for GpuRequirements {
    fn default() -> Self {
        Self {
            gpu_type: None,
            gpu_count: 0,
            gpu_memory_gb: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ComputeRequirements {
    pub cpu_cores: Option<u32>,
    pub memory_gb: Option<u32>,
    pub disk_gb: Option<u32>,
    pub gpu: Option<GpuRequirements>,
}

impl Default for ComputeRequirements {
    fn default() -> Self {
        Self {
            cpu_cores: Some(1),
            memory_gb: Some(4),
            disk_gb: Some(10),
            gpu: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ResourceRequirements {
    pub compute: ComputeRequirements,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
    pub priority: Option<u32>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            compute: ComputeRequirements::default(),
            timeout_seconds: Some(3600), // 1 hour default
            max_retries: Some(0),
            priority: Some(5),
        }
    }
}

// ===== Reproducibility Settings =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReproducibilitySettings {
    pub random_seed: Option<u64>,
    pub deterministic_mode: bool,
    pub track_environment: bool,
    pub track_code_version: bool,
    pub track_dependencies: bool,
    pub snapshot_dataset: bool,
    pub snapshot_model: bool,
}

impl Default for ReproducibilitySettings {
    fn default() -> Self {
        Self {
            random_seed: None,
            deterministic_mode: true,
            track_environment: true,
            track_code_version: true,
            track_dependencies: true,
            snapshot_dataset: true,
            snapshot_model: false,
        }
    }
}

// ===== Complete Experiment Configuration =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct ExperimentConfig {
    pub model_configs: Vec<ModelConfig>,
    pub dataset_refs: Vec<DatasetRef>,
    pub metric_configs: Vec<MetricConfig>,
    pub parameters: ExperimentParameters,
    pub resource_requirements: ResourceRequirements,
    pub reproducibility_settings: ReproducibilitySettings,
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            model_configs: Vec::new(),
            dataset_refs: Vec::new(),
            metric_configs: Vec::new(),
            parameters: ExperimentParameters::default(),
            resource_requirements: ResourceRequirements::default(),
            reproducibility_settings: ReproducibilitySettings::default(),
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_default() {
        let params = ModelParameters::default();
        assert_eq!(params.temperature, Some(1.0));
        assert_eq!(params.max_tokens, Some(1024));
    }

    #[test]
    fn test_sample_size() {
        let all = SampleSize::All;
        let count = SampleSize::Count(100);
        let percentage = SampleSize::Percentage(50);

        assert_ne!(all, count);
        assert_ne!(count, percentage);
    }

    #[test]
    fn test_parameter_value() {
        let string_val = ParameterValue::from("test".to_string());
        let int_val = ParameterValue::from(42i64);
        let _float_val = ParameterValue::from(3.14f64);
        let _bool_val = ParameterValue::from(true);

        match string_val {
            ParameterValue::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected String variant"),
        }

        match int_val {
            ParameterValue::Integer(i) => assert_eq!(i, 42),
            _ => panic!("Expected Integer variant"),
        }
    }
}
