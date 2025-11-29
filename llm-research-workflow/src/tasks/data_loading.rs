use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use llm_research_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Task, TaskContext, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLoadingConfig {
    pub source: String,
    pub batch_size: usize,
    pub stream: bool,
    pub limit: Option<usize>,
}

pub struct DataLoadingTask {
    config: DataLoadingConfig,
}

impl DataLoadingTask {
    pub fn new(config: DataLoadingConfig) -> Self {
        Self { config }
    }

    /// Load data in batches
    async fn load_batched(&self) -> Result<Vec<serde_json::Value>> {
        tracing::info!(
            "Loading data from {} in batches of {}",
            self.config.source,
            self.config.batch_size
        );

        // Mock implementation - in real system, would load from S3, database, etc.
        let total_samples = self.config.limit.unwrap_or(1000);
        let mut batches = Vec::new();

        for batch_start in (0..total_samples).step_by(self.config.batch_size) {
            let batch_end = (batch_start + self.config.batch_size).min(total_samples);
            let batch_size = batch_end - batch_start;

            let batch = json!({
                "batch_id": batch_start / self.config.batch_size,
                "start": batch_start,
                "end": batch_end,
                "size": batch_size,
                "samples": (batch_start..batch_end)
                    .map(|i| json!({"id": i, "text": format!("Sample {}", i)}))
                    .collect::<Vec<_>>()
            });

            batches.push(batch);

            // Simulate some loading time
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(batches)
    }

    /// Load data as a stream
    async fn load_streaming(&self) -> Result<Vec<serde_json::Value>> {
        tracing::info!("Loading data from {} as stream", self.config.source);

        let total_samples = self.config.limit.unwrap_or(1000);

        // Create a stream of data
        let data_stream = stream::iter(0..total_samples)
            .map(|i| json!({"id": i, "text": format!("Sample {}", i)}))
            .chunks(self.config.batch_size);

        let batches: Vec<Vec<serde_json::Value>> = data_stream.collect().await;

        let result = batches
            .into_iter()
            .enumerate()
            .map(|(idx, samples)| {
                json!({
                    "batch_id": idx,
                    "size": samples.len(),
                    "samples": samples
                })
            })
            .collect();

        Ok(result)
    }
}

#[async_trait]
impl Task for DataLoadingTask {
    async fn execute(&self, context: TaskContext) -> Result<TaskResult> {
        tracing::info!(
            "Loading data for experiment: {} from {}",
            context.experiment_id,
            self.config.source
        );

        let batches = if self.config.stream {
            self.load_streaming().await?
        } else {
            self.load_batched().await?
        };

        let total_samples: usize = batches
            .iter()
            .filter_map(|b| b.get("size"))
            .filter_map(|s| s.as_u64())
            .map(|s| s as usize)
            .sum();

        let output = json!({
            "source": self.config.source,
            "batches_loaded": batches.len(),
            "total_samples": total_samples,
            "batch_size": self.config.batch_size,
            "streaming": self.config.stream,
            "batches": batches,
        });

        Ok(TaskResult::success(output))
    }

    fn name(&self) -> &str {
        "data_loading"
    }
}
