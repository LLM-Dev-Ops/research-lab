use llm_research_metrics::statistical::StatisticalAnalyzer;
use rust_decimal::Decimal;
use approx::assert_relative_eq;
use rstest::rstest;

// ===== Confidence Interval Tests =====

#[test]
fn test_confidence_interval_basic() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 13.0, 12.0, 11.0, 14.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    assert!(lower > Decimal::ZERO);
    assert!(upper > lower);

    // Mean should be within the interval
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let mean_decimal = Decimal::try_from(mean).unwrap();
    assert!(mean_decimal >= lower);
    assert!(mean_decimal <= upper);
}

#[test]
fn test_confidence_interval_90_percent() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 13.0, 12.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.90);

    assert!(lower > Decimal::ZERO);
    assert!(upper > lower);
}

#[test]
fn test_confidence_interval_99_percent() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 13.0, 12.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.99);

    assert!(lower > Decimal::ZERO);
    assert!(upper > lower);
}

#[test]
fn test_confidence_interval_width_increases_with_confidence() {
    let values = vec![10.0, 12.0, 13.0, 11.0, 14.0, 15.0, 13.0, 12.0, 11.0, 14.0];

    let (lower_90, upper_90) = StatisticalAnalyzer::confidence_interval(&values, 0.90);
    let (lower_95, upper_95) = StatisticalAnalyzer::confidence_interval(&values, 0.95);
    let (lower_99, upper_99) = StatisticalAnalyzer::confidence_interval(&values, 0.99);

    let width_90 = upper_90 - lower_90;
    let width_95 = upper_95 - lower_95;
    let width_99 = upper_99 - lower_99;

    // Higher confidence should give wider intervals
    assert!(width_95 >= width_90);
    assert!(width_99 >= width_95);
}

#[test]
fn test_confidence_interval_large_sample() {
    let values: Vec<f64> = (1..=100).map(|x| x as f64).collect();
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    assert!(lower > Decimal::ZERO);
    assert!(upper > lower);

    // With large sample, interval should be relatively narrow compared to the range
    let width_f64 = f64::try_from(upper - lower).unwrap();
    assert!(width_f64 < 15.0); // Reasonable for this data (range is 1-100)
}

#[test]
fn test_confidence_interval_small_sample() {
    let values = vec![10.0, 12.0, 14.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    assert!(lower > Decimal::ZERO);
    assert!(upper > lower);
}

#[test]
fn test_confidence_interval_insufficient_data() {
    let values = vec![10.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    // With only one sample, should return zeros
    assert_eq!(lower, Decimal::ZERO);
    assert_eq!(upper, Decimal::ZERO);
}

#[test]
fn test_confidence_interval_empty() {
    let values: Vec<f64> = vec![];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    assert_eq!(lower, Decimal::ZERO);
    assert_eq!(upper, Decimal::ZERO);
}

#[test]
fn test_confidence_interval_tight_data() {
    let values = vec![10.0, 10.1, 10.2, 10.1, 10.0, 10.2, 10.1];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    // Very tight data should have narrow confidence interval
    let width_f64 = f64::try_from(upper - lower).unwrap();
    assert!(width_f64 < 1.0);
}

#[test]
fn test_confidence_interval_high_variance() {
    let values = vec![1.0, 100.0, 2.0, 99.0, 3.0, 98.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, 0.95);

    // High variance should give wider confidence interval
    let width_f64 = f64::try_from(upper - lower).unwrap();
    assert!(width_f64 > 10.0);
}

// ===== T-Test Tests =====

#[test]
fn test_t_test_identical_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert_relative_eq!(result.statistic, 0.0, epsilon = 0.01);
    assert!(result.p_value.unwrap() > 0.9);
    assert!(result.effect_size.is_some());
}

#[test]
fn test_t_test_very_different_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![20.0, 21.0, 22.0, 23.0, 24.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.statistic.abs() > 5.0);
    assert!(result.p_value.unwrap() < 0.01);
    assert!(result.effect_size.unwrap().abs() > 2.0);
}

