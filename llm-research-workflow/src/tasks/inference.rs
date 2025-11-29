use async_trait::async_trait;
use llm_research_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration, Instant};

use super::{Task, TaskContext, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InferenceProvider {
    OpenAI,
    Anthropic,
    Cohere,
    HuggingFace,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub provider: InferenceProvider,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub rate_limit_per_minute: usize,
    pub max_retries: usize,
    pub timeout_seconds: u64,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            provider: InferenceProvider::OpenAI,
            model: "gpt-4".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            rate_limit_per_minute: 60,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }
}

pub struct InferenceTask {
    config: InferenceConfig,
}

impl InferenceTask {
    pub fn new(config: InferenceConfig) -> Self {
        Self { config }
    }

    /// Execute inference with rate limiting
    async fn execute_with_rate_limit(
        &self,
        prompts: &[String],
    ) -> Result<Vec<InferenceResult>> {
        let rate_limiter = Arc::new(Semaphore::new(self.config.rate_limit_per_minute));
        let mut handles = Vec::new();

        for (idx, prompt) in prompts.iter().enumerate() {
            let rate_limiter = Arc::clone(&rate_limiter);
            let config = self.config.clone();
            let prompt = prompt.clone();

            let handle = tokio::spawn(async move {
                // Acquire rate limit permit
                let _permit = rate_limiter.acquire().await.unwrap();

                let start = Instant::now();
                let result = Self::execute_single_inference(&config, &prompt, idx).await;
                let latency = start.elapsed().as_millis() as u64;

                // Release permit after minimum delay (to maintain rate limit)
                let min_delay = Duration::from_millis(60_000 / config.rate_limit_per_minute as u64);
                if start.elapsed() < min_delay {
                    sleep(min_delay - start.elapsed()).await;
                }

                (result, latency)
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            let (result, latency) = handle.await.map_err(|e| {
                llm_research_core::CoreError::Internal(format!("Inference task failed: {}", e))
            })?;
            let mut result = result?;
            result.latency_ms = latency;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute single inference with retries
    async fn execute_single_inference(
        config: &InferenceConfig,
        prompt: &str,
        index: usize,
    ) -> Result<InferenceResult> {
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            match Self::call_provider(config, prompt, index).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < config.max_retries {
                        tracing::warn!(
                            "Inference attempt {} failed, retrying...",
                            attempt + 1
                        );
                        sleep(Duration::from_secs(2_u64.pow(attempt as u32))).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Mock call to inference provider
    async fn call_provider(
        config: &InferenceConfig,
        prompt: &str,
        index: usize,
    ) -> Result<InferenceResult> {
        // Simulate API call
        sleep(Duration::from_millis(100 + (index % 200) as u64)).await;

        // Mock response based on provider
        let response = match config.provider {
            InferenceProvider::OpenAI => format!("OpenAI {} response to: {}", config.model, prompt),
            InferenceProvider::Anthropic => format!("Claude {} response to: {}", config.model, prompt),
            InferenceProvider::Cohere => format!("Cohere response to: {}", prompt),
            InferenceProvider::HuggingFace => format!("HF {} response to: {}", config.model, prompt),
            InferenceProvider::Local => format!("Local model response to: {}", prompt),
        };

        let tokens_used = prompt.len() / 4 + response.len() / 4;

        Ok(InferenceResult {
            index,
            prompt: prompt.to_string(),
            response,
            tokens_used,
            latency_ms: 0, // Will be set by caller
            provider: format!("{:?}", config.provider),
            model: config.model.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub index: usize,
    pub prompt: String,
    pub response: String,
    pub tokens_used: usize,
    pub latency_ms: u64,
    pub provider: String,
    pub model: String,
}

#[async_trait]
impl Task for InferenceTask {
    async fn execute(&self, context: TaskContext) -> Result<TaskResult> {
        tracing::info!(
            "Running inference for experiment: {} using {:?}",
            context.experiment_id,
            self.config.provider
        );

        // Mock prompts - in real system, these would come from data loading task
        let prompts: Vec<String> = (0..10)
            .map(|i| format!("Test prompt {}", i))
            .collect();

        let start = Instant::now();
        let results = self.execute_with_rate_limit(&prompts).await?;
        let total_duration = start.elapsed();

        let total_tokens: usize = results.iter().map(|r| r.tokens_used).sum();
        let avg_latency: f64 = results.iter().map(|r| r.latency_ms as f64).sum::<f64>()
            / results.len() as f64;

        let output = json!({
            "provider": format!("{:?}", self.config.provider),
            "model": self.config.model,
            "predictions_generated": results.len(),
            "total_tokens": total_tokens,
            "avg_latency_ms": avg_latency,
            "total_duration_ms": total_duration.as_millis(),
            "results": results,
        });

        Ok(TaskResult::success(output))
    }

    fn name(&self) -> &str {
        "inference"
    }
}
