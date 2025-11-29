use llm_research_workflow::*;
use uuid::Uuid;
use std::collections::HashMap;

// ===== Workflow Struct Tests =====

#[test]
fn test_workflow_creation() {
    let steps = vec![
        WorkflowStep::new(
            "Step 1".to_string(),
            "test_task".to_string(),
            serde_json::json!({"key": "value"}),
        ),
    ];

    let workflow = Workflow::new("Test Workflow".to_string(), steps.clone());

    assert_eq!(workflow.name, "Test Workflow");
    assert_eq!(workflow.status, WorkflowStatus::Pending);
    assert_eq!(workflow.steps.len(), 1);
    assert!(workflow.error.is_none());
    assert!(workflow.started_at.is_none());
    assert!(workflow.completed_at.is_none());
}

#[test]
fn test_workflow_empty_steps() {
    let workflow = Workflow::new("Empty Workflow".to_string(), vec![]);
    assert_eq!(workflow.steps.len(), 0);
    assert_eq!(workflow.status, WorkflowStatus::Pending);
}

// ===== WorkflowStep Struct Tests =====

#[test]
fn test_workflow_step_creation() {
    let config = serde_json::json!({
        "timeout": 30,
        "retry_policy": "exponential"
    });

    let step = WorkflowStep::new(
        "Data Loading".to_string(),
        "data_load".to_string(),
        config.clone(),
    );

    assert_eq!(step.name, "Data Loading");
    assert_eq!(step.task_type, "data_load");
    assert_eq!(step.config, config);
    assert_eq!(step.status, WorkflowStatus::Pending);
    assert_eq!(step.dependencies.len(), 0);
    assert_eq!(step.retry_count, 0);
    assert_eq!(step.max_retries, 3);
    assert!(step.error.is_none());
}

#[test]
fn test_workflow_step_with_dependencies() {
    let dep1 = Uuid::new_v4();
    let dep2 = Uuid::new_v4();

    let step = WorkflowStep::new(
        "Step with deps".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![dep1, dep2]);

    assert_eq!(step.dependencies.len(), 2);
    assert!(step.dependencies.contains(&dep1));
    assert!(step.dependencies.contains(&dep2));
}

#[test]
fn test_workflow_step_with_max_retries() {
    let step = WorkflowStep::new(
        "Retryable Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    )
    .with_max_retries(5);

    assert_eq!(step.max_retries, 5);
    assert_eq!(step.retry_count, 0);
}

#[test]
fn test_workflow_step_builder_pattern() {
    let dep_id = Uuid::new_v4();

    let step = WorkflowStep::new(
        "Complex Step".to_string(),
        "complex_task".to_string(),
        serde_json::json!({"param": "value"}),
    )
    .with_dependencies(vec![dep_id])
    .with_max_retries(10);

    assert_eq!(step.name, "Complex Step");
    assert_eq!(step.dependencies.len(), 1);
    assert_eq!(step.max_retries, 10);
}

// ===== WorkflowStatus State Transitions =====

#[test]
fn test_workflow_status_equality() {
    assert_eq!(WorkflowStatus::Pending, WorkflowStatus::Pending);
    assert_eq!(WorkflowStatus::Running, WorkflowStatus::Running);
    assert_eq!(WorkflowStatus::Completed, WorkflowStatus::Completed);
    assert_eq!(WorkflowStatus::Failed, WorkflowStatus::Failed);
    assert_eq!(WorkflowStatus::Paused, WorkflowStatus::Paused);
    assert_eq!(WorkflowStatus::Cancelled, WorkflowStatus::Cancelled);
}

#[test]
fn test_workflow_status_serialization() {
    let status = WorkflowStatus::Running;
    let serialized = serde_json::to_string(&status).unwrap();
    assert_eq!(serialized, "\"running\"");

    let deserialized: WorkflowStatus = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, WorkflowStatus::Running);
}

#[test]
fn test_workflow_status_all_variants() {
    let statuses = vec![
        WorkflowStatus::Pending,
        WorkflowStatus::Running,
        WorkflowStatus::Paused,
        WorkflowStatus::Completed,
        WorkflowStatus::Failed,
        WorkflowStatus::Cancelled,
    ];

    for status in statuses {
        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: WorkflowStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(status, deserialized);
    }
}

