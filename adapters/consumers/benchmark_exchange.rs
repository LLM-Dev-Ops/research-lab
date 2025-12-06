//! LLM-Benchmark-Exchange Consumption Adapter
//!
//! Thin adapter for consuming community benchmarks, shared evaluation corpora,
//! and standardized scoring sets from the LLM-Benchmark-Exchange service.
//! This adapter does not modify any existing research logic or scoring systems.
//!
//! # Consumed Data Types
//!
//! - Community benchmarks (shared benchmark definitions)
//! - Evaluation corpora (test datasets, prompt collections)
//! - Standardized scoring sets (baseline scores, leaderboard data)
//! - Benchmark metadata (authors, versions, licenses)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ConsumerResult, ConsumptionMetadata, ExternalServiceConfig, HealthCheckable};

/// Configuration specific to LLM-Benchmark-Exchange consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkExchangeConfig {
    /// Base service configuration
    #[serde(flatten)]
    pub base: ExternalServiceConfig,
    /// Registry namespace for benchmarks
    pub registry_namespace: Option<String>,
    /// Whether to verify checksums on download
    pub verify_checksums: bool,
    /// Cache directory for downloaded benchmarks
    pub cache_dir: Option<String>,
}

impl Default for BenchmarkExchangeConfig {
    fn default() -> Self {
        Self {
            base: ExternalServiceConfig {
                endpoint: "https://api.benchmark-exchange.local".to_string(),
                ..Default::default()
            },
            registry_namespace: None,
            verify_checksums: true,
            cache_dir: None,
        }
    }
}

/// A community benchmark from the Exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityBenchmark {
    /// Benchmark identifier
    pub benchmark_id: String,
    /// Human-readable name
    pub name: String,
    /// Version string
    pub version: String,
    /// Description
    pub description: String,
    /// Benchmark category
    pub category: BenchmarkCategory,
    /// Task types covered
    pub task_types: Vec<String>,
    /// Test cases/samples
    pub test_cases: Vec<BenchmarkTestCase>,
    /// Scoring configuration
    pub scoring_config: ScoringConfiguration,
    /// Benchmark metadata
    pub benchmark_metadata: BenchmarkMeta,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Categories of benchmarks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkCategory {
    Reasoning,
    CodeGeneration,
    MathematicalReasoning,
    NaturalLanguageUnderstanding,
    Summarization,
    Translation,
    QuestionAnswering,
    DialogueSystems,
    Safety,
    Factuality,
    Custom(String),
}

/// A single test case in a benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTestCase {
    /// Test case identifier
    pub case_id: String,
    /// Input prompt or question
    pub input: String,
    /// Expected output (if deterministic)
    pub expected_output: Option<String>,
    /// Reference outputs (for comparison-based scoring)
    pub reference_outputs: Vec<String>,
    /// Ground truth labels or annotations
    pub ground_truth: Option<Value>,
    /// Difficulty level (1-5)
    pub difficulty: Option<u8>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

/// Configuration for how a benchmark is scored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfiguration {
    /// Primary metric name
    pub primary_metric: String,
    /// Additional metrics
    pub additional_metrics: Vec<String>,
    /// Scoring method
    pub scoring_method: ScoringMethod,
    /// Normalization range
    pub normalization: Option<NormalizationConfig>,
    /// Passing threshold (if applicable)
    pub passing_threshold: Option<f64>,
}

/// Method used for scoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScoringMethod {
    ExactMatch,
    FuzzyMatch,
    SemanticSimilarity,
    ReferenceComparison,
    LlmJudge,
    HumanEvaluation,
    Custom(String),
}

/// Normalization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    /// Minimum score
    pub min: f64,
    /// Maximum score
    pub max: f64,
    /// Whether to invert (lower is better)
    pub invert: bool,
}

/// Metadata about a benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMeta {
    /// Authors/contributors
    pub authors: Vec<String>,
    /// License identifier
    pub license: String,
    /// Citation/reference
    pub citation: Option<String>,
    /// Homepage/documentation URL
    pub homepage: Option<String>,
    /// Creation date
    pub created_at: String,
    /// Last updated date
    pub updated_at: String,
    /// Download count
    pub download_count: Option<u64>,
}

