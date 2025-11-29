use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use llm_research_api::*;
use llm_research_core::domain::{
    config::ExperimentConfig,
    model::ModelProvider,
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

// ===== Test Helper Functions =====

/// Create a mock AppState for testing
fn create_mock_app_state() -> AppState {
    // For now, we'll use a dummy connection string and clients
    // In a real test, you'd use a test database
    use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
    use aws_sdk_s3::Client as S3Client;

    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .credentials_provider(Credentials::new("test", "test", None, None, "test"))
        .build();

    let s3_client = S3Client::from_conf(s3_config);

    // Create a dummy pool - this won't be used in these tests
    let pool = PgPool::connect_lazy("postgres://test:test@localhost/test")
        .expect("Failed to create dummy pool");

    AppState::new(pool, s3_client, "test-bucket".to_string())
}

fn create_test_experiment_config() -> ExperimentConfig {
    ExperimentConfig::default()
}

// ===== Experiment Handler Tests =====

#[tokio::test]
async fn test_create_experiment_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request_body = CreateExperimentRequest {
        name: "Test Experiment".to_string(),
        description: Some("A test experiment".to_string()),
        hypothesis: Some("Testing hypothesis".to_string()),
        owner_id,
        collaborators: None,
        tags: Some(vec!["test".to_string()]),
        config,
    };

    let request = Request::builder()
        .uri("/experiments")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_experiment_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request_body = CreateExperimentRequest {
        name: "".to_string(), // Invalid: empty name
        description: None,
        hypothesis: None,
        owner_id,
        collaborators: None,
        tags: None,
        config,
    };

    let request = Request::builder()
        .uri("/experiments")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_experiments_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/experiments?limit=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_experiments_invalid_limit() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    // Limit > 100 should fail validation
    let request = Request::builder()
        .uri("/experiments?limit=150")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_experiment_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/experiments/{}", experiment_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_experiment_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request_body = UpdateExperimentRequest {
        name: Some("".to_string()), // Invalid: empty name
        description: None,
        hypothesis: None,
        tags: None,
        config: None,
    };

    let request = Request::builder()
        .uri(&format!("/experiments/{}", experiment_id))
        .method("PUT")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_delete_experiment_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/experiments/{}", experiment_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_start_experiment_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/experiments/{}/start", experiment_id))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ===== Run Handler Tests =====

#[tokio::test]
async fn test_create_run_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request_body = CreateRunRequest {
        config_overrides: Some(json!({"param": "value"})),
    };

    let request = Request::builder()
        .uri(&format!("/experiments/{}/runs", experiment_id))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_list_runs_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/experiments/{}/runs?limit=20", experiment_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_complete_run_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/experiments/{}/runs/{}/complete", experiment_id, run_id))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_fail_run_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    let request_body = FailRunRequest {
        error: "".to_string(), // Invalid: empty error message
    };

    let request = Request::builder()
        .uri(&format!("/experiments/{}/runs/{}/fail", experiment_id, run_id))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ===== Model Handler Tests =====

#[tokio::test]
async fn test_create_model_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreateModelRequest {
        name: "GPT-4".to_string(),
        provider: ModelProvider::OpenAI,
        model_identifier: "gpt-4".to_string(),
        version: Some("0613".to_string()),
        config: json!({"temperature": 0.7}),
    };

    let request = Request::builder()
        .uri("/models")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_model_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreateModelRequest {
        name: "".to_string(), // Invalid: empty name
        provider: ModelProvider::OpenAI,
        model_identifier: "gpt-4".to_string(),
        version: None,
        config: json!({}),
    };

    let request = Request::builder()
        .uri("/models")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_models_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/models?limit=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_model_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let model_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/models/{}", model_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_model_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let model_id = Uuid::new_v4();

    let request_body = UpdateModelRequest {
        name: Some("Updated Model".to_string()),
        version: Some("v2".to_string()),
        config: None,
    };

    let request = Request::builder()
        .uri(&format!("/models/{}", model_id))
        .method("PUT")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_model_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let model_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/models/{}", model_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_list_providers_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/models/providers")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ===== Dataset Handler Tests =====

