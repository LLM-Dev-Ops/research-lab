use llm_research_core::domain::*;
use std::collections::HashMap;

// ===== ModelConfig Tests =====

#[test]
fn test_model_parameters_default() {
    let params = ModelParameters::default();

    assert_eq!(params.temperature, Some(1.0));
    assert_eq!(params.max_tokens, Some(1024));
    assert_eq!(params.top_p, None);
    assert_eq!(params.top_k, None);
    assert_eq!(params.frequency_penalty, None);
    assert_eq!(params.presence_penalty, None);
    assert_eq!(params.stop_sequences, None);
    assert_eq!(params.seed, None);
}

#[test]
fn test_model_config_serialization() {
    let config = ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "gpt-4".to_string(),
        model_version: Some("0613".to_string()),
        parameters: ModelParameters::default(),
        system_prompt: Some("You are a helpful assistant.".to_string()),
        metadata: HashMap::new(),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ModelConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.provider, deserialized.provider);
    assert_eq!(config.model_name, deserialized.model_name);
    assert_eq!(config.model_version, deserialized.model_version);
}

#[test]
fn test_model_config_validation() {
    use validator::Validate;

    let config = ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "gpt-4".to_string(),
        model_version: None,
        parameters: ModelParameters::default(),
        system_prompt: None,
        metadata: HashMap::new(),
    };

    assert!(config.validate().is_ok());

    // Test with empty model name (should fail)
    let invalid_config = ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "".to_string(),
        model_version: None,
        parameters: ModelParameters::default(),
        system_prompt: None,
        metadata: HashMap::new(),
    };

    assert!(invalid_config.validate().is_err());
}

// ===== ExperimentConfig Tests =====

#[test]
fn test_experiment_config_default() {
    let config = ExperimentConfig::default();

    assert!(config.model_configs.is_empty());
    assert!(config.dataset_refs.is_empty());
    assert!(config.metric_configs.is_empty());
    assert_eq!(config.parameters.concurrent_trials, Some(1));
}

#[test]
fn test_experiment_config_serialization() {
    let config = ExperimentConfig::default();

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ExperimentConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.model_configs.len(), deserialized.model_configs.len());
    assert_eq!(config.dataset_refs.len(), deserialized.dataset_refs.len());
}

#[test]
fn test_experiment_config_with_models() {
    let model_config = ModelConfig {
        provider: ModelProvider::Anthropic,
        model_name: "claude-3-sonnet".to_string(),
        model_version: None,
        parameters: ModelParameters::default(),
        system_prompt: None,
        metadata: HashMap::new(),
    };

    let mut config = ExperimentConfig::default();
    config.model_configs.push(model_config);

    assert_eq!(config.model_configs.len(), 1);
}

// ===== ResourceRequirements Tests =====

#[test]
fn test_resource_requirements_default() {
    let resources = ResourceRequirements::default();

    assert_eq!(resources.compute.cpu_cores, Some(1));
    assert_eq!(resources.compute.memory_gb, Some(4));
    assert_eq!(resources.compute.disk_gb, Some(10));
    assert_eq!(resources.timeout_seconds, Some(3600));
    assert_eq!(resources.max_retries, Some(0));
    assert_eq!(resources.priority, Some(5));
}

#[test]
fn test_compute_requirements() {
    let compute = ComputeRequirements {
        cpu_cores: Some(8),
        memory_gb: Some(32),
        disk_gb: Some(100),
        gpu: Some(GpuRequirements {
            gpu_type: Some(GpuType::A100),
            gpu_count: 2,
            gpu_memory_gb: Some(40),
        }),
    };

    assert_eq!(compute.cpu_cores, Some(8));
    assert_eq!(compute.memory_gb, Some(32));
    assert!(compute.gpu.is_some());

    let gpu = compute.gpu.unwrap();
    assert_eq!(gpu.gpu_count, 2);
    assert_eq!(gpu.gpu_memory_gb, Some(40));
}

#[test]
fn test_gpu_requirements_default() {
    let gpu = GpuRequirements::default();

    assert_eq!(gpu.gpu_type, None);
    assert_eq!(gpu.gpu_count, 0);
    assert_eq!(gpu.gpu_memory_gb, None);
}

#[test]
fn test_gpu_type_variants() {
    let types = vec![
        GpuType::T4,
        GpuType::V100,
        GpuType::A100,
        GpuType::H100,
        GpuType::Custom("RTX4090".to_string()),
    ];

    assert_eq!(types.len(), 5);
}

