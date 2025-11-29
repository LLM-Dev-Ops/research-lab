use llm_research_core::domain::*;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

// ===== Model Serialization Tests =====

#[test]
fn test_model_serialization_roundtrip() {
    let config = json!({
        "api_key": "test-key",
        "endpoint": "https://api.example.com"
    });

    let original = Model::new(
        "GPT-4".to_string(),
        ModelProvider::OpenAI,
        "gpt-4-turbo".to_string(),
        Some("2024-01".to_string()),
        config,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Model = serde_json::from_str(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.provider, deserialized.provider);
    assert_eq!(original.model_identifier, deserialized.model_identifier);
    assert_eq!(original.version, deserialized.version);
    assert_eq!(original.config, deserialized.config);
}

#[test]
fn test_model_provider_serialization() {
    let providers = vec![
        (ModelProvider::OpenAI, "openai"),
        (ModelProvider::Anthropic, "anthropic"),
        (ModelProvider::Google, "google"),
        (ModelProvider::Cohere, "cohere"),
        (ModelProvider::HuggingFace, "huggingface"),
        (ModelProvider::Azure, "azure"),
        (ModelProvider::AWS, "aws"),
        (ModelProvider::Local, "local"),
        (ModelProvider::Custom, "custom"),
    ];

    for (provider, expected_json) in providers {
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, format!("\"{}\"", expected_json));

        let deserialized: ModelProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, provider);
    }
}

#[test]
fn test_model_with_unicode_name() {
    let config = json!({});
    let original = Model::new(
        "GPT-4 模型 🤖 Модель".to_string(),
        ModelProvider::OpenAI,
        "gpt-4".to_string(),
        None,
        config,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Model = serde_json::from_str(&json).unwrap();

    assert_eq!(original.name, deserialized.name);
    assert!(original.name.contains("🤖"));
}

#[test]
fn test_model_with_special_characters() {
    let config = json!({
        "special": "value with \"quotes\" and \n newlines \t tabs"
    });

    let original = Model::new(
        "Model with \"quotes\" and \\backslashes\\".to_string(),
        ModelProvider::Custom,
        "test".to_string(),
        None,
        config,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Model = serde_json::from_str(&json).unwrap();

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.config, deserialized.config);
}

// ===== Dataset Serialization Tests =====

#[test]
fn test_dataset_serialization_roundtrip() {
    let schema = json!({
        "type": "object",
        "properties": {
            "input": {"type": "string"},
            "output": {"type": "string"}
        }
    });

    let original = Dataset::new(
        "Test Dataset".to_string(),
        Some("A description".to_string()),
        "s3://bucket/data.parquet".to_string(),
        1000,
        schema,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Dataset = serde_json::from_str(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.description, deserialized.description);
    assert_eq!(original.s3_path, deserialized.s3_path);
    assert_eq!(original.sample_count, deserialized.sample_count);
    assert_eq!(original.schema, deserialized.schema);
}

#[test]
fn test_dataset_with_complex_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "nested": {
                "type": "object",
                "properties": {
                    "deep": {
                        "type": "array",
                        "items": {"type": "number"}
                    }
                }
            },
            "unicode": "测试 тест テスト",
            "emoji": "🔥💡✨"
        }
    });

    let original = Dataset::new(
        "Complex Dataset".to_string(),
        None,
        "s3://bucket/complex.json".to_string(),
        500,
        schema,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Dataset = serde_json::from_str(&json).unwrap();

    assert_eq!(original.schema, deserialized.schema);
}

