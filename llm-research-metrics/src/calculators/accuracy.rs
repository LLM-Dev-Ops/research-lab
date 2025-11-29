use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{MetricInput, MetricOutput};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonMode {
    ExactMatch,
    Contains,
    SemanticSimilarity,
    CaseInsensitive,
}

#[derive(Debug, Clone)]
pub struct AccuracyCalculator {
    pub mode: ComparisonMode,
    pub similarity_threshold: f64,
}

impl AccuracyCalculator {
    pub fn new(mode: ComparisonMode) -> Self {
        Self {
            mode,
            similarity_threshold: 0.8,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    /// Exact string match (case-sensitive)
    fn exact_match(&self, predicted: &str, reference: &str) -> bool {
        predicted.trim() == reference.trim()
    }

    /// Case-insensitive match
    fn case_insensitive_match(&self, predicted: &str, reference: &str) -> bool {
        predicted.trim().to_lowercase() == reference.trim().to_lowercase()
    }

    /// Check if predicted contains reference (or vice versa)
    fn contains_match(&self, predicted: &str, reference: &str) -> bool {
        let pred = predicted.trim().to_lowercase();
        let refer = reference.trim().to_lowercase();
        pred.contains(&refer) || refer.contains(&pred)
    }

    /// Simple semantic similarity using Jaccard similarity of words
    fn semantic_similarity(&self, predicted: &str, reference: &str) -> f64 {
        let pred_lower = predicted.to_lowercase();
        let pred_words: std::collections::HashSet<_> = pred_lower
            .split_whitespace()
            .collect();
        let ref_lower = reference.to_lowercase();
        let ref_words: std::collections::HashSet<_> = ref_lower
            .split_whitespace()
            .collect();

        if pred_words.is_empty() && ref_words.is_empty() {
            return 1.0;
        }
        if pred_words.is_empty() || ref_words.is_empty() {
            return 0.0;
        }

        let intersection = pred_words.intersection(&ref_words).count();
        let union = pred_words.union(&ref_words).count();

        intersection as f64 / union as f64
    }
}

impl Default for AccuracyCalculator {
    fn default() -> Self {
        Self::new(ComparisonMode::ExactMatch)
    }
}

#[async_trait]
impl MetricCalculator for AccuracyCalculator {
    type Input = MetricInput;
    type Output = MetricOutput;

    async fn calculate(&self, input: Self::Input) -> Result<Self::Output> {
        let score = if let Some(reference) = input.reference {
            let is_match = match self.mode {
                ComparisonMode::ExactMatch => {
                    self.exact_match(&input.predicted, &reference)
                }
                ComparisonMode::CaseInsensitive => {
                    self.case_insensitive_match(&input.predicted, &reference)
                }
                ComparisonMode::Contains => {
                    self.contains_match(&input.predicted, &reference)
                }
                ComparisonMode::SemanticSimilarity => {
                    let similarity = self.semantic_similarity(&input.predicted, &reference);
                    similarity >= self.similarity_threshold
                }
            };

            if is_match {
                Decimal::ONE
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        Ok(MetricOutput {
            score,
            metadata: json!({
                "metric": "accuracy",
                "comparison_mode": self.mode,
                "threshold": self.similarity_threshold,
            }),
        })
    }
}
