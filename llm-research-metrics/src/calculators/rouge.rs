use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use super::{MetricInput, MetricOutput};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RougeVariant {
    RougeN { n: usize },
    RougeL,
    RougeW { weight: usize },
}

#[derive(Debug, Clone)]
pub struct RougeCalculator {
    pub variant: RougeVariant,
}

impl RougeCalculator {
    pub fn new(variant: RougeVariant) -> Self {
        Self { variant }
    }

    pub fn rouge_1() -> Self {
        Self::new(RougeVariant::RougeN { n: 1 })
    }

    pub fn rouge_2() -> Self {
        Self::new(RougeVariant::RougeN { n: 2 })
    }

    pub fn rouge_l() -> Self {
        Self::new(RougeVariant::RougeL)
    }

    /// Extract n-grams from text
    fn extract_ngrams(&self, text: &str, n: usize) -> Vec<Vec<String>> {
        let words: Vec<String> = text
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        if words.len() < n {
            if n == 1 && !words.is_empty() {
                return words.iter().map(|w| vec![w.clone()]).collect();
            }
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

    /// Calculate ROUGE-N score
    fn rouge_n(&self, predicted: &str, reference: &str, n: usize) -> (f64, f64, f64) {
        let pred_ngrams = self.extract_ngrams(predicted, n);
        let ref_ngrams = self.extract_ngrams(reference, n);

        if ref_ngrams.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let pred_counts = self.count_ngrams(&pred_ngrams);
        let ref_counts = self.count_ngrams(&ref_ngrams);

        let mut overlap = 0;
        for (ngram, ref_count) in ref_counts.iter() {
            if let Some(pred_count) = pred_counts.get(ngram) {
                overlap += (*pred_count).min(*ref_count);
            }
        }

        let precision = if pred_ngrams.is_empty() {
            0.0
        } else {
            overlap as f64 / pred_ngrams.len() as f64
        };

        let recall = overlap as f64 / ref_ngrams.len() as f64;

        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        (precision, recall, f1)
    }

    /// Calculate longest common subsequence length
    fn lcs_length(&self, text1: &[String], text2: &[String]) -> usize {
        let m = text1.len();
        let n = text2.len();

        if m == 0 || n == 0 {
            return 0;
        }

        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if text1[i - 1] == text2[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        dp[m][n]
    }

    /// Calculate ROUGE-L score (based on longest common subsequence)
    fn calculate_rouge_l(&self, predicted: &str, reference: &str) -> (f64, f64, f64) {
        let pred_words: Vec<String> = predicted
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        let ref_words: Vec<String> = reference
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        if ref_words.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let lcs_len = self.lcs_length(&pred_words, &ref_words);

        let precision = if pred_words.is_empty() {
            0.0
        } else {
            lcs_len as f64 / pred_words.len() as f64
        };

        let recall = lcs_len as f64 / ref_words.len() as f64;

        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        (precision, recall, f1)
    }

    /// Calculate weighted LCS with position weighting
    fn rouge_w(&self, predicted: &str, reference: &str, weight: usize) -> (f64, f64, f64) {
        // Simplified ROUGE-W using standard LCS with position awareness
        // In practice, this would use weighted LCS algorithm
        let (precision, recall, f1) = self.calculate_rouge_l(predicted, reference);

        // Apply a simple weight factor based on consecutive matches
        let weight_factor = 1.0 + (weight as f64 * 0.1);

        (
            precision * weight_factor.min(1.0),
            recall * weight_factor.min(1.0),
            f1 * weight_factor.min(1.0),
        )
    }
}

impl Default for RougeCalculator {
    fn default() -> Self {
        Self::rouge_l()
    }
}

#[async_trait]
impl MetricCalculator for RougeCalculator {
    type Input = MetricInput;
    type Output = MetricOutput;

    async fn calculate(&self, input: Self::Input) -> Result<Self::Output> {
        let (precision, recall, f1) = if let Some(reference) = input.reference {
            match self.variant {
                RougeVariant::RougeN { n } => {
                    self.rouge_n(&input.predicted, &reference, n)
                }
                RougeVariant::RougeL => {
                    self.calculate_rouge_l(&input.predicted, &reference)
                }
                RougeVariant::RougeW { weight } => {
                    self.rouge_w(&input.predicted, &reference, weight)
                }
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        let score = Decimal::try_from(f1).unwrap_or(Decimal::ZERO);

        Ok(MetricOutput {
            score,
            metadata: json!({
                "metric": "rouge",
                "variant": self.variant,
                "precision": precision,
                "recall": recall,
                "f1": f1,
            }),
        })
    }
}
