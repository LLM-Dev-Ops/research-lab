use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub mean: Decimal,
    pub median: Decimal,
    pub std_dev: Decimal,
    pub min: Decimal,
    pub max: Decimal,
    pub p50: Decimal,
    pub p90: Decimal,
    pub p95: Decimal,
    pub p99: Decimal,
    pub count: usize,
    pub sum: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub bins: Vec<HistogramBin>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBin {
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub count: usize,
    pub frequency: f64,
}

pub struct MetricAggregator;

impl MetricAggregator {
    pub fn aggregate(values: &[f64]) -> AggregatedMetrics {
        if values.is_empty() {
            return Self::empty();
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let sum: f64 = values.iter().sum();
        let mean = sum / values.len() as f64;
        let median = Self::percentile(&sorted, 50.0);
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        AggregatedMetrics {
            mean: Decimal::try_from(mean).unwrap_or_default(),
            median: Decimal::try_from(median).unwrap_or_default(),
            std_dev: Decimal::try_from(std_dev).unwrap_or_default(),
            min: Decimal::try_from(sorted[0]).unwrap_or_default(),
            max: Decimal::try_from(sorted[sorted.len() - 1]).unwrap_or_default(),
            p50: Decimal::try_from(Self::percentile(&sorted, 50.0)).unwrap_or_default(),
            p90: Decimal::try_from(Self::percentile(&sorted, 90.0)).unwrap_or_default(),
            p95: Decimal::try_from(Self::percentile(&sorted, 95.0)).unwrap_or_default(),
            p99: Decimal::try_from(Self::percentile(&sorted, 99.0)).unwrap_or_default(),
            count: values.len(),
            sum: Decimal::try_from(sum).unwrap_or_default(),
        }
    }

    /// Calculate weighted average
    pub fn weighted_average(values: &[f64], weights: &[f64]) -> Option<f64> {
        if values.len() != weights.len() || values.is_empty() {
            return None;
        }

        let weighted_sum: f64 = values
            .iter()
            .zip(weights.iter())
            .map(|(v, w)| v * w)
            .sum();

        let weight_sum: f64 = weights.iter().sum();

        if weight_sum == 0.0 {
            return None;
        }

        Some(weighted_sum / weight_sum)
    }

    /// Calculate weighted metrics
    pub fn aggregate_weighted(values: &[f64], weights: &[f64]) -> Option<AggregatedMetrics> {
        if values.len() != weights.len() || values.is_empty() {
            return None;
        }

        let weighted_mean = Self::weighted_average(values, weights)?;

        // For simplicity, use unweighted percentiles
        // A more sophisticated approach would weight the percentile calculation
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Weighted variance
        let weight_sum: f64 = weights.iter().sum();
        let weighted_variance: f64 = values
            .iter()
            .zip(weights.iter())
            .map(|(v, w)| w * (v - weighted_mean).powi(2))
            .sum::<f64>() / weight_sum;

        let std_dev = weighted_variance.sqrt();

        Some(AggregatedMetrics {
            mean: Decimal::try_from(weighted_mean).unwrap_or_default(),
            median: Decimal::try_from(Self::percentile(&sorted, 50.0)).unwrap_or_default(),
            std_dev: Decimal::try_from(std_dev).unwrap_or_default(),
            min: Decimal::try_from(sorted[0]).unwrap_or_default(),
            max: Decimal::try_from(sorted[sorted.len() - 1]).unwrap_or_default(),
            p50: Decimal::try_from(Self::percentile(&sorted, 50.0)).unwrap_or_default(),
            p90: Decimal::try_from(Self::percentile(&sorted, 90.0)).unwrap_or_default(),
            p95: Decimal::try_from(Self::percentile(&sorted, 95.0)).unwrap_or_default(),
            p99: Decimal::try_from(Self::percentile(&sorted, 99.0)).unwrap_or_default(),
            count: values.len(),
            sum: Decimal::try_from(values.iter().sum::<f64>()).unwrap_or_default(),
        })
    }

    /// Generate histogram with specified number of bins
    pub fn histogram(values: &[f64], num_bins: usize) -> Histogram {
        if values.is_empty() || num_bins == 0 {
            return Histogram {
                bins: vec![],
                total_count: 0,
            };
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if min == max {
            return Histogram {
                bins: vec![HistogramBin {
                    lower_bound: min,
                    upper_bound: max,
                    count: values.len(),
                    frequency: 1.0,
                }],
                total_count: values.len(),
            };
        }

        let bin_width = (max - min) / num_bins as f64;
        let mut bins = vec![0usize; num_bins];

        for &value in values {
            let mut bin_index = ((value - min) / bin_width).floor() as usize;
            if bin_index >= num_bins {
                bin_index = num_bins - 1;
            }
            bins[bin_index] += 1;
        }

        let total = values.len();
        let histogram_bins: Vec<HistogramBin> = bins
            .into_iter()
            .enumerate()
            .map(|(i, count)| {
                let lower_bound = min + (i as f64 * bin_width);
                let upper_bound = lower_bound + bin_width;
                let frequency = count as f64 / total as f64;
                HistogramBin {
                    lower_bound,
                    upper_bound,
                    count,
                    frequency,
                }
            })
            .collect();

        Histogram {
            bins: histogram_bins,
            total_count: total,
        }
    }

    /// Generate histogram with custom bin edges
    pub fn histogram_custom(values: &[f64], bin_edges: &[f64]) -> Histogram {
        if values.is_empty() || bin_edges.len() < 2 {
            return Histogram {
                bins: vec![],
                total_count: 0,
            };
        }

        let num_bins = bin_edges.len() - 1;
        let mut bins = vec![0usize; num_bins];

        for &value in values {
            for i in 0..num_bins {
                if value >= bin_edges[i] && value < bin_edges[i + 1] {
                    bins[i] += 1;
                    break;
                } else if i == num_bins - 1 && value == bin_edges[i + 1] {
                    bins[i] += 1;
                    break;
                }
            }
        }

        let total = values.len();
        let histogram_bins: Vec<HistogramBin> = bins
            .into_iter()
            .enumerate()
            .map(|(i, count)| {
                let frequency = count as f64 / total as f64;
                HistogramBin {
                    lower_bound: bin_edges[i],
                    upper_bound: bin_edges[i + 1],
                    count,
                    frequency,
                }
            })
            .collect();

        Histogram {
            bins: histogram_bins,
            total_count: total,
        }
    }

    fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }
        let index = (percentile / 100.0 * (sorted_values.len() - 1) as f64).round() as usize;
        sorted_values[index.min(sorted_values.len() - 1)]
    }

    fn empty() -> AggregatedMetrics {
        AggregatedMetrics {
            mean: Decimal::ZERO,
            median: Decimal::ZERO,
            std_dev: Decimal::ZERO,
            min: Decimal::ZERO,
            max: Decimal::ZERO,
            p50: Decimal::ZERO,
            p90: Decimal::ZERO,
            p95: Decimal::ZERO,
            p99: Decimal::ZERO,
            count: 0,
            sum: Decimal::ZERO,
        }
    }
}
