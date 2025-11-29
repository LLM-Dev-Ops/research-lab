use async_trait::async_trait;
use llm_research_core::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Semaphore, RwLock, broadcast};
use uuid::Uuid;

use crate::tasks::{Task, TaskContext, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: Uuid,
    pub task_name: String,
    pub progress: f64, // 0.0 to 1.0
    pub status: String,
    pub message: Option<String>,
}

pub struct TaskExecutor {
    max_concurrency: usize,
    progress_tx: Arc<RwLock<Option<broadcast::Sender<TaskProgress>>>>,
    cancellation_tokens: Arc<RwLock<std::collections::HashMap<Uuid, tokio_util::sync::CancellationToken>>>,
}

impl TaskExecutor {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency,
            progress_tx: Arc::new(RwLock::new(None)),
            cancellation_tokens: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Enable progress tracking
    pub async fn enable_progress_tracking(&self) -> broadcast::Receiver<TaskProgress> {
        let (tx, rx) = broadcast::channel(100);
        *self.progress_tx.write().await = Some(tx);
        rx
    }

    /// Report progress for a task
    async fn report_progress(&self, progress: TaskProgress) {
        if let Some(tx) = self.progress_tx.read().await.as_ref() {
            let _ = tx.send(progress);
        }
    }

    /// Cancel a task by ID
    pub async fn cancel_task(&self, task_id: Uuid) {
        if let Some(token) = self.cancellation_tokens.read().await.get(&task_id) {
            token.cancel();
        }
    }

    /// Execute a single task with progress tracking and cancellation support
    pub async fn execute_one(
        &self,
        task: Arc<dyn Task>,
        context: TaskContext,
    ) -> Result<TaskResult> {
        let task_id = Uuid::new_v4();
        let task_name = task.name().to_string();

        // Create cancellation token
        let cancel_token = tokio_util::sync::CancellationToken::new();
        self.cancellation_tokens.write().await.insert(task_id, cancel_token.clone());

        // Report start
        self.report_progress(TaskProgress {
            task_id,
            task_name: task_name.clone(),
            progress: 0.0,
            status: "starting".to_string(),
            message: None,
        }).await;

        // Execute task with cancellation support
        let result = tokio::select! {
            _ = cancel_token.cancelled() => {
                self.report_progress(TaskProgress {
                    task_id,
                    task_name: task_name.clone(),
                    progress: 0.0,
                    status: "cancelled".to_string(),
                    message: Some("Task was cancelled".to_string()),
                }).await;

                Ok(TaskResult::failure("Task cancelled".to_string()))
            }
            result = task.execute(context) => {
                match result {
                    Ok(task_result) => {
                        self.report_progress(TaskProgress {
                            task_id,
                            task_name: task_name.clone(),
                            progress: 1.0,
                            status: if task_result.success { "completed" } else { "failed" }.to_string(),
                            message: task_result.error.clone(),
                        }).await;
                        Ok(task_result)
                    }
                    Err(e) => {
                        self.report_progress(TaskProgress {
                            task_id,
                            task_name: task_name.clone(),
                            progress: 0.0,
                            status: "error".to_string(),
                            message: Some(e.to_string()),
                        }).await;
                        Ok(TaskResult::failure(e.to_string()))
                    }
                }
            }
        };

        // Cleanup cancellation token
        self.cancellation_tokens.write().await.remove(&task_id);

        result
    }

    /// Execute multiple tasks in batch with concurrency control
    pub async fn execute_batch(
        &self,
        tasks: Vec<Arc<dyn Task>>,
        context: TaskContext,
    ) -> Result<Vec<TaskResult>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let mut handles = vec![];

        for task in tasks {
            let semaphore = semaphore.clone();
            let context = context.clone();
            let executor = self.clone_for_task();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                executor.execute_one(task, context).await
            });

            handles.push(handle);
        }

        let mut results = vec![];
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => results.push(TaskResult::failure(e.to_string())),
                Err(e) => results.push(TaskResult::failure(format!("Task panicked: {}", e))),
            }
        }

        Ok(results)
    }

    fn clone_for_task(&self) -> Self {
        Self {
            max_concurrency: self.max_concurrency,
            progress_tx: Arc::clone(&self.progress_tx),
            cancellation_tokens: Arc::clone(&self.cancellation_tokens),
        }
    }
}
