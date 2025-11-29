use llm_research_core::domain::*;
use llm_research_core::error::CoreError;
use pretty_assertions::assert_eq;
use proptest::prelude::*;
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

// ===== Model Tests =====

#[test]
fn test_model_creation() {
    let config = json!({
        "api_key": "test_key",
        "endpoint": "https://api.example.com"
    });

    let model = Model::new(
        "GPT-4".to_string(),
        ModelProvider::OpenAI,
        "gpt-4-turbo".to_string(),
        Some("2024-01-25".to_string()),
        config.clone(),
    );

    assert_eq!(model.name, "GPT-4");
    assert_eq!(model.provider, ModelProvider::OpenAI);
    assert_eq!(model.model_identifier, "gpt-4-turbo");
    assert_eq!(model.version, Some("2024-01-25".to_string()));
    assert_eq!(model.config, config);
    assert!(model.created_at <= model.updated_at);
}

#[test]
fn test_model_validation() {
    let config = json!({});

    // Valid model
    let valid_model = Model::new(
        "Test Model".to_string(),
        ModelProvider::Anthropic,
        "claude-3-opus".to_string(),
        None,
        config.clone(),
    );
    assert!(valid_model.validate().is_ok());

    // Invalid: empty name
    let invalid_model = Model::new(
        "".to_string(),
        ModelProvider::Anthropic,
        "claude-3-opus".to_string(),
        None,
        config.clone(),
    );
    assert!(invalid_model.validate().is_err());

    // Invalid: empty model_identifier
    let invalid_model = Model::new(
        "Test".to_string(),
        ModelProvider::Anthropic,
        "".to_string(),
        None,
        config,
    );
    assert!(invalid_model.validate().is_err());
}

#[rstest]
#[case(ModelProvider::OpenAI)]
#[case(ModelProvider::Anthropic)]
#[case(ModelProvider::Google)]
#[case(ModelProvider::Cohere)]
#[case(ModelProvider::HuggingFace)]
#[case(ModelProvider::Azure)]
#[case(ModelProvider::AWS)]
#[case(ModelProvider::Local)]
#[case(ModelProvider::Custom)]
fn test_model_providers(#[case] provider: ModelProvider) {
    let config = json!({});
    let model = Model::new(
        "Test".to_string(),
        provider.clone(),
        "test-model".to_string(),
        None,
        config,
    );
    assert_eq!(model.provider, provider);
}

#[test]
fn test_model_with_complex_config() {
    let config = json!({
        "api_key": "sk-test-key-12345",
        "endpoint": "https://api.openai.com/v1",
        "timeout_seconds": 60,
        "max_retries": 3,
        "headers": {
            "X-Custom-Header": "value"
        },
        "rate_limit": {
            "requests_per_minute": 60,
            "tokens_per_minute": 100000
        }
    });

    let model = Model::new(
        "GPT-4 Custom".to_string(),
        ModelProvider::OpenAI,
        "gpt-4".to_string(),
        Some("1.0.0".to_string()),
        config.clone(),
    );

    assert_eq!(model.config["timeout_seconds"], 60);
    assert_eq!(model.config["headers"]["X-Custom-Header"], "value");
    assert_eq!(model.config["rate_limit"]["requests_per_minute"], 60);
}

// ===== Dataset Tests =====

#[test]
fn test_dataset_creation() {
    let schema = json!({
        "type": "object",
        "properties": {
            "input": {"type": "string"},
            "output": {"type": "string"}
        }
    });

    let dataset = Dataset::new(
        "Test Dataset".to_string(),
        Some("A test dataset".to_string()),
        "s3://bucket/datasets/test.parquet".to_string(),
        1000,
        schema.clone(),
    );

    assert_eq!(dataset.name, "Test Dataset");
    assert_eq!(dataset.description, Some("A test dataset".to_string()));
    assert_eq!(dataset.s3_path, "s3://bucket/datasets/test.parquet");
    assert_eq!(dataset.sample_count, 1000);
    assert_eq!(dataset.schema, schema);
    assert!(dataset.created_at <= dataset.updated_at);
}

