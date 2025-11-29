use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result, CoreError};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerplexityInput {
    /// Log probabilities for each token
    pub log_probs: Vec<f64>,
    /// Optional token count (if different from log_probs length)
    pub token_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerplexityOutput {
    pub perplexity: f64,
    pub cross_entropy: f64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PerplexityCalculator {
    pub base: f64,
}

impl PerplexityCalculator {
    pub fn new() -> Self {
        Self { base: std::f64::consts::E }
    }

    pub fn with_base(mut self, base: f64) -> Self {
        self.base = base;
        self
    }

    /// Calculate perplexity from log probabilities
    /// Perplexity = exp(average negative log likelihood)
    pub fn calculate_perplexity(&self, log_probs: &[f64]) -> Result<(f64, f64)> {
        if log_probs.is_empty() {
            return Err(CoreError::Validation(
                "Cannot calculate perplexity with empty log probabilities".to_string()
            ));
        }

        // Calculate cross-entropy (average negative log likelihood)
        let sum_log_probs: f64 = log_probs.iter().sum();
        let cross_entropy = -sum_log_probs / log_probs.len() as f64;

        // Calculate perplexity
        // If log_probs are natural log, perplexity = exp(cross_entropy)
        // If log_probs are log base 2, perplexity = 2^(cross_entropy)
        let perplexity = if self.base == std::f64::consts::E {
            cross_entropy.exp()
        } else if self.base == 2.0 {
            2_f64.powf(cross_entropy)
        } else {
            self.base.powf(cross_entropy)
        };

        Ok((perplexity, cross_entropy))
    }

    /// Calculate perplexity from raw probabilities (converts to log probs)
    pub fn calculate_from_probs(&self, probs: &[f64]) -> Result<(f64, f64)> {
        if probs.is_empty() {
            return Err(CoreError::Validation(
                "Cannot calculate perplexity with empty probabilities".to_string()
            ));
        }

        let log_probs: Vec<f64> = probs
            .iter()
            .map(|&p| {
                if p <= 0.0 {
                    -1000.0 // Very small probability
                } else if self.base == std::f64::consts::E {
                    p.ln()
                } else if self.base == 2.0 {
                    p.log2()
                } else {
                    p.log(self.base)
                }
            })
            .collect();

        self.calculate_perplexity(&log_probs)
    }
}

impl Default for PerplexityCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MetricCalculator for PerplexityCalculator {
    type Input = PerplexityInput;
    type Output = PerplexityOutput;

    async fn calculate(&self, input: Self::Input) -> Result<Self::Output> {
        let (perplexity, cross_entropy) = self.calculate_perplexity(&input.log_probs)?;

        Ok(PerplexityOutput {
            perplexity,
            cross_entropy,
            metadata: json!({
                "metric": "perplexity",
                "base": self.base,
                "token_count": input.token_count.unwrap_or(input.log_probs.len()),
            }),
        })
    }
}
