use chrono::Utc;
use fake::{Fake, Faker};
use llm_research_core::domain::{
    Experiment, ExperimentConfig, ExperimentStatus, Model, ModelProvider, Dataset,
    PromptTemplate, Evaluation, ExperimentRun, RunStatus,
    ids::{ExperimentId, UserId, RunId},
    config::ParameterValue,
    run::{EnvironmentSnapshot, RunMetrics, LogSummary, RunError},
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

/// Generate a random Experiment for testing
pub fn create_test_experiment() -> Experiment {
    let id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    Experiment {
        id: ExperimentId(id),
        name: Faker.fake::<String>(),
        description: Some(Faker.fake::<String>()),
        hypothesis: Some(Faker.fake::<String>()),
        owner_id: UserId(owner_id),
        collaborators: vec![UserId(Uuid::new_v4()), UserId(Uuid::new_v4())],
        tags: vec!["test".to_string(), "experiment".to_string()],
        status: ExperimentStatus::Draft,
        config: ExperimentConfig::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        archived_at: None,
        metadata: HashMap::new(),
    }
}

/// Generate a random Experiment with specific status
pub fn create_test_experiment_with_status(status: ExperimentStatus) -> Experiment {
    let mut experiment = create_test_experiment();
    experiment.status = status;
    experiment
}

/// Generate a random Model for testing
pub fn create_test_model() -> Model {
    Model {
        id: Uuid::new_v4(),
        name: Faker.fake::<String>(),
        provider: ModelProvider::OpenAI,
        model_identifier: "gpt-4".to_string(),
        version: Some("2024-01".to_string()),
        config: serde_json::json!({
            "temperature": 0.7,
            "max_tokens": 2048
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Generate a random Model with specific provider
pub fn create_test_model_with_provider(provider: ModelProvider) -> Model {
    let mut model = create_test_model();
    model.provider = provider;
    model
}

/// Generate a random Dataset for testing
pub fn create_test_dataset() -> Dataset {
    Dataset {
        id: Uuid::new_v4(),
        name: Faker.fake::<String>(),
        description: Some(Faker.fake::<String>()),
        s3_path: format!("s3://bucket/datasets/{}", Uuid::new_v4()),
        sample_count: (100..10000).fake(),
        schema: serde_json::json!({
            "fields": [
                {"name": "input", "type": "string"},
                {"name": "output", "type": "string"}
            ]
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Generate a random PromptTemplate for testing
pub fn create_test_prompt_template() -> PromptTemplate {
    PromptTemplate {
        id: Uuid::new_v4(),
        name: Faker.fake::<String>(),
        description: Some(Faker.fake::<String>()),
        template: "Hello {{name}}, how are you?".to_string(),
        variables: vec!["name".to_string()],
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Generate a random Evaluation for testing
pub fn create_test_evaluation() -> Evaluation {
    Evaluation {
        id: Uuid::new_v4(),
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: Faker.fake::<String>(),
        output: Faker.fake::<String>(),
        expected_output: Some(Faker.fake::<String>()),
        latency_ms: (10..5000).fake(),
        token_count: (10..1000).fake(),
        cost: Some(Decimal::new(123, 2)), // $1.23
        metrics: serde_json::json!({
            "accuracy": 0.95,
            "f1_score": 0.92
        }),
        created_at: Utc::now(),
    }
}

/// Generate a random ExperimentRun for testing
pub fn create_test_run() -> ExperimentRun {
    let mut parameters = HashMap::new();
    parameters.insert(
        "temperature".to_string(),
        ParameterValue::Float(0.7),
    );
    parameters.insert(
        "max_tokens".to_string(),
        ParameterValue::Integer(2048),
    );

    use llm_research_core::domain::run::{OsInfo, HardwareInfo, RuntimeInfo, GitState};

    ExperimentRun {
        id: RunId(Uuid::new_v4()),
        experiment_id: ExperimentId(Uuid::new_v4()),
        run_number: 1,
        name: Faker.fake::<String>(),
        status: RunStatus::Pending,
        parameters,
        environment: Some(EnvironmentSnapshot {
            os: OsInfo {
                name: "Linux".to_string(),
                version: "5.15.0".to_string(),
                architecture: "x86_64".to_string(),
                hostname: Some("test-host".to_string()),
            },
            hardware: HardwareInfo {
                cpu_model: Some("Intel Core i7".to_string()),
                cpu_cores: Some(8),
                memory_total_gb: Some(16),
                gpu_model: None,
                gpu_count: None,
                gpu_memory_gb: None,
            },
            runtime: RuntimeInfo {
                python_version: Some("3.11".to_string()),
                cuda_version: None,
                pytorch_version: None,
                tensorflow_version: None,
                transformers_version: None,
                additional: HashMap::new(),
            },
            dependencies: vec![],
            git_state: Some(GitState {
                repository_url: None,
                branch: Some("main".to_string()),
                commit_hash: Some("abc123".to_string()),
                is_dirty: false,
                diff: None,
            }),
            container: None,
            environment_variables: HashMap::new(),
            captured_at: Utc::now(),
        }),
        metrics: RunMetrics::default(),
        artifacts: vec![],
        logs: LogSummary::default(),
        parent_run_id: None,
        tags: vec!["test".to_string()],
        started_at: None,
        ended_at: None,
        created_at: Utc::now(),
        created_by: UserId(Uuid::new_v4()),
        error: None,
        metadata: HashMap::new(),
    }
}

/// Generate a random ExperimentRun with specific status
pub fn create_test_run_with_status(status: RunStatus) -> ExperimentRun {
    let mut run = create_test_run();
    run.status = status;

    match status {
        RunStatus::Running => {
            run.started_at = Some(Utc::now());
        }
        RunStatus::Completed => {
            run.started_at = Some(Utc::now());
            run.ended_at = Some(Utc::now());
        }
        RunStatus::Failed => {
            run.started_at = Some(Utc::now());
            run.ended_at = Some(Utc::now());
            run.error = Some(RunError {
                error_type: "TestError".to_string(),
                message: "Test error message".to_string(),
                stacktrace: Some("Test stacktrace".to_string()),
                occurred_at: Utc::now(),
                is_retryable: false,
                metadata: HashMap::new(),
            });
        }
        _ => {}
    }

    run
}

/// Helper to create S3 key paths
pub fn create_artifact_path(experiment_id: &Uuid, run_id: &Uuid, artifact_name: &str) -> String {
    format!(
        "experiments/{}/runs/{}/artifacts/{}",
        experiment_id, run_id, artifact_name
    )
}

/// Helper to create dataset S3 path
pub fn create_dataset_path(dataset_id: &Uuid) -> String {
    format!("datasets/{}/data.parquet", dataset_id)
}

/// Calculate SHA256 hash for content
pub fn calculate_content_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Generate random bytes for testing
pub fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|_| (0..255u8).fake()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_experiment() {
        let experiment = create_test_experiment();
        assert_eq!(experiment.status, ExperimentStatus::Draft);
        assert!(!experiment.name.is_empty());
        assert_eq!(experiment.collaborators.len(), 2);
        assert_eq!(experiment.tags.len(), 2);
    }

    #[test]
    fn test_create_test_experiment_with_status() {
        let experiment = create_test_experiment_with_status(ExperimentStatus::Active);
        assert_eq!(experiment.status, ExperimentStatus::Active);
    }

    #[test]
    fn test_create_test_model() {
        let model = create_test_model();
        assert_eq!(model.provider, ModelProvider::OpenAI);
        assert!(!model.name.is_empty());
    }

    #[test]
    fn test_create_test_model_with_provider() {
        let model = create_test_model_with_provider(ModelProvider::Anthropic);
        assert_eq!(model.provider, ModelProvider::Anthropic);
    }

    #[test]
    fn test_create_artifact_path() {
        let exp_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let path = create_artifact_path(&exp_id, &run_id, "model.bin");
        assert!(path.contains(&exp_id.to_string()));
        assert!(path.contains(&run_id.to_string()));
        assert!(path.ends_with("model.bin"));
    }

    #[test]
    fn test_calculate_content_hash() {
        let data = b"test data";
        let hash = calculate_content_hash(data);
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters

        // Same data should produce same hash
        let hash2 = calculate_content_hash(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_generate_test_data() {
        let data = generate_test_data(100);
        assert_eq!(data.len(), 100);
    }

    #[test]
    fn test_create_test_run_with_status() {
        let run = create_test_run_with_status(RunStatus::Failed);
        assert_eq!(run.status, RunStatus::Failed);
        assert!(run.error.is_some());
        assert!(run.started_at.is_some());
        assert!(run.ended_at.is_some());
    }
}
