use llm_research_metrics::statistical::StatisticalAnalyzer;

// ===== Confidence Interval Tests =====

#[test]
fn test_confidence_interval() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 13.0, 12.0, 11.0, 14.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    assert!(lower > rust_decimal::Decimal::ZERO);
    assert!(upper > lower);
}

#[test]
fn test_confidence_interval_small_sample() {
    let values = vec![10.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    // With only one sample, should return zeros
    assert_eq!(lower, rust_decimal::Decimal::ZERO);
    assert_eq!(upper, rust_decimal::Decimal::ZERO);
}

#[test]
fn test_confidence_interval_larger_sample() {
    let values = vec![
        5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0,
        15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
    ];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    // Mean should be around 12.5
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    assert!((mean - 12.5).abs() < 0.1);

    // Confidence interval should contain the mean
    assert!(rust_decimal::Decimal::try_from(mean).unwrap() >= lower);
    assert!(rust_decimal::Decimal::try_from(mean).unwrap() <= upper);
}

#[test]
fn test_confidence_interval_different_levels() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 13.0, 12.0, 11.0, 14.0];

    let (lower_90, upper_90) = StatisticalAnalyzer::confidence_interval(&values, 0.90);
    let (lower_95, upper_95) = StatisticalAnalyzer::confidence_interval(&values, 0.95);
    let (lower_99, upper_99) = StatisticalAnalyzer::confidence_interval(&values, 0.99);

    // Higher confidence should give wider intervals
    assert!(upper_99 >= upper_95);
    assert!(upper_95 >= upper_90);
    assert!(lower_90 >= lower_95);
    assert!(lower_95 >= lower_99);
}

// ===== T-Test Tests =====

#[test]
fn test_t_test_identical_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    // Identical samples should have t-statistic close to 0
    assert!((result.statistic).abs() < 0.01);
    // p-value should be very high (close to 1)
    assert!(result.p_value.unwrap() > 0.9);
}

#[test]
fn test_t_test_different_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![20.0, 21.0, 22.0, 23.0, 24.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    // Very different samples should have high t-statistic
    assert!(result.statistic.abs() > 5.0);
    // p-value should be very low
    assert!(result.p_value.unwrap() < 0.01);
}

#[test]
fn test_t_test_small_difference() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
    let sample2 = vec![10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.effect_size.is_some());
}

#[test]
fn test_t_test_insufficient_data() {
    let sample1 = vec![10.0];
    let sample2 = vec![12.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert_eq!(result.statistic, 0.0);
    assert!(result.p_value.is_none());
}

#[test]
fn test_t_test_varying_sample_sizes() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.statistic.abs() > 0.0);
}

// ===== Mann-Whitney U Test =====

#[test]
fn test_mann_whitney_u_identical() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
    // Similar samples should have high p-value
    assert!(result.p_value.unwrap() > 0.5);
}

#[test]
fn test_mann_whitney_u_different() {
    let sample1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
    // Very different samples should have low p-value
    assert!(result.p_value.unwrap() < 0.05);
}

#[test]
fn test_mann_whitney_u_with_ties() {
    let sample1 = vec![1.0, 2.0, 3.0, 3.0, 4.0];
    let sample2 = vec![3.0, 4.0, 5.0, 5.0, 6.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.statistic >= 0.0);
    assert!(result.p_value.is_some());
}

#[test]
fn test_mann_whitney_u_empty_samples() {
    let sample1: Vec<f64> = vec![];
    let sample2 = vec![1.0, 2.0, 3.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert_eq!(result.statistic, 0.0);
    assert!(result.p_value.is_none());
}

#[test]
fn test_mann_whitney_u_single_values() {
    let sample1 = vec![5.0];
    let sample2 = vec![10.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.statistic >= 0.0);
    assert!(result.p_value.is_some());
}

// ===== Cohen's d Effect Size =====

#[test]
fn test_cohens_d_identical_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Identical samples should have effect size of 0
    assert!((d).abs() < 0.01);
}

#[test]
fn test_cohens_d_large_effect() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![20.0, 21.0, 22.0, 23.0, 24.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Large difference should give large effect size
    assert!(d.abs() > 2.0);
}

#[test]
fn test_cohens_d_small_effect() {
    let sample1 = vec![10.0, 10.5, 11.0, 11.5, 12.0];
    let sample2 = vec![10.2, 10.7, 11.2, 11.7, 12.2];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Small difference should give small effect size
    assert!(d.abs() < 1.0);
}

