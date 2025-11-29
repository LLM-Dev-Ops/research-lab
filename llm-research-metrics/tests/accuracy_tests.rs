use llm_research_core::MetricCalculator;
use llm_research_metrics::calculators::{AccuracyCalculator, ComparisonMode, MetricInput};
use rust_decimal::Decimal;
use rstest::rstest;

// ===== ExactMatch Tests =====

#[tokio::test]
async fn test_exact_match_identical_strings() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);

    let metadata = result.metadata.as_object().unwrap();
    assert_eq!(metadata.get("metric").unwrap().as_str().unwrap(), "accuracy");
}

#[tokio::test]
async fn test_exact_match_case_sensitive() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "Hello World".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_exact_match_with_trimming() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "  hello world  ".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_exact_match_different_strings() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_exact_match_empty_strings() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_exact_match_one_empty() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== CaseInsensitive Tests =====

#[tokio::test]
async fn test_case_insensitive_different_cases() {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: "Hello World".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_case_insensitive_all_caps() {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: "HELLO WORLD".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_case_insensitive_mixed_case() {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: "HeLLo WoRLD".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_case_insensitive_with_whitespace() {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: "  HELLO WORLD  ".to_string(),
        reference: Some("hello world".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_case_insensitive_different_content() {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: "HELLO".to_string(),
        reference: Some("goodbye".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== Contains/Normalized Tests =====

#[tokio::test]
async fn test_contains_substring_match() {
    let calculator = AccuracyCalculator::new(ComparisonMode::Contains);

    let input = MetricInput {
        predicted: "the quick brown fox jumps over the lazy dog".to_string(),
        reference: Some("quick brown fox".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_contains_reverse_containment() {
    let calculator = AccuracyCalculator::new(ComparisonMode::Contains);

    let input = MetricInput {
        predicted: "fox".to_string(),
        reference: Some("the quick brown fox jumps".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_contains_no_match() {
    let calculator = AccuracyCalculator::new(ComparisonMode::Contains);

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: Some("goodbye universe".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_contains_case_insensitive() {
    let calculator = AccuracyCalculator::new(ComparisonMode::Contains);

    let input = MetricInput {
        predicted: "The Quick Brown Fox".to_string(),
        reference: Some("quick brown".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

// ===== Semantic Similarity Tests =====

#[tokio::test]
async fn test_semantic_high_similarity() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.5);

    let input = MetricInput {
        predicted: "the quick brown fox jumps".to_string(),
        reference: Some("quick brown fox jumps".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_semantic_low_similarity() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.8);

    let input = MetricInput {
        predicted: "completely different words here".to_string(),
        reference: Some("unrelated text content".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_semantic_partial_overlap() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.3);

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the dog sat on the floor".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_semantic_threshold_boundary() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.5);

    let input = MetricInput {
        predicted: "word1 word2 word3".to_string(),
        reference: Some("word1 word4 word5".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Jaccard similarity: intersection=1, union=5, similarity=0.2 < 0.5
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_semantic_both_empty() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.8);

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Empty strings are considered identical (similarity = 1.0)
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_semantic_one_empty() {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(0.5);

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== Edge Cases =====

#[tokio::test]
async fn test_no_reference() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_unicode_exact_match() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "こんにちは世界".to_string(),
        reference: Some("こんにちは世界".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_unicode_different() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "こんにちは".to_string(),
        reference: Some("さようなら".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_emoji_exact_match() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "Hello 👋 World 🌍".to_string(),
        reference: Some("Hello 👋 World 🌍".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_whitespace_only() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "   ".to_string(),
        reference: Some("   ".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_newlines_and_tabs() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "hello\nworld\t!".to_string(),
        reference: Some("hello\nworld\t!".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

// ===== Parameterized Tests with rstest =====

#[rstest]
#[case("hello", "hello", true)]
#[case("hello", "HELLO", false)]
#[case("  hello  ", "hello", true)]
#[case("hello world", "hello", false)]
#[case("", "", true)]
#[tokio::test]
async fn test_exact_match_cases(
    #[case] predicted: &str,
    #[case] reference: &str,
    #[case] expected_match: bool,
) {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: predicted.to_string(),
        reference: Some(reference.to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let expected = if expected_match { Decimal::ONE } else { Decimal::ZERO };
    assert_eq!(result.score, expected);
}

#[rstest]
#[case("hello", "HELLO", true)]
#[case("HeLLo", "hello", true)]
#[case("WORLD", "world", true)]
#[case("hello", "goodbye", false)]
#[case("  HELLO  ", "hello", true)]
#[tokio::test]
async fn test_case_insensitive_cases(
    #[case] predicted: &str,
    #[case] reference: &str,
    #[case] expected_match: bool,
) {
    let calculator = AccuracyCalculator::new(ComparisonMode::CaseInsensitive);

    let input = MetricInput {
        predicted: predicted.to_string(),
        reference: Some(reference.to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let expected = if expected_match { Decimal::ONE } else { Decimal::ZERO };
    assert_eq!(result.score, expected);
}

#[rstest]
#[case(0.5, "hello world from rust", "hello world", true)]
#[case(0.9, "hello world from rust", "hello world", false)]
#[case(0.3, "the cat sat", "the dog ran", false)]
#[case(0.8, "completely different", "unrelated words", false)]
#[tokio::test]
async fn test_semantic_with_thresholds(
    #[case] threshold: f64,
    #[case] predicted: &str,
    #[case] reference: &str,
    #[case] expected_match: bool,
) {
    let calculator = AccuracyCalculator::new(ComparisonMode::SemanticSimilarity)
        .with_threshold(threshold);

    let input = MetricInput {
        predicted: predicted.to_string(),
        reference: Some(reference.to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let expected = if expected_match { Decimal::ONE } else { Decimal::ZERO };
    assert_eq!(result.score, expected);
}

// ===== Perfect Match and No Match Tests =====

#[tokio::test]
async fn test_perfect_match_score() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "perfect match test".to_string(),
        reference: Some("perfect match test".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}

#[tokio::test]
async fn test_no_match_score() {
    let calculator = AccuracyCalculator::new(ComparisonMode::ExactMatch);

    let input = MetricInput {
        predicted: "completely".to_string(),
        reference: Some("different".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_default_calculator() {
    let calculator = AccuracyCalculator::default();

    let input = MetricInput {
        predicted: "test".to_string(),
        reference: Some("test".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ONE);
}
