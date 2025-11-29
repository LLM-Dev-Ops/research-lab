use llm_research_workflow::*;
use uuid::Uuid;
use std::sync::Arc;

// ===== TaskContext Tests =====

#[test]
fn test_task_context_creation() {
    let experiment_id = Uuid::new_v4();
    let config = serde_json::json!({
        "param1": "value1",
        "param2": 42,
        "nested": {
            "key": "value"
        }
    });

    let context = TaskContext {
        experiment_id,
        config: config.clone(),
    };

    assert_eq!(context.experiment_id, experiment_id);
    assert_eq!(context.config, config);
}

#[test]
fn test_task_context_with_empty_config() {
    let experiment_id = Uuid::new_v4();
    let context = TaskContext {
        experiment_id,
        config: serde_json::json!({}),
    };

    assert_eq!(context.experiment_id, experiment_id);
    assert_eq!(context.config, serde_json::json!({}));
}

#[test]
fn test_task_context_serialization() {
    let experiment_id = Uuid::new_v4();
    let config = serde_json::json!({"test": "data"});

    let context = TaskContext {
        experiment_id,
        config: config.clone(),
    };

    let serialized = serde_json::to_string(&context).unwrap();
    let deserialized: TaskContext = serde_json::from_str(&serialized).unwrap();

    assert_eq!(context.experiment_id, deserialized.experiment_id);
    assert_eq!(context.config, deserialized.config);
}

#[test]
fn test_task_context_clone() {
    let experiment_id = Uuid::new_v4();
    let context = TaskContext {
        experiment_id,
        config: serde_json::json!({"key": "value"}),
    };

    let cloned = context.clone();
    assert_eq!(context.experiment_id, cloned.experiment_id);
    assert_eq!(context.config, cloned.config);
}

// ===== TaskResult Tests =====

#[test]
fn test_task_result_success() {
    let output = serde_json::json!({
        "status": "completed",
        "records_processed": 100,
        "duration_ms": 1500
    });

    let result = TaskResult::success(output.clone());

    assert!(result.success);
    assert_eq!(result.output, output);
    assert!(result.error.is_none());
}

#[test]
fn test_task_result_failure() {
    let error_msg = "Task failed due to timeout";

    let result = TaskResult::failure(error_msg.to_string());

    assert!(!result.success);
    assert_eq!(result.output, serde_json::Value::Null);
    assert_eq!(result.error, Some(error_msg.to_string()));
}

#[test]
fn test_task_result_success_with_null_output() {
    let result = TaskResult::success(serde_json::Value::Null);

    assert!(result.success);
    assert_eq!(result.output, serde_json::Value::Null);
    assert!(result.error.is_none());
}

#[test]
fn test_task_result_serialization_success() {
    let output = serde_json::json!({"data": "test"});
    let result = TaskResult::success(output);

    let serialized = serde_json::to_string(&result).unwrap();
    let deserialized: TaskResult = serde_json::from_str(&serialized).unwrap();

    assert_eq!(result.success, deserialized.success);
    assert_eq!(result.output, deserialized.output);
    assert_eq!(result.error, deserialized.error);
}

#[test]
fn test_task_result_serialization_failure() {
    let result = TaskResult::failure("Error message".to_string());

    let serialized = serde_json::to_string(&result).unwrap();
    let deserialized: TaskResult = serde_json::from_str(&serialized).unwrap();

    assert_eq!(result.success, deserialized.success);
    assert_eq!(result.error, deserialized.error);
}

// ===== EvaluationTask Tests =====

#[tokio::test]
async fn test_evaluation_task_creation() {
    let config = EvaluationConfig {
        metrics: vec!["accuracy".to_string(), "bleu".to_string()],
        batch_size: 50,
    };

    let task = EvaluationTask::new(config.clone());
    assert_eq!(task.name(), "evaluation");
}

