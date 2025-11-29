use llm_research_metrics::aggregators::MetricAggregator;
use rust_decimal::Decimal;
use approx::assert_relative_eq;
use rstest::rstest;

// ===== Basic Aggregation Tests =====

#[test]
fn test_aggregate_basic_stats() {
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.count, 5);
    assert_eq!(aggregated.min, Decimal::try_from(10.0).unwrap());
    assert_eq!(aggregated.max, Decimal::try_from(50.0).unwrap());

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 30.0, epsilon = 0.01);

    let median_f64 = f64::try_from(aggregated.median).unwrap();
    assert_relative_eq!(median_f64, 30.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_mean_calculation() {
    let values = vec![5.0, 10.0, 15.0];
    let aggregated = MetricAggregator::aggregate(&values);

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 10.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_median_odd_count() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let aggregated = MetricAggregator::aggregate(&values);

    let median_f64 = f64::try_from(aggregated.median).unwrap();
    assert_relative_eq!(median_f64, 3.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_median_even_count() {
    let values = vec![1.0, 2.0, 3.0, 4.0];
    let aggregated = MetricAggregator::aggregate(&values);

    let median_f64 = f64::try_from(aggregated.median).unwrap();
    // For even count, median should be around middle
    assert!(median_f64 >= 2.0 && median_f64 <= 3.0);
}

#[test]
fn test_aggregate_std_dev() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0];
    let aggregated = MetricAggregator::aggregate(&values);

    let std_dev_f64 = f64::try_from(aggregated.std_dev).unwrap();
    // Standard deviation should be positive
    assert!(std_dev_f64 > 0.0);
    assert!(std_dev_f64 < 5.0); // Reasonable range for this data
}

#[test]
fn test_aggregate_std_dev_identical_values() {
    let values = vec![5.0, 5.0, 5.0, 5.0, 5.0];
    let aggregated = MetricAggregator::aggregate(&values);

    let std_dev_f64 = f64::try_from(aggregated.std_dev).unwrap();
    assert_relative_eq!(std_dev_f64, 0.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_min_max() {
    let values = vec![15.0, 3.0, 42.0, 7.0, 28.0];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.min, Decimal::try_from(3.0).unwrap());
    assert_eq!(aggregated.max, Decimal::try_from(42.0).unwrap());
}

// ===== Percentile Tests =====

#[test]
fn test_aggregate_percentiles() {
    let values: Vec<f64> = (1..=100).map(|x| x as f64).collect();
    let aggregated = MetricAggregator::aggregate(&values);

    let p50_f64 = f64::try_from(aggregated.p50).unwrap();
    let p90_f64 = f64::try_from(aggregated.p90).unwrap();
    let p95_f64 = f64::try_from(aggregated.p95).unwrap();
    let p99_f64 = f64::try_from(aggregated.p99).unwrap();

    assert!(p50_f64 >= 45.0 && p50_f64 <= 55.0);
    assert!(p90_f64 >= 85.0 && p90_f64 <= 95.0);
    assert!(p95_f64 >= 90.0 && p95_f64 <= 100.0);
    assert!(p99_f64 >= 95.0 && p99_f64 <= 100.0);
}

#[test]
fn test_aggregate_p50_equals_median() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.p50, aggregated.median);
}

#[test]
fn test_aggregate_percentile_ordering() {
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
    let aggregated = MetricAggregator::aggregate(&values);

    // Percentiles should be in increasing order
    assert!(aggregated.p50 <= aggregated.p90);
    assert!(aggregated.p90 <= aggregated.p95);
    assert!(aggregated.p95 <= aggregated.p99);
}

// ===== Sum Tests =====

#[test]
fn test_aggregate_sum() {
    let values = vec![10.0, 20.0, 30.0];
    let aggregated = MetricAggregator::aggregate(&values);

    let sum_f64 = f64::try_from(aggregated.sum).unwrap();
    assert_relative_eq!(sum_f64, 60.0, epsilon = 0.01);
}

// ===== Edge Cases =====

#[test]
fn test_aggregate_empty_values() {
    let values: Vec<f64> = vec![];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.count, 0);
    assert_eq!(aggregated.mean, Decimal::ZERO);
    assert_eq!(aggregated.median, Decimal::ZERO);
    assert_eq!(aggregated.std_dev, Decimal::ZERO);
    assert_eq!(aggregated.min, Decimal::ZERO);
    assert_eq!(aggregated.max, Decimal::ZERO);
}

