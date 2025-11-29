use llm_research_workflow::pipeline::*;
use uuid::Uuid;
use std::collections::HashSet;

// ===== Pipeline Construction Tests =====

#[test]
fn test_pipeline_creation() {
    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test Pipeline".to_string(),
        stages: vec![],
    };

    assert_eq!(pipeline.name, "Test Pipeline");
    assert!(pipeline.stages.is_empty());
}

#[test]
fn test_pipeline_with_stages() {
    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage 1".to_string(),
        parallel: false,
        tasks: vec![],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test Pipeline".to_string(),
        stages: vec![stage],
    };

    assert_eq!(pipeline.stages.len(), 1);
    assert_eq!(pipeline.stages[0].name, "Stage 1");
}

#[test]
fn test_pipeline_task_creation() {
    let task = PipelineTask::new(
        "Load Data".to_string(),
        "data_loading".to_string(),
        serde_json::json!({"path": "/data"}),
    );

    assert_eq!(task.name, "Load Data");
    assert_eq!(task.task_type, "data_loading");
    assert!(task.dependencies.is_empty());
}

#[test]
fn test_pipeline_task_with_dependencies() {
    let dep_id = Uuid::new_v4();
    let task = PipelineTask::new(
        "Process Data".to_string(),
        "processing".to_string(),
        serde_json::json!({}),
    ).with_dependencies(vec![dep_id]);

    assert_eq!(task.dependencies.len(), 1);
    assert_eq!(task.dependencies[0], dep_id);
}

#[test]
fn test_default_pipeline() {
    let pipeline = ExperimentPipeline::default_pipeline();

    assert_eq!(pipeline.name, "Default Experiment Pipeline");
    assert_eq!(pipeline.stages.len(), 4);

    // Check stage names
    assert_eq!(pipeline.stages[0].name, "Data Loading");
    assert_eq!(pipeline.stages[1].name, "Model Inference");
    assert_eq!(pipeline.stages[2].name, "Evaluation");
    assert_eq!(pipeline.stages[3].name, "Reporting");
}

// ===== DAG Construction Tests =====

#[test]
fn test_dag_from_simple_pipeline() {
    let task1 = PipelineTask::new(
        "Task 1".to_string(),
        "type1".to_string(),
        serde_json::json!({}),
    );

    let task2 = PipelineTask::new(
        "Task 2".to_string(),
        "type2".to_string(),
        serde_json::json!({}),
    ).with_dependencies(vec![task1.id]);

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline);
    assert!(dag.is_ok());
}

#[test]
fn test_dag_multiple_stages() {
    let task1 = PipelineTask::new(
        "Task 1".to_string(),
        "type1".to_string(),
        serde_json::json!({}),
    );
    let task1_id = task1.id;

    let task2 = PipelineTask::new(
        "Task 2".to_string(),
        "type2".to_string(),
        serde_json::json!({}),
    ).with_dependencies(vec![task1_id]);

    let stage1 = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage 1".to_string(),
        parallel: false,
        tasks: vec![task1],
    };

    let stage2 = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage 2".to_string(),
        parallel: false,
        tasks: vec![task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Multi-Stage Pipeline".to_string(),
        stages: vec![stage1, stage2],
    };

    let dag = TaskDAG::from_pipeline(&pipeline);
    assert!(dag.is_ok());
}

// ===== Topological Sort Tests =====

#[test]
fn test_topological_sort_simple() {
    let task1 = PipelineTask::new(
        "Task 1".to_string(),
        "type1".to_string(),
        serde_json::json!({}),
    );
    let task1_id = task1.id;

    let task2 = PipelineTask::new(
        "Task 2".to_string(),
        "type2".to_string(),
        serde_json::json!({}),
    ).with_dependencies(vec![task1_id]);
    let task2_id = task2.id;

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();
    let sorted = dag.topological_sort().unwrap();

    assert_eq!(sorted.len(), 2);
    // Task 1 should come before Task 2
    let task1_pos = sorted.iter().position(|&id| id == task1_id).unwrap();
    let task2_pos = sorted.iter().position(|&id| id == task2_id).unwrap();
    assert!(task1_pos < task2_pos);
}

#[test]
fn test_topological_sort_complex() {
    // Create a DAG: task1 -> task2 -> task4
    //                task1 -> task3 -> task4
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task1_id = task1.id;

    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id]);
    let task2_id = task2.id;

    let task3 = PipelineTask::new("Task 3".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id]);
    let task3_id = task3.id;

    let task4 = PipelineTask::new("Task 4".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task2_id, task3_id]);
    let task4_id = task4.id;

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2, task3, task4],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Complex Pipeline".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();
    let sorted = dag.topological_sort().unwrap();

    assert_eq!(sorted.len(), 4);

    // Verify ordering constraints
    let task1_pos = sorted.iter().position(|&id| id == task1_id).unwrap();
    let task2_pos = sorted.iter().position(|&id| id == task2_id).unwrap();
    let task3_pos = sorted.iter().position(|&id| id == task3_id).unwrap();
    let task4_pos = sorted.iter().position(|&id| id == task4_id).unwrap();

    assert!(task1_pos < task2_pos);
    assert!(task1_pos < task3_pos);
    assert!(task2_pos < task4_pos);
    assert!(task3_pos < task4_pos);
}

#[test]
fn test_topological_sort_independent_tasks() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}));
    let task3 = PipelineTask::new("Task 3".to_string(), "type".to_string(), serde_json::json!({}));

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Parallel Stage".to_string(),
        parallel: true,
        tasks: vec![task1, task2, task3],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Parallel Pipeline".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();
    let sorted = dag.topological_sort().unwrap();

    // All tasks should be included
    assert_eq!(sorted.len(), 3);
}

