pub mod accuracy;
pub mod bleu;
pub mod rouge;
pub mod perplexity;
pub mod latency;

pub use accuracy::*;
pub use bleu::*;
pub use rouge::*;
pub use perplexity::*;
pub use latency::*;

use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricInput {
    pub predicted: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricOutput {
    pub score: Decimal,
    pub metadata: serde_json::Value,
}