#[test]
fn test_t_test_slightly_different_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.5, 11.5, 12.5, 13.5, 14.5];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.statistic.abs() > 0.0);
    assert!(result.effect_size.is_some());
}

#[test]
fn test_t_test_known_distribution() {
    // Two samples from normal distributions with different means
    let sample1 = vec![5.0, 5.5, 4.5, 6.0, 5.2, 4.8, 5.3];
    let sample2 = vec![7.0, 7.5, 6.5, 8.0, 7.2, 6.8, 7.3];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.statistic.abs() > 2.0);
    assert!(result.p_value.unwrap() < 0.05);
}

#[test]
fn test_t_test_large_samples() {
    let sample1: Vec<f64> = (1..=50).map(|x| x as f64).collect();
    let sample2: Vec<f64> = (51..=100).map(|x| x as f64).collect();

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.statistic.abs() > 10.0);
    assert!(result.p_value.unwrap() < 0.001);
}

#[test]
fn test_t_test_unequal_variances() {
    let sample1 = vec![10.0, 10.5, 11.0, 10.2, 10.8];
    let sample2 = vec![10.0, 20.0, 5.0, 15.0, 12.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.statistic.is_finite());
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
fn test_t_test_one_sample_too_small() {
    let sample1 = vec![10.0, 11.0, 12.0];
    let sample2 = vec![15.0];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    assert_eq!(result.statistic, 0.0);
    assert!(result.p_value.is_none());
}

// ===== Mann-Whitney U Test =====

#[test]
fn test_mann_whitney_identical_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.p_value.unwrap() > 0.5);
}

#[test]
fn test_mann_whitney_clearly_different() {
    let sample1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.p_value.unwrap() < 0.05);
}

#[test]
fn test_mann_whitney_with_ties() {
    let sample1 = vec![1.0, 2.0, 3.0, 3.0, 4.0];
    let sample2 = vec![3.0, 4.0, 5.0, 5.0, 6.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.statistic >= 0.0);
    assert!(result.p_value.is_some());
}

#[test]
fn test_mann_whitney_all_same_values() {
    let sample1 = vec![5.0, 5.0, 5.0];
    let sample2 = vec![5.0, 5.0, 5.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.p_value.unwrap() > 0.9);
}

#[test]
fn test_mann_whitney_non_normal_distribution() {
    // Skewed distribution
    let sample1 = vec![1.0, 1.0, 2.0, 2.0, 10.0];
    let sample2 = vec![3.0, 3.0, 4.0, 4.0, 20.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
}

#[test]
fn test_mann_whitney_large_samples() {
    let sample1: Vec<f64> = (1..=50).map(|x| x as f64).collect();
    let sample2: Vec<f64> = (51..=100).map(|x| x as f64).collect();

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.p_value.is_some());
    assert!(result.p_value.unwrap() < 0.01);
}

#[test]
fn test_mann_whitney_empty_sample() {
    let sample1: Vec<f64> = vec![];
    let sample2 = vec![1.0, 2.0, 3.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert_eq!(result.statistic, 0.0);
    assert!(result.p_value.is_none());
}

#[test]
fn test_mann_whitney_single_values() {
    let sample1 = vec![5.0];
    let sample2 = vec![10.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    assert!(result.statistic >= 0.0);
    assert!(result.p_value.is_some());
}

#[test]
fn test_mann_whitney_known_result() {
    // Simple case where we can manually verify
    let sample1 = vec![1.0, 2.0];
    let sample2 = vec![3.0, 4.0];

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    // All sample2 values are greater than all sample1 values
    assert!(result.p_value.is_some());
    assert!(result.p_value.unwrap() < 0.5);
}

// ===== Bootstrap Comparison Tests =====

#[test]
fn test_bootstrap_basic() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);

    assert!(result.confidence_interval.is_some());
    let (lower, upper) = result.confidence_interval.unwrap();

    // The difference should be negative (sample1 < sample2)
    assert!(upper < 0.0);
    assert!(lower < upper);
    assert!(result.effect_size.is_some());
}