#[test]
fn test_dataset_validation() {
    let schema = json!({});

    // Valid dataset
    let valid = Dataset::new(
        "Valid Dataset".to_string(),
        None,
        "s3://bucket/data.json".to_string(),
        100,
        schema.clone(),
    );
    assert!(valid.validate().is_ok());

    // Invalid: empty name
    let invalid = Dataset::new(
        "".to_string(),
        None,
        "s3://bucket/data.json".to_string(),
        100,
        schema,
    );
    assert!(invalid.validate().is_err());
}

#[test]
fn test_dataset_with_zero_samples() {
    let schema = json!({});
    let dataset = Dataset::new(
        "Empty Dataset".to_string(),
        None,
        "s3://bucket/empty.json".to_string(),
        0,
        schema,
    );
    assert_eq!(dataset.sample_count, 0);
}

#[test]
fn test_dataset_with_large_sample_count() {
    let schema = json!({});
    let dataset = Dataset::new(
        "Large Dataset".to_string(),
        None,
        "s3://bucket/large.parquet".to_string(),
        1_000_000_000,
        schema,
    );
    assert_eq!(dataset.sample_count, 1_000_000_000);
}

#[test]
fn test_dataset_complex_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "input": {
                "type": "object",
                "properties": {
                    "question": {"type": "string"},
                    "context": {"type": "array", "items": {"type": "string"}},
                    "metadata": {"type": "object"}
                }
            },
            "expected_output": {
                "type": "string"
            },
            "difficulty": {
                "type": "string",
                "enum": ["easy", "medium", "hard"]
            }
        },
        "required": ["input", "expected_output"]
    });

    let dataset = Dataset::new(
        "QA Dataset".to_string(),
        Some("Question answering dataset with context".to_string()),
        "s3://datasets/qa/v1.parquet".to_string(),
        5000,
        schema.clone(),
    );

    assert_eq!(dataset.schema, schema);
    assert!(dataset.schema["properties"]["input"]["properties"]["context"]["items"].is_object());
}

// ===== DatasetSample Tests =====

#[test]
fn test_dataset_sample_creation() {
    let dataset_id = Uuid::new_v4();
    let input = json!({"text": "What is the capital of France?"});
    let expected = json!({"answer": "Paris"});
    let metadata = json!({"difficulty": "easy", "category": "geography"});

    let sample = DatasetSample::new(
        dataset_id,
        42,
        input.clone(),
        Some(expected.clone()),
        metadata.clone(),
    );

    assert_eq!(sample.dataset_id, dataset_id);
    assert_eq!(sample.index, 42);
    assert_eq!(sample.input, input);
    assert_eq!(sample.expected_output, Some(expected));
    assert_eq!(sample.metadata, metadata);
}

#[test]
fn test_dataset_sample_without_expected_output() {
    let dataset_id = Uuid::new_v4();
    let input = json!({"prompt": "Generate a story about..."});
    let metadata = json!({});

    let sample = DatasetSample::new(
        dataset_id,
        0,
        input.clone(),
        None,
        metadata.clone(),
    );

    assert_eq!(sample.expected_output, None);
    assert_eq!(sample.input, input);
}

#[test]
fn test_dataset_sample_with_complex_data() {
    let dataset_id = Uuid::new_v4();
    let input = json!({
        "messages": [
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": "Explain quantum computing"}
        ],
        "parameters": {
            "temperature": 0.7,
            "max_tokens": 500
        }
    });

    let expected = json!({
        "content": "Quantum computing is...",
        "tokens": 450,
        "finish_reason": "stop"
    });

    let metadata = json!({
        "source": "human_annotated",
        "quality_score": 0.95,
        "tags": ["science", "technology", "quantum"]
    });

    let sample = DatasetSample::new(
        dataset_id,
        100,
        input.clone(),
        Some(expected.clone()),
        metadata.clone(),
    );

    assert_eq!(sample.input["messages"][0]["role"], "system");
    assert_eq!(sample.expected_output.as_ref().unwrap()["tokens"], 450);
    assert_eq!(sample.metadata["quality_score"], 0.95);
}

// ===== PromptTemplate Tests =====

#[test]
fn test_prompt_template_creation() {
    let template = "Hello {{name}}, welcome to {{place}}!";
    let prompt = PromptTemplate::new(
        "Greeting Template".to_string(),
        Some("A simple greeting".to_string()),
        template.to_string(),
    );

    assert_eq!(prompt.name, "Greeting Template");
    assert_eq!(prompt.description, Some("A simple greeting".to_string()));
    assert_eq!(prompt.template, template);
    assert_eq!(prompt.variables.len(), 2);
    assert!(prompt.variables.contains(&"name".to_string()));
    assert!(prompt.variables.contains(&"place".to_string()));
    assert_eq!(prompt.version, 1);
}