#[test]
fn test_aggregate_single_value() {
    let values = vec![42.0];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.count, 1);

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 42.0, epsilon = 0.01);

    let median_f64 = f64::try_from(aggregated.median).unwrap();
    assert_relative_eq!(median_f64, 42.0, epsilon = 0.01);

    assert_eq!(aggregated.min, Decimal::try_from(42.0).unwrap());
    assert_eq!(aggregated.max, Decimal::try_from(42.0).unwrap());

    let std_dev_f64 = f64::try_from(aggregated.std_dev).unwrap();
    assert_relative_eq!(std_dev_f64, 0.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_two_values() {
    let values = vec![10.0, 20.0];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.count, 2);

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 15.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_negative_values() {
    let values = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.min, Decimal::try_from(-10.0).unwrap());
    assert_eq!(aggregated.max, Decimal::try_from(10.0).unwrap());

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 0.0, epsilon = 0.01);
}

#[test]
fn test_aggregate_very_small_values() {
    let values = vec![0.001, 0.002, 0.003, 0.004, 0.005];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.count, 5);
    assert!(aggregated.mean > Decimal::ZERO);
    assert!(aggregated.std_dev >= Decimal::ZERO);
}

#[test]
fn test_aggregate_very_large_values() {
    let values = vec![1e6, 2e6, 3e6, 4e6, 5e6];
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.count, 5);
    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 3e6, epsilon = 1e3);
}

#[test]
fn test_aggregate_decimal_values() {
    let values = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let aggregated = MetricAggregator::aggregate(&values);

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 0.3, epsilon = 0.01);
}

// ===== Weighted Average Tests =====

#[test]
fn test_weighted_average_equal_weights() {
    let values = vec![10.0, 20.0, 30.0];
    let weights = vec![1.0, 1.0, 1.0];

    let avg = MetricAggregator::weighted_average(&values, &weights).unwrap();
    assert_relative_eq!(avg, 20.0, epsilon = 0.01);
}

#[test]
fn test_weighted_average_different_weights() {
    let values = vec![10.0, 20.0, 30.0];
    let weights = vec![1.0, 2.0, 1.0];

    let avg = MetricAggregator::weighted_average(&values, &weights).unwrap();
    // (10*1 + 20*2 + 30*1) / (1+2+1) = 80/4 = 20
    assert_relative_eq!(avg, 20.0, epsilon = 0.01);
}

#[test]
fn test_weighted_average_heavy_weight() {
    let values = vec![10.0, 100.0];
    let weights = vec![9.0, 1.0];

    let avg = MetricAggregator::weighted_average(&values, &weights).unwrap();
    // (10*9 + 100*1) / (9+1) = 190/10 = 19
    assert_relative_eq!(avg, 19.0, epsilon = 0.01);
}

#[test]
fn test_weighted_average_mismatched_lengths() {
    let values = vec![10.0, 20.0, 30.0];
    let weights = vec![1.0, 1.0];

    let result = MetricAggregator::weighted_average(&values, &weights);
    assert!(result.is_none());
}

#[test]
fn test_weighted_average_empty() {
    let values: Vec<f64> = vec![];
    let weights: Vec<f64> = vec![];

    let result = MetricAggregator::weighted_average(&values, &weights);
    assert!(result.is_none());
}

#[test]
fn test_weighted_average_zero_weights() {
    let values = vec![10.0, 20.0, 30.0];
    let weights = vec![0.0, 0.0, 0.0];

    let result = MetricAggregator::weighted_average(&values, &weights);
    assert!(result.is_none());
}

// ===== Aggregate Weighted Tests =====

#[test]
fn test_aggregate_weighted() {
    let values = vec![10.0, 20.0, 30.0];
    let weights = vec![1.0, 2.0, 1.0];

    let aggregated = MetricAggregator::aggregate_weighted(&values, &weights).unwrap();

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert_relative_eq!(mean_f64, 20.0, epsilon = 0.01);

    assert_eq!(aggregated.count, 3);
}

