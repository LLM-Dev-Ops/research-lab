use async_trait::async_trait;
use llm_research_core::{MetricCalculator, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub std_dev: f64,
    /// Time to first token (ms)
    pub ttft: Option<f64>,
    /// Tokens per second
    pub tokens_per_second: Option<f64>,
    /// Total tokens generated
    pub total_tokens: Option<usize>,
}

impl LatencyMetrics {
    /// Calculate latency metrics from a collection of latency measurements
    pub fn from_measurements(measurements: &[f64]) -> Self {
        if measurements.is_empty() {
            return Self::empty();
        }

        let mut sorted = measurements.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let mean = measurements.iter().sum::<f64>() / measurements.len() as f64;
        let median = percentile(&sorted, 50.0);

        let variance = measurements
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / measurements.len() as f64;
        let std_dev = variance.sqrt();

        Self {
            min,
            max,
            mean,
            median,
            p50: percentile(&sorted, 50.0),
            p90: percentile(&sorted, 90.0),
            p95: percentile(&sorted, 95.0),
            p99: percentile(&sorted, 99.0),
            std_dev,
            ttft: None,
            tokens_per_second: None,
            total_tokens: None,
        }
    }

    pub fn with_ttft(mut self, ttft: f64) -> Self {
        self.ttft = Some(ttft);
        self
    }

    pub fn with_throughput(mut self, tokens: usize, total_time_ms: f64) -> Self {
        self.total_tokens = Some(tokens);
        if total_time_ms > 0.0 {
            self.tokens_per_second = Some((tokens as f64 / total_time_ms) * 1000.0);
        }
        self
    }

    fn empty() -> Self {
        Self {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            median: 0.0,
            p50: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
            std_dev: 0.0,
            ttft: None,
            tokens_per_second: None,
            total_tokens: None,
        }
    }
}

fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let index = (percentile / 100.0 * (sorted_values.len() - 1) as f64).round() as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

pub struct LatencyCalculator;

impl LatencyCalculator {
    pub fn start() -> Instant {
        Instant::now()
    }

    pub fn measure(start: Instant) -> f64 {
        start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn measure_micros(start: Instant) -> f64 {
        start.elapsed().as_micros() as f64
    }
}

pub struct LatencyInput {
    pub measurements: Vec<f64>,
    pub ttft: Option<f64>,
    pub total_tokens: Option<usize>,
}

pub struct LatencyOutput {
    pub metrics: LatencyMetrics,
    pub metadata: serde_json::Value,
}

#[async_trait]
impl MetricCalculator for LatencyCalculator {
    type Input = LatencyInput;
    type Output = LatencyOutput;

    async fn calculate(&self, input: Self::Input) -> Result<Self::Output> {
        let mut metrics = LatencyMetrics::from_measurements(&input.measurements);

        if let Some(ttft) = input.ttft {
            metrics = metrics.with_ttft(ttft);
        }

        if let Some(tokens) = input.total_tokens {
            let total_time = input.measurements.iter().sum::<f64>();
            metrics = metrics.with_throughput(tokens, total_time);
        }

        Ok(LatencyOutput {
            metrics,
            metadata: json!({
                "metric": "latency",
                "unit": "milliseconds",
                "sample_count": input.measurements.len(),
            }),
        })
    }
}
