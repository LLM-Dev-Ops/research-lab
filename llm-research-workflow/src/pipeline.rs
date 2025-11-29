use async_trait::async_trait;
use llm_research_core::{Result, CoreError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: Uuid,
    pub name: String,
    pub stages: Vec<PipelineStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub id: Uuid,
    pub name: String,
    pub parallel: bool,
    pub tasks: Vec<PipelineTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTask {
    pub id: Uuid,
    pub name: String,
    pub task_type: String,
    pub config: serde_json::Value,
    /// Task IDs that must complete before this task can run
    pub dependencies: Vec<Uuid>,
}

impl PipelineTask {
    pub fn new(name: String, task_type: String, config: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            task_type,
            config,
            dependencies: vec![],
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<Uuid>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

/// Directed Acyclic Graph (DAG) representation for task dependencies
#[derive(Debug, Clone)]
pub struct TaskDAG {
    tasks: HashMap<Uuid, PipelineTask>,
    edges: HashMap<Uuid, Vec<Uuid>>, // task_id -> dependent_task_ids
}

impl TaskDAG {
    pub fn from_pipeline(pipeline: &Pipeline) -> Result<Self> {
        let mut tasks = HashMap::new();
        let mut edges: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        // Collect all tasks
        for stage in &pipeline.stages {
            for task in &stage.tasks {
                tasks.insert(task.id, task.clone());
                edges.entry(task.id).or_insert_with(Vec::new);

                for dep_id in &task.dependencies {
                    edges.entry(*dep_id).or_insert_with(Vec::new).push(task.id);
                }
            }
        }

        let dag = Self { tasks, edges };

        // Validate no cycles
        if dag.has_cycle() {
            return Err(CoreError::Validation(
                "Pipeline contains circular dependencies".to_string()
            ));
        }

        Ok(dag)
    }

    /// Topological sort to get execution order
    pub fn topological_sort(&self) -> Result<Vec<Uuid>> {
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = Vec::new();

        // Calculate in-degrees
        for task_id in self.tasks.keys() {
            in_degree.insert(*task_id, 0);
        }

        for task in self.tasks.values() {
            for dep_id in &task.dependencies {
                *in_degree.entry(task.id).or_insert(0) += 1;
            }
        }

        // Find tasks with no dependencies
        for (task_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push(*task_id);
            }
        }

        while let Some(task_id) = queue.pop() {
            result.push(task_id);

            if let Some(dependents) = self.edges.get(&task_id) {
                for dependent_id in dependents {
                    let degree = in_degree.get_mut(dependent_id).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(*dependent_id);
                    }
                }
            }
        }

        if result.len() != self.tasks.len() {
            return Err(CoreError::Validation(
                "Pipeline contains circular dependencies".to_string()
            ));
        }

        Ok(result)
    }

    fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in self.tasks.keys() {
            if self.has_cycle_util(*task_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    fn has_cycle_util(
        &self,
        task_id: Uuid,
        visited: &mut HashSet<Uuid>,
        rec_stack: &mut HashSet<Uuid>,
    ) -> bool {
        if rec_stack.contains(&task_id) {
            return true;
        }
        if visited.contains(&task_id) {
            return false;
        }

        visited.insert(task_id);
        rec_stack.insert(task_id);

        if let Some(dependents) = self.edges.get(&task_id) {
            for dependent in dependents {
                if self.has_cycle_util(*dependent, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(&task_id);
        false
    }

    pub fn get_ready_tasks(&self, completed: &HashSet<Uuid>) -> Vec<Uuid> {
        self.tasks
            .values()
            .filter(|task| {
                !completed.contains(&task.id)
                    && task.dependencies.iter().all(|dep| completed.contains(dep))
            })
            .map(|task| task.id)
            .collect()
    }
}

#[async_trait]
pub trait PipelineExecutor {
    async fn run(&self, pipeline: &Pipeline) -> Result<HashMap<Uuid, serde_json::Value>>;
}

pub struct ExperimentPipeline;

impl ExperimentPipeline {
    pub fn new() -> Self {
        Self
    }

    pub fn default_pipeline() -> Pipeline {
        Pipeline {
            id: Uuid::new_v4(),
            name: "Default Experiment Pipeline".to_string(),
            stages: vec![
                PipelineStage {
                    id: Uuid::new_v4(),
                    name: "Data Loading".to_string(),
                    parallel: false,
                    tasks: vec![PipelineTask::new(
                        "load_dataset".to_string(),
                        "data_loading".to_string(),
                        serde_json::json!({}),
                    )],
                },
                PipelineStage {
                    id: Uuid::new_v4(),
                    name: "Model Inference".to_string(),
                    parallel: true,
                    tasks: vec![PipelineTask::new(
                        "run_inference".to_string(),
                        "inference".to_string(),
                        serde_json::json!({}),
                    )],
                },
                PipelineStage {
                    id: Uuid::new_v4(),
                    name: "Evaluation".to_string(),
                    parallel: true,
                    tasks: vec![
                        PipelineTask::new(
                            "calculate_metrics".to_string(),
                            "evaluation".to_string(),
                            serde_json::json!({}),
                        ),
                        PipelineTask::new(
                            "aggregate_results".to_string(),
                            "aggregation".to_string(),
                            serde_json::json!({}),
                        ),
                    ],
                },
                PipelineStage {
                    id: Uuid::new_v4(),
                    name: "Reporting".to_string(),
                    parallel: false,
                    tasks: vec![
                        PipelineTask::new(
                            "generate_report".to_string(),
                            "reporting".to_string(),
                            serde_json::json!({}),
                        ),
                        PipelineTask::new(
                            "save_results".to_string(),
                            "storage".to_string(),
                            serde_json::json!({}),
                        ),
                    ],
                },
            ],
        }
    }
}

impl Default for ExperimentPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineExecutor for ExperimentPipeline {
    async fn run(&self, pipeline: &Pipeline) -> Result<HashMap<Uuid, serde_json::Value>> {
        tracing::info!("Running pipeline: {}", pipeline.name);

        let mut task_outputs = HashMap::new();

        for stage in &pipeline.stages {
            tracing::info!("Executing stage: {}", stage.name);

            if stage.parallel && stage.tasks.len() > 1 {
                // Execute tasks in parallel
                let handles: Vec<_> = stage
                    .tasks
                    .iter()
                    .map(|task| {
                        let task = task.clone();
                        tokio::spawn(async move {
                            tracing::info!("Executing task: {}", task.name);
                            // Mock task execution
                            (
                                task.id,
                                serde_json::json!({
                                    "task": task.name,
                                    "status": "completed"
                                }),
                            )
                        })
                    })
                    .collect();

                for handle in handles {
                    let (task_id, output) = handle.await.map_err(|e| {
                        CoreError::Internal(format!("Task failed: {}", e))
                    })?;
                    task_outputs.insert(task_id, output);
                }
            } else {
                // Execute tasks sequentially
                for task in &stage.tasks {
                    tracing::info!("Executing task: {}", task.name);
                    // Mock task execution
                    task_outputs.insert(
                        task.id,
                        serde_json::json!({
                            "task": task.name,
                            "status": "completed"
                        }),
                    );
                }
            }
        }

        Ok(task_outputs)
    }
}