#[test]
fn test_aggregate_weighted_variance() {
    let values = vec![10.0, 20.0, 30.0];
    let weights = vec![1.0, 1.0, 1.0];

    let aggregated = MetricAggregator::aggregate_weighted(&values, &weights).unwrap();

    let std_dev_f64 = f64::try_from(aggregated.std_dev).unwrap();
    assert!(std_dev_f64 > 0.0);
}

#[test]
fn test_aggregate_weighted_invalid() {
    let values = vec![10.0, 20.0];
    let weights = vec![1.0, 2.0, 3.0];

    let result = MetricAggregator::aggregate_weighted(&values, &weights);
    assert!(result.is_none());
}

// ===== Histogram Tests =====

#[test]
fn test_histogram_basic() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let histogram = MetricAggregator::histogram(&values, 5);

    assert_eq!(histogram.bins.len(), 5);
    assert_eq!(histogram.total_count, 10);

    // Each bin should have 2 values
    for bin in &histogram.bins {
        assert_eq!(bin.count, 2);
        assert_relative_eq!(bin.frequency, 0.2, epsilon = 0.01);
    }
}

#[test]
fn test_histogram_bin_bounds() {
    let values = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0];
    let histogram = MetricAggregator::histogram(&values, 5);

    // Check that bins cover the full range
    assert_relative_eq!(histogram.bins[0].lower_bound, 0.0, epsilon = 0.01);
    assert_relative_eq!(histogram.bins[4].upper_bound, 50.0, epsilon = 0.01);
}

#[test]
fn test_histogram_frequency_sum() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let histogram = MetricAggregator::histogram(&values, 3);

    let total_frequency: f64 = histogram.bins.iter().map(|b| b.frequency).sum();
    assert_relative_eq!(total_frequency, 1.0, epsilon = 0.01);
}

#[test]
fn test_histogram_empty_values() {
    let values: Vec<f64> = vec![];
    let histogram = MetricAggregator::histogram(&values, 5);

    assert_eq!(histogram.bins.len(), 0);
    assert_eq!(histogram.total_count, 0);
}

#[test]
fn test_histogram_single_value() {
    let values = vec![42.0];
    let histogram = MetricAggregator::histogram(&values, 5);

    assert_eq!(histogram.bins.len(), 1);
    assert_eq!(histogram.bins[0].count, 1);
    assert_relative_eq!(histogram.bins[0].frequency, 1.0, epsilon = 0.01);
}

#[test]
fn test_histogram_identical_values() {
    let values = vec![5.0, 5.0, 5.0, 5.0, 5.0];
    let histogram = MetricAggregator::histogram(&values, 3);

    // All values are the same, so they should all be in one bin
    assert_eq!(histogram.bins.len(), 1);
    assert_eq!(histogram.bins[0].count, 5);
}

#[test]
fn test_histogram_zero_bins() {
    let values = vec![1.0, 2.0, 3.0];
    let histogram = MetricAggregator::histogram(&values, 0);

    assert_eq!(histogram.bins.len(), 0);
    assert_eq!(histogram.total_count, 0);
}

// ===== Histogram Custom Bins Tests =====

#[test]
fn test_histogram_custom_bins() {
    let values = vec![1.0, 5.0, 15.0, 25.0, 35.0, 45.0];
    let bin_edges = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0];

    let histogram = MetricAggregator::histogram_custom(&values, &bin_edges);

    assert_eq!(histogram.bins.len(), 5);
    assert_eq!(histogram.total_count, 6);
}

#[test]
fn test_histogram_custom_bins_counts() {
    let values = vec![1.0, 5.0, 15.0, 25.0];
    let bin_edges = vec![0.0, 10.0, 20.0, 30.0];

    let histogram = MetricAggregator::histogram_custom(&values, &bin_edges);

    assert_eq!(histogram.bins[0].count, 2); // 1.0, 5.0
    assert_eq!(histogram.bins[1].count, 1); // 15.0
    assert_eq!(histogram.bins[2].count, 1); // 25.0
}

#[test]
fn test_histogram_custom_bins_bounds() {
    let values = vec![5.0, 15.0, 25.0];
    let bin_edges = vec![0.0, 10.0, 20.0, 30.0];

    let histogram = MetricAggregator::histogram_custom(&values, &bin_edges);

    assert_eq!(histogram.bins[0].lower_bound, 0.0);
    assert_eq!(histogram.bins[0].upper_bound, 10.0);
    assert_eq!(histogram.bins[1].lower_bound, 10.0);
    assert_eq!(histogram.bins[1].upper_bound, 20.0);
}