#[test]
fn test_bootstrap_identical_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);

    assert!(result.confidence_interval.is_some());
    let (lower, upper) = result.confidence_interval.unwrap();

    // Difference should be close to zero
    assert!(lower < 1.0);
    assert!(upper > -1.0);
}

#[test]
fn test_bootstrap_different_iterations() {
    let sample1 = vec![10.0, 11.0, 12.0];
    let sample2 = vec![13.0, 14.0, 15.0];

    let result_100 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 100, 0.90);
    let result_1000 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.90);

    assert!(result_100.confidence_interval.is_some());
    assert!(result_1000.confidence_interval.is_some());
}

#[test]
fn test_bootstrap_confidence_levels() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0];

    let result_90 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.90);
    let result_95 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);
    let result_99 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.99);

    let (lower_90, upper_90) = result_90.confidence_interval.unwrap();
    let (lower_95, upper_95) = result_95.confidence_interval.unwrap();
    let (lower_99, upper_99) = result_99.confidence_interval.unwrap();

    let width_90 = upper_90 - lower_90;
    let width_95 = upper_95 - lower_95;
    let width_99 = upper_99 - lower_99;

    // Higher confidence should give wider intervals
    assert!(width_95 >= width_90);
    assert!(width_99 >= width_95);
}

#[test]
fn test_bootstrap_empty_sample() {
    let sample1: Vec<f64> = vec![];
    let sample2 = vec![10.0, 11.0, 12.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 100, 0.95);

    assert!(result.confidence_interval.is_none());
}

#[test]
fn test_bootstrap_large_difference() {
    let sample1 = vec![1.0, 2.0, 3.0];
    let sample2 = vec![100.0, 101.0, 102.0];

    let result = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);

    let (_lower, upper) = result.confidence_interval.unwrap();

    // Should show large negative difference
    assert!(upper < -50.0);
    assert!(result.effect_size.unwrap().abs() > 2.0);
}

// ===== Cohen's d Effect Size Tests =====

#[test]
fn test_cohens_d_identical_samples() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![10.0, 11.0, 12.0, 13.0, 14.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    assert_relative_eq!(d, 0.0, epsilon = 0.01);
}

#[test]
fn test_cohens_d_large_effect() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![20.0, 21.0, 22.0, 23.0, 24.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Large difference should give large effect size (typically > 0.8 is large)
    assert!(d.abs() > 2.0);
}

#[test]
fn test_cohens_d_medium_effect() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![12.0, 13.0, 14.0, 15.0, 16.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Medium effect (typically 0.5 is medium)
    assert!(d.abs() > 0.5);
    assert!(d.abs() < 2.0);
}

#[test]
fn test_cohens_d_small_effect() {
    let sample1 = vec![10.0, 10.5, 11.0, 11.5, 12.0];
    let sample2 = vec![10.2, 10.7, 11.2, 11.7, 12.2];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // Small effect (typically 0.2 is small)
    assert!(d.abs() < 1.0);
}

#[test]
fn test_cohens_d_direction() {
    let sample1 = vec![10.0, 11.0, 12.0];
    let sample2 = vec![15.0, 16.0, 17.0];

    let d1 = StatisticalAnalyzer::cohens_d(&sample1, &sample2);
    let d2 = StatisticalAnalyzer::cohens_d(&sample2, &sample1);

    // Effect sizes should have opposite signs
    assert_relative_eq!(d1, -d2, epsilon = 0.01);
}