#[test]
fn test_prompt_template_validation() {
    // Valid
    let valid = PromptTemplate::new(
        "Test".to_string(),
        None,
        "Template {{var}}".to_string(),
    );
    assert!(valid.validate().is_ok());

    // Invalid: empty name
    let mut invalid = PromptTemplate::new(
        "".to_string(),
        None,
        "Template".to_string(),
    );
    invalid.name = "".to_string();
    assert!(invalid.validate().is_err());

    // Invalid: empty template
    let mut invalid = PromptTemplate::new(
        "Test".to_string(),
        None,
        "Template".to_string(),
    );
    invalid.template = "".to_string();
    assert!(invalid.validate().is_err());
}

#[test]
fn test_prompt_template_variable_extraction() {
    let template = "{{var1}} and {{var2}} but not {var3} or {{var1}} again";
    let prompt = PromptTemplate::new(
        "Test".to_string(),
        None,
        template.to_string(),
    );

    // Should extract var1 and var2, but may have duplicates
    assert!(prompt.variables.contains(&"var1".to_string()));
    assert!(prompt.variables.contains(&"var2".to_string()));
    assert!(!prompt.variables.contains(&"var3".to_string()));
}

#[test]
fn test_prompt_template_render_success() {
    let template = "Dear {{name}},\n\nYour order #{{order_id}} is ready.";
    let prompt = PromptTemplate::new(
        "Order Notification".to_string(),
        None,
        template.to_string(),
    );

    let context = json!({
        "name": "Alice",
        "order_id": 12345
    });

    let result = prompt.render(&context);
    assert!(result.is_ok());
    let rendered = result.unwrap();
    assert!(rendered.contains("Dear Alice"));
    assert!(rendered.contains("order #12345"));
}

#[test]
fn test_prompt_template_render_missing_variable() {
    let template = "Hello {{name}}!";
    let prompt = PromptTemplate::new(
        "Test".to_string(),
        None,
        template.to_string(),
    );

    let context = json!({
        "wrong_key": "value"
    });

    let result = prompt.render(&context);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing variable"));
}

#[test]
fn test_prompt_template_render_with_non_string_values() {
    let template = "Score: {{score}}, Passed: {{passed}}";
    let prompt = PromptTemplate::new(
        "Test".to_string(),
        None,
        template.to_string(),
    );

    let context = json!({
        "score": 95,
        "passed": true
    });

    let result = prompt.render(&context);
    assert!(result.is_ok());
    let rendered = result.unwrap();
    assert!(rendered.contains("Score: 95"));
    assert!(rendered.contains("Passed: true"));
}

#[test]
fn test_prompt_template_no_variables() {
    let template = "This is a static template with no variables.";
    let prompt = PromptTemplate::new(
        "Static".to_string(),
        None,
        template.to_string(),
    );

    assert_eq!(prompt.variables.len(), 0);

    let context = json!({});
    let result = prompt.render(&context);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), template);
}

#[test]
fn test_prompt_template_complex() {
    let template = r#"System: {{system_prompt}}

User: {{user_query}}

Context:
{{context}}

Instructions:
1. {{instruction_1}}
2. {{instruction_2}}

Format: {{output_format}}"#;

    let prompt = PromptTemplate::new(
        "Complex Prompt".to_string(),
        Some("Multi-section prompt template".to_string()),
        template.to_string(),
    );

    assert_eq!(prompt.variables.len(), 6);

    let context = json!({
        "system_prompt": "You are an AI assistant",
        "user_query": "What is ML?",
        "context": "Machine Learning context...",
        "instruction_1": "Be concise",
        "instruction_2": "Provide examples",
        "output_format": "JSON"
    });

    let result = prompt.render(&context);
    assert!(result.is_ok());
    let rendered = result.unwrap();
    assert!(rendered.contains("System: You are an AI assistant"));
    assert!(rendered.contains("Format: JSON"));
}

// ===== Evaluation Tests =====