/// An evaluation corpus from the Exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCorpus {
    /// Corpus identifier
    pub corpus_id: String,
    /// Corpus name
    pub name: String,
    /// Description
    pub description: String,
    /// Number of samples
    pub sample_count: usize,
    /// Sample format/schema
    pub sample_schema: Value,
    /// Sample entries (or download URL for large corpora)
    pub samples: CorpusSamples,
    /// Languages covered
    pub languages: Vec<String>,
    /// Domains covered
    pub domains: Vec<String>,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Corpus samples - either inline or reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CorpusSamples {
    /// Inline samples for small corpora
    Inline(Vec<Value>),
    /// Reference to download location
    Reference {
        download_url: String,
        checksum: String,
        format: String,
    },
}

/// Standardized scoring set with baseline scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardizedScoringSet {
    /// Scoring set identifier
    pub scoring_set_id: String,
    /// Associated benchmark ID
    pub benchmark_id: String,
    /// Baseline scores from known models
    pub baseline_scores: Vec<BaselineScore>,
    /// Leaderboard entries
    pub leaderboard: Vec<LeaderboardEntry>,
    /// Statistical summary
    pub statistics: ScoringStatistics,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Baseline score from a known model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineScore {
    /// Model identifier
    pub model_id: String,
    /// Model name
    pub model_name: String,
    /// Primary score
    pub score: f64,
    /// Additional metric scores
    pub metric_scores: Value,
    /// Evaluation date
    pub evaluated_at: String,
    /// Evaluation configuration
    pub eval_config: Option<Value>,
}

/// Leaderboard entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Rank
    pub rank: u32,
    /// Model identifier
    pub model_id: String,
    /// Model name
    pub model_name: String,
    /// Score
    pub score: f64,
    /// Submission date
    pub submitted_at: String,
    /// Verified status
    pub verified: bool,
}

/// Statistical summary of scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringStatistics {
    /// Number of submissions
    pub submission_count: u64,
    /// Mean score
    pub mean_score: f64,
    /// Median score
    pub median_score: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Min score
    pub min_score: f64,
    /// Max score
    pub max_score: f64,
}

/// Query parameters for Benchmark Exchange consumption.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkQuery {
    /// Filter by category
    pub category: Option<BenchmarkCategory>,
    /// Filter by task types
    pub task_types: Option<Vec<String>>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Minimum version
    pub min_version: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Sort by field
    pub sort_by: Option<String>,
}

/// Trait for consuming benchmarks from LLM-Benchmark-Exchange.
#[async_trait]
pub trait BenchmarkExchangeConsumer: HealthCheckable {
    /// Consume a specific community benchmark by ID.
    async fn consume_benchmark(&self, benchmark_id: &str) -> ConsumerResult<CommunityBenchmark>;

    /// Search and consume benchmarks matching a query.
    async fn consume_benchmarks(
        &self,
        query: &BenchmarkQuery,
    ) -> ConsumerResult<Vec<CommunityBenchmark>>;

    /// Consume an evaluation corpus by ID.
    async fn consume_corpus(&self, corpus_id: &str) -> ConsumerResult<EvaluationCorpus>;

    /// Consume standardized scoring set for a benchmark.
    async fn consume_scoring_set(
        &self,
        benchmark_id: &str,
    ) -> ConsumerResult<StandardizedScoringSet>;

    /// List available benchmarks.
    async fn list_available_benchmarks(
        &self,
        query: &BenchmarkQuery,
    ) -> ConsumerResult<Vec<String>>;

    /// Get benchmark metadata without full test cases.
    async fn get_benchmark_metadata(&self, benchmark_id: &str) -> ConsumerResult<BenchmarkMeta>;
}

/// Client implementation for consuming from LLM-Benchmark-Exchange.
pub struct BenchmarkExchangeClient {
    config: BenchmarkExchangeConfig,
}

impl BenchmarkExchangeClient {
    /// Create a new benchmark exchange client with the given configuration.
    pub fn new(config: BenchmarkExchangeConfig) -> Self {
        Self { config }
    }