#[tokio::test]
async fn test_create_dataset_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreateDatasetRequest {
        name: "Test Dataset".to_string(),
        description: Some("A test dataset".to_string()),
        s3_path: "s3://bucket/dataset.jsonl".to_string(),
        schema: json!({"type": "object"}),
    };

    let request = Request::builder()
        .uri("/datasets")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_dataset_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreateDatasetRequest {
        name: "".to_string(), // Invalid: empty name
        description: None,
        s3_path: "s3://bucket/dataset.jsonl".to_string(),
        schema: json!({}),
    };

    let request = Request::builder()
        .uri("/datasets")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_datasets_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/datasets?limit=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_dataset_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/datasets/{}", dataset_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_dataset_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request_body = UpdateDatasetRequest {
        name: Some("Updated Dataset".to_string()),
        description: Some("Updated description".to_string()),
        schema: None,
    };

    let request = Request::builder()
        .uri(&format!("/datasets/{}", dataset_id))
        .method("PUT")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_dataset_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/datasets/{}", dataset_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_create_dataset_version_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request_body = CreateDatasetVersionRequest {
        s3_path: "s3://bucket/dataset-v2.jsonl".to_string(),
        schema: Some(json!({"type": "object"})),
    };

    let request = Request::builder()
        .uri(&format!("/datasets/{}/versions", dataset_id))
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_list_dataset_versions_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/datasets/{}/versions?limit=20", dataset_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_dataset_upload_url_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/datasets/{}/upload", dataset_id))
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_dataset_download_url_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let dataset_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/datasets/{}/download", dataset_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ===== Prompt Template Handler Tests =====

#[tokio::test]
async fn test_create_prompt_template_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreatePromptTemplateRequest {
        name: "Test Template".to_string(),
        description: Some("A test template".to_string()),
        template: "Hello {{name}}!".to_string(),
    };

    let request = Request::builder()
        .uri("/prompts")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_prompt_template_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreatePromptTemplateRequest {
        name: "".to_string(), // Invalid: empty name
        description: None,
        template: "Template".to_string(),
    };

    let request = Request::builder()
        .uri("/prompts")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_prompts_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/prompts?limit=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_prompt_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let prompt_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/prompts/{}", prompt_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_prompt_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let prompt_id = Uuid::new_v4();

    let request_body = UpdatePromptTemplateRequest {
        name: Some("Updated Template".to_string()),
        description: None,
        template: Some("Updated template".to_string()),
    };

    let request = Request::builder()
        .uri(&format!("/prompts/{}", prompt_id))
        .method("PUT")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_prompt_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let prompt_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/prompts/{}", prompt_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

// ===== Evaluation Handler Tests =====

#[tokio::test]
async fn test_create_evaluation_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreateEvaluationRequest {
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "Test input".to_string(),
        output: "Test output".to_string(),
        expected_output: Some("Expected".to_string()),
        latency_ms: 100,
        token_count: 50,
        cost: Some(rust_decimal::Decimal::new(5, 3)), // 0.005
        metrics: json!({"accuracy": 0.95}),
    };

    let request = Request::builder()
        .uri("/evaluations")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_evaluation_validation_error() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request_body = CreateEvaluationRequest {
        experiment_id: Uuid::new_v4(),
        sample_id: Uuid::new_v4(),
        input: "".to_string(), // Invalid: empty input
        output: "Test output".to_string(),
        expected_output: None,
        latency_ms: 100,
        token_count: 50,
        cost: None,
        metrics: json!({}),
    };

    let request = Request::builder()
        .uri("/evaluations")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_evaluations_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/evaluations?limit=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_evaluation_not_found() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let evaluation_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/evaluations/{}", evaluation_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_experiment_metrics_success() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let experiment_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(&format!("/experiments/{}/metrics", experiment_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ===== Health Check Tests =====

#[tokio::test]
async fn test_health_check() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ===== HTTP Method Tests =====

#[tokio::test]
async fn test_method_not_allowed() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    // POST to a GET-only endpoint
    let request = Request::builder()
        .uri("/health")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// ===== Content Type Tests =====

#[tokio::test]
async fn test_missing_content_type_for_json_body() {
    let state = create_mock_app_state();
    let app = llm_research_api::routes(state);

    let owner_id = Uuid::new_v4();
    let config = create_test_experiment_config();

    let request_body = CreateExperimentRequest {
        name: "Test".to_string(),
        description: None,
        hypothesis: None,
        owner_id,
        collaborators: None,
        tags: None,
        config,
    };

    let request = Request::builder()
        .uri("/experiments")
        .method("POST")
        // Missing Content-Type header
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should still work as Axum is lenient with Content-Type
    // But in production, you might want to enforce it
    assert!(response.status().is_success() || response.status().is_client_error());
}