// ===== ParameterValue Tests =====

#[test]
fn test_parameter_value_string() {
    let value = ParameterValue::from("test".to_string());

    match value {
        ParameterValue::String(s) => assert_eq!(s, "test"),
        _ => panic!("Expected String variant"),
    }
}

#[test]
fn test_parameter_value_integer() {
    let value = ParameterValue::from(42i64);

    match value {
        ParameterValue::Integer(i) => assert_eq!(i, 42),
        _ => panic!("Expected Integer variant"),
    }
}

#[test]
fn test_parameter_value_float() {
    let value = ParameterValue::from(3.14f64);

    match value {
        ParameterValue::Float(f) => assert!((f - 3.14).abs() < 1e-10),
        _ => panic!("Expected Float variant"),
    }
}

#[test]
fn test_parameter_value_boolean() {
    let value = ParameterValue::from(true);

    match value {
        ParameterValue::Boolean(b) => assert!(b),
        _ => panic!("Expected Boolean variant"),
    }
}

#[test]
fn test_parameter_value_array() {
    let arr = vec![
        ParameterValue::from(1i64),
        ParameterValue::from(2i64),
        ParameterValue::from(3i64),
    ];
    let value = ParameterValue::Array(arr);

    match value {
        ParameterValue::Array(a) => assert_eq!(a.len(), 3),
        _ => panic!("Expected Array variant"),
    }
}

#[test]
fn test_parameter_value_object() {
    let mut obj = HashMap::new();
    obj.insert("key1".to_string(), ParameterValue::from("value1".to_string()));
    obj.insert("key2".to_string(), ParameterValue::from(42i64));

    let value = ParameterValue::Object(obj);

    match value {
        ParameterValue::Object(o) => assert_eq!(o.len(), 2),
        _ => panic!("Expected Object variant"),
    }
}

#[test]
fn test_parameter_value_serialization() {
    let value = ParameterValue::from(42i64);
    let json = serde_json::to_string(&value).unwrap();
    let deserialized: ParameterValue = serde_json::from_str(&json).unwrap();

    match deserialized {
        ParameterValue::Integer(i) => assert_eq!(i, 42),
        _ => panic!("Expected Integer variant"),
    }
}

// ===== DatasetRef Tests =====

#[test]
fn test_dataset_ref_creation() {
    let dataset_id = DatasetId::new();
    let dataset_ref = DatasetRef {
        dataset_id,
        version: DatasetVersionSelector::Latest,
        split: Some(DataSplit::Train),
        sample: None,
        filters: HashMap::new(),
    };

    assert_eq!(dataset_ref.dataset_id, dataset_id);
    assert_eq!(dataset_ref.version, DatasetVersionSelector::Latest);
    assert_eq!(dataset_ref.split, Some(DataSplit::Train));
}

#[test]
fn test_dataset_version_selector() {
    let selectors = vec![
        DatasetVersionSelector::Latest,
        DatasetVersionSelector::Tag("v1.0".to_string()),
        DatasetVersionSelector::Specific(DatasetVersionId::new()),
        DatasetVersionSelector::SemanticVersion(SemanticVersion::new(1, 0, 0)),
    ];

    assert_eq!(selectors.len(), 4);
}

#[test]
fn test_data_split_variants() {
    let splits = vec![
        DataSplit::Train,
        DataSplit::Validation,
        DataSplit::Test,
        DataSplit::Custom("holdout".to_string()),
    ];

    assert_eq!(splits.len(), 4);
}

#[test]
fn test_sample_config_default() {
    let config = SampleConfig::default();

    assert_eq!(config.strategy, SampleStrategy::Sequential);
    assert_eq!(config.size, SampleSize::All);
    assert_eq!(config.seed, None);
    assert_eq!(config.stratify_by, None);
}

#[test]
fn test_sample_strategy_variants() {
    let strategies = vec![
        SampleStrategy::Random,
        SampleStrategy::Sequential,
        SampleStrategy::Stratified,
        SampleStrategy::Custom("weighted".to_string()),
    ];

    assert_eq!(strategies.len(), 4);
}

#[test]
fn test_sample_size_variants() {
    let size_all = SampleSize::All;
    let size_count = SampleSize::Count(100);
    let size_percent = SampleSize::Percentage(50);

    assert_ne!(size_all, size_count);
    assert_ne!(size_count, size_percent);
}