// ===== Cycle Detection Tests =====

#[test]
fn test_cycle_detection_simple() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task1_id = task1.id;

    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id]);
    let task2_id = task2.id;

    // Create cycle: task1 depends on task2, task2 depends on task1
    let mut task1_cyclic = task1.clone();
    task1_cyclic.dependencies = vec![task2_id];

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1_cyclic, task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Cyclic Pipeline".to_string(),
        stages: vec![stage],
    };

    let result = TaskDAG::from_pipeline(&pipeline);
    assert!(result.is_err());
}

#[test]
fn test_cycle_detection_self_reference() {
    let mut task = PipelineTask::new("Task".to_string(), "type".to_string(), serde_json::json!({}));
    let task_id = task.id;
    task.dependencies = vec![task_id]; // Self-reference

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Self-Referencing Pipeline".to_string(),
        stages: vec![stage],
    };

    let result = TaskDAG::from_pipeline(&pipeline);
    assert!(result.is_err());
}

#[test]
fn test_no_cycle_in_valid_pipeline() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task1_id = task1.id;

    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id]);

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Valid Pipeline".to_string(),
        stages: vec![stage],
    };

    let result = TaskDAG::from_pipeline(&pipeline);
    assert!(result.is_ok());
}

// ===== Parallel Task Execution Tests =====

#[test]
fn test_get_ready_tasks_initial() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task1_id = task1.id;

    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}));
    let task2_id = task2.id;

    let task3 = PipelineTask::new("Task 3".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id]);

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2, task3],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();
    let completed = HashSet::new();
    let ready = dag.get_ready_tasks(&completed);

    // Task 1 and Task 2 have no dependencies, so should be ready
    assert_eq!(ready.len(), 2);
    assert!(ready.contains(&task1_id));
    assert!(ready.contains(&task2_id));
}

#[test]
fn test_get_ready_tasks_after_completion() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task1_id = task1.id;

    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id]);
    let task2_id = task2.id;

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();

    // Initially, only task1 is ready
    let completed = HashSet::new();
    let ready = dag.get_ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&task1_id));

    // After task1 completes, task2 becomes ready
    let mut completed = HashSet::new();
    completed.insert(task1_id);
    let ready = dag.get_ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&task2_id));
}

#[test]
fn test_get_ready_tasks_multiple_dependencies() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task1_id = task1.id;

    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}));
    let task2_id = task2.id;

    let task3 = PipelineTask::new("Task 3".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![task1_id, task2_id]);
    let task3_id = task3.id;

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2, task3],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();

    // Only task1 completed
    let mut completed = HashSet::new();
    completed.insert(task1_id);
    let ready = dag.get_ready_tasks(&completed);
    // task2 is ready (no deps), but task3 is not (still needs task2)
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&task2_id));

    // Both task1 and task2 completed
    completed.insert(task2_id);
    let ready = dag.get_ready_tasks(&completed);
    // Now task3 is ready
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&task3_id));
}

// ===== Pipeline Executor Tests =====

#[tokio::test]
async fn test_pipeline_executor_simple() {
    let pipeline = ExperimentPipeline::default_pipeline();
    let executor = ExperimentPipeline::new();

    let result = executor.run(&pipeline).await;
    assert!(result.is_ok());

    let outputs = result.unwrap();
    // Should have output for each task
    assert!(outputs.len() > 0);
}

#[tokio::test]
async fn test_pipeline_executor_parallel_stage() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}));
    let task3 = PipelineTask::new("Task 3".to_string(), "type".to_string(), serde_json::json!({}));

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Parallel Stage".to_string(),
        parallel: true,
        tasks: vec![task1, task2, task3],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Parallel Test".to_string(),
        stages: vec![stage],
    };

    let executor = ExperimentPipeline::new();
    let result = executor.run(&pipeline).await;
    assert!(result.is_ok());

    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 3);
}

#[tokio::test]
async fn test_pipeline_executor_sequential_stage() {
    let task1 = PipelineTask::new("Task 1".to_string(), "type".to_string(), serde_json::json!({}));
    let task2 = PipelineTask::new("Task 2".to_string(), "type".to_string(), serde_json::json!({}));

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Sequential Stage".to_string(),
        parallel: false,
        tasks: vec![task1, task2],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Sequential Test".to_string(),
        stages: vec![stage],
    };

    let executor = ExperimentPipeline::new();
    let result = executor.run(&pipeline).await;
    assert!(result.is_ok());

    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 2);
}

// ===== Edge Cases =====

#[test]
fn test_empty_pipeline() {
    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Empty".to_string(),
        stages: vec![],
    };

    let dag = TaskDAG::from_pipeline(&pipeline);
    assert!(dag.is_ok());
}

#[test]
fn test_pipeline_with_empty_stage() {
    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Empty Stage".to_string(),
        parallel: false,
        tasks: vec![],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline);
    assert!(dag.is_ok());
}

#[test]
fn test_task_with_nonexistent_dependency() {
    let task = PipelineTask::new("Task".to_string(), "type".to_string(), serde_json::json!({}))
        .with_dependencies(vec![Uuid::new_v4()]); // Non-existent dependency

    let stage = PipelineStage {
        id: Uuid::new_v4(),
        name: "Stage".to_string(),
        parallel: false,
        tasks: vec![task],
    };

    let pipeline = Pipeline {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        stages: vec![stage],
    };

    let dag = TaskDAG::from_pipeline(&pipeline).unwrap();
    // This should work, but task will never be ready since dependency doesn't exist
    let ready = dag.get_ready_tasks(&HashSet::new());
    assert_eq!(ready.len(), 0); // Task can't run because dependency is missing
}