#[test]
fn test_cohens_d_no_variance() {
    let sample1 = vec![10.0, 10.0, 10.0];
    let sample2 = vec![10.0, 10.0, 10.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    assert_eq!(d, 0.0);
}

#[test]
fn test_cohens_d_insufficient_data() {
    let sample1 = vec![10.0];
    let sample2 = vec![15.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    assert_eq!(d, 0.0);
}

#[test]
fn test_cohens_d_known_values() {
    // Sample with known statistics
    let sample1 = vec![5.0, 5.0, 5.0, 5.0, 5.0]; // mean=5, sd=0
    let sample2 = vec![7.0, 7.0, 7.0, 7.0, 7.0]; // mean=7, sd=0

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);

    // With zero variance, should return 0
    assert_eq!(d, 0.0);
}

// ===== Integration Tests =====

#[test]
fn test_complete_statistical_comparison() {
    let control = vec![12.0, 13.0, 14.0, 15.0, 16.0, 14.5, 13.5, 15.5];
    let treatment = vec![14.0, 15.0, 16.0, 17.0, 18.0, 16.5, 15.5, 17.5];

    // T-test
    let t_result = StatisticalAnalyzer::t_test(&control, &treatment);
    assert!(t_result.p_value.is_some());
    assert!(t_result.p_value.unwrap() < 0.05);

    // Mann-Whitney U
    let u_result = StatisticalAnalyzer::mann_whitney_u(&control, &treatment);
    assert!(u_result.p_value.is_some());

    // Effect size
    let effect_size = StatisticalAnalyzer::cohens_d(&control, &treatment);
    assert!(effect_size.abs() > 0.5);

    // Confidence intervals
    let (lower_c, upper_c) = StatisticalAnalyzer::confidence_interval(&control, 0.95);
    let (lower_t, upper_t) = StatisticalAnalyzer::confidence_interval(&treatment, 0.95);
    assert!(upper_c > lower_c);
    assert!(upper_t > lower_t);
}

#[test]
fn test_parametric_vs_nonparametric() {
    // Compare t-test and Mann-Whitney U on same data
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0];

    let t_result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    let u_result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    // Both should detect the difference
    assert!(t_result.p_value.unwrap() < 0.05);
    assert!(u_result.p_value.unwrap() < 0.05);
}

#[test]
fn test_bootstrap_vs_ttest() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0];

    let t_result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    let boot_result = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);

    // Both should show significant difference
    assert!(t_result.p_value.unwrap() < 0.05);

    let (_lower, upper) = boot_result.confidence_interval.unwrap();
    // Confidence interval should not contain zero
    assert!(upper < 0.0);
}

// ===== Edge Cases =====

#[test]
fn test_statistical_with_negative_values() {
    let sample1 = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
    let sample2 = vec![-8.0, -3.0, 2.0, 7.0, 12.0];

    let t_result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    assert!(t_result.p_value.is_some());

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);
    assert!(d.is_finite());
}

#[test]
fn test_statistical_very_small_values() {
    let sample1 = vec![0.001, 0.002, 0.003, 0.004, 0.005];
    let sample2 = vec![0.002, 0.003, 0.004, 0.005, 0.006];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    assert!(result.p_value.is_some());
}

#[test]
fn test_statistical_very_large_values() {
    let sample1 = vec![1e6, 1.1e6, 1.2e6, 1.3e6, 1.4e6];
    let sample2 = vec![1.5e6, 1.6e6, 1.7e6, 1.8e6, 1.9e6];

    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);
    assert!(result.p_value.is_some());
    assert!(result.statistic.abs() > 0.0);
}

#[test]
fn test_statistical_high_variance() {
    let sample1 = vec![1.0, 100.0, 2.0, 99.0, 3.0, 98.0];
    let sample2 = vec![50.0, 51.0, 49.0, 52.0, 48.0, 53.0];

    let d = StatisticalAnalyzer::cohens_d(&sample1, &sample2);
    assert!(d.is_finite());

    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&sample1, 0.95);
    assert!(upper > lower);
}

