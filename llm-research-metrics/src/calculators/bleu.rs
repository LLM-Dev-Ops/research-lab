use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use super::{MetricInput, MetricOutput};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmoothingMethod {
    None,
    Add1,
    Add01,
}

#[derive(Debug, Clone)]
pub struct BleuCalculator {
    pub max_n: usize,
    pub smoothing: SmoothingMethod,
}

impl BleuCalculator {
    pub fn new(max_n: usize) -> Self {
        Self {
            max_n,
            smoothing: SmoothingMethod::None,
        }
    }

    pub fn with_smoothing(mut self, smoothing: SmoothingMethod) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// Extract n-grams from text
    fn extract_ngrams(&self, text: &str, n: usize) -> Vec<Vec<String>> {
        let words: Vec<String> = text
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        if words.len() < n {
            return vec![];
        }

        words
            .windows(n)
            .map(|window| window.to_vec())
            .collect()
    }

    /// Count n-grams
    fn count_ngrams(&self, ngrams: &[Vec<String>]) -> HashMap<Vec<String>, usize> {
        let mut counts = HashMap::new();
        for ngram in ngrams {
            *counts.entry(ngram.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Calculate precision for a given n
    fn modified_precision(&self, predicted: &str, reference: &str, n: usize) -> f64 {
        let pred_ngrams = self.extract_ngrams(predicted, n);
        let ref_ngrams = self.extract_ngrams(reference, n);

        if pred_ngrams.is_empty() {
            return 0.0;
        }

        let pred_counts = self.count_ngrams(&pred_ngrams);
        let ref_counts = self.count_ngrams(&ref_ngrams);

        let mut clipped_count = 0;
        let mut total_count = 0;

        for (ngram, pred_count) in pred_counts.iter() {
            let ref_count = ref_counts.get(ngram).unwrap_or(&0);
            clipped_count += (*pred_count).min(*ref_count);
            total_count += pred_count;
        }

        if total_count == 0 {
            return 0.0;
        }

        let precision = clipped_count as f64 / total_count as f64;

        // Apply smoothing
        match self.smoothing {
            SmoothingMethod::None => precision,
            SmoothingMethod::Add1 => {
                (clipped_count as f64 + 1.0) / (total_count as f64 + 1.0)
            }
            SmoothingMethod::Add01 => {
                (clipped_count as f64 + 0.1) / (total_count as f64 + 0.1)
            }
        }
    }

    /// Calculate brevity penalty
    fn brevity_penalty(&self, predicted_len: usize, reference_len: usize) -> f64 {
        if predicted_len > reference_len {
            1.0
        } else if reference_len == 0 {
            1.0
        } else {
            (1.0 - (reference_len as f64 / predicted_len as f64)).exp()
        }
    }

    /// Calculate BLEU score
    pub fn calculate_bleu(&self, predicted: &str, reference: &str) -> (f64, Vec<f64>) {
        let pred_words: Vec<_> = predicted.split_whitespace().collect();
        let ref_words: Vec<_> = reference.split_whitespace().collect();

        if pred_words.is_empty() {
            return (0.0, vec![0.0; self.max_n]);
        }

        let mut precisions = Vec::new();
        let mut log_precision_sum = 0.0;

        for n in 1..=self.max_n {
            let precision = self.modified_precision(predicted, reference, n);
            precisions.push(precision);

            if precision > 0.0 {
                log_precision_sum += precision.ln();
            } else {
                // If any precision is 0, BLEU is 0
                return (0.0, precisions);
            }
        }

        let geometric_mean = (log_precision_sum / self.max_n as f64).exp();
        let bp = self.brevity_penalty(pred_words.len(), ref_words.len());
        let bleu = bp * geometric_mean;

        (bleu, precisions)
    }
}

impl Default for BleuCalculator {
    fn default() -> Self {
        Self::new(4)
    }
}

#[async_trait]
impl MetricCalculator for BleuCalculator {
    type Input = MetricInput;
    type Output = MetricOutput;

    async fn calculate(&self, input: Self::Input) -> Result<Self::Output> {
        let score = if let Some(reference) = input.reference {
            let (bleu, precisions) = self.calculate_bleu(&input.predicted, &reference);
            Decimal::try_from(bleu).unwrap_or(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };

        Ok(MetricOutput {
            score,
            metadata: json!({
                "metric": "bleu",
                "max_n": self.max_n,
                "smoothing": self.smoothing,
            }),
        })
    }
}