#[test]
fn test_evaluation_creation() {
    let experiment_id = Uuid::new_v4();
    let sample_id = Uuid::new_v4();
    let metrics = json!({
        "accuracy": 0.95,
        "precision": 0.93,
        "recall": 0.97
    });

    let eval = Evaluation::new(
        experiment_id,
        sample_id,
        "What is 2+2?".to_string(),
        "4".to_string(),
        Some("4".to_string()),
        150,
        10,
        Some(rust_decimal::Decimal::new(15, 5)), // 0.00015
        metrics.clone(),
    );

    assert_eq!(eval.experiment_id, experiment_id);
    assert_eq!(eval.sample_id, sample_id);
    assert_eq!(eval.input, "What is 2+2?");
    assert_eq!(eval.output, "4");
    assert_eq!(eval.expected_output, Some("4".to_string()));
    assert_eq!(eval.latency_ms, 150);
    assert_eq!(eval.token_count, 10);
    assert!(eval.cost.is_some());
    assert_eq!(eval.metrics, metrics);
}

#[test]
fn test_evaluation_without_expected_output() {
    let experiment_id = Uuid::new_v4();
    let sample_id = Uuid::new_v4();

    let eval = Evaluation::new(
        experiment_id,
        sample_id,
        "Generate a poem".to_string(),
        "Roses are red...".to_string(),
        None,
        500,
        50,
        None,
        json!({}),
    );

    assert_eq!(eval.expected_output, None);
    assert_eq!(eval.cost, None);
}

#[test]
fn test_evaluation_with_high_latency() {
    let experiment_id = Uuid::new_v4();
    let sample_id = Uuid::new_v4();

    let eval = Evaluation::new(
        experiment_id,
        sample_id,
        "Complex task".to_string(),
        "Result".to_string(),
        None,
        30_000, // 30 seconds
        1000,
        None,
        json!({"latency_class": "slow"}),
    );

    assert_eq!(eval.latency_ms, 30_000);
}

#[test]
fn test_evaluation_metrics() {
    let metrics = EvaluationMetrics {
        accuracy: Some(rust_decimal::Decimal::new(95, 2)), // 0.95
        precision: Some(rust_decimal::Decimal::new(93, 2)), // 0.93
        recall: Some(rust_decimal::Decimal::new(97, 2)), // 0.97
        f1_score: Some(rust_decimal::Decimal::new(95, 2)), // 0.95
        bleu_score: Some(rust_decimal::Decimal::new(82, 2)), // 0.82
        rouge_scores: Some(json!({
            "rouge-1": 0.85,
            "rouge-2": 0.78,
            "rouge-l": 0.81
        })),
        custom_metrics: json!({
            "perplexity": 15.3,
            "diversity": 0.72
        }),
    };

    assert!(metrics.accuracy.is_some());
    assert!(metrics.bleu_score.is_some());
    assert_eq!(metrics.custom_metrics["perplexity"], 15.3);
}

// ===== ExperimentConfig Tests =====

#[test]
fn test_experiment_config_default() {
    let config = ExperimentConfig::default();

    assert!(config.model_configs.is_empty());
    assert!(config.dataset_refs.is_empty());
    assert!(config.metric_configs.is_empty());
    assert_eq!(config.parameters.concurrent_trials, Some(1));
    assert_eq!(config.resource_requirements.timeout_seconds, Some(3600));
    assert!(config.reproducibility_settings.deterministic_mode);
}

#[test]
fn test_experiment_config_with_models() {
    let mut config = ExperimentConfig::default();

    let model_config = ModelConfig {
        provider: ModelProvider::OpenAI,
        model_name: "gpt-4".to_string(),
        model_version: Some("turbo".to_string()),
        parameters: ModelParameters::default(),
        system_prompt: Some("You are helpful".to_string()),
        metadata: HashMap::new(),
    };

    config.model_configs.push(model_config);
    assert_eq!(config.model_configs.len(), 1);
}

#[test]
fn test_model_parameters_default() {
    let params = ModelParameters::default();

    assert_eq!(params.temperature, Some(1.0));
    assert_eq!(params.max_tokens, Some(1024));
    assert_eq!(params.top_p, None);
    assert_eq!(params.top_k, None);
    assert!(params.additional.is_empty());
}

