use llm_research_core::MetricCalculator;
use llm_research_metrics::calculators::{MetricInput, RougeCalculator, RougeVariant};
use rust_decimal::Decimal;
use approx::assert_relative_eq;
use rstest::rstest;

// ===== ROUGE-1 Tests =====

#[tokio::test]
async fn test_rouge1_perfect_match() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Perfect match should give F1 close to 1.0
    assert!(result.score > Decimal::ZERO);

    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(precision, 1.0, epsilon = 0.01);
    assert_relative_eq!(recall, 1.0, epsilon = 0.01);
    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_rouge1_partial_overlap() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "the cat sat on mat".to_string(),
        reference: Some("the dog sat on floor".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    // "the", "sat", "on" overlap (3 out of 5 in each)
    assert!(precision > 0.0);
    assert!(recall > 0.0);
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge1_no_overlap() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: Some("goodbye universe".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);

    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    assert_eq!(precision, 0.0);
    assert_eq!(recall, 0.0);
}

#[tokio::test]
async fn test_rouge1_precision_vs_recall() {
    let calculator = RougeCalculator::rouge_1();

    // Predicted is longer
    let input = MetricInput {
        predicted: "the cat sat on the mat and played".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    // All reference words appear in predicted, so recall should be 1.0
    assert_relative_eq!(recall, 1.0, epsilon = 0.01);
    // But precision is lower because predicted has extra words
    assert!(precision < recall);
}

#[tokio::test]
async fn test_rouge1_case_insensitivity() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "The Cat Sat".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

// ===== ROUGE-2 Tests =====

#[tokio::test]
async fn test_rouge2_perfect_match() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(precision, 1.0, epsilon = 0.01);
    assert_relative_eq!(recall, 1.0, epsilon = 0.01);
    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_rouge2_bigram_overlap() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the floor".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // Should have overlapping bigrams: "the cat", "cat sat", "sat on", "on the"
    assert!(result.score > Decimal::ZERO);

    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    assert!(precision > 0.5);
    assert!(recall > 0.5);
}