#[test]
fn test_histogram_custom_bins_edge_values() {
    let values = vec![0.0, 10.0, 20.0, 30.0];
    let bin_edges = vec![0.0, 10.0, 20.0, 30.0];

    let histogram = MetricAggregator::histogram_custom(&values, &bin_edges);

    // Each bin should have at least one value
    let total_count: usize = histogram.bins.iter().map(|b| b.count).sum();
    assert_eq!(total_count, 4);
}

#[test]
fn test_histogram_custom_bins_invalid() {
    let values = vec![1.0, 2.0, 3.0];
    let bin_edges = vec![0.0]; // Need at least 2 edges

    let histogram = MetricAggregator::histogram_custom(&values, &bin_edges);

    assert_eq!(histogram.bins.len(), 0);
    assert_eq!(histogram.total_count, 0);
}

// ===== Parameterized Tests =====

#[rstest]
#[case(vec![1.0, 2.0, 3.0], 2.0, 2.0)]
#[case(vec![10.0, 20.0, 30.0, 40.0, 50.0], 30.0, 30.0)]
#[case(vec![100.0], 100.0, 100.0)]
#[case(vec![5.5, 6.5, 7.5], 6.5, 6.5)]
fn test_aggregate_mean_median(
    #[case] values: Vec<f64>,
    #[case] expected_mean: f64,
    #[case] expected_median: f64,
) {
    let aggregated = MetricAggregator::aggregate(&values);

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    let median_f64 = f64::try_from(aggregated.median).unwrap();

    assert_relative_eq!(mean_f64, expected_mean, epsilon = 0.01);
    assert_relative_eq!(median_f64, expected_median, epsilon = 0.01);
}

#[rstest]
#[case(vec![1.0, 2.0, 3.0, 4.0, 5.0], 1.0, 5.0)]
#[case(vec![-10.0, 0.0, 10.0], -10.0, 10.0)]
#[case(vec![42.0], 42.0, 42.0)]
fn test_aggregate_min_max_cases(
    #[case] values: Vec<f64>,
    #[case] expected_min: f64,
    #[case] expected_max: f64,
) {
    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.min, Decimal::try_from(expected_min).unwrap());
    assert_eq!(aggregated.max, Decimal::try_from(expected_max).unwrap());
}

// ===== Integration Tests =====

#[test]
fn test_aggregate_realistic_metrics() {
    // Simulating accuracy scores from a batch of predictions
    let accuracy_scores = vec![
        0.85, 0.92, 0.88, 0.91, 0.87, 0.89, 0.90, 0.86, 0.93, 0.84,
    ];

    let aggregated = MetricAggregator::aggregate(&accuracy_scores);

    assert_eq!(aggregated.count, 10);

    let mean_f64 = f64::try_from(aggregated.mean).unwrap();
    assert!(mean_f64 > 0.8 && mean_f64 < 1.0);

    let std_dev_f64 = f64::try_from(aggregated.std_dev).unwrap();
    assert!(std_dev_f64 > 0.0 && std_dev_f64 < 0.1);
}

#[test]
fn test_aggregate_latency_metrics() {
    // Simulating latency measurements in milliseconds
    let latencies = vec![
        12.5, 15.3, 11.8, 14.2, 13.7, 16.1, 12.9, 14.8, 13.4, 15.6,
    ];

    let aggregated = MetricAggregator::aggregate(&latencies);

    let p95_f64 = f64::try_from(aggregated.p95).unwrap();
    let p99_f64 = f64::try_from(aggregated.p99).unwrap();

    // p95 and p99 should be among the higher values
    assert!(p95_f64 >= 14.0);
    assert!(p99_f64 >= p95_f64);
}

#[test]
fn test_aggregate_with_outliers() {
    let values = vec![10.0, 11.0, 12.0, 13.0, 14.0, 100.0]; // 100.0 is an outlier

    let aggregated = MetricAggregator::aggregate(&values);

    assert_eq!(aggregated.max, Decimal::try_from(100.0).unwrap());

    let median_f64 = f64::try_from(aggregated.median).unwrap();
    // Median should be less affected by outlier
    assert!(median_f64 < 20.0);
}
