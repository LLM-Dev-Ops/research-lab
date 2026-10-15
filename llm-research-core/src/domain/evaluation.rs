use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub sample_id: Uuid,
    pub input: String,
    pub output: String,
    pub expected_output: Option<String>,
    pub latency_ms: i64,
    pub token_count: i32,
    pub cost: Option<Decimal>,
    pub metrics: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Inputs used to create a new evaluation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvaluation {
    pub experiment_id: Uuid,
    pub sample_id: Uuid,
    pub input: String,
    pub output: String,
    pub expected_output: Option<String>,
    pub latency_ms: i64,
    pub token_count: i32,
    pub cost: Option<Decimal>,
    pub metrics: serde_json::Value,
}

impl Evaluation {
    pub fn new(details: NewEvaluation) -> Self {
        Self {
            id: Uuid::new_v4(),
            experiment_id: details.experiment_id,
            sample_id: details.sample_id,
            input: details.input,
            output: details.output,
            expected_output: details.expected_output,
            latency_ms: details.latency_ms,
            token_count: details.token_count,
            cost: details.cost,
            metrics: details.metrics,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub accuracy: Option<Decimal>,
    pub precision: Option<Decimal>,
    pub recall: Option<Decimal>,
    pub f1_score: Option<Decimal>,
    pub bleu_score: Option<Decimal>,
    pub rouge_scores: Option<serde_json::Value>,
    pub custom_metrics: serde_json::Value,
}