#[test]
fn test_model_parameters_custom() {
    let mut additional = HashMap::new();
    additional.insert("custom_param".to_string(), json!("value"));

    let params = ModelParameters {
        temperature: Some(0.7),
        max_tokens: Some(2048),
        top_p: Some(0.9),
        top_k: Some(50),
        frequency_penalty: Some(0.5),
        presence_penalty: Some(0.3),
        stop_sequences: Some(vec!["STOP".to_string(), "END".to_string()]),
        seed: Some(42),
        additional,
    };

    assert_eq!(params.temperature, Some(0.7));
    assert_eq!(params.stop_sequences.as_ref().unwrap().len(), 2);
    assert_eq!(params.seed, Some(42));
    assert!(params.additional.contains_key("custom_param"));
}

// ===== DatasetRef and Configuration Tests =====

#[test]
fn test_dataset_version_selector() {
    let latest = DatasetVersionSelector::Latest;
    let specific = DatasetVersionSelector::Specific(DatasetVersionId::new());
    let tag = DatasetVersionSelector::Tag("v1.0".to_string());
    let semver = DatasetVersionSelector::SemanticVersion(SemanticVersion::new(1, 0, 0));

    assert_ne!(latest, specific);
    assert_ne!(tag, semver);
}

#[rstest]
#[case(DataSplit::Train)]
#[case(DataSplit::Validation)]
#[case(DataSplit::Test)]
#[case(DataSplit::Custom("holdout".to_string()))]
fn test_data_split_variants(#[case] split: DataSplit) {
    let dataset_ref = DatasetRef {
        dataset_id: DatasetId::new(),
        version: DatasetVersionSelector::Latest,
        split: Some(split.clone()),
        sample: None,
        filters: HashMap::new(),
    };

    assert_eq!(dataset_ref.split, Some(split));
}

#[test]
fn test_sample_strategy() {
    let random = SampleStrategy::Random;
    let sequential = SampleStrategy::Sequential;
    let stratified = SampleStrategy::Stratified;
    let custom = SampleStrategy::Custom("weighted".to_string());

    assert_ne!(random, sequential);
    assert_ne!(stratified, custom);
}