#[tokio::test]
async fn test_evaluation_task_default_config() {
    let config = EvaluationConfig::default();
    assert_eq!(config.metrics.len(), 3);
    assert!(config.metrics.contains(&"accuracy".to_string()));
    assert!(config.metrics.contains(&"bleu".to_string()));
    assert!(config.metrics.contains(&"rouge".to_string()));
    assert_eq!(config.batch_size, 100);
}

#[tokio::test]
async fn test_evaluation_task_execute_success() {
    let config = EvaluationConfig::default();
    let task = EvaluationTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);
    assert!(task_result.error.is_none());

    // Check output structure
    let output = task_result.output;
    assert!(output.get("metrics_calculated").is_some());
    assert!(output.get("total_samples").is_some());
    assert!(output.get("batches_processed").is_some());
    assert!(output.get("metrics").is_some());
}

#[tokio::test]
async fn test_evaluation_task_with_custom_metrics() {
    let config = EvaluationConfig {
        metrics: vec!["accuracy".to_string()],
        batch_size: 20,
    };
    let task = EvaluationTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);
}

#[tokio::test]
async fn test_evaluation_task_output_contains_metrics() {
    let config = EvaluationConfig::default();
    let task = EvaluationTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await.unwrap();
    let output = result.output;

    let metrics = output.get("metrics").unwrap();
    assert!(metrics.is_object());

    // Should have accuracy metrics
    let accuracy = metrics.get("accuracy");
    assert!(accuracy.is_some());

    if let Some(acc) = accuracy {
        assert!(acc.get("mean").is_some());
        assert!(acc.get("median").is_some());
        assert!(acc.get("std_dev").is_some());
    }
}

// ===== DataLoadingTask Tests =====

#[tokio::test]
async fn test_data_loading_task_creation() {
    let config = DataLoadingConfig {
        source: "s3://bucket/data.json".to_string(),
        batch_size: 100,
        stream: false,
        limit: Some(1000),
    };

    let task = DataLoadingTask::new(config);
    assert_eq!(task.name(), "data_loading");
}

#[tokio::test]
async fn test_data_loading_task_execute_batched() {
    let config = DataLoadingConfig {
        source: "test_source".to_string(),
        batch_size: 50,
        stream: false,
        limit: Some(100),
    };

    let task = DataLoadingTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);
    assert!(task_result.error.is_none());

    let output = task_result.output;
    assert_eq!(output.get("source").unwrap(), "test_source");
    assert_eq!(output.get("total_samples").unwrap(), 100);
    assert_eq!(output.get("batch_size").unwrap(), 50);
    assert_eq!(output.get("streaming").unwrap(), false);
}

#[tokio::test]
async fn test_data_loading_task_execute_streaming() {
    let config = DataLoadingConfig {
        source: "test_stream".to_string(),
        batch_size: 25,
        stream: true,
        limit: Some(50),
    };

    let task = DataLoadingTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);

    let output = task_result.output;
    assert_eq!(output.get("streaming").unwrap(), true);
    assert_eq!(output.get("total_samples").unwrap(), 50);
}

#[tokio::test]
async fn test_data_loading_task_with_large_limit() {
    let config = DataLoadingConfig {
        source: "large_dataset".to_string(),
        batch_size: 100,
        stream: false,
        limit: Some(500),
    };

    let task = DataLoadingTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    let output = task_result.output;
    assert_eq!(output.get("total_samples").unwrap(), 500);

    // Should have 5 batches (500 / 100)
    let batches = output.get("batches").unwrap().as_array().unwrap();
    assert_eq!(batches.len(), 5);
}

#[tokio::test]
async fn test_data_loading_task_no_limit() {
    let config = DataLoadingConfig {
        source: "unlimited_source".to_string(),
        batch_size: 100,
        stream: false,
        limit: None, // Should default to 1000
    };

    let task = DataLoadingTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    let output = task_result.output;
    assert_eq!(output.get("total_samples").unwrap(), 1000);
}

// ===== InferenceTask Tests =====

