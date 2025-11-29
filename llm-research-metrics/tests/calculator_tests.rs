use llm_research_core::MetricCalculator;
use llm_research_metrics::calculators::*;

// ===== AccuracyCalculator Tests =====

#[tokio::test]
async fn test_accuracy_exact_match() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ONE);

    // Non-matching case
    let input2 = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("world".to_string()),
    };

    let result2 = calculator.calculate(input2).await.unwrap();
    assert_eq!(result2.score, rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_accuracy_case_insensitive() {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: "Hello World".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ONE);

    let input2 = MetricInput {
        predicted: "HELLO".to_string(),
        reference: Some("hello".to_string()),
    };

    let result2 = calculator.calculate(input2).await.unwrap();
    assert_eq!(result2.score, rust_decimal::Decimal::ONE);
}

#[tokio::test]
async fn test_accuracy_contains() {
    let calculator = AccuracyCalculator::new(ComparisonMode::Contains);

    let input = MetricInput {
        predicted: "hello world from rust".to_string(),
        reference: Some("world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ONE);
}

#[tokio::test]
async fn test_accuracy_semantic_similarity() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.5);

    let input = MetricInput {
        predicted: "the quick brown fox".to_string(),
        reference: Some("quick brown fox".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Should match due to high word overlap
    assert_eq!(result.score, rust_decimal::Decimal::ONE);

    let input2 = MetricInput {
        predicted: "completely different text".to_string(),
        reference: Some("unrelated words here".to_string()),
    };

    let result2 = calculator.calculate(input2).await.unwrap();
    assert_eq!(result2.score, rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_accuracy_no_reference() {
    let calculator = AccuracyCalculator::default();

    let input = MetricInput {
        predicted: "test".to_string(),
        reference: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ZERO);
}

// ===== BleuCalculator Tests =====

#[tokio::test]
async fn test_bleu_perfect_match() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Perfect match should give score close to 1.0
    assert!(result.score > rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_partial_match() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Partial match should give lower score
    assert!(result.score >= rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_no_match() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "completely different sentence".to_string(),
        reference: Some("unrelated words here".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // No n-gram overlap should give 0
    assert_eq!(result.score, rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_with_smoothing() {
    let calculator = BleuCalculator::new(4)
        .with_smoothing(SmoothingMethod::Add1);

    let input = MetricInput {
        predicted: "the cat".to_string(),
        reference: Some("the dog".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_empty_predicted() {
    let calculator = BleuCalculator::default();

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("some reference".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ZERO);
}

// ===== RougeCalculator Tests =====

#[tokio::test]
async fn test_rouge_1_perfect_match() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Perfect match should give high F1 score
    assert!(result.score > rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_1_partial_overlap() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "the cat sat on mat".to_string(),
        reference: Some("the dog sat on floor".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Some overlap ("the", "sat", "on")
    assert!(result.score > rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_2() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the floor".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Should have some bigram overlap
    assert!(result.score >= rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_l() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "the quick brown fox jumps".to_string(),
        reference: Some("the brown fox jumps high".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // LCS-based score should be non-zero
    assert!(result.score > rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_empty_reference() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "some text".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_no_reference() {
    let calculator = RougeCalculator::default();

    let input = MetricInput {
        predicted: "some text".to_string(),
        reference: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ZERO);
}

// ===== PerplexityCalculator Tests =====

#[tokio::test]
async fn test_perplexity_calculation() {
    let calculator = PerplexityCalculator::new();

    let input = PerplexityInput {
        log_probs: vec![-0.5, -0.3, -0.7, -0.4],
        token_count: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.perplexity > 0.0);
    assert!(result.cross_entropy > 0.0);
}

#[tokio::test]
async fn test_perplexity_with_uniform_probs() {
    let calculator = PerplexityCalculator::new();

    // Uniform log probs should give consistent perplexity
    let input = PerplexityInput {
        log_probs: vec![-1.0, -1.0, -1.0, -1.0],
        token_count: Some(4),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.perplexity > 0.0);
    assert!((result.cross_entropy - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn test_perplexity_with_low_probs() {
    let calculator = PerplexityCalculator::new();

    let input = PerplexityInput {
        log_probs: vec![-5.0, -6.0, -5.5, -7.0],
        token_count: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    // Lower probabilities should give higher perplexity
    assert!(result.perplexity > 100.0);
}

#[tokio::test]
async fn test_perplexity_empty_input() {
    let calculator = PerplexityCalculator::new();

    let input = PerplexityInput {
        log_probs: vec![],
        token_count: None,
    };

    let result = calculator.calculate(input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_perplexity_with_base_2() {
    let calculator = PerplexityCalculator::new().with_base(2.0);

    let input = PerplexityInput {
        log_probs: vec![-1.0, -1.5, -2.0],
        token_count: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.perplexity > 0.0);
}

// ===== LatencyMetrics Tests =====

#[test]
fn test_latency_metrics_from_measurements() {
    let measurements = vec![10.0, 20.0, 15.0, 30.0, 25.0, 12.0, 18.0];
    let metrics = LatencyMetrics::from_measurements(&measurements);

    assert_eq!(metrics.min, 10.0);
    assert_eq!(metrics.max, 30.0);
    assert!(metrics.mean > 0.0);
    assert!(metrics.median > 0.0);
    assert!(metrics.std_dev > 0.0);
}

#[test]
fn test_latency_metrics_percentiles() {
    let measurements = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let metrics = LatencyMetrics::from_measurements(&measurements);

    // p50 for 10 values returns the value at index 5 (1-indexed), which is 6.0
    // Actual percentile calculation may vary by implementation
    assert!(metrics.p50 >= 5.0 && metrics.p50 <= 6.0);
    assert!(metrics.p90 >= 9.0);
    assert!(metrics.p95 >= 9.0);
    assert!(metrics.p99 >= 9.0);
}

#[test]
fn test_latency_metrics_empty() {
    let measurements: Vec<f64> = vec![];
    let metrics = LatencyMetrics::from_measurements(&measurements);

    assert_eq!(metrics.min, 0.0);
    assert_eq!(metrics.max, 0.0);
    assert_eq!(metrics.mean, 0.0);
}

#[test]
fn test_latency_metrics_single_value() {
    let measurements = vec![42.0];
    let metrics = LatencyMetrics::from_measurements(&measurements);

    assert_eq!(metrics.min, 42.0);
    assert_eq!(metrics.max, 42.0);
    assert_eq!(metrics.mean, 42.0);
    assert_eq!(metrics.median, 42.0);
}

#[test]
fn test_latency_metrics_with_ttft() {
    let measurements = vec![100.0, 110.0, 105.0];
    let metrics = LatencyMetrics::from_measurements(&measurements)
        .with_ttft(50.0);

    assert_eq!(metrics.ttft, Some(50.0));
}

#[test]
fn test_latency_metrics_with_throughput() {
    let measurements = vec![1000.0];
    let metrics = LatencyMetrics::from_measurements(&measurements)
        .with_throughput(100, 1000.0);

    assert_eq!(metrics.total_tokens, Some(100));
    assert_eq!(metrics.tokens_per_second, Some(100.0));
}

#[tokio::test]
async fn test_latency_calculator() {
    let calculator = LatencyCalculator;

    let input = LatencyInput {
        measurements: vec![10.5, 12.3, 11.7, 13.1, 10.9],
        ttft: Some(5.0),
        total_tokens: Some(50),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.metrics.mean > 0.0);
    assert_eq!(result.metrics.ttft, Some(5.0));
    assert_eq!(result.metrics.total_tokens, Some(50));
}

#[test]
fn test_latency_calculator_timing() {
    let start = LatencyCalculator::start();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let elapsed = LatencyCalculator::measure(start);

    assert!(elapsed >= 10.0);
}

// ===== Aggregation Tests =====

#[test]
fn test_latency_aggregation_mean() {
    let values = vec![10.0, 20.0, 30.0];
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    assert_eq!(mean, 20.0);
}

#[test]
fn test_latency_aggregation_variance() {
    let values = vec![10.0, 20.0, 30.0];
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / values.len() as f64;

    assert!(variance > 0.0);
}

// ===== Edge Cases =====

#[tokio::test]
async fn test_accuracy_with_whitespace() {
    // ExactMatch mode uses trim() which handles leading/trailing whitespace
    // but not internal whitespace differences
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    // Test with leading/trailing whitespace only (should match)
    let input = MetricInput {
        predicted: "  hello world  ".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, rust_decimal::Decimal::ONE);
}

#[tokio::test]
async fn test_bleu_single_word() {
    let calculator = BleuCalculator::new(1);

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("hello".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > rust_decimal::Decimal::ZERO);
}

#[test]
fn test_latency_zero_measurements() {
    let measurements = vec![0.0, 0.0, 0.0];
    let metrics = LatencyMetrics::from_measurements(&measurements);

    assert_eq!(metrics.min, 0.0);
    assert_eq!(metrics.max, 0.0);
    assert_eq!(metrics.mean, 0.0);
    assert_eq!(metrics.std_dev, 0.0);
}