// ===== MetricConfig Tests =====

#[test]
fn test_metric_config_creation() {
    use validator::Validate;

    let config = MetricConfig {
        name: "accuracy".to_string(),
        metric_type: "classification".to_string(),
        parameters: HashMap::new(),
        threshold: None,
        weight: Some(1.0),
        tags: vec!["primary".to_string()],
    };

    assert!(config.validate().is_ok());
    assert_eq!(config.name, "accuracy");
    assert_eq!(config.weight, Some(1.0));
}

#[test]
fn test_metric_config_with_threshold() {
    let threshold = MetricThreshold {
        direction: ThresholdDirection::Above,
        value: 0.8,
        max_value: None,
    };

    let config = MetricConfig {
        name: "f1_score".to_string(),
        metric_type: "classification".to_string(),
        parameters: HashMap::new(),
        threshold: Some(threshold),
        weight: None,
        tags: vec![],
    };

    assert!(config.threshold.is_some());
    let t = config.threshold.unwrap();
    assert_eq!(t.direction, ThresholdDirection::Above);
    assert_eq!(t.value, 0.8);
}

#[test]
fn test_threshold_direction_variants() {
    let directions = vec![
        ThresholdDirection::Above,
        ThresholdDirection::Below,
        ThresholdDirection::Equal,
        ThresholdDirection::Between,
    ];

    assert_eq!(directions.len(), 4);
}

#[test]
fn test_metric_threshold_between() {
    let threshold = MetricThreshold {
        direction: ThresholdDirection::Between,
        value: 0.5,
        max_value: Some(0.9),
    };

    assert_eq!(threshold.direction, ThresholdDirection::Between);
    assert_eq!(threshold.value, 0.5);
    assert_eq!(threshold.max_value, Some(0.9));
}

// ===== ExperimentParameters Tests =====

#[test]
fn test_experiment_parameters_default() {
    let params = ExperimentParameters::default();

    assert!(params.fixed.is_empty());
    assert!(params.search_spaces.is_empty());
    assert_eq!(params.search_strategy, None);
    assert_eq!(params.max_trials, None);
    assert_eq!(params.concurrent_trials, Some(1));
}

#[test]
fn test_search_space_creation() {
    let search_space = SearchSpace {
        parameter_name: "learning_rate".to_string(),
        values: vec![
            ParameterValue::from(0.001f64),
            ParameterValue::from(0.01f64),
            ParameterValue::from(0.1f64),
        ],
        distribution: Some("log-uniform".to_string()),
    };

    assert_eq!(search_space.parameter_name, "learning_rate");
    assert_eq!(search_space.values.len(), 3);
    assert_eq!(search_space.distribution, Some("log-uniform".to_string()));
}

#[test]
fn test_search_strategy_variants() {
    let strategies = vec![
        SearchStrategy::Grid,
        SearchStrategy::Random,
        SearchStrategy::Bayesian,
        SearchStrategy::Hyperband,
        SearchStrategy::Custom("genetic".to_string()),
    ];

    assert_eq!(strategies.len(), 5);
}

// ===== ReproducibilitySettings Tests =====

#[test]
fn test_reproducibility_settings_default() {
    let settings = ReproducibilitySettings::default();

    assert_eq!(settings.random_seed, None);
    assert!(settings.deterministic_mode);
    assert!(settings.track_environment);
    assert!(settings.track_code_version);
    assert!(settings.track_dependencies);
    assert!(settings.snapshot_dataset);
    assert!(!settings.snapshot_model);
}

#[test]
fn test_reproducibility_settings_with_seed() {
    let settings = ReproducibilitySettings {
        random_seed: Some(42),
        deterministic_mode: true,
        track_environment: true,
        track_code_version: true,
        track_dependencies: true,
        snapshot_dataset: true,
        snapshot_model: true,
    };

    assert_eq!(settings.random_seed, Some(42));
    assert!(settings.snapshot_model);
}

#[test]
fn test_reproducibility_settings_serialization() {
    let settings = ReproducibilitySettings::default();

    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: ReproducibilitySettings = serde_json::from_str(&json).unwrap();

    assert_eq!(settings.deterministic_mode, deserialized.deterministic_mode);
    assert_eq!(settings.track_environment, deserialized.track_environment);
}