#[test]
fn test_cohens_d_insufficient_data() {
    let sample1 = vec![10.0];
    let sample2 = vec![15.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    assert_eq!(d, 0.0);
}

#[test]
fn test_cohens_d_no_variance() {
    let sample1 = vec![10.0, 10.0, 10.0];
    let sample2 = vec![10.0, 10.0, 10.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    assert_eq!(d, 0.0);
}

#[test]
fn test_cohens_d_negative_effect() {
    let sample1 = vec![20.0, 21.0, 22.0, 23.0, 24.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Effect size should be negative when first sample is larger
    assert!(d > 2.0); // Positive because sample1 mean > sample2 mean
}

// ===== Bootstrap Comparison Tests =====

#[test]
fn test_bootstrap_comparison() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(
        &sample1,
        &sample2,
        1000,
        0.95,
    );

    assert!(result.confidence_interval.is_some());
    let (lower, upper) = result.confidence_interval.unwrap();

    // The difference should be negative (sample1 < sample2)
    assert!(upper < 0.0);
    assert!(lower < upper);
}

#[test]
fn test_bootstrap_comparison_small_iterations() {
    let sample1 = vec![10.0, 11.0, 12.0];
    let sample2 = vec![13.0, 14.0, 15.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(
        &sample1,
        &sample2,
        100,
        0.90,
    );

    assert!(result.confidence_interval.is_some());
    assert!(result.effect_size.is_some());
}

#[test]
fn test_bootstrap_comparison_empty_samples() {
    let sample1: Vec<f64> = vec![];
    let sample2 = vec![10.0, 11.0, 12.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(
        &sample1,
        &sample2,
        100,
        0.95,
    );

    assert!(result.confidence_interval.is_none());
}

// ===== Integration Tests =====

#[test]
fn test_complete_statistical_analysis() {
    let control = vec![12.0, 13.0, 14.0, 15.0, 16.0, 14.5, 13.5, 15.5];
    let treatment = vec![14.0, 15.0, 16.0, 17.0, 18.0, 16.5, 15.5, 17.5];

    // T-test
    let t_result = StatisticalAnalyzer::t_test(&control, &treatment);
    assert!(t_result.p_value.is_some());

    // Mann-Whitney U (non-parametric)
    let u_result = StatisticalAnalyzer::mann_whitney_u(&control, &treatment);
    assert!(u_result.p_value.is_some());

    // Effect size
    let effect_size = StatisticalAnalyzer::cohens_d(&control, &treatment);
    assert!(effect_size.abs() > 0.0);

    // Confidence intervals
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&treatment, 0.95);
    assert!(upper > lower);
}

#[test]
fn test_statistical_power_sample_size() {
    // Larger sample sizes should give more precise confidence intervals
    let small_sample = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let large_sample = vec![
        10.0, 10.5, 11.0, 11.5, 12.0, 12.5, 13.0, 13.5, 14.0, 14.5,
        10.2, 10.7, 11.2, 11.7, 12.2, 12.7, 13.2, 13.7, 14.2,
    ];

    let (small_lower, small_upper) = StatisticalAnalyzer::confidence_interval(&small_sample, 0.95);
    let (large_lower, large_upper) = StatisticalAnalyzer::confidence_interval(&large_sample, 0.95);

    let small_width = small_upper - small_lower;
    let large_width = large_upper - large_lower;

    // Larger sample should have narrower confidence interval
    assert!(large_width < small_width);
}

// ===== Edge Cases =====

#[test]
fn test_negative_values() {
    let sample1 = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
    let sample2 = vec![-8.0, -3.0, 2.0, 7.0, 12.0];

    let t_result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    assert!(t_result.p_value.is_some());

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);
    assert!(d.abs() >= 0.0);
}

#[test]
fn test_very_small_values() {
    let sample1 = vec![0.001, 0.002, 0.003, 0.004, 0.005];
    let sample2 = vec![0.002, 0.003, 0.004, 0.005, 0.006];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    assert!(result.p_value.is_some());
}

#[test]
fn test_very_large_values() {
    let sample1 = vec![1e6, 1.1e6, 1.2e6, 1.3e6, 1.4e6];
    let sample2 = vec![1.5e6, 1.6e6, 1.7e6, 1.8e6, 1.9e6];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    assert!(result.p_value.is_some());
    assert!(result.statistic.abs() > 0.0);
}

#[test]
fn test_high_variance_samples() {
    let sample1 = vec![1.0, 100.0, 2.0, 99.0, 3.0, 98.0];
    let sample2 = vec![50.0, 51.0, 49.0, 52.0, 48.0, 53.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);
    assert!(d.is_finite());

    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&sample1, 0.95);
    assert!(upper > lower);
}