// ===== DefaultWorkflowEngine Tests =====

#[test]
fn test_default_workflow_engine_creation() {
    let engine = DefaultWorkflowEngine::new();
    // Engine should be created successfully
    assert!(true);
}

#[test]
fn test_default_workflow_engine_default_trait() {
    let engine = DefaultWorkflowEngine::default();
    // Engine should be created successfully via Default trait
    assert!(true);
}

// ===== Workflow Execution Tests =====

#[tokio::test]
async fn test_execute_empty_workflow() {
    let workflow = Workflow::new("Empty Workflow".to_string(), vec![]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert!(state.workflow.started_at.is_some());
    assert!(state.workflow.completed_at.is_some());
    assert_eq!(state.step_outputs.len(), 0);
}

#[tokio::test]
async fn test_execute_single_step_workflow() {
    let step = WorkflowStep::new(
        "Single Step".to_string(),
        "test_task".to_string(),
        serde_json::json!({"test": "data"}),
    );

    let workflow = Workflow::new("Single Step Workflow".to_string(), vec![step]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert_eq!(state.step_outputs.len(), 1);
    assert!(state.workflow.completed_at.is_some());
}

#[tokio::test]
async fn test_execute_workflow_with_sequential_steps() {
    let step1 = WorkflowStep::new(
        "Step 1".to_string(),
        "task1".to_string(),
        serde_json::json!({}),
    );
    let step1_id = step1.id;

    let step2 = WorkflowStep::new(
        "Step 2".to_string(),
        "task2".to_string(),
        serde_json::json!({}),
    );
    let step2_id = step2.id;

    let workflow = Workflow::new("Sequential Workflow".to_string(), vec![step1, step2]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert_eq!(state.step_outputs.len(), 2);
    assert!(state.step_outputs.contains_key(&step1_id));
    assert!(state.step_outputs.contains_key(&step2_id));
}

// ===== Step Dependency Resolution Tests =====

#[tokio::test]
async fn test_execute_workflow_with_dependencies() {
    let step1 = WorkflowStep::new(
        "Step 1".to_string(),
        "task1".to_string(),
        serde_json::json!({}),
    );
    let step1_id = step1.id;

    let step2 = WorkflowStep::new(
        "Step 2".to_string(),
        "task2".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![step1_id]);

    let workflow = Workflow::new("Dependent Workflow".to_string(), vec![step1, step2]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert_eq!(state.step_outputs.len(), 2);
}

#[tokio::test]
async fn test_execute_workflow_with_multiple_dependencies() {
    let step1 = WorkflowStep::new(
        "Step 1".to_string(),
        "task1".to_string(),
        serde_json::json!({}),
    );
    let step1_id = step1.id;

    let step2 = WorkflowStep::new(
        "Step 2".to_string(),
        "task2".to_string(),
        serde_json::json!({}),
    );
    let step2_id = step2.id;

    let step3 = WorkflowStep::new(
        "Step 3".to_string(),
        "task3".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![step1_id, step2_id]);

    let workflow = Workflow::new(
        "Multiple Dependencies".to_string(),
        vec![step1, step2, step3],
    );
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert_eq!(state.step_outputs.len(), 3);
}

// ===== Deadlock Detection Tests =====

#[tokio::test]
async fn test_deadlock_detection_circular_dependency() {
    let step1 = WorkflowStep::new(
        "Step 1".to_string(),
        "task1".to_string(),
        serde_json::json!({}),
    );
    let step1_id = step1.id;

    let step2 = WorkflowStep::new(
        "Step 2".to_string(),
        "task2".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![step1_id]);
    let step2_id = step2.id;

    // Create circular dependency by modifying step1
    let mut step1_circular = step1;
    step1_circular.dependencies = vec![step2_id];

    let workflow = Workflow::new(
        "Circular Dependency".to_string(),
        vec![step1_circular, step2],
    );
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_err());

    // Should detect deadlock
    let error = result.unwrap_err();
    assert!(error.to_string().contains("deadlock"));
}

#[tokio::test]
async fn test_deadlock_detection_missing_dependency() {
    let non_existent_dep = Uuid::new_v4();

    let step = WorkflowStep::new(
        "Step with missing dep".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![non_existent_dep]);

    let workflow = Workflow::new("Missing Dependency".to_string(), vec![step]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_err());

    // Should detect deadlock because dependency never completes
    let error = result.unwrap_err();
    assert!(error.to_string().contains("deadlock"));
}

// ===== Retry Logic Tests =====

#[tokio::test]
async fn test_step_with_zero_retries() {
    let step = WorkflowStep::new(
        "No Retry Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    )
    .with_max_retries(0);

    let workflow = Workflow::new("No Retry Workflow".to_string(), vec![step]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    // Should still succeed (mock tasks always succeed)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_step_with_high_retry_count() {
    let step = WorkflowStep::new(
        "High Retry Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    )
    .with_max_retries(100);

    let workflow = Workflow::new("High Retry Workflow".to_string(), vec![step]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());
}

// ===== Workflow State Management =====

#[tokio::test]
async fn test_workflow_state_includes_timestamps() {
    let step = WorkflowStep::new(
        "Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    );

    let workflow = Workflow::new("Timestamped Workflow".to_string(), vec![step]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert!(state.workflow.started_at.is_some());
    assert!(state.workflow.completed_at.is_some());

    // Completed time should be after or equal to started time
    let started = state.workflow.started_at.unwrap();
    let completed = state.workflow.completed_at.unwrap();
    assert!(completed >= started);
}

#[tokio::test]
async fn test_workflow_state_step_outputs() {
    let step1 = WorkflowStep::new(
        "Step 1".to_string(),
        "task1".to_string(),
        serde_json::json!({"input": "data1"}),
    );
    let step1_id = step1.id;

    let step2 = WorkflowStep::new(
        "Step 2".to_string(),
        "task2".to_string(),
        serde_json::json!({"input": "data2"}),
    );
    let step2_id = step2.id;

    let workflow = Workflow::new("Output Test".to_string(), vec![step1, step2]);
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.step_outputs.len(), 2);

    // Check outputs exist and contain expected structure
    let output1 = state.step_outputs.get(&step1_id).unwrap();
    assert!(output1.get("task_type").is_some());
    assert_eq!(output1.get("status").unwrap(), "completed");

    let output2 = state.step_outputs.get(&step2_id).unwrap();
    assert!(output2.get("task_type").is_some());
    assert_eq!(output2.get("status").unwrap(), "completed");
}

// ===== Pause/Resume/Cancel Operations =====

#[tokio::test]
async fn test_pause_workflow() {
    let step = WorkflowStep::new(
        "Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    );

    let workflow = Workflow::new("Pausable Workflow".to_string(), vec![step]);
    let workflow_id = workflow.id;
    let engine = DefaultWorkflowEngine::new();

    // Execute workflow first
    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    // Pause the workflow
    let pause_result = engine.pause(workflow_id).await;
    assert!(pause_result.is_ok());
}

#[tokio::test]
async fn test_pause_nonexistent_workflow() {
    let engine = DefaultWorkflowEngine::new();
    let fake_id = Uuid::new_v4();

    let result = engine.pause(fake_id).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_cancel_workflow() {
    let step = WorkflowStep::new(
        "Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    );

    let workflow = Workflow::new("Cancellable Workflow".to_string(), vec![step]);
    let workflow_id = workflow.id;
    let engine = DefaultWorkflowEngine::new();

    // Execute workflow first
    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    // Cancel the workflow
    let cancel_result = engine.cancel(workflow_id).await;
    assert!(cancel_result.is_ok());
}

#[tokio::test]
async fn test_cancel_nonexistent_workflow() {
    let engine = DefaultWorkflowEngine::new();
    let fake_id = Uuid::new_v4();

    let result = engine.cancel(fake_id).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_resume_paused_workflow() {
    let step = WorkflowStep::new(
        "Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    );

    let workflow = Workflow::new("Resumable Workflow".to_string(), vec![step]);
    let workflow_id = workflow.id;
    let engine = DefaultWorkflowEngine::new();

    // Execute workflow
    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    // Pause workflow
    let pause_result = engine.pause(workflow_id).await;
    assert!(pause_result.is_ok());

    // Resume workflow
    let resume_result = engine.resume(workflow_id).await;
    assert!(resume_result.is_ok());
}

#[tokio::test]
async fn test_resume_non_paused_workflow() {
    let step = WorkflowStep::new(
        "Step".to_string(),
        "task".to_string(),
        serde_json::json!({}),
    );

    let workflow = Workflow::new("Running Workflow".to_string(), vec![step]);
    let workflow_id = workflow.id;
    let engine = DefaultWorkflowEngine::new();

    // Execute workflow (it will be completed, not paused)
    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    // Try to resume a non-paused workflow
    let resume_result = engine.resume(workflow_id).await;
    assert!(resume_result.is_err());

    let error = resume_result.unwrap_err();
    assert!(error.to_string().contains("not paused"));
}

#[tokio::test]
async fn test_resume_nonexistent_workflow() {
    let engine = DefaultWorkflowEngine::new();
    let fake_id = Uuid::new_v4();

    let result = engine.resume(fake_id).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("not found"));
}

// ===== Concurrent Step Execution Tests =====

#[tokio::test]
async fn test_parallel_independent_steps() {
    // Create 5 independent steps that could run in parallel
    let steps: Vec<WorkflowStep> = (0..5)
        .map(|i| {
            WorkflowStep::new(
                format!("Parallel Step {}", i),
                format!("task{}", i),
                serde_json::json!({"index": i}),
            )
        })
        .collect();

    let workflow = Workflow::new("Parallel Workflow".to_string(), steps);
    let engine = DefaultWorkflowEngine::new();

    let start = std::time::Instant::now();
    let result = engine.execute(&workflow).await;
    let duration = start.elapsed();

    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert_eq!(state.step_outputs.len(), 5);

    // All steps should complete
    for step in &state.workflow.steps {
        assert_eq!(step.status, WorkflowStatus::Completed);
    }
}

#[tokio::test]
async fn test_complex_dependency_graph() {
    // Create a complex DAG:
    //     step1
    //    /     \
    // step2   step3
    //    \     /
    //     step4

    let step1 = WorkflowStep::new(
        "Step 1".to_string(),
        "task1".to_string(),
        serde_json::json!({}),
    );
    let step1_id = step1.id;

    let step2 = WorkflowStep::new(
        "Step 2".to_string(),
        "task2".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![step1_id]);
    let step2_id = step2.id;

    let step3 = WorkflowStep::new(
        "Step 3".to_string(),
        "task3".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![step1_id]);
    let step3_id = step3.id;

    let step4 = WorkflowStep::new(
        "Step 4".to_string(),
        "task4".to_string(),
        serde_json::json!({}),
    )
    .with_dependencies(vec![step2_id, step3_id]);

    let workflow = Workflow::new(
        "Complex DAG".to_string(),
        vec![step1, step2, step3, step4],
    );
    let engine = DefaultWorkflowEngine::new();

    let result = engine.execute(&workflow).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state.workflow.status, WorkflowStatus::Completed);
    assert_eq!(state.step_outputs.len(), 4);
}

// ===== Workflow Serialization Tests =====

#[test]
fn test_workflow_serialization() {
    let step = WorkflowStep::new(
        "Step".to_string(),
        "task".to_string(),
        serde_json::json!({"key": "value"}),
    );

    let workflow = Workflow::new("Test Workflow".to_string(), vec![step]);

    let serialized = serde_json::to_string(&workflow).unwrap();
    let deserialized: Workflow = serde_json::from_str(&serialized).unwrap();

    assert_eq!(workflow.id, deserialized.id);
    assert_eq!(workflow.name, deserialized.name);
    assert_eq!(workflow.status, deserialized.status);
    assert_eq!(workflow.steps.len(), deserialized.steps.len());
}

#[test]
fn test_workflow_step_serialization() {
    let step = WorkflowStep::new(
        "Test Step".to_string(),
        "test_task".to_string(),
        serde_json::json!({"param": "value"}),
    )
    .with_dependencies(vec![Uuid::new_v4()])
    .with_max_retries(5);

    let serialized = serde_json::to_string(&step).unwrap();
    let deserialized: WorkflowStep = serde_json::from_str(&serialized).unwrap();

    assert_eq!(step.id, deserialized.id);
    assert_eq!(step.name, deserialized.name);
    assert_eq!(step.task_type, deserialized.task_type);
    assert_eq!(step.max_retries, deserialized.max_retries);
    assert_eq!(step.dependencies.len(), deserialized.dependencies.len());
}
