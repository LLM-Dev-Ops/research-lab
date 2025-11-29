use async_trait::async_trait;
use llm_research_core::{Result, CoreError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub status: WorkflowStatus,
    pub steps: Vec<WorkflowStep>,
    pub error: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Workflow {
    pub fn new(name: String, steps: Vec<WorkflowStep>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            status: WorkflowStatus::Pending,
            steps,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: Uuid,
    pub name: String,
    pub task_type: String,
    pub config: serde_json::Value,
    pub dependencies: Vec<Uuid>,
    pub status: WorkflowStatus,
    pub error: Option<String>,
    pub retry_count: usize,
    pub max_retries: usize,
}

impl WorkflowStep {
    pub fn new(name: String, task_type: String, config: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            task_type,
            config,
            dependencies: vec![],
            status: WorkflowStatus::Pending,
            error: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<Uuid>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowState {
    pub workflow: Workflow,
    pub step_outputs: HashMap<Uuid, serde_json::Value>,
}

#[async_trait]
pub trait WorkflowEngine: Send + Sync {
    async fn execute(&self, workflow: &Workflow) -> Result<WorkflowState>;
    async fn pause(&self, workflow_id: Uuid) -> Result<()>;
    async fn resume(&self, workflow_id: Uuid) -> Result<()>;
    async fn cancel(&self, workflow_id: Uuid) -> Result<()>;
}

pub struct DefaultWorkflowEngine {
    states: Arc<RwLock<HashMap<Uuid, WorkflowState>>>,
}

impl DefaultWorkflowEngine {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn execute_step(
        &self,
        step: &mut WorkflowStep,
        state: &WorkflowState,
    ) -> Result<serde_json::Value> {
        // Check dependencies are completed
        for dep_id in &step.dependencies {
            if !state.step_outputs.contains_key(dep_id) {
                return Err(CoreError::InvalidState(format!(
                    "Dependency step {} not completed",
                    dep_id
                )));
            }
        }

        step.status = WorkflowStatus::Running;

        // Simulate step execution with retries
        let mut last_error = None;
        for attempt in 0..=step.max_retries {
            step.retry_count = attempt;

            // Here we would actually execute the task
            // For now, return a mock result
            match Self::execute_task_type(&step.task_type, &step.config, state).await {
                Ok(output) => {
                    step.status = WorkflowStatus::Completed;
                    return Ok(output);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < step.max_retries {
                        tracing::warn!(
                            "Step {} failed, attempt {}/{}",
                            step.name,
                            attempt + 1,
                            step.max_retries + 1
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(1 << attempt)).await;
                    }
                }
            }
        }

        let error = last_error.unwrap();
        step.status = WorkflowStatus::Failed;
        step.error = Some(error.to_string());
        Err(error)
    }

    async fn execute_task_type(
        task_type: &str,
        config: &serde_json::Value,
        _state: &WorkflowState,
    ) -> Result<serde_json::Value> {
        // Mock implementation - in real system, this would dispatch to actual task executors
        tracing::info!("Executing task type: {}", task_type);
        Ok(serde_json::json!({
            "task_type": task_type,
            "status": "completed",
            "config": config,
        }))
    }

    fn check_dependencies_met(&self, step: &WorkflowStep, state: &WorkflowState) -> bool {
        step.dependencies
            .iter()
            .all(|dep_id| state.step_outputs.contains_key(dep_id))
    }
}

impl Default for DefaultWorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowEngine for DefaultWorkflowEngine {
    async fn execute(&self, workflow: &Workflow) -> Result<WorkflowState> {
        let mut state = WorkflowState {
            workflow: workflow.clone(),
            step_outputs: HashMap::new(),
        };

        state.workflow.status = WorkflowStatus::Running;
        state.workflow.started_at = Some(chrono::Utc::now());

        tracing::info!("Executing workflow: {}", workflow.name);

        // Store initial state
        {
            let mut states = self.states.write().await;
            states.insert(workflow.id, state.clone());
        }

        // Execute steps in dependency order
        let mut completed_steps = std::collections::HashSet::new();

        while completed_steps.len() < state.workflow.steps.len() {
            let mut made_progress = false;

            for i in 0..state.workflow.steps.len() {
                let step_id = state.workflow.steps[i].id;

                if completed_steps.contains(&step_id) {
                    continue;
                }

                // Check if dependencies are met
                if !self.check_dependencies_met(&state.workflow.steps[i], &state) {
                    continue;
                }

                // Execute step - clone step to avoid borrow issues
                let mut step = state.workflow.steps[i].clone();
                match self.execute_step(&mut step, &state).await {
                    Ok(output) => {
                        state.workflow.steps[i] = step;
                        state.step_outputs.insert(step_id, output);
                        completed_steps.insert(step_id);
                        made_progress = true;
                    }
                    Err(e) => {
                        state.workflow.status = WorkflowStatus::Failed;
                        state.workflow.error = Some(e.to_string());
                        state.workflow.completed_at = Some(chrono::Utc::now());

                        // Update state
                        let mut states = self.states.write().await;
                        states.insert(workflow.id, state.clone());

                        return Err(e);
                    }
                }

                // Update state after each step
                let mut states = self.states.write().await;
                states.insert(workflow.id, state.clone());
            }

            if !made_progress {
                let error = CoreError::InvalidState(
                    "Workflow deadlock: no steps can be executed".to_string()
                );
                state.workflow.status = WorkflowStatus::Failed;
                state.workflow.error = Some(error.to_string());
                state.workflow.completed_at = Some(chrono::Utc::now());
                return Err(error);
            }
        }

        state.workflow.status = WorkflowStatus::Completed;
        state.workflow.completed_at = Some(chrono::Utc::now());

        // Update final state
        {
            let mut states = self.states.write().await;
            states.insert(workflow.id, state.clone());
        }

        tracing::info!("Workflow completed: {}", workflow.name);
        Ok(state)
    }

    async fn pause(&self, workflow_id: Uuid) -> Result<()> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(&workflow_id) {
            state.workflow.status = WorkflowStatus::Paused;
            tracing::info!("Paused workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(CoreError::NotFound(format!("Workflow {} not found", workflow_id)))
        }
    }

    async fn resume(&self, workflow_id: Uuid) -> Result<()> {
        let workflow_to_resume = {
            let states = self.states.read().await;
            if let Some(state) = states.get(&workflow_id) {
                if state.workflow.status == WorkflowStatus::Paused {
                    Some(state.workflow.clone())
                } else {
                    return Err(CoreError::InvalidState(format!(
                        "Workflow {} is not paused",
                        workflow_id
                    )));
                }
            } else {
                return Err(CoreError::NotFound(format!("Workflow {} not found", workflow_id)));
            }
        };

        if let Some(workflow) = workflow_to_resume {
            self.execute(&workflow).await?;
        }
        Ok(())
    }

    async fn cancel(&self, workflow_id: Uuid) -> Result<()> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(&workflow_id) {
            state.workflow.status = WorkflowStatus::Cancelled;
            state.workflow.completed_at = Some(chrono::Utc::now());
            tracing::info!("Cancelled workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(CoreError::NotFound(format!("Workflow {} not found", workflow_id)))
        }
    }
}