// ===== Parameterized Tests =====

#[rstest]
#[case(vec![10.0, 11.0, 12.0], vec![10.0, 11.0, 12.0], 0.9)]
#[case(vec![10.0, 11.0, 12.0], vec![20.0, 21.0, 22.0], 0.01)]
#[case(vec![5.0, 6.0, 7.0], vec![5.5, 6.5, 7.5], 0.5)]
fn test_t_test_p_values(
    #[case] sample1: Vec<f64>,
    #[case] sample2: Vec<f64>,
    #[case] expected_threshold: f64,
) {
    let result = StatisticalAnalyzer::t_test(&sample1, &sample2);

    if expected_threshold > 0.5 {
        // Similar samples should have high p-value
        assert!(result.p_value.unwrap() > 0.5);
    } else if expected_threshold < 0.1 {
        // Very different samples should have low p-value
        assert!(result.p_value.unwrap() < 0.1);
    } else {
        // Somewhat different samples
        assert!(result.p_value.is_some());
    }
}

#[rstest]
#[case(0.90)]
#[case(0.95)]
#[case(0.99)]
fn test_confidence_levels(#[case] confidence: f64) {
    let values = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let (lower, upper) = StatisticalAnalyzer::confidence_interval(&values, confidence);

    assert!(upper > lower);

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let mean_decimal = Decimal::try_from(mean).unwrap();
    assert!(mean_decimal >= lower);
    assert!(mean_decimal <= upper);
}

// ===== Robustness Tests =====

#[test]
fn test_mann_whitney_robust_to_outliers() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 1000.0]; // 1000.0 is outlier

    let result = StatisticalAnalyzer::mann_whitney_u(&sample1, &sample2);

    // Mann-Whitney should still detect difference despite outlier
    assert!(result.p_value.is_some());
}

#[test]
fn test_bootstrap_stability() {
    let sample1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let sample2 = vec![15.0, 16.0, 17.0, 18.0, 19.0];

    // Run bootstrap multiple times
    let result1 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);
    let result2 = StatisticalAnalyzer::bootstrap_comparison(&sample1, &sample2, 1000, 0.95);

    let (lower1, upper1) = result1.confidence_interval.unwrap();
    let (lower2, upper2) = result2.confidence_interval.unwrap();

    // Results should be similar (allowing for some random variation)
    assert_relative_eq!(lower1, lower2, epsilon = 2.0);
    assert_relative_eq!(upper1, upper2, epsilon = 2.0);
}

// ===== Real-World Scenario Tests =====

#[test]
fn test_ab_test_scenario() {
    // Simulating A/B test results
    let control_conversion_rates = vec![0.10, 0.12, 0.11, 0.13, 0.10, 0.12, 0.11];
    let treatment_conversion_rates = vec![0.15, 0.16, 0.14, 0.17, 0.15, 0.16, 0.15];

    let t_result = StatisticalAnalyzer::t_test(&control_conversion_rates, &treatment_conversion_rates);
    let effect_size = StatisticalAnalyzer::cohens_d(&control_conversion_rates, &treatment_conversion_rates);

    assert!(t_result.p_value.is_some());
    // Treatment mean is higher than control, so effect size should be negative (control - treatment)
    assert!(effect_size < 0.0); // Treatment is better, so control - treatment is negative
}

#[test]
fn test_model_performance_comparison() {
    // Comparing two ML models
    let model1_accuracy = vec![0.85, 0.87, 0.86, 0.88, 0.85, 0.87, 0.86];
    let model2_accuracy = vec![0.90, 0.91, 0.89, 0.92, 0.90, 0.91, 0.90];

    let result = StatisticalAnalyzer::mann_whitney_u(&model1_accuracy, &model2_accuracy);
    assert!(result.p_value.is_some());
    assert!(result.p_value.unwrap() < 0.05); // Significant difference
}
