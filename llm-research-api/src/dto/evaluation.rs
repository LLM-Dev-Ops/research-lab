use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateEvaluationRequest {
    pub experiment_id: Uuid,
    pub sample_id: Uuid,
    #[validate(length(min = 1))]
    pub input: String,
    #[validate(length(min = 1))]
    pub output: String,
    pub expected_output: Option<String>,
    pub latency_ms: i64,
    pub token_count: i32,
    pub cost: Option<Decimal>,
    pub metrics: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationResponse {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub experiment_id: Uuid,
    pub total_samples: i64,
    pub avg_latency_ms: f64,
    pub total_tokens: i64,
    pub total_cost: Option<Decimal>,
    pub accuracy: Option<Decimal>,
    pub custom_metrics: serde_json::Value,
}