#[tokio::test]
async fn test_rouge2_no_bigram_overlap() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "a b c d".to_string(),
        reference: Some("e f g h".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge2_partial_bigrams() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "the quick brown fox".to_string(),
        reference: Some("the quick red fox".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // "the quick" matches
    assert!(result.score > Decimal::ZERO);
}

// ===== ROUGE-L Tests =====

#[tokio::test]
async fn test_rougel_perfect_match() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(precision, 1.0, epsilon = 0.01);
    assert_relative_eq!(recall, 1.0, epsilon = 0.01);
    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_rougel_subsequence_match() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "the quick brown fox jumps".to_string(),
        reference: Some("the brown fox jumps high".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // LCS: "the brown fox jumps" (4 words)
    assert!(result.score > Decimal::ZERO);

    let metadata = result.metadata.as_object().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    // LCS is 4 words, reference is 5 words
    assert_relative_eq!(recall, 0.8, epsilon = 0.01);
}

#[tokio::test]
async fn test_rougel_reordered_words() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "fox brown quick the".to_string(),
        reference: Some("the quick brown fox".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // LCS considers order, so this should be low
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_rougel_insertion_deletion() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the big cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // LCS: "the cat sat" (3 words)
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    assert_relative_eq!(precision, 1.0, epsilon = 0.01); // All predicted words in LCS
    assert_relative_eq!(recall, 0.75, epsilon = 0.01); // 3 out of 4 reference words
}

// LCS tests via the public calculate_rouge_l method (accessed through metadata)

#[tokio::test]
async fn test_lcs_perfect_match() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    // Perfect LCS match
    assert_relative_eq!(precision, 1.0, epsilon = 0.01);
    assert_relative_eq!(recall, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_lcs_with_insertions() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "a b c".to_string(),
        reference: Some("a x b y c".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();

    // All words of predicted are in LCS
    assert_relative_eq!(precision, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_lcs_different_sequences() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "a b c".to_string(),
        reference: Some("d e f".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // No LCS
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_lcs_empty_reference() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "a b".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // Empty reference gives zero score
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== Precision, Recall, F1 Tests =====

#[tokio::test]
async fn test_rouge_precision_calculation() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "a b c d e f".to_string(), // 6 words
        reference: Some("a b c".to_string()), // 3 words, all match
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    // Precision: 3 matching words / 6 predicted words = 0.5
    assert_relative_eq!(precision, 0.5, epsilon = 0.01);
    // Recall: 3 matching words / 3 reference words = 1.0
    assert_relative_eq!(recall, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_rouge_recall_calculation() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "a b c".to_string(), // 3 words
        reference: Some("a b c d e f".to_string()), // 6 words, 3 match
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();

    // Precision: 3 matching words / 3 predicted words = 1.0
    assert_relative_eq!(precision, 1.0, epsilon = 0.01);
    // Recall: 3 matching words / 6 reference words = 0.5
    assert_relative_eq!(recall, 0.5, epsilon = 0.01);
}

#[tokio::test]
async fn test_rouge_f1_calculation() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "a b c d".to_string(),
        reference: Some("a b e f".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let precision = metadata.get("precision").unwrap().as_f64().unwrap();
    let recall = metadata.get("recall").unwrap().as_f64().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    // 2 words overlap: "a", "b"
    // Precision: 2/4 = 0.5
    // Recall: 2/4 = 0.5
    // F1: 2 * 0.5 * 0.5 / (0.5 + 0.5) = 0.5
    assert_relative_eq!(precision, 0.5, epsilon = 0.01);
    assert_relative_eq!(recall, 0.5, epsilon = 0.01);
    assert_relative_eq!(f1, 0.5, epsilon = 0.01);
}

// ===== Real Text Examples =====

#[tokio::test]
async fn test_rouge1_real_text_summary() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "The study shows that climate change affects biodiversity".to_string(),
        reference: Some("Climate change impacts biodiversity according to the study".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge2_real_text_summary() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "Machine learning models require large datasets for training".to_string(),
        reference: Some("Large datasets are required for training machine learning models".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Should have some bigram overlap
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_rougel_real_text_summary() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "The quick brown fox jumps over the lazy dog".to_string(),
        reference: Some("A quick brown fox jumps over a lazy dog".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Most words match in order
    assert!(result.score > Decimal::new(5, 1)); // > 0.5
}

#[tokio::test]
async fn test_rouge_real_news_headline() {
    let calculator = RougeCalculator::rouge_l();

    let input = MetricInput {
        predicted: "Scientists discover new species in Amazon rainforest".to_string(),
        reference: Some("New species discovered by scientists in Amazon".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

// ===== Edge Cases =====

#[tokio::test]
async fn test_rouge_empty_predicted() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("some reference text".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_empty_reference() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "some predicted text".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_both_empty() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_no_reference() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "some text".to_string(),
        reference: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_single_word_match() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("hello".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_rouge_single_word_no_match() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("goodbye".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_whitespace_handling() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "  the   cat   sat  ".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

#[tokio::test]
async fn test_rouge_unicode_text() {
    let calculator = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "こんにちは 世界".to_string(),
        reference: Some("こんにちは 世界".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    let metadata = result.metadata.as_object().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(f1, 1.0, epsilon = 0.01);
}

// ===== Variant Tests =====

#[tokio::test]
async fn test_rouge_variant_n() {
    let calculator = RougeCalculator::new(RougeVariant::RougeN { n: 3 });

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_rouge_variant_w() {
    let calculator = RougeCalculator::new(RougeVariant::RougeW { weight: 2 });

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

// ===== Parameterized Tests =====

#[rstest]
#[case(1, "the cat sat", "the cat sat", 1.0)]
#[case(2, "the cat sat", "the cat sat", 1.0)]
#[case(1, "a b c", "d e f", 0.0)]
#[case(2, "a b c", "d e f", 0.0)]
#[tokio::test]
async fn test_rouge_n_variants(
    #[case] n: usize,
    #[case] predicted: &str,
    #[case] reference: &str,
    #[case] expected_f1: f64,
) {
    let calculator = RougeCalculator::new(RougeVariant::RougeN { n });

    let input = MetricInput {
        predicted: predicted.to_string(),
        reference: Some(reference.to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();
    let f1 = metadata.get("f1").unwrap().as_f64().unwrap();

    assert_relative_eq!(f1, expected_f1, epsilon = 0.01);
}

// ===== Comparison with Other Metrics =====

#[tokio::test]
async fn test_rouge_vs_exact_match() {
    let rouge = RougeCalculator::rouge_1();

    let input = MetricInput {
        predicted: "the cat sat on mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = rouge.calculate(input).await.unwrap();

    // ROUGE should give partial credit (not 0, not 1)
    assert!(result.score > Decimal::ZERO);
    assert!(result.score < Decimal::ONE);
}

// ===== Default Tests =====

#[tokio::test]
async fn test_rouge_default_is_rougel() {
    let calculator = RougeCalculator::default();

    let input = MetricInput {
        predicted: "test".to_string(),
        reference: Some("test".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    // Default should be ROUGE-L
    match calculator.variant {
        RougeVariant::RougeL => {},
        _ => panic!("Default should be ROUGE-L"),
    }

    // Ensure result is valid
    assert!(result.score >= Decimal::ZERO);
}

// ===== Metadata Tests =====

#[tokio::test]
async fn test_rouge_metadata_structure() {
    let calculator = RougeCalculator::rouge_2();

    let input = MetricInput {
        predicted: "test text".to_string(),
        reference: Some("test text".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();

    assert_eq!(metadata.get("metric").unwrap().as_str().unwrap(), "rouge");
    assert!(metadata.contains_key("variant"));
    assert!(metadata.contains_key("precision"));
    assert!(metadata.contains_key("recall"));
    assert!(metadata.contains_key("f1"));
}