#[test]
fn test_dataset_sample_serialization_roundtrip() {
    let dataset_id = Uuid::new_v4();
    let input = json!({
        "question": "What is AI?",
        "context": ["AI is...", "Machine learning..."]
    });
    let expected = json!({"answer": "Artificial Intelligence"});
    let metadata = json!({"source": "human", "quality": 0.95});

    let original = DatasetSample::new(
        dataset_id,
        42,
        input,
        Some(expected),
        metadata,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: DatasetSample = serde_json::from_str(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.dataset_id, deserialized.dataset_id);
    assert_eq!(original.index, deserialized.index);
    assert_eq!(original.input, deserialized.input);
    assert_eq!(original.expected_output, deserialized.expected_output);
    assert_eq!(original.metadata, deserialized.metadata);
}

// ===== PromptTemplate Serialization Tests =====

#[test]
fn test_prompt_template_serialization_roundtrip() {
    let original = PromptTemplate::new(
        "Test Template".to_string(),
        Some("Description".to_string()),
        "Hello {{name}}, your score is {{score}}".to_string(),
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PromptTemplate = serde_json::from_str(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.description, deserialized.description);
    assert_eq!(original.template, deserialized.template);
    assert_eq!(original.variables, deserialized.variables);
    assert_eq!(original.version, deserialized.version);
}

#[test]
fn test_prompt_template_with_multiline() {
    let template = "System: {{system_prompt}}\n\nUser: {{user_input}}\n\nAssistant:";

    let original = PromptTemplate::new(
        "Chat Template".to_string(),
        None,
        template.to_string(),
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PromptTemplate = serde_json::from_str(&json).unwrap();

    assert_eq!(original.template, deserialized.template);
    assert!(deserialized.template.contains("\n\n"));
}

#[test]
fn test_prompt_template_with_unicode() {
    let template = "你好 {{name}}! Привет {{greeting}}! こんにちは {{message}}!";

    let original = PromptTemplate::new(
        "Multilingual Template".to_string(),
        Some("Unicode test 🌍".to_string()),
        template.to_string(),
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PromptTemplate = serde_json::from_str(&json).unwrap();

    assert_eq!(original.template, deserialized.template);
    assert!(deserialized.template.contains("你好"));
    assert!(deserialized.template.contains("Привет"));
    assert!(deserialized.template.contains("こんにちは"));
}

// ===== Evaluation Serialization Tests =====

#[test]
fn test_evaluation_serialization_roundtrip() {
    let experiment_id = Uuid::new_v4();
    let sample_id = Uuid::new_v4();
    let metrics = json!({
        "accuracy": 0.95,
        "f1": 0.92
    });

    let original = Evaluation::new(
        experiment_id,
        sample_id,
        "Input text".to_string(),
        "Output text".to_string(),
        Some("Expected".to_string()),
        250,
        20,
        Some(rust_decimal::Decimal::new(25, 5)),
        metrics,
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Evaluation = serde_json::from_str(&json).unwrap();

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.experiment_id, deserialized.experiment_id);
    assert_eq!(original.sample_id, deserialized.sample_id);
    assert_eq!(original.input, deserialized.input);
    assert_eq!(original.output, deserialized.output);
    assert_eq!(original.latency_ms, deserialized.latency_ms);
    assert_eq!(original.token_count, deserialized.token_count);
}

#[test]
fn test_evaluation_metrics_serialization() {
    let original = EvaluationMetrics {
        accuracy: Some(rust_decimal::Decimal::new(95, 2)),
        precision: Some(rust_decimal::Decimal::new(93, 2)),
        recall: Some(rust_decimal::Decimal::new(97, 2)),
        f1_score: Some(rust_decimal::Decimal::new(95, 2)),
        bleu_score: Some(rust_decimal::Decimal::new(82, 2)),
        rouge_scores: Some(json!({"rouge-1": 0.85})),
        custom_metrics: json!({"perplexity": 15.3}),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: EvaluationMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(original.accuracy, deserialized.accuracy);
    assert_eq!(original.precision, deserialized.precision);
    assert_eq!(original.custom_metrics, deserialized.custom_metrics);
}

// ===== Configuration Serialization Tests =====

#[test]
fn test_experiment_config_serialization_roundtrip() {
    let mut config = ExperimentConfig::default();

    let model_config = ModelConfig {
        provider: ModelProvider::Anthropic,
        model_name: "claude-3-opus".to_string(),
        model_version: Some("20240229".to_string()),
        parameters: ModelParameters::default(),
        system_prompt: Some("You are helpful".to_string()),
        metadata: HashMap::new(),
    };

    config.model_configs.push(model_config);

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ExperimentConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.model_configs.len(), deserialized.model_configs.len());
    assert_eq!(
        config.model_configs[0].model_name,
        deserialized.model_configs[0].model_name
    );
}

#[test]
fn test_model_parameters_serialization() {
    let mut additional = HashMap::new();
    additional.insert("custom".to_string(), json!("value"));

    let original = ModelParameters {
        temperature: Some(0.7),
        max_tokens: Some(2048),
        top_p: Some(0.9),
        top_k: Some(50),
        frequency_penalty: Some(0.5),
        presence_penalty: Some(0.3),
        stop_sequences: Some(vec!["STOP".to_string()]),
        seed: Some(42),
        additional,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ModelParameters = serde_json::from_str(&json).unwrap();

    assert_eq!(original.temperature, deserialized.temperature);
    assert_eq!(original.max_tokens, deserialized.max_tokens);
    assert_eq!(original.top_p, deserialized.top_p);
    assert_eq!(original.seed, deserialized.seed);
}

#[test]
fn test_dataset_ref_serialization() {
    let mut filters = HashMap::new();
    filters.insert("category".to_string(), json!("test"));

    let original = DatasetRef {
        dataset_id: DatasetId::new(),
        version: DatasetVersionSelector::Tag("v1.0".to_string()),
        split: Some(DataSplit::Train),
        sample: Some(SampleConfig::default()),
        filters,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: DatasetRef = serde_json::from_str(&json).unwrap();

    assert_eq!(original.dataset_id, deserialized.dataset_id);
    assert_eq!(original.split, deserialized.split);
}

#[test]
fn test_data_split_serialization() {
    let splits = vec![
        (DataSplit::Train, "train"),
        (DataSplit::Validation, "validation"),
        (DataSplit::Test, "test"),
    ];

    for (split, expected) in splits {
        let json = serde_json::to_string(&split).unwrap();
        assert!(json.contains(expected));

        let deserialized: DataSplit = serde_json::from_str(&json).unwrap();
        assert_eq!(split, deserialized);
    }
}

#[test]
fn test_data_split_custom_serialization() {
    let split = DataSplit::Custom("holdout".to_string());
    let json = serde_json::to_string(&split).unwrap();
    let deserialized: DataSplit = serde_json::from_str(&json).unwrap();

    match deserialized {
        DataSplit::Custom(name) => assert_eq!(name, "holdout"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_sample_strategy_serialization() {
    let strategies = vec![
        SampleStrategy::Random,
        SampleStrategy::Sequential,
        SampleStrategy::Stratified,
        SampleStrategy::Custom("weighted".to_string()),
    ];

    for strategy in strategies {
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: SampleStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, deserialized);
    }
}

#[test]
fn test_sample_size_serialization() {
    let sizes = vec![
        SampleSize::All,
        SampleSize::Count(1000),
        SampleSize::Percentage(50),
    ];

    for size in sizes {
        let json = serde_json::to_string(&size).unwrap();
        let deserialized: SampleSize = serde_json::from_str(&json).unwrap();
        assert_eq!(size, deserialized);
    }
}

#[test]
fn test_parameter_value_serialization() {
    let values = vec![
        ParameterValue::String("test".to_string()),
        ParameterValue::Integer(42),
        ParameterValue::Float(3.14),
        ParameterValue::Boolean(true),
    ];

    for value in values {
        let json = serde_json::to_string(&value).unwrap();
        let deserialized: ParameterValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, deserialized);
    }
}

#[test]
fn test_parameter_value_nested_serialization() {
    let array = ParameterValue::Array(vec![
        ParameterValue::Integer(1),
        ParameterValue::Integer(2),
        ParameterValue::Integer(3),
    ]);

    let json = serde_json::to_string(&array).unwrap();
    let deserialized: ParameterValue = serde_json::from_str(&json).unwrap();

    match deserialized {
        ParameterValue::Array(arr) => assert_eq!(arr.len(), 3),
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_search_strategy_serialization() {
    let strategies = vec![
        SearchStrategy::Grid,
        SearchStrategy::Random,
        SearchStrategy::Bayesian,
        SearchStrategy::Hyperband,
        SearchStrategy::Custom("genetic".to_string()),
    ];

    for strategy in strategies {
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: SearchStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, deserialized);
    }
}

#[test]
fn test_gpu_type_serialization() {
    let types = vec![
        GpuType::T4,
        GpuType::V100,
        GpuType::A100,
        GpuType::H100,
        GpuType::Custom("TPU".to_string()),
    ];

    for gpu_type in types {
        let json = serde_json::to_string(&gpu_type).unwrap();
        let deserialized: GpuType = serde_json::from_str(&json).unwrap();
        assert_eq!(gpu_type, deserialized);
    }
}

#[test]
fn test_gpu_requirements_serialization() {
    let original = GpuRequirements {
        gpu_type: Some(GpuType::A100),
        gpu_count: 4,
        gpu_memory_gb: Some(80),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: GpuRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(original.gpu_type, deserialized.gpu_type);
    assert_eq!(original.gpu_count, deserialized.gpu_count);
    assert_eq!(original.gpu_memory_gb, deserialized.gpu_memory_gb);
}

#[test]
fn test_compute_requirements_serialization() {
    let original = ComputeRequirements {
        cpu_cores: Some(16),
        memory_gb: Some(64),
        disk_gb: Some(500),
        gpu: Some(GpuRequirements::default()),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ComputeRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(original.cpu_cores, deserialized.cpu_cores);
    assert_eq!(original.memory_gb, deserialized.memory_gb);
}

#[test]
fn test_resource_requirements_serialization() {
    let original = ResourceRequirements {
        compute: ComputeRequirements::default(),
        timeout_seconds: Some(7200),
        max_retries: Some(3),
        priority: Some(10),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ResourceRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(original.timeout_seconds, deserialized.timeout_seconds);
    assert_eq!(original.max_retries, deserialized.max_retries);
    assert_eq!(original.priority, deserialized.priority);
}

#[test]
fn test_reproducibility_settings_serialization() {
    let original = ReproducibilitySettings {
        random_seed: Some(42),
        deterministic_mode: true,
        track_environment: true,
        track_code_version: true,
        track_dependencies: true,
        snapshot_dataset: true,
        snapshot_model: false,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ReproducibilitySettings = serde_json::from_str(&json).unwrap();

    assert_eq!(original.random_seed, deserialized.random_seed);
    assert_eq!(original.deterministic_mode, deserialized.deterministic_mode);
    assert_eq!(original.snapshot_model, deserialized.snapshot_model);
}

#[test]
fn test_threshold_direction_serialization() {
    let directions = vec![
        ThresholdDirection::Above,
        ThresholdDirection::Below,
        ThresholdDirection::Equal,
        ThresholdDirection::Between,
    ];

    for direction in directions {
        let json = serde_json::to_string(&direction).unwrap();
        let deserialized: ThresholdDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(direction, deserialized);
    }
}

#[test]
fn test_metric_threshold_serialization() {
    let original = MetricThreshold {
        direction: ThresholdDirection::Between,
        value: 0.8,
        max_value: Some(0.95),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: MetricThreshold = serde_json::from_str(&json).unwrap();

    assert_eq!(original.direction, deserialized.direction);
    assert_eq!(original.value, deserialized.value);
    assert_eq!(original.max_value, deserialized.max_value);
}

#[test]
fn test_metric_config_serialization() {
    let mut params = HashMap::new();
    params.insert("average".to_string(), json!("weighted"));

    let original = MetricConfig {
        name: "F1 Score".to_string(),
        metric_type: "f1_score".to_string(),
        parameters: params,
        threshold: Some(MetricThreshold {
            direction: ThresholdDirection::Above,
            value: 0.85,
            max_value: None,
        }),
        weight: Some(1.5),
        tags: vec!["classification".to_string()],
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: MetricConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.metric_type, deserialized.metric_type);
    assert_eq!(original.tags, deserialized.tags);
}

// ===== ID Type Serialization Tests =====

#[test]
fn test_experiment_id_serialization() {
    let original = ExperimentId::new();
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ExperimentId = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_dataset_id_serialization() {
    let original = DatasetId::new();
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: DatasetId = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_semantic_version_serialization() {
    let original = SemanticVersion::new(1, 2, 3)
        .with_pre_release("alpha.1".to_string())
        .with_build_metadata("build.123".to_string());

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: SemanticVersion = serde_json::from_str(&json).unwrap();

    assert_eq!(original.major, deserialized.major);
    assert_eq!(original.minor, deserialized.minor);
    assert_eq!(original.patch, deserialized.patch);
    assert_eq!(original.pre_release, deserialized.pre_release);
    assert_eq!(original.build_metadata, deserialized.build_metadata);
}

#[test]
fn test_content_hash_serialization() {
    let original = ContentHash::from_str("test data for hashing");
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ContentHash = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
    assert_eq!(original.as_str(), deserialized.as_str());
}

// ===== Edge Cases and Error Conditions =====

#[test]
fn test_empty_string_serialization() {
    let original = PromptTemplate::new(
        "Empty vars".to_string(),
        Some("".to_string()), // Empty description
        "No variables here".to_string(),
    );

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PromptTemplate = serde_json::from_str(&json).unwrap();

    assert_eq!(original.description, deserialized.description);
    assert_eq!(deserialized.description, Some("".to_string()));
}

#[test]
fn test_large_numbers_serialization() {
    let dataset = Dataset::new(
        "Large".to_string(),
        None,
        "s3://bucket/large".to_string(),
        i64::MAX,
        json!({}),
    );

    let json_str = serde_json::to_string(&dataset).unwrap();
    let deserialized: Dataset = serde_json::from_str(&json_str).unwrap();

    assert_eq!(dataset.sample_count, deserialized.sample_count);
    assert_eq!(deserialized.sample_count, i64::MAX);
}

#[test]
fn test_json_pretty_print() {
    let config = ExperimentConfig::default();
    let pretty_json = serde_json::to_string_pretty(&config).unwrap();

    assert!(pretty_json.contains('\n'));
    assert!(pretty_json.len() > serde_json::to_string(&config).unwrap().len());

    let deserialized: ExperimentConfig = serde_json::from_str(&pretty_json).unwrap();
    assert_eq!(config.model_configs.len(), deserialized.model_configs.len());
}

#[test]
fn test_nested_json_values() {
    let deeply_nested = json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "data": [1, 2, 3, 4, 5],
                        "text": "deep value"
                    }
                }
            }
        }
    });

    let dataset = Dataset::new(
        "Nested".to_string(),
        None,
        "s3://bucket/nested".to_string(),
        100,
        deeply_nested.clone(),
    );

    let json = serde_json::to_string(&dataset).unwrap();
    let deserialized: Dataset = serde_json::from_str(&json).unwrap();

    assert_eq!(dataset.schema, deserialized.schema);
    assert_eq!(
        deserialized.schema["level1"]["level2"]["level3"]["level4"]["text"],
        "deep value"
    );
}

#[test]
fn test_special_float_values() {
    let params = ModelParameters {
        temperature: Some(0.0),
        max_tokens: Some(1),
        top_p: Some(1.0),
        top_k: None,
        frequency_penalty: Some(-2.0),
        presence_penalty: Some(2.0),
        stop_sequences: None,
        seed: None,
        additional: HashMap::new(),
    };

    let json = serde_json::to_string(&params).unwrap();
    let deserialized: ModelParameters = serde_json::from_str(&json).unwrap();

    assert_eq!(params.temperature, deserialized.temperature);
    assert_eq!(params.frequency_penalty, deserialized.frequency_penalty);
}

#[test]
fn test_optional_fields_serialization() {
    // Test with all None values
    let model = Model::new(
        "Test".to_string(),
        ModelProvider::Local,
        "test-id".to_string(),
        None, // version is None
        json!({}),
    );

    let json = serde_json::to_string(&model).unwrap();
    let deserialized: Model = serde_json::from_str(&json).unwrap();

    assert_eq!(model.version, deserialized.version);
    assert!(deserialized.version.is_none());
}

#[test]
fn test_dataset_version_selector_variants_serialization() {
    let selectors = vec![
        DatasetVersionSelector::Latest,
        DatasetVersionSelector::Specific(DatasetVersionId::new()),
        DatasetVersionSelector::Tag("v1.0.0".to_string()),
        DatasetVersionSelector::SemanticVersion(SemanticVersion::new(1, 0, 0)),
    ];

    for selector in selectors {
        let json = serde_json::to_string(&selector).unwrap();
        let deserialized: DatasetVersionSelector = serde_json::from_str(&json).unwrap();
        assert_eq!(selector, deserialized);
    }
}

#[test]
fn test_complete_experiment_config_roundtrip() {
    let mut config = ExperimentConfig::default();

    // Add model config
    let model_config = ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "gpt-4".to_string(),
        model_version: Some("turbo".to_string()),
        parameters: ModelParameters {
            temperature: Some(0.7),
            max_tokens: Some(2048),
            top_p: Some(0.9),
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Some(vec!["STOP".to_string()]),
            seed: Some(42),
            additional: HashMap::new(),
        },
        system_prompt: Some("You are helpful".to_string()),
        metadata: HashMap::new(),
    };
    config.model_configs.push(model_config);

    // Add dataset ref
    let dataset_ref = DatasetRef {
        dataset_id: DatasetId::new(),
        version: DatasetVersionSelector::Latest,
        split: Some(DataSplit::Train),
        sample: Some(SampleConfig {
            strategy: SampleStrategy::Random,
            size: SampleSize::Percentage(80),
            seed: Some(42),
            stratify_by: None,
        }),
        filters: HashMap::new(),
    };
    config.dataset_refs.push(dataset_ref);

    // Add metric config
    let metric_config = MetricConfig {
        name: "Accuracy".to_string(),
        metric_type: "accuracy".to_string(),
        parameters: HashMap::new(),
        threshold: Some(MetricThreshold {
            direction: ThresholdDirection::Above,
            value: 0.9,
            max_value: None,
        }),
        weight: Some(1.0),
        tags: vec!["primary".to_string()],
    };
    config.metric_configs.push(metric_config);

    // Serialize and deserialize
    let json = serde_json::to_string_pretty(&config).unwrap();
    let deserialized: ExperimentConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.model_configs.len(), deserialized.model_configs.len());
    assert_eq!(config.dataset_refs.len(), deserialized.dataset_refs.len());
    assert_eq!(config.metric_configs.len(), deserialized.metric_configs.len());
    assert_eq!(
        config.model_configs[0].model_name,
        deserialized.model_configs[0].model_name
    );
    assert_eq!(
        config.metric_configs[0].name,
        deserialized.metric_configs[0].name
    );
}