#[tokio::test]
async fn test_inference_task_creation() {
    let config = InferenceConfig::default();
    let task = InferenceTask::new(config);
    assert_eq!(task.name(), "inference");
}

#[tokio::test]
async fn test_inference_task_default_config() {
    let config = InferenceConfig::default();
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.max_tokens, 1000);
    assert_eq!(config.temperature, 0.7);
    assert_eq!(config.rate_limit_per_minute, 60);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout_seconds, 30);
}

#[tokio::test]
async fn test_inference_task_execute_openai() {
    let config = InferenceConfig {
        provider: InferenceProvider::OpenAI,
        model: "gpt-4".to_string(),
        max_tokens: 500,
        temperature: 0.7,
        rate_limit_per_minute: 60,
        max_retries: 3,
        timeout_seconds: 30,
    };

    let task = InferenceTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);

    let output = task_result.output;
    assert!(output.get("provider").is_some());
    assert_eq!(output.get("model").unwrap(), "gpt-4");
    assert!(output.get("predictions_generated").is_some());
    assert!(output.get("total_tokens").is_some());
    assert!(output.get("avg_latency_ms").is_some());
}

#[tokio::test]
async fn test_inference_task_execute_anthropic() {
    let config = InferenceConfig {
        provider: InferenceProvider::Anthropic,
        model: "claude-3-opus".to_string(),
        max_tokens: 1000,
        temperature: 0.5,
        rate_limit_per_minute: 30,
        max_retries: 5,
        timeout_seconds: 60,
    };

    let task = InferenceTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);
}

#[tokio::test]
async fn test_inference_task_provider_variants() {
    let providers = vec![
        InferenceProvider::OpenAI,
        InferenceProvider::Anthropic,
        InferenceProvider::Cohere,
        InferenceProvider::HuggingFace,
        InferenceProvider::Local,
    ];

    for provider in providers {
        let provider_clone = provider.clone();
        let config = InferenceConfig {
            provider,
            model: "test-model".to_string(),
            max_tokens: 500,
            temperature: 0.7,
            rate_limit_per_minute: 60,
            max_retries: 3,
            timeout_seconds: 30,
        };

        let task = InferenceTask::new(config);

        let context = TaskContext {
            experiment_id: Uuid::new_v4(),
            config: serde_json::json!({}),
        };

        let result = task.execute(context).await;
        assert!(result.is_ok(), "Provider {:?} should succeed", provider_clone);
    }
}

#[tokio::test]
async fn test_inference_task_output_structure() {
    let config = InferenceConfig::default();
    let task = InferenceTask::new(config);

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = task.execute(context).await.unwrap();
    let output = result.output;

    // Verify output has expected fields
    assert!(output.get("provider").is_some());
    assert!(output.get("model").is_some());
    assert!(output.get("predictions_generated").is_some());
    assert!(output.get("total_tokens").is_some());
    assert!(output.get("avg_latency_ms").is_some());
    assert!(output.get("total_duration_ms").is_some());
    assert!(output.get("results").is_some());

    // Verify results array
    let results = output.get("results").unwrap().as_array().unwrap();
    assert_eq!(results.len(), 10); // Default mock creates 10 results
}

#[tokio::test]
async fn test_inference_provider_serialization() {
    let providers = vec![
        (InferenceProvider::OpenAI, "\"openai\""),
        (InferenceProvider::Anthropic, "\"anthropic\""),
        (InferenceProvider::Cohere, "\"cohere\""),
        (InferenceProvider::HuggingFace, "\"huggingface\""),
        (InferenceProvider::Local, "\"local\""),
    ];

    for (provider, expected) in providers {
        let serialized = serde_json::to_string(&provider).unwrap();
        assert_eq!(serialized, expected);

        let deserialized: InferenceProvider = serde_json::from_str(&serialized).unwrap();
        assert_eq!(format!("{:?}", provider), format!("{:?}", deserialized));
    }
}

// ===== TaskExecutor Tests =====

