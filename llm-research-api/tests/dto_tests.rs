use chrono::Utc;
use llm_research_api::*;
use llm_research_core::{
    domain::{
        config::ExperimentConfig,
        experiment::{Experiment, ExperimentStatus},
        ids::{ExperimentId, UserId},
        model::{Model, ModelProvider},
    },
    Dataset, Evaluation, PromptTemplate,
};
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

// ===== Helper Functions =====

fn create_test_experiment_config() -> ExperimentConfig {
    ExperimentConfig::default()
}

// ===== Experiment DTO Tests =====

#[test]
fn test_create_experiment_request_validation_success() {
    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request = CreateExperimentRequest {
        name: "Valid Experiment".to_string(),
        description: Some("Description".to_string()),
        hypothesis: Some("Hypothesis".to_string()),
        owner_id,
        collaborators: None,
        tags: Some(vec!["tag1".to_string()]),
        config,
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_experiment_request_validation_empty_name() {
    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request = CreateExperimentRequest {
        name: "".to_string(), // Invalid
        description: None,
        hypothesis: None,
        owner_id,
        collaborators: None,
        tags: None,
        config,
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_create_experiment_request_validation_name_too_long() {
    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request = CreateExperimentRequest {
        name: "x".repeat(256), // Too long (max 255)
        description: None,
        hypothesis: None,
        owner_id,
        collaborators: None,
        tags: None,
        config,
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_create_experiment_request_serialization() {
    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request = CreateExperimentRequest {
        name: "Test".to_string(),
        description: Some("Desc".to_string()),
        hypothesis: None,
        owner_id,
        collaborators: Some(vec![Uuid::new_v4()]),
        tags: Some(vec!["tag".to_string()]),
        config,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("Test"));
    assert!(json.contains("Desc"));
}

#[test]
fn test_create_experiment_request_deserialization() {
    let owner_id = Uuid::new_v4();
    let json = format!(
        r#"{{
            "name": "Test Experiment",
            "description": "A test",
            "hypothesis": null,
            "owner_id": "{}",
            "collaborators": null,
            "tags": ["ml"],
            "config": {{
                "model_configs": [],
                "dataset_refs": [],
                "metric_configs": [],
                "parameters": {{
                    "fixed": {{}},
                    "search_spaces": [],
                    "concurrent_trials": 1
                }},
                "resource_requirements": {{
                    "compute": {{
                        "cpu_cores": 2,
                        "memory_gb": 8,
                        "disk_gb": 20
                    }},
                    "timeout_seconds": 3600,
                    "max_retries": 3,
                    "priority": 5
                }},
                "reproducibility_settings": {{
                    "deterministic_mode": true,
                    "track_environment": true,
                    "track_code_version": true,
                    "track_dependencies": true,
                    "snapshot_dataset": true,
                    "snapshot_model": false
                }}
            }}
        }}"#,
        owner_id
    );

    let request: CreateExperimentRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request.name, "Test Experiment");
    assert_eq!(request.description, Some("A test".to_string()));
}

#[test]
fn test_update_experiment_request_validation() {
    let request = UpdateExperimentRequest {
        name: Some("Updated Name".to_string()),
        description: Some("Updated description".to_string()),
        hypothesis: None,
        tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        config: None,
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_update_experiment_request_empty_name() {
    let request = UpdateExperimentRequest {
        name: Some("".to_string()), // Invalid
        description: None,
        hypothesis: None,
        tags: None,
        config: None,
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_experiment_response_from_domain() {
    let owner_id = UserId::from(Uuid::new_v4());
    let config = create_test_experiment_config();

    let experiment = Experiment::new(
        "Test Experiment".to_string(),
        Some("Description".to_string()),
        Some("Hypothesis".to_string()),
        owner_id,
        config,
    );

    let response = ExperimentResponse::from(experiment.clone());

    assert_eq!(response.id, experiment.id.0);
    assert_eq!(response.name, experiment.name);
    assert_eq!(response.description, experiment.description);
    assert_eq!(response.status, ExperimentStatus::Draft);
}

#[test]
fn test_experiment_response_serialization() {
    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let response = ExperimentResponse {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        description: Some("Desc".to_string()),
        hypothesis: None,
        owner_id,
        collaborators: vec![],
        tags: vec!["ml".to_string()],
        status: ExperimentStatus::Draft,
        config,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        archived_at: None,
        metadata: HashMap::new(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Test"));
    // ExperimentStatus uses snake_case serialization
    assert!(json.contains("draft"));
}

// ===== Run DTO Tests =====

#[test]
fn test_create_run_request_validation() {
    let request = CreateRunRequest {
        config_overrides: Some(json!({"param": "value"})),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_run_request_no_overrides() {
    let request = CreateRunRequest {
        config_overrides: None,
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_fail_run_request_validation() {
    let request = FailRunRequest {
        error: "Something went wrong".to_string(),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_fail_run_request_empty_error() {
    let request = FailRunRequest {
        error: "".to_string(), // Invalid
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_run_response_serialization() {
    let response = RunResponse {
        id: Uuid::new_v4(),
        experiment_id: Uuid::new_v4(),
        status: "running".to_string(),
        config: json!({"param": "value"}),
        started_at: Utc::now(),
        completed_at: None,
        error: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("running"));
}

// ===== Model DTO Tests =====

#[test]
fn test_create_model_request_validation_success() {
    let request = CreateModelRequest {
        name: "GPT-4".to_string(),
        provider: ModelProvider::OpenAI,
        model_identifier: "gpt-4".to_string(),
        version: Some("0613".to_string()),
        config: json!({"temperature": 0.7}),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_model_request_validation_empty_name() {
    let request = CreateModelRequest {
        name: "".to_string(), // Invalid
        provider: ModelProvider::OpenAI,
        model_identifier: "gpt-4".to_string(),
        version: None,
        config: json!({}),
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_create_model_request_validation_empty_identifier() {
    let request = CreateModelRequest {
        name: "Model".to_string(),
        provider: ModelProvider::OpenAI,
        model_identifier: "".to_string(), // Invalid
        version: None,
        config: json!({}),
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_create_model_request_all_providers() {
    let providers = vec![
        ModelProvider::OpenAI,
        ModelProvider::Anthropic,
        ModelProvider::Google,
        ModelProvider::Cohere,
        ModelProvider::HuggingFace,
        ModelProvider::Azure,
        ModelProvider::AWS,
        ModelProvider::Local,
        ModelProvider::Custom,
    ];

    for provider in providers {
        let request = CreateModelRequest {
            name: "Model".to_string(),
            provider,
            model_identifier: "model-id".to_string(),
            version: None,
            config: json!({}),
        };

        assert!(request.validate().is_ok());
    }
}

#[test]
fn test_update_model_request_validation() {
    let request = UpdateModelRequest {
        name: Some("Updated Model".to_string()),
        version: Some("v2".to_string()),
        config: Some(json!({"temperature": 0.8})),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_model_response_from_domain() {
    let model = Model::new(
        "GPT-4".to_string(),
        ModelProvider::OpenAI,
        "gpt-4".to_string(),
        Some("0613".to_string()),
        json!({"temperature": 0.7}),
    );

    let response = ModelResponse::from(model.clone());

    assert_eq!(response.id, model.id);
    assert_eq!(response.name, model.name);
    assert_eq!(response.provider, model.provider);
}

#[test]
fn test_provider_response_serialization() {
    let response = ProviderResponse {
        name: "openai".to_string(),
        display_name: "OpenAI".to_string(),
        description: Some("OpenAI models".to_string()),
        supported_models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("openai"));
    assert!(json.contains("OpenAI"));
    assert!(json.contains("gpt-4"));
}

// ===== Dataset DTO Tests =====

#[test]
fn test_create_dataset_request_validation_success() {
    let request = CreateDatasetRequest {
        name: "Test Dataset".to_string(),
        description: Some("A test dataset".to_string()),
        s3_path: "s3://bucket/dataset.jsonl".to_string(),
        schema: json!({"type": "object"}),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_dataset_request_validation_empty_name() {
    let request = CreateDatasetRequest {
        name: "".to_string(), // Invalid
        description: None,
        s3_path: "s3://bucket/dataset.jsonl".to_string(),
        schema: json!({}),
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_update_dataset_request_validation() {
    let request = UpdateDatasetRequest {
        name: Some("Updated Dataset".to_string()),
        description: Some("Updated description".to_string()),
        schema: Some(json!({"type": "array"})),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_dataset_version_request_validation() {
    let request = CreateDatasetVersionRequest {
        s3_path: "s3://bucket/dataset-v2.jsonl".to_string(),
        schema: Some(json!({"type": "object"})),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_dataset_response_serialization() {
    let response = DatasetResponse {
        id: Uuid::new_v4(),
        name: "Test Dataset".to_string(),
        description: Some("Description".to_string()),
        s3_path: "s3://bucket/dataset.jsonl".to_string(),
        sample_count: 100,
        schema: json!({"type": "object"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: Some(1),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Test Dataset"));
    assert!(json.contains("s3://bucket/dataset.jsonl"));
}

#[test]
fn test_dataset_version_response_serialization() {
    let response = DatasetVersionResponse {
        version: 2,
        dataset_id: Uuid::new_v4(),
        s3_path: "s3://bucket/dataset-v2.jsonl".to_string(),
        sample_count: 150,
        created_at: Utc::now(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"version\":2"));
}

#[test]
fn test_upload_url_response_serialization() {
    let response = UploadUrlResponse {
        upload_url: "https://s3.amazonaws.com/presigned-url".to_string(),
        s3_path: "s3://bucket/dataset.jsonl".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("presigned-url"));
}

#[test]
fn test_download_url_response_serialization() {
    let response = DownloadUrlResponse {
        download_url: "https://s3.amazonaws.com/presigned-download".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("presigned-download"));
}

// ===== Prompt Template DTO Tests =====

#[test]
fn test_create_prompt_template_request_validation_success() {
    let request = CreatePromptTemplateRequest {
        name: "Test Template".to_string(),
        description: Some("A test template".to_string()),
        template: "Hello {{name}}!".to_string(),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_prompt_template_request_validation_empty_name() {
    let request = CreatePromptTemplateRequest {
        name: "".to_string(), // Invalid
        description: None,
        template: "Template".to_string(),
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_create_prompt_template_request_validation_empty_template() {
    let request = CreatePromptTemplateRequest {
        name: "Template".to_string(),
        description: None,
        template: "".to_string(), // Invalid
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_update_prompt_template_request_validation() {
    let request = UpdatePromptTemplateRequest {
        name: Some("Updated Template".to_string()),
        description: Some("Updated description".to_string()),
        template: Some("Updated {{variable}}".to_string()),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_prompt_template_response_serialization() {
    let response = PromptTemplateResponse {
        id: Uuid::new_v4(),
        name: "Test Template".to_string(),
        description: Some("Description".to_string()),
        template: "Hello {{name}}!".to_string(),
        variables: vec!["name".to_string()],
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Test Template"));
    assert!(json.contains("{{name}}"));
}

// ===== Evaluation DTO Tests =====

#[test]
fn test_create_evaluation_request_validation_success() {
    let request = CreateEvaluationRequest {
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "Test input".to_string(),
        output: "Test output".to_string(),
        expected_output: Some("Expected".to_string()),
        latency_ms: 100,
        token_count: 50,
        cost: Some(Decimal::new(5, 3)), // 0.005
        metrics: json!({"accuracy": 0.95}),
    };

    assert!(request.validate().is_ok());
}

#[test]
fn test_create_evaluation_request_validation_empty_input() {
    let request = CreateEvaluationRequest {
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "".to_string(), // Invalid
        output: "Output".to_string(),
        expected_output: None,
        latency_ms: 100,
        token_count: 50,
        cost: None,
        metrics: json!({}),
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_create_evaluation_request_validation_empty_output() {
    let request = CreateEvaluationRequest {
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "Input".to_string(),
        output: "".to_string(), // Invalid
        expected_output: None,
        latency_ms: 100,
        token_count: 50,
        cost: None,
        metrics: json!({}),
    };

    let result = request.validate();
    assert!(result.is_err());
}

#[test]
fn test_evaluation_response_serialization() {
    let response = EvaluationResponse {
        id: Uuid::new_v4(),
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "Input".to_string(),
        output: "Output".to_string(),
        expected_output: Some("Expected".to_string()),
        latency_ms: 100,
        token_count: 50,
        cost: Some(Decimal::new(5, 3)),
        metrics: json!({"accuracy": 0.95}),
        created_at: Utc::now(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Input"));
    assert!(json.contains("Output"));
}

#[test]
fn test_metrics_response_serialization() {
    let response = MetricsResponse {
        experiment_id: Uuid::new_v4(),
        total_samples: 100,
        avg_latency_ms: 150.5,
        total_tokens: 5000,
        total_cost: Some(Decimal::new(125, 2)), // 1.25
        accuracy: Some(Decimal::new(95, 2)),    // 0.95
        custom_metrics: json!({"f1_score": 0.93}),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"total_samples\":100"));
    assert!(json.contains("f1_score"));
}

// ===== Pagination DTO Tests =====

#[test]
fn test_pagination_query_validation_success() {
    let query = PaginationQuery {
        limit: Some(20),
        cursor: None,
    };

    assert!(query.validate().is_ok());
}

#[test]
fn test_pagination_query_validation_limit_too_high() {
    let query = PaginationQuery {
        limit: Some(150), // Max is 100
        cursor: None,
    };

    let result = query.validate();
    assert!(result.is_err());
}

#[test]
fn test_pagination_query_validation_limit_too_low() {
    let query = PaginationQuery {
        limit: Some(0), // Min is 1
        cursor: None,
    };

    let result = query.validate();
    assert!(result.is_err());
}

#[test]
fn test_pagination_query_default() {
    let query = PaginationQuery::default();

    assert_eq!(query.limit, Some(20));
    assert_eq!(query.cursor, None);
}

#[test]
fn test_paginated_response_serialization() {
    let params = llm_research_api::PaginationParams::new().with_page_size(10);
    let response: PaginatedResponse<ExperimentResponse> = PaginatedResponse::new(
        vec![],
        &params,
        100,
        None,
    );

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"has_next\":true"));
    assert!(json.contains("\"total_count\":100"));
}

#[test]
fn test_paginated_response_no_more_data() {
    let params = llm_research_api::PaginationParams::new().with_page_size(10);
    let response: PaginatedResponse<String> = PaginatedResponse::new(
        vec!["item1".to_string(), "item2".to_string()],
        &params,
        2,
        None,
    );

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"has_next\":false"));
    assert!(json.contains("item1"));
}

// ===== Error Response DTO Tests =====

#[test]
fn test_error_response_serialization() {
    let response = ErrorResponse {
        error: "Not found".to_string(),
        details: Some("Resource with ID xyz not found".to_string()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Not found"));
    assert!(json.contains("Resource with ID xyz"));
}

#[test]
fn test_error_response_no_details() {
    let response = ErrorResponse {
        error: "Internal server error".to_string(),
        details: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Internal server error"));
    // Note: With default serde, None fields serialize as null
    // If skip_serializing_if is used, this would be absent
    // The test validates that error is present
}

// ===== Complex DTO Conversion Tests =====

#[test]
fn test_experiment_response_with_collaborators() {
    let owner_id = UserId::from(Uuid::new_v4());
    let collaborator1 = UserId::from(Uuid::new_v4());
    let collaborator2 = UserId::from(Uuid::new_v4());
    let config = create_test_experiment_config();

    let mut experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    experiment = experiment.with_collaborators(vec![collaborator1, collaborator2]);

    let response = ExperimentResponse::from(experiment);

    assert_eq!(response.collaborators.len(), 2);
}

#[test]
fn test_experiment_response_with_tags() {
    let owner_id = UserId::from(Uuid::new_v4());
    let config = create_test_experiment_config();

    let mut experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    experiment = experiment.with_tags(vec![
        "ml".to_string(),
        "nlp".to_string(),
        "production".to_string(),
    ]);

    let response = ExperimentResponse::from(experiment);

    assert_eq!(response.tags.len(), 3);
    assert!(response.tags.contains(&"ml".to_string()));
    assert!(response.tags.contains(&"nlp".to_string()));
}

// ===== JSON Edge Cases =====

#[test]
fn test_deserialize_experiment_request_with_null_fields() {
    let json = format!(
        r#"{{
            "name": "Test",
            "description": null,
            "hypothesis": null,
            "owner_id": "{}",
            "collaborators": null,
            "tags": null,
            "config": {{
                "model_configs": [],
                "dataset_refs": [],
                "metric_configs": [],
                "parameters": {{
                    "fixed": {{}},
                    "search_spaces": [],
                    "concurrent_trials": 1
                }},
                "resource_requirements": {{
                    "compute": {{
                        "cpu_cores": 2,
                        "memory_gb": 8,
                        "disk_gb": 20
                    }},
                    "timeout_seconds": 3600,
                    "max_retries": 3,
                    "priority": 5
                }},
                "reproducibility_settings": {{
                    "deterministic_mode": true,
                    "track_environment": true,
                    "track_code_version": true,
                    "track_dependencies": true,
                    "snapshot_dataset": true,
                    "snapshot_model": false
                }}
            }}
        }}"#,
        Uuid::new_v4()
    );

    let request: CreateExperimentRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request.name, "Test");
    assert_eq!(request.description, None);
    assert_eq!(request.collaborators, None);
}

#[test]
fn test_serialize_deserialize_round_trip() {
    let original = CreateModelRequest {
        name: "Test Model".to_string(),
        provider: ModelProvider::Anthropic,
        model_identifier: "claude-3".to_string(),
        version: Some("20240307".to_string()),
        config: json!({"max_tokens": 1000}),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: CreateModelRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, original.name);
    assert_eq!(deserialized.provider, original.provider);
    assert_eq!(deserialized.model_identifier, original.model_identifier);
}

// ===== Decimal Precision Tests =====

#[test]
fn test_decimal_cost_precision() {
    let request = CreateEvaluationRequest {
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "Input".to_string(),
        output: "Output".to_string(),
        expected_output: None,
        latency_ms: 100,
        token_count: 50,
        cost: Some(Decimal::new(123456, 5)), // 1.23456
        metrics: json!({}),
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: CreateEvaluationRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.cost, request.cost);
}

// ===== UUID Validation Tests =====

#[test]
fn test_valid_uuid_in_request() {
    let valid_uuid = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request = CreateExperimentRequest {
        name: "Test".to_string(),
        description: None,
        hypothesis: None,
        owner_id: valid_uuid,
        collaborators: None,
        tags: None,
        config,
    };

    assert_eq!(request.owner_id, valid_uuid);
}