#[test]
fn test_sample_size_variants() {
    let all = SampleSize::All;
    let count = SampleSize::Count(1000);
    let percentage = SampleSize::Percentage(50);

    assert_ne!(all, count);
    assert_ne!(count, percentage);
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
fn test_sample_config_custom() {
    let config = SampleConfig {
        strategy: SampleStrategy::Stratified,
        size: SampleSize::Percentage(20),
        seed: Some(42),
        stratify_by: Some("label".to_string()),
    };

    assert_eq!(config.strategy, SampleStrategy::Stratified);
    match config.size {
        SampleSize::Percentage(p) => assert_eq!(p, 20),
        _ => panic!("Expected Percentage variant"),
    }
}

// ===== Parameter Value Tests =====

#[test]
fn test_parameter_value_conversions() {
    let string_val = ParameterValue::from("test".to_string());
    let int_val = ParameterValue::from(42i64);
    let float_val = ParameterValue::from(3.14f64);
    let bool_val = ParameterValue::from(true);

    match string_val {
        ParameterValue::String(s) => assert_eq!(s, "test"),
        _ => panic!("Expected String"),
    }

    match int_val {
        ParameterValue::Integer(i) => assert_eq!(i, 42),
        _ => panic!("Expected Integer"),
    }

    match float_val {
        ParameterValue::Float(f) => assert_eq!(f, 3.14),
        _ => panic!("Expected Float"),
    }

    match bool_val {
        ParameterValue::Boolean(b) => assert!(b),
        _ => panic!("Expected Boolean"),
    }
}

#[test]
fn test_parameter_value_nested() {
    let array = ParameterValue::Array(vec![
        ParameterValue::from(1i64),
        ParameterValue::from(2i64),
        ParameterValue::from(3i64),
    ]);

    match array {
        ParameterValue::Array(arr) => assert_eq!(arr.len(), 3),
        _ => panic!("Expected Array"),
    }

    let mut map = HashMap::new();
    map.insert("key1".to_string(), ParameterValue::from("value1".to_string()));
    map.insert("key2".to_string(), ParameterValue::from(42i64));

    let object = ParameterValue::Object(map);
    match object {
        ParameterValue::Object(obj) => assert_eq!(obj.len(), 2),
        _ => panic!("Expected Object"),
    }
}

// ===== Search and Optimization Tests =====

#[rstest]
#[case(SearchStrategy::Grid)]
#[case(SearchStrategy::Random)]
#[case(SearchStrategy::Bayesian)]
#[case(SearchStrategy::Hyperband)]
#[case(SearchStrategy::Custom("genetic".to_string()))]
fn test_search_strategies(#[case] strategy: SearchStrategy) {
    let params = ExperimentParameters {
        fixed: HashMap::new(),
        search_spaces: Vec::new(),
        search_strategy: Some(strategy.clone()),
        max_trials: Some(100),
        concurrent_trials: Some(4),
    };

    assert_eq!(params.search_strategy, Some(strategy));
}

#[test]
fn test_search_space() {
    let space = SearchSpace {
        parameter_name: "learning_rate".to_string(),
        values: vec![
            ParameterValue::from(0.001),
            ParameterValue::from(0.01),
            ParameterValue::from(0.1),
        ],
        distribution: Some("log-uniform".to_string()),
    };

    assert_eq!(space.parameter_name, "learning_rate");
    assert_eq!(space.values.len(), 3);
    assert_eq!(space.distribution, Some("log-uniform".to_string()));
}

#[test]
fn test_experiment_parameters_default() {
    let params = ExperimentParameters::default();

    assert!(params.fixed.is_empty());
    assert!(params.search_spaces.is_empty());
    assert_eq!(params.search_strategy, None);
    assert_eq!(params.max_trials, None);
    assert_eq!(params.concurrent_trials, Some(1));
}

// ===== Resource Requirements Tests =====

#[test]
fn test_gpu_requirements_default() {
    let gpu = GpuRequirements::default();

    assert_eq!(gpu.gpu_type, None);
    assert_eq!(gpu.gpu_count, 0);
    assert_eq!(gpu.gpu_memory_gb, None);
}

#[rstest]
#[case(GpuType::T4)]
#[case(GpuType::V100)]
#[case(GpuType::A100)]
#[case(GpuType::H100)]
#[case(GpuType::Custom("TPU-v4".to_string()))]
fn test_gpu_types(#[case] gpu_type: GpuType) {
    let gpu = GpuRequirements {
        gpu_type: Some(gpu_type.clone()),
        gpu_count: 8,
        gpu_memory_gb: Some(80),
    };

    assert_eq!(gpu.gpu_type, Some(gpu_type));
    assert_eq!(gpu.gpu_count, 8);
}

#[test]
fn test_compute_requirements_default() {
    let compute = ComputeRequirements::default();

    assert_eq!(compute.cpu_cores, Some(1));
    assert_eq!(compute.memory_gb, Some(4));
    assert_eq!(compute.disk_gb, Some(10));
    assert_eq!(compute.gpu, None);
}

#[test]
fn test_compute_requirements_with_gpu() {
    let gpu = GpuRequirements {
        gpu_type: Some(GpuType::A100),
        gpu_count: 4,
        gpu_memory_gb: Some(80),
    };

    let compute = ComputeRequirements {
        cpu_cores: Some(32),
        memory_gb: Some(256),
        disk_gb: Some(1000),
        gpu: Some(gpu),
    };

    assert!(compute.gpu.is_some());
    assert_eq!(compute.gpu.as_ref().unwrap().gpu_count, 4);
}

#[test]
fn test_resource_requirements_default() {
    let resources = ResourceRequirements::default();

    assert_eq!(resources.timeout_seconds, Some(3600));
    assert_eq!(resources.max_retries, Some(0));
    assert_eq!(resources.priority, Some(5));
}

// ===== Reproducibility Settings Tests =====

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
fn test_reproducibility_settings_custom() {
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

// ===== Metric Configuration Tests =====

#[rstest]
#[case(ThresholdDirection::Above)]
#[case(ThresholdDirection::Below)]
#[case(ThresholdDirection::Equal)]
#[case(ThresholdDirection::Between)]
fn test_threshold_directions(#[case] direction: ThresholdDirection) {
    let threshold = MetricThreshold {
        direction: direction.clone(),
        value: 0.9,
        max_value: None,
    };

    assert_eq!(threshold.direction, direction);
}

#[test]
fn test_metric_threshold_between() {
    let threshold = MetricThreshold {
        direction: ThresholdDirection::Between,
        value: 0.8,
        max_value: Some(0.95),
    };

    assert_eq!(threshold.direction, ThresholdDirection::Between);
    assert_eq!(threshold.value, 0.8);
    assert_eq!(threshold.max_value, Some(0.95));
}

#[test]
fn test_metric_config() {
    let mut params = HashMap::new();
    params.insert("average".to_string(), json!("weighted"));

    let metric = MetricConfig {
        name: "F1 Score".to_string(),
        metric_type: "f1_score".to_string(),
        parameters: params,
        threshold: Some(MetricThreshold {
            direction: ThresholdDirection::Above,
            value: 0.85,
            max_value: None,
        }),
        weight: Some(1.5),
        tags: vec!["classification".to_string(), "primary".to_string()],
    };

    assert_eq!(metric.name, "F1 Score");
    assert!(metric.threshold.is_some());
    assert_eq!(metric.weight, Some(1.5));
    assert_eq!(metric.tags.len(), 2);
}

#[test]
fn test_metric_config_validation() {
    let metric = MetricConfig {
        name: "Accuracy".to_string(),
        metric_type: "accuracy".to_string(),
        parameters: HashMap::new(),
        threshold: None,
        weight: None,
        tags: Vec::new(),
    };

    assert!(metric.validate().is_ok());

    // Invalid: empty name
    let mut invalid = metric.clone();
    invalid.name = "".to_string();
    assert!(invalid.validate().is_err());
}

// ===== CoreError Tests =====

#[test]
fn test_core_error_variants() {
    let validation_err = CoreError::Validation("Invalid input".to_string());
    assert_eq!(validation_err.to_string(), "Validation error: Invalid input");

    let not_found_err = CoreError::NotFound("Resource not found".to_string());
    assert_eq!(not_found_err.to_string(), "Not found: Resource not found");

    let exists_err = CoreError::AlreadyExists("Duplicate key".to_string());
    assert_eq!(exists_err.to_string(), "Already exists: Duplicate key");

    let state_err = CoreError::InvalidState("Cannot transition".to_string());
    assert_eq!(state_err.to_string(), "Invalid state: Cannot transition");

    let unauth_err = CoreError::Unauthorized("No permission".to_string());
    assert_eq!(unauth_err.to_string(), "Unauthorized: No permission");

    let internal_err = CoreError::Internal("System error".to_string());
    assert_eq!(internal_err.to_string(), "Internal error: System error");

    let db_err = CoreError::Database("Connection failed".to_string());
    assert_eq!(db_err.to_string(), "Database error: Connection failed");

    let ser_err = CoreError::Serialization("Parse error".to_string());
    assert_eq!(ser_err.to_string(), "Serialization error: Parse error");
}

#[test]
fn test_core_error_from_serde() {
    let json_str = "invalid json{";
    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(json_str);

    if let Err(e) = parse_result {
        let core_err: CoreError = e.into();
        match core_err {
            CoreError::Serialization(_) => {},
            _ => panic!("Expected Serialization error"),
        }
    }
}

// ===== Property-based Tests =====

proptest! {
    #[test]
    fn test_model_name_arbitrary_strings(name in "\\PC{1,255}") {
        let config = json!({});
        let model = Model::new(
            name.clone(),
            ModelProvider::OpenAI,
            "test-model".to_string(),
            None,
            config,
        );
        assert_eq!(model.name, name);
    }

    #[test]
    fn test_dataset_sample_count_positive(count in 0i64..1_000_000i64) {
        let schema = json!({});
        let dataset = Dataset::new(
            "Test".to_string(),
            None,
            "s3://bucket/data".to_string(),
            count,
            schema,
        );
        assert_eq!(dataset.sample_count, count);
    }

    #[test]
    fn test_model_parameters_temperature_range(temp in 0.0f64..2.0f64) {
        let params = ModelParameters {
            temperature: Some(temp),
            max_tokens: Some(1024),
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            seed: None,
            additional: HashMap::new(),
        };
        assert_eq!(params.temperature, Some(temp));
    }

    #[test]
    fn test_sample_size_percentage_valid(pct in 0u8..=100u8) {
        let size = SampleSize::Percentage(pct);
        match size {
            SampleSize::Percentage(p) => assert!(p <= 100),
            _ => panic!("Expected Percentage"),
        }
    }
}