    /// Create a client with default configuration and custom endpoint.
    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            config: BenchmarkExchangeConfig {
                base: ExternalServiceConfig {
                    endpoint: endpoint.to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &BenchmarkExchangeConfig {
        &self.config
    }
}

#[async_trait]
impl HealthCheckable for BenchmarkExchangeClient {
    async fn health_check(&self) -> ConsumerResult<bool> {
        Ok(!self.config.base.endpoint.is_empty())
    }
}

#[async_trait]
impl BenchmarkExchangeConsumer for BenchmarkExchangeClient {
    async fn consume_benchmark(&self, benchmark_id: &str) -> ConsumerResult<CommunityBenchmark> {
        // Implementation would fetch benchmark from the Exchange API
        Ok(CommunityBenchmark {
            benchmark_id: benchmark_id.to_string(),
            name: format!("Benchmark {}", benchmark_id),
            version: "1.0.0".to_string(),
            description: "Placeholder benchmark".to_string(),
            category: BenchmarkCategory::Reasoning,
            task_types: vec![],
            test_cases: vec![],
            scoring_config: ScoringConfiguration {
                primary_metric: "accuracy".to_string(),
                additional_metrics: vec![],
                scoring_method: ScoringMethod::ExactMatch,
                normalization: None,
                passing_threshold: None,
            },
            benchmark_metadata: BenchmarkMeta {
                authors: vec![],
                license: "MIT".to_string(),
                citation: None,
                homepage: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                download_count: None,
            },
            metadata: ConsumptionMetadata::new("llm-benchmark-exchange"),
        })
    }

    async fn consume_benchmarks(
        &self,
        _query: &BenchmarkQuery,
    ) -> ConsumerResult<Vec<CommunityBenchmark>> {
        // Implementation would query benchmarks with filters
        Ok(vec![])
    }

    async fn consume_corpus(&self, corpus_id: &str) -> ConsumerResult<EvaluationCorpus> {
        // Implementation would fetch evaluation corpus
        Ok(EvaluationCorpus {
            corpus_id: corpus_id.to_string(),
            name: format!("Corpus {}", corpus_id),
            description: "Placeholder corpus".to_string(),
            sample_count: 0,
            sample_schema: serde_json::json!({}),
            samples: CorpusSamples::Inline(vec![]),
            languages: vec!["en".to_string()],
            domains: vec![],
            metadata: ConsumptionMetadata::new("llm-benchmark-exchange"),
        })
    }

    async fn consume_scoring_set(
        &self,
        benchmark_id: &str,
    ) -> ConsumerResult<StandardizedScoringSet> {
        // Implementation would fetch scoring set
        Ok(StandardizedScoringSet {
            scoring_set_id: Uuid::new_v4().to_string(),
            benchmark_id: benchmark_id.to_string(),
            baseline_scores: vec![],
            leaderboard: vec![],
            statistics: ScoringStatistics {
                submission_count: 0,
                mean_score: 0.0,
                median_score: 0.0,
                std_dev: 0.0,
                min_score: 0.0,
                max_score: 0.0,
            },
            metadata: ConsumptionMetadata::new("llm-benchmark-exchange"),
        })
    }

    async fn list_available_benchmarks(
        &self,
        _query: &BenchmarkQuery,
    ) -> ConsumerResult<Vec<String>> {
        // Implementation would list benchmark IDs
        Ok(vec![])
    }

    async fn get_benchmark_metadata(&self, benchmark_id: &str) -> ConsumerResult<BenchmarkMeta> {
        // Implementation would fetch only metadata
        Ok(BenchmarkMeta {
            authors: vec![],
            license: "MIT".to_string(),
            citation: None,
            homepage: Some(format!(
                "{}/benchmarks/{}",
                self.config.base.endpoint, benchmark_id
            )),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            download_count: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_exchange_config_default() {
        let config = BenchmarkExchangeConfig::default();
        assert!(config.verify_checksums);
        assert!(config.cache_dir.is_none());
    }

    #[test]
    fn test_client_creation() {
        let client = BenchmarkExchangeClient::with_endpoint("https://exchange.example.com");
        assert_eq!(
            client.config().base.endpoint,
            "https://exchange.example.com"
        );
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = BenchmarkExchangeClient::with_endpoint("https://exchange.example.com");
        let result = client.health_check().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_benchmark_category_serialization() {
        let category = BenchmarkCategory::CodeGeneration;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"code_generation\"");
    }

    #[test]
    fn test_scoring_method_serialization() {
        let method = ScoringMethod::SemanticSimilarity;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, "\"semantic_similarity\"");
    }

    #[test]
    fn test_corpus_samples_inline() {
        let samples = CorpusSamples::Inline(vec![serde_json::json!({"text": "test"})]);
        let json = serde_json::to_string(&samples).unwrap();
        assert!(json.contains("text"));
    }
}
