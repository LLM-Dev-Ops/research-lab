pub mod data_loading;
pub mod inference;
pub mod evaluation;
pub mod reporting;

pub use data_loading::*;
pub use inference::*;
pub use evaluation::*;
pub use reporting::*;

use async_trait::async_trait;
use llm_research_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub experiment_id: uuid::Uuid,
    pub config: serde_json::Value,
}

#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self, context: TaskContext) -> Result<TaskResult>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

impl TaskResult {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(error),
        }
    }
}
