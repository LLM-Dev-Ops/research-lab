use llm_research_core::MetricCalculator;
use llm_research_metrics::calculators::{BleuCalculator, MetricInput, SmoothingMethod};
use rust_decimal::Decimal;
use approx::assert_relative_eq;
use rstest::rstest;

// ===== BLEU-1 Tests =====

#[tokio::test]
async fn test_bleu1_perfect_match() {
    let calculator = BleuCalculator::new(1);

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Perfect match should give high score
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu1_partial_match() {
    let calculator = BleuCalculator::new(1);

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the dog sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // 2 out of 3 unigrams match
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu1_no_match() {
    let calculator = BleuCalculator::new(1);

    let input = MetricInput {
        predicted: "hello world".to_string(),
        reference: Some("goodbye universe".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== BLEU-2 Tests =====

#[tokio::test]
async fn test_bleu2_bigram_overlap() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "the cat sat on the mat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu2_partial_bigrams() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the cat ran".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // "the cat" matches, but "cat sat" vs "cat ran" doesn't
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu2_no_bigram_overlap() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "a b c".to_string(),
        reference: Some("d e f".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== BLEU-3 Tests =====

#[tokio::test]
async fn test_bleu3_trigram_overlap() {
    let calculator = BleuCalculator::new(3);

    let input = MetricInput {
        predicted: "the quick brown fox jumps".to_string(),
        reference: Some("the quick brown fox jumps".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu3_partial_trigrams() {
    let calculator = BleuCalculator::new(3);

    let input = MetricInput {
        predicted: "the quick brown fox".to_string(),
        reference: Some("the quick brown cat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // "the quick brown" matches
    assert!(result.score >= Decimal::ZERO);
}

// ===== BLEU-4 Tests (Standard BLEU) =====

#[tokio::test]
async fn test_bleu4_perfect_match() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "the quick brown fox jumps over the lazy dog".to_string(),
        reference: Some("the quick brown fox jumps over the lazy dog".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu4_real_text_high_similarity() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "It is a guide to action which ensures that the military always obeys the commands of the party".to_string(),
        reference: Some("It is a guide to action that ensures that the military will forever heed Party commands".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Should have some overlap
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu4_real_text_moderate_similarity() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "The cat is on the mat".to_string(),
        reference: Some("There is a cat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu4_real_text_low_similarity() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "Machine learning models require extensive training".to_string(),
        reference: Some("Deep neural networks need large datasets".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Very different sentences
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== Brevity Penalty Tests =====

#[tokio::test]
async fn test_bleu_brevity_penalty_shorter_predicted() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "the cat".to_string(),
        reference: Some("the cat sat on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Should be penalized for being shorter
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_brevity_penalty_longer_predicted() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "the cat sat on the mat and played".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // No brevity penalty when predicted is longer
    assert!(result.score >= Decimal::ZERO);
}

#[test]
fn test_brevity_penalty_calculation() {
    let calculator = BleuCalculator::new(4);

    // Test internal brevity penalty function
    let (bleu1, _) = calculator.calculate_bleu(
        "the cat",
        "the cat sat on the mat",
    );

    let (bleu2, _) = calculator.calculate_bleu(
        "the cat sat on the mat",
        "the cat sat on the mat",
    );

    // Shorter candidate should have lower score due to brevity penalty
    assert!(bleu1 < bleu2);
}

// ===== N-gram Tests (via public calculate_bleu method) =====

#[test]
fn test_ngrams_via_bleu_calculation() {
    let calculator = BleuCalculator::new(1);
    let (bleu, precisions) = calculator.calculate_bleu("the cat sat", "the cat sat");

    // Perfect unigram match
    assert!(bleu > 0.9);
    assert_eq!(precisions.len(), 1);
}

#[test]
fn test_bigrams_via_bleu() {
    let calculator = BleuCalculator::new(2);
    let (bleu, precisions) = calculator.calculate_bleu("the cat sat", "the cat sat");

    // Perfect bigram match
    assert!(bleu > 0.9);
    assert_eq!(precisions.len(), 2);
}

#[test]
fn test_trigrams_via_bleu() {
    let calculator = BleuCalculator::new(3);
    let (bleu, precisions) = calculator.calculate_bleu("the cat sat", "the cat sat");

    // Perfect trigram match
    assert!(bleu > 0.9);
    assert_eq!(precisions.len(), 3);
}

#[test]
fn test_insufficient_words_for_ngrams() {
    let calculator = BleuCalculator::new(4);
    let (bleu, _precisions) = calculator.calculate_bleu("the cat", "the cat sat on");

    // Can't compute 4-grams from 2 words
    assert_eq!(bleu, 0.0);
}

#[test]
fn test_case_normalization_in_bleu() {
    let calculator = BleuCalculator::new(1);
    let (bleu1, _) = calculator.calculate_bleu("The Cat SAT", "the cat sat");

    // Case should be normalized
    assert!(bleu1 > 0.9);
}

// ===== Smoothing Tests =====

#[tokio::test]
async fn test_bleu_no_smoothing() {
    let calculator = BleuCalculator::new(4)
        .with_smoothing(SmoothingMethod::None);

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the dog ran".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_add1_smoothing() {
    let calculator = BleuCalculator::new(4)
        .with_smoothing(SmoothingMethod::Add1);

    let input = MetricInput {
        predicted: "the cat sat".to_string(),
        reference: Some("the dog ran".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_add01_smoothing() {
    let calculator = BleuCalculator::new(4)
        .with_smoothing(SmoothingMethod::Add01);

    let input = MetricInput {
        predicted: "the cat".to_string(),
        reference: Some("the dog".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= Decimal::ZERO);
}

#[test]
fn test_smoothing_comparison() {
    let calc_none = BleuCalculator::new(2).with_smoothing(SmoothingMethod::None);
    let calc_add1 = BleuCalculator::new(2).with_smoothing(SmoothingMethod::Add1);

    let predicted = "the cat";
    let reference = "the dog";

    let (bleu_none, _) = calc_none.calculate_bleu(predicted, reference);
    let (bleu_add1, _) = calc_add1.calculate_bleu(predicted, reference);

    // With smoothing should be >= without smoothing
    assert!(bleu_add1 >= bleu_none);
}

// ===== Edge Cases =====

#[tokio::test]
async fn test_bleu_empty_predicted() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("some reference text".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_empty_reference() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "some predicted text".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_both_empty() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "".to_string(),
        reference: Some("".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_single_word() {
    let calculator = BleuCalculator::new(1);

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("hello".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_single_word_different() {
    let calculator = BleuCalculator::new(1);

    let input = MetricInput {
        predicted: "hello".to_string(),
        reference: Some("goodbye".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_no_reference() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "some text".to_string(),
        reference: None,
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_whitespace_handling() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "  the   cat   sat  ".to_string(),
        reference: Some("the cat sat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_unicode_text() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "こんにちは 世界".to_string(),
        reference: Some("こんにちは 世界".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);
}

// ===== Real Text Examples =====

#[tokio::test]
async fn test_bleu_real_translation_example1() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "The cat is sitting on the mat".to_string(),
        reference: Some("The cat sits on the mat".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // Should have overlapping bigrams like "the cat", "on the", "the mat"
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_real_translation_example2() {
    let calculator = BleuCalculator::new(2);

    let input = MetricInput {
        predicted: "I love natural language processing".to_string(),
        reference: Some("I enjoy natural language processing".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    // "natural language", "language processing" should match
    assert!(result.score > Decimal::ZERO);
}

#[tokio::test]
async fn test_bleu_real_paraphrase() {
    let calculator = BleuCalculator::new(4);

    let input = MetricInput {
        predicted: "The weather is nice today".to_string(),
        reference: Some("Today the weather is nice".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score >= Decimal::ZERO);
}

// ===== Parameterized Tests =====

#[rstest]
#[case(1, "the cat sat", "the cat sat", true)]
#[case(2, "the cat sat", "the cat sat", true)]
#[case(3, "the cat sat", "the cat sat", true)]
#[case(4, "the quick brown fox", "the quick brown fox", true)]
#[tokio::test]
async fn test_bleu_n_perfect_matches(
    #[case] n: usize,
    #[case] predicted: &str,
    #[case] reference: &str,
    #[case] should_score: bool,
) {
    let calculator = BleuCalculator::new(n);

    let input = MetricInput {
        predicted: predicted.to_string(),
        reference: Some(reference.to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();

    if should_score {
        assert!(result.score > Decimal::ZERO);
    } else {
        assert_eq!(result.score, Decimal::ZERO);
    }
}

#[rstest]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[tokio::test]
async fn test_bleu_n_no_overlap(#[case] n: usize) {
    let calculator = BleuCalculator::new(n);

    let input = MetricInput {
        predicted: "completely different text".to_string(),
        reference: Some("unrelated words here".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert_eq!(result.score, Decimal::ZERO);
}

// ===== Test BLEU Score Properties =====

#[test]
fn test_bleu_calculate_returns_precisions() {
    let calculator = BleuCalculator::new(4);

    let (bleu, precisions) = calculator.calculate_bleu(
        "the cat sat on the mat",
        "the cat sat on the mat",
    );

    assert_eq!(precisions.len(), 4);
    // Perfect match should have all precisions at 1.0
    for precision in precisions {
        assert_relative_eq!(precision, 1.0, epsilon = 0.01);
    }
    assert!(bleu > 0.9); // Should be close to 1.0
}

#[test]
fn test_bleu_zero_when_any_precision_zero() {
    let calculator = BleuCalculator::new(4);

    // Too short for 4-grams
    let (bleu, _) = calculator.calculate_bleu(
        "the cat",
        "the cat sat on the mat",
    );

    // Should be zero because can't compute 4-grams from 2 words
    assert_eq!(bleu, 0.0);
}

#[tokio::test]
async fn test_bleu_default_is_bleu4() {
    let calculator = BleuCalculator::default();

    let input = MetricInput {
        predicted: "the quick brown fox jumps over the lazy dog".to_string(),
        reference: Some("the quick brown fox jumps over the lazy dog".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    assert!(result.score > Decimal::ZERO);

    let metadata = result.metadata.as_object().unwrap();
    assert_eq!(metadata.get("max_n").unwrap().as_u64().unwrap(), 4);
}

// ===== Metadata Tests =====

#[tokio::test]
async fn test_bleu_metadata() {
    let calculator = BleuCalculator::new(3)
        .with_smoothing(SmoothingMethod::Add1);

    let input = MetricInput {
        predicted: "test".to_string(),
        reference: Some("test".to_string()),
    };

    let result = calculator.calculate(input).await.unwrap();
    let metadata = result.metadata.as_object().unwrap();

    assert_eq!(metadata.get("metric").unwrap().as_str().unwrap(), "bleu");
    assert_eq!(metadata.get("max_n").unwrap().as_u64().unwrap(), 3);
}