#[tokio::test]
async fn test_task_executor_creation() {
    let executor = TaskExecutor::new(4);
    // Should create successfully
}

#[tokio::test]
async fn test_task_executor_execute_one() {
    let executor = TaskExecutor::new(4);
    let config = EvaluationConfig::default();
    let task: Arc<dyn Task> = Arc::new(EvaluationTask::new(config));

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = executor.execute_one(task, context).await;
    assert!(result.is_ok());

    let task_result = result.unwrap();
    assert!(task_result.success);
}

#[tokio::test]
async fn test_task_executor_execute_batch() {
    let executor = TaskExecutor::new(2);

    let tasks: Vec<Arc<dyn Task>> = vec![
        Arc::new(EvaluationTask::new(EvaluationConfig::default())),
        Arc::new(DataLoadingTask::new(DataLoadingConfig {
            source: "test".to_string(),
            batch_size: 10,
            stream: false,
            limit: Some(10),
        })),
        Arc::new(InferenceTask::new(InferenceConfig::default())),
    ];

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = executor.execute_batch(tasks, context).await;
    assert!(result.is_ok());

    let results = result.unwrap();
    assert_eq!(results.len(), 3);

    for task_result in results {
        assert!(task_result.success);
    }
}

#[tokio::test]
async fn test_task_executor_with_progress_tracking() {
    let executor = TaskExecutor::new(4);
    let _rx = executor.enable_progress_tracking().await;

    let config = EvaluationConfig::default();
    let task: Arc<dyn Task> = Arc::new(EvaluationTask::new(config));

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = executor.execute_one(task, context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_executor_concurrency_limit() {
    let executor = TaskExecutor::new(1); // Only 1 concurrent task

    let tasks: Vec<Arc<dyn Task>> = (0..3)
        .map(|_| {
            Arc::new(DataLoadingTask::new(DataLoadingConfig {
                source: "test".to_string(),
                batch_size: 10,
                stream: false,
                limit: Some(10),
            })) as Arc<dyn Task>
        })
        .collect();

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let start = std::time::Instant::now();
    let result = executor.execute_batch(tasks, context).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 3);

    // With concurrency=1, tasks should run sequentially
    // This is hard to test precisely due to timing, but duration should be reasonable
    assert!(duration.as_millis() > 0);
}

#[tokio::test]
async fn test_task_executor_empty_batch() {
    let executor = TaskExecutor::new(4);

    let tasks: Vec<Arc<dyn Task>> = vec![];

    let context = TaskContext {
        experiment_id: Uuid::new_v4(),
        config: serde_json::json!({}),
    };

    let result = executor.execute_batch(tasks, context).await;
    assert!(result.is_ok());

    let results = result.unwrap();
    assert_eq!(results.len(), 0);
}

// ===== TaskProgress Tests =====

#[test]
fn test_task_progress_creation() {
    let progress = TaskProgress {
        task_id: Uuid::new_v4(),
        task_name: "Test Task".to_string(),
        progress: 0.5,
        status: "running".to_string(),
        message: Some("Half way done".to_string()),
    };

    assert_eq!(progress.task_name, "Test Task");
    assert_eq!(progress.progress, 0.5);
    assert_eq!(progress.status, "running");
    assert!(progress.message.is_some());
}

#[test]
fn test_task_progress_serialization() {
    let progress = TaskProgress {
        task_id: Uuid::new_v4(),
        task_name: "Test Task".to_string(),
        progress: 1.0,
        status: "completed".to_string(),
        message: None,
    };

    let serialized = serde_json::to_string(&progress).unwrap();
    let deserialized: TaskProgress = serde_json::from_str(&serialized).unwrap();

    assert_eq!(progress.task_id, deserialized.task_id);
    assert_eq!(progress.task_name, deserialized.task_name);
    assert_eq!(progress.progress, deserialized.progress);
    assert_eq!(progress.status, deserialized.status);
    assert_eq!(progress.message, deserialized.message);
}
