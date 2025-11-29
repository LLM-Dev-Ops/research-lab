use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result};
use llm_research_metrics::{
    AccuracyCalculator, BleuCalculator, RougeCalculator, ComparisonMode,
    MetricAggregator, MetricInput,
};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Task, TaskContext, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationConfig {
    pub metrics: Vec<String>,
    pub batch_size: usize,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            metrics: vec!["accuracy".to_string(), "bleu".to_string(), "rouge".to_string()],
            batch_size: 100,
        }
    }
}

pub struct EvaluationTask {
    config: EvaluationConfig,
}

impl EvaluationTask {
    pub fn new(config: EvaluationConfig) -> Self {
        Self { config }
    }

    /// Evaluate a batch of predictions
    async fn evaluate_batch(
        &self,
        predictions: &[(String, String)], // (predicted, reference) pairs
    ) -> Result<BatchEvaluationResult> {
        let mut accuracy_scores = Vec::new();
        let mut bleu_scores = Vec::new();
        let mut rouge_scores = Vec::new();

        let accuracy_calc = AccuracyCalculator::new(ComparisonMode::ExactMatch);
        let bleu_calc = BleuCalculator::default();
        let rouge_calc = RougeCalculator::rouge_l();

        for (predicted, reference) in predictions {
            if self.config.metrics.contains(&"accuracy".to_string()) {
                let input = MetricInput {
                    predicted: predicted.clone(),
                    reference: Some(reference.clone()),
                };
                let result = accuracy_calc.calculate(input).await?;
                accuracy_scores.push(result.score.to_f64().unwrap_or(0.0));
            }

            if self.config.metrics.contains(&"bleu".to_string()) {
                let input = MetricInput {
                    predicted: predicted.clone(),
                    reference: Some(reference.clone()),
                };
                let result = bleu_calc.calculate(input).await?;
                bleu_scores.push(result.score.to_f64().unwrap_or(0.0));
            }

            if self.config.metrics.contains(&"rouge".to_string()) {
                let input = MetricInput {
                    predicted: predicted.clone(),
                    reference: Some(reference.clone()),
                };
                let result = rouge_calc.calculate(input).await?;
                rouge_scores.push(result.score.to_f64().unwrap_or(0.0));
            }
        }

        Ok(BatchEvaluationResult {
            accuracy_scores,
            bleu_scores,
            rouge_scores,
        })
    }

    /// Process evaluation in batches
    async fn evaluate_batched(
        &self,
        pairs: Vec<(String, String)>,
    ) -> Result<Vec<BatchEvaluationResult>> {
        let mut results = Vec::new();

        for chunk in pairs.chunks(self.config.batch_size) {
            let batch_result = self.evaluate_batch(chunk).await?;
            results.push(batch_result);
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
struct BatchEvaluationResult {
    accuracy_scores: Vec<f64>,
    bleu_scores: Vec<f64>,
    rouge_scores: Vec<f64>,
}

#[async_trait]
impl Task for EvaluationTask {
    async fn execute(&self, context: TaskContext) -> Result<TaskResult> {
        tracing::info!(
            "Evaluating results for experiment: {}",
            context.experiment_id
        );

        // Mock prediction/reference pairs - in real system, these would come from previous tasks
        let pairs: Vec<(String, String)> = (0..100)
            .map(|i| {
                let predicted = format!("Predicted response {}", i);
                let reference = if i % 10 == 0 {
                    format!("Predicted response {}", i) // Exact match
                } else {
                    format!("Reference response {}", i)
                };
                (predicted, reference)
            })
            .collect();

        let batch_results = self.evaluate_batched(pairs).await?;

        // Aggregate results
        let all_accuracy: Vec<f64> = batch_results
            .iter()
            .flat_map(|r| r.accuracy_scores.iter())
            .copied()
            .collect();

        let all_bleu: Vec<f64> = batch_results
            .iter()
            .flat_map(|r| r.bleu_scores.iter())
            .copied()
            .collect();

        let all_rouge: Vec<f64> = batch_results
            .iter()
            .flat_map(|r| r.rouge_scores.iter())
            .copied()
            .collect();

        let mut metrics_calculated = Vec::new();
        let mut metric_values = serde_json::Map::new();

        if !all_accuracy.is_empty() {
            let agg = MetricAggregator::aggregate(&all_accuracy);
            metrics_calculated.push("accuracy");
            metric_values.insert("accuracy".to_string(), json!({
                "mean": agg.mean,
                "median": agg.median,
                "std_dev": agg.std_dev,
                "min": agg.min,
                "max": agg.max,
            }));
        }

        if !all_bleu.is_empty() {
            let agg = MetricAggregator::aggregate(&all_bleu);
            metrics_calculated.push("bleu");
            metric_values.insert("bleu".to_string(), json!({
                "mean": agg.mean,
                "median": agg.median,
                "std_dev": agg.std_dev,
            }));
        }

        if !all_rouge.is_empty() {
            let agg = MetricAggregator::aggregate(&all_rouge);
            metrics_calculated.push("rouge");
            metric_values.insert("rouge_l".to_string(), json!({
                "mean": agg.mean,
                "median": agg.median,
                "std_dev": agg.std_dev,
            }));
        }

        let output = json!({
            "metrics_calculated": metrics_calculated,
            "total_samples": all_accuracy.len(),
            "batches_processed": batch_results.len(),
            "metrics": metric_values,
        });

        Ok(TaskResult::success(output))
    }

    fn name(&self) -> &str {
        "evaluation"
    }
}
