# LLM-Research-Lab Pseudocode - Part 2: Metric Benchmarking & Dataset Versioning

> **SPARC Phase 2: Pseudocode (2 of 3)**
> Part of the LLM DevOps Ecosystem

---

## 3. Metric Benchmarking Framework

### 3.1 Metric Trait and Built-in Metrics

```rust
//! Metric benchmarking framework for LLM evaluation
//! Provides extensible metric definitions with statistical rigor

use async_trait::async_trait;
use std::sync::Arc;

/// Core metric trait - all metrics implement this interface
#[async_trait]
pub trait Metric: Send + Sync {
    /// Unique metric name
    fn name(&self) -> &str;

    /// Metric version for reproducibility
    fn version(&self) -> SemanticVersion;

    /// Compute metric value from input
    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError>;

    /// Metric properties for validation and aggregation
    fn properties(&self) -> MetricProperties;

    /// Optional batch computation for efficiency
    async fn compute_batch(&self, inputs: Vec<MetricInput>) -> Result<Vec<MetricOutput>, MetricError> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.compute(input).await?);
        }
        Ok(results)
    }

    /// Validate metric configuration
    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<(), MetricError> {
        Ok(())
    }
}

/// Input to metric computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricInput {
    pub prompt: String,
    pub response: String,
    pub expected: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub model_info: Option<ModelInfo>,
    pub latency_ms: Option<u64>,
    pub token_counts: Option<TokenCounts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: String,
    pub model_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Output from metric computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricOutput {
    pub value: MetricValue,
    pub confidence: Option<f64>,
    pub breakdown: Option<HashMap<String, f64>>,
    pub explanation: Option<String>,
    pub computation_time_ms: u64,
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Scalar(f64),
    Boolean(bool),
    Categorical(String),
    Distribution(DistributionValue),
    MultiLabel(Vec<String>),
    Structured(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionValue {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub percentiles: HashMap<String, f64>,
}

/// Metric properties for validation and aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricProperties {
    pub metric_type: MetricType,
    pub value_range: Option<ValueRange>,
    pub higher_is_better: bool,
    pub requires_reference: bool,
    pub supports_batch: bool,
    pub deterministic: bool,
    pub aggregation_method: AggregationMethod,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Quality,        // Response quality metrics
    Safety,         // Safety and alignment metrics
    Performance,    // Latency, throughput metrics
    Cost,           // Token usage, API cost metrics
    Factuality,     // Factual accuracy metrics
    Coherence,      // Text coherence metrics
    Relevance,      // Response relevance metrics
    Custom,         // User-defined metrics
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueRange {
    pub min: f64,
    pub max: f64,
    pub inclusive: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationMethod {
    Mean,
    Median,
    WeightedMean,
    Max,
    Min,
    Sum,
    Mode,
    Custom,
}

/// Built-in accuracy metric (exact match)
pub struct ExactMatchMetric {
    case_sensitive: bool,
    strip_whitespace: bool,
}

impl ExactMatchMetric {
    pub fn new(case_sensitive: bool, strip_whitespace: bool) -> Self {
        Self {
            case_sensitive,
            strip_whitespace,
        }
    }

    fn normalize(&self, text: &str) -> String {
        let mut result = text.to_string();
        if self.strip_whitespace {
            result = result.trim().to_string();
        }
        if !self.case_sensitive {
            result = result.to_lowercase();
        }
        result
    }
}

#[async_trait]
impl Metric for ExactMatchMetric {
    fn name(&self) -> &str {
        "exact_match"
    }

    fn version(&self) -> SemanticVersion {
        SemanticVersion::new(1, 0, 0)
    }

    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError> {
        let start = std::time::Instant::now();

        let expected = input.expected.ok_or(MetricError::MissingReference {
            metric: "exact_match".to_string(),
        })?;

        let normalized_response = self.normalize(&input.response);
        let normalized_expected = self.normalize(&expected);

        let matches = normalized_response == normalized_expected;

        Ok(MetricOutput {
            value: MetricValue::Boolean(matches),
            confidence: Some(1.0), // Deterministic
            breakdown: None,
            explanation: Some(if matches {
                "Response exactly matches expected".to_string()
            } else {
                "Response does not match expected".to_string()
            }),
            computation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn properties(&self) -> MetricProperties {
        MetricProperties {
            metric_type: MetricType::Quality,
            value_range: Some(ValueRange {
                min: 0.0,
                max: 1.0,
                inclusive: true,
            }),
            higher_is_better: true,
            requires_reference: true,
            supports_batch: true,
            deterministic: true,
            aggregation_method: AggregationMethod::Mean,
            tags: vec!["accuracy".to_string(), "exact".to_string()],
        }
    }
}

/// BLEU score metric for text similarity
pub struct BleuMetric {
    max_ngram: usize,
    smoothing: SmoothingMethod,
}

#[derive(Debug, Clone, Copy)]
pub enum SmoothingMethod {
    None,
    Add1,
    AddK(f64),
    Exponential,
}

impl BleuMetric {
    pub fn new(max_ngram: usize, smoothing: SmoothingMethod) -> Self {
        Self { max_ngram, smoothing }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect()
    }

    fn get_ngrams(&self, tokens: &[String], n: usize) -> HashMap<Vec<String>, usize> {
        let mut ngrams = HashMap::new();
        if tokens.len() >= n {
            for window in tokens.windows(n) {
                *ngrams.entry(window.to_vec()).or_insert(0) += 1;
            }
        }
        ngrams
    }

    fn modified_precision(&self, candidate: &[String], reference: &[String], n: usize) -> f64 {
        let candidate_ngrams = self.get_ngrams(candidate, n);
        let reference_ngrams = self.get_ngrams(reference, n);

        let mut clipped_count = 0;
        let mut total_count = 0;

        for (ngram, count) in &candidate_ngrams {
            let ref_count = reference_ngrams.get(ngram).copied().unwrap_or(0);
            clipped_count += (*count).min(ref_count);
            total_count += count;
        }

        if total_count == 0 {
            return 0.0;
        }

        match self.smoothing {
            SmoothingMethod::None => clipped_count as f64 / total_count as f64,
            SmoothingMethod::Add1 => (clipped_count as f64 + 1.0) / (total_count as f64 + 1.0),
            SmoothingMethod::AddK(k) => (clipped_count as f64 + k) / (total_count as f64 + k),
            SmoothingMethod::Exponential => {
                if clipped_count == 0 {
                    1.0 / (2_f64.powi(self.max_ngram as i32 - n as i32 + 1))
                } else {
                    clipped_count as f64 / total_count as f64
                }
            }
        }
    }

    fn brevity_penalty(&self, candidate_len: usize, reference_len: usize) -> f64 {
        if candidate_len >= reference_len {
            1.0
        } else {
            (1.0 - reference_len as f64 / candidate_len as f64).exp()
        }
    }
}

#[async_trait]
impl Metric for BleuMetric {
    fn name(&self) -> &str {
        "bleu"
    }

    fn version(&self) -> SemanticVersion {
        SemanticVersion::new(1, 0, 0)
    }

    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError> {
        let start = std::time::Instant::now();

        let expected = input.expected.ok_or(MetricError::MissingReference {
            metric: "bleu".to_string(),
        })?;

        let candidate_tokens = self.tokenize(&input.response);
        let reference_tokens = self.tokenize(&expected);

        if candidate_tokens.is_empty() || reference_tokens.is_empty() {
            return Ok(MetricOutput {
                value: MetricValue::Scalar(0.0),
                confidence: Some(1.0),
                breakdown: None,
                explanation: Some("Empty input".to_string()),
                computation_time_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Compute modified precisions for each n-gram level
        let mut log_precisions = 0.0;
        let mut breakdown = HashMap::new();

        for n in 1..=self.max_ngram {
            let precision = self.modified_precision(&candidate_tokens, &reference_tokens, n);
            breakdown.insert(format!("precision_{}", n), precision);

            if precision > 0.0 {
                log_precisions += precision.ln() / self.max_ngram as f64;
            } else {
                log_precisions = f64::NEG_INFINITY;
                break;
            }
        }

        let brevity_penalty =
            self.brevity_penalty(candidate_tokens.len(), reference_tokens.len());
        breakdown.insert("brevity_penalty".to_string(), brevity_penalty);

        let bleu = if log_precisions.is_finite() {
            brevity_penalty * log_precisions.exp()
        } else {
            0.0
        };

        Ok(MetricOutput {
            value: MetricValue::Scalar(bleu),
            confidence: Some(1.0),
            breakdown: Some(breakdown),
            explanation: Some(format!(
                "BLEU-{} score: {:.4} (BP: {:.4})",
                self.max_ngram, bleu, brevity_penalty
            )),
            computation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn properties(&self) -> MetricProperties {
        MetricProperties {
            metric_type: MetricType::Quality,
            value_range: Some(ValueRange {
                min: 0.0,
                max: 1.0,
                inclusive: true,
            }),
            higher_is_better: true,
            requires_reference: true,
            supports_batch: true,
            deterministic: true,
            aggregation_method: AggregationMethod::Mean,
            tags: vec!["similarity".to_string(), "translation".to_string()],
        }
    }
}

/// ROUGE metric for summarization evaluation
pub struct RougeMetric {
    rouge_type: RougeType,
}

#[derive(Debug, Clone, Copy)]
pub enum RougeType {
    RougeL,  // Longest common subsequence
    Rouge1,  // Unigram overlap
    Rouge2,  // Bigram overlap
    RougeN(usize),
}

impl RougeMetric {
    pub fn new(rouge_type: RougeType) -> Self {
        Self { rouge_type }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect()
    }

    fn lcs_length(&self, a: &[String], b: &[String]) -> usize {
        let m = a.len();
        let n = b.len();
        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        dp[m][n]
    }

    fn ngram_overlap(&self, candidate: &[String], reference: &[String], n: usize) -> (f64, f64, f64) {
        let get_ngrams = |tokens: &[String]| -> HashMap<Vec<String>, usize> {
            let mut ngrams = HashMap::new();
            if tokens.len() >= n {
                for window in tokens.windows(n) {
                    *ngrams.entry(window.to_vec()).or_insert(0) += 1;
                }
            }
            ngrams
        };

        let candidate_ngrams = get_ngrams(candidate);
        let reference_ngrams = get_ngrams(reference);

        let mut overlap = 0;
        for (ngram, count) in &candidate_ngrams {
            if let Some(&ref_count) = reference_ngrams.get(ngram) {
                overlap += (*count).min(ref_count);
            }
        }

        let candidate_total: usize = candidate_ngrams.values().sum();
        let reference_total: usize = reference_ngrams.values().sum();

        let precision = if candidate_total > 0 {
            overlap as f64 / candidate_total as f64
        } else {
            0.0
        };

        let recall = if reference_total > 0 {
            overlap as f64 / reference_total as f64
        } else {
            0.0
        };

        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        (precision, recall, f1)
    }
}

#[async_trait]
impl Metric for RougeMetric {
    fn name(&self) -> &str {
        match self.rouge_type {
            RougeType::RougeL => "rouge_l",
            RougeType::Rouge1 => "rouge_1",
            RougeType::Rouge2 => "rouge_2",
            RougeType::RougeN(n) => "rouge_n",
        }
    }

    fn version(&self) -> SemanticVersion {
        SemanticVersion::new(1, 0, 0)
    }

    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError> {
        let start = std::time::Instant::now();

        let expected = input.expected.ok_or(MetricError::MissingReference {
            metric: self.name().to_string(),
        })?;

        let candidate_tokens = self.tokenize(&input.response);
        let reference_tokens = self.tokenize(&expected);

        if candidate_tokens.is_empty() || reference_tokens.is_empty() {
            return Ok(MetricOutput {
                value: MetricValue::Scalar(0.0),
                confidence: Some(1.0),
                breakdown: None,
                explanation: Some("Empty input".to_string()),
                computation_time_ms: start.elapsed().as_millis() as u64,
            });
        }

        let (precision, recall, f1) = match self.rouge_type {
            RougeType::RougeL => {
                let lcs = self.lcs_length(&candidate_tokens, &reference_tokens);
                let p = lcs as f64 / candidate_tokens.len() as f64;
                let r = lcs as f64 / reference_tokens.len() as f64;
                let f = if p + r > 0.0 { 2.0 * p * r / (p + r) } else { 0.0 };
                (p, r, f)
            }
            RougeType::Rouge1 => self.ngram_overlap(&candidate_tokens, &reference_tokens, 1),
            RougeType::Rouge2 => self.ngram_overlap(&candidate_tokens, &reference_tokens, 2),
            RougeType::RougeN(n) => self.ngram_overlap(&candidate_tokens, &reference_tokens, n),
        };

        let mut breakdown = HashMap::new();
        breakdown.insert("precision".to_string(), precision);
        breakdown.insert("recall".to_string(), recall);
        breakdown.insert("f1".to_string(), f1);

        Ok(MetricOutput {
            value: MetricValue::Scalar(f1),
            confidence: Some(1.0),
            breakdown: Some(breakdown),
            explanation: Some(format!(
                "{}: F1={:.4}, P={:.4}, R={:.4}",
                self.name().to_uppercase(),
                f1,
                precision,
                recall
            )),
            computation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn properties(&self) -> MetricProperties {
        MetricProperties {
            metric_type: MetricType::Quality,
            value_range: Some(ValueRange {
                min: 0.0,
                max: 1.0,
                inclusive: true,
            }),
            higher_is_better: true,
            requires_reference: true,
            supports_batch: true,
            deterministic: true,
            aggregation_method: AggregationMethod::Mean,
            tags: vec!["similarity".to_string(), "summarization".to_string()],
        }
    }
}

/// Latency metric for performance evaluation
pub struct LatencyMetric {
    unit: TimeUnit,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeUnit {
    Milliseconds,
    Seconds,
}

#[async_trait]
impl Metric for LatencyMetric {
    fn name(&self) -> &str {
        "latency"
    }

    fn version(&self) -> SemanticVersion {
        SemanticVersion::new(1, 0, 0)
    }

    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError> {
        let start = std::time::Instant::now();

        let latency_ms = input.latency_ms.ok_or(MetricError::MissingField {
            field: "latency_ms".to_string(),
        })?;

        let value = match self.unit {
            TimeUnit::Milliseconds => latency_ms as f64,
            TimeUnit::Seconds => latency_ms as f64 / 1000.0,
        };

        Ok(MetricOutput {
            value: MetricValue::Scalar(value),
            confidence: Some(1.0),
            breakdown: None,
            explanation: Some(format!("Latency: {:.2}ms", latency_ms)),
            computation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn properties(&self) -> MetricProperties {
        MetricProperties {
            metric_type: MetricType::Performance,
            value_range: Some(ValueRange {
                min: 0.0,
                max: f64::MAX,
                inclusive: true,
            }),
            higher_is_better: false, // Lower latency is better
            requires_reference: false,
            supports_batch: true,
            deterministic: true,
            aggregation_method: AggregationMethod::Mean,
            tags: vec!["performance".to_string(), "latency".to_string()],
        }
    }
}

/// Token cost metric
pub struct TokenCostMetric {
    cost_per_input_token: f64,
    cost_per_output_token: f64,
}

impl TokenCostMetric {
    pub fn new(cost_per_input_token: f64, cost_per_output_token: f64) -> Self {
        Self {
            cost_per_input_token,
            cost_per_output_token,
        }
    }
}

#[async_trait]
impl Metric for TokenCostMetric {
    fn name(&self) -> &str {
        "token_cost"
    }

    fn version(&self) -> SemanticVersion {
        SemanticVersion::new(1, 0, 0)
    }

    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError> {
        let start = std::time::Instant::now();

        let tokens = input.token_counts.ok_or(MetricError::MissingField {
            field: "token_counts".to_string(),
        })?;

        let input_cost = tokens.prompt_tokens as f64 * self.cost_per_input_token;
        let output_cost = tokens.completion_tokens as f64 * self.cost_per_output_token;
        let total_cost = input_cost + output_cost;

        let mut breakdown = HashMap::new();
        breakdown.insert("input_cost".to_string(), input_cost);
        breakdown.insert("output_cost".to_string(), output_cost);
        breakdown.insert("input_tokens".to_string(), tokens.prompt_tokens as f64);
        breakdown.insert("output_tokens".to_string(), tokens.completion_tokens as f64);

        Ok(MetricOutput {
            value: MetricValue::Scalar(total_cost),
            confidence: Some(1.0),
            breakdown: Some(breakdown),
            explanation: Some(format!(
                "Cost: ${:.6} ({} input + {} output tokens)",
                total_cost, tokens.prompt_tokens, tokens.completion_tokens
            )),
            computation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn properties(&self) -> MetricProperties {
        MetricProperties {
            metric_type: MetricType::Cost,
            value_range: Some(ValueRange {
                min: 0.0,
                max: f64::MAX,
                inclusive: true,
            }),
            higher_is_better: false,
            requires_reference: false,
            supports_batch: true,
            deterministic: true,
            aggregation_method: AggregationMethod::Sum,
            tags: vec!["cost".to_string(), "tokens".to_string()],
        }
    }
}

/// LLM-as-judge metric for subjective evaluation
pub struct LlmJudgeMetric {
    judge_client: Arc<dyn LlmClient>,
    evaluation_prompt: String,
    scoring_rubric: ScoringRubric,
    model_config: ModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRubric {
    pub criteria: Vec<EvaluationCriterion>,
    pub scale: ScoreScale,
    pub aggregation: AggregationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriterion {
    pub name: String,
    pub description: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreScale {
    pub min: f64,
    pub max: f64,
    pub labels: Option<HashMap<i32, String>>, // e.g., 1: "Poor", 5: "Excellent"
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str, config: &ModelConfig) -> Result<LlmResponse, MetricError>;
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub usage: TokenCounts,
    pub latency_ms: u64,
}

impl LlmJudgeMetric {
    pub fn new(
        judge_client: Arc<dyn LlmClient>,
        evaluation_prompt: String,
        scoring_rubric: ScoringRubric,
        model_config: ModelConfig,
    ) -> Self {
        Self {
            judge_client,
            evaluation_prompt,
            scoring_rubric,
            model_config,
        }
    }

    fn build_judge_prompt(&self, input: &MetricInput) -> String {
        let criteria_text = self
            .scoring_rubric
            .criteria
            .iter()
            .map(|c| format!("- {}: {}", c.name, c.description))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "{}\n\nCriteria:\n{}\n\nPrompt: {}\n\nResponse: {}\n\nProvide scores as JSON.",
            self.evaluation_prompt, criteria_text, input.prompt, input.response
        )
    }

    fn parse_scores(&self, response: &str) -> Result<HashMap<String, f64>, MetricError> {
        // Extract JSON from response
        let json_start = response.find('{').ok_or(MetricError::ParseError {
            message: "No JSON found in judge response".to_string(),
        })?;
        let json_end = response.rfind('}').ok_or(MetricError::ParseError {
            message: "No JSON found in judge response".to_string(),
        })? + 1;

        let json_str = &response[json_start..json_end];
        let scores: HashMap<String, f64> =
            serde_json::from_str(json_str).map_err(|e| MetricError::ParseError {
                message: format!("Failed to parse scores: {}", e),
            })?;

        Ok(scores)
    }
}

#[async_trait]
impl Metric for LlmJudgeMetric {
    fn name(&self) -> &str {
        "llm_judge"
    }

    fn version(&self) -> SemanticVersion {
        SemanticVersion::new(1, 0, 0)
    }

    async fn compute(&self, input: MetricInput) -> Result<MetricOutput, MetricError> {
        let start = std::time::Instant::now();

        let judge_prompt = self.build_judge_prompt(&input);
        let response = self.judge_client.complete(&judge_prompt, &self.model_config).await?;

        let scores = self.parse_scores(&response.content)?;

        // Compute weighted average
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for criterion in &self.scoring_rubric.criteria {
            if let Some(&score) = scores.get(&criterion.name) {
                // Normalize to 0-1 scale
                let normalized = (score - self.scoring_rubric.scale.min)
                    / (self.scoring_rubric.scale.max - self.scoring_rubric.scale.min);
                weighted_sum += normalized * criterion.weight;
                total_weight += criterion.weight;
            }
        }

        let final_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        Ok(MetricOutput {
            value: MetricValue::Scalar(final_score),
            confidence: Some(0.8), // LLM judgments have some uncertainty
            breakdown: Some(scores),
            explanation: Some(format!("LLM judge score: {:.2}", final_score)),
            computation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn properties(&self) -> MetricProperties {
        MetricProperties {
            metric_type: MetricType::Quality,
            value_range: Some(ValueRange {
                min: 0.0,
                max: 1.0,
                inclusive: true,
            }),
            higher_is_better: true,
            requires_reference: false,
            supports_batch: false,
            deterministic: false,
            aggregation_method: AggregationMethod::Mean,
            tags: vec!["subjective".to_string(), "llm-judge".to_string()],
        }
    }
}
```

### 3.2 Benchmark Runner

```rust
/// Benchmark runner for executing metric evaluations
pub struct BenchmarkRunner {
    metric_registry: Arc<MetricRegistry>,
    executor: Arc<BenchmarkExecutor>,
    result_store: Arc<dyn BenchmarkResultStore>,
    event_publisher: Arc<dyn EventPublisher>,
    config: BenchmarkConfig,
}

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub max_concurrent_evaluations: usize,
    pub timeout_per_evaluation: std::time::Duration,
    pub retry_policy: RetryPolicy,
    pub checkpoint_interval: Option<std::time::Duration>,
    pub progress_reporting: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            max_concurrent_evaluations: 10,
            timeout_per_evaluation: std::time::Duration::from_secs(60),
            retry_policy: RetryPolicy::default(),
            checkpoint_interval: Some(std::time::Duration::from_secs(300)),
            progress_reporting: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
    pub exponential_base: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(10),
            exponential_base: 2.0,
        }
    }
}

/// Benchmark suite definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub metrics: Vec<MetricConfig>,
    pub test_cases: Vec<TestCase>,
    pub aggregation_config: AggregationConfig,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: Uuid,
    pub name: Option<String>,
    pub prompt: String,
    pub expected: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    pub method: AggregationMethod,
    pub weighted_by_test_case: bool,
    pub include_confidence_intervals: bool,
    pub bootstrap_samples: Option<usize>,
}

/// Model reference for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub model_id: String,
    pub provider: ModelProvider,
    pub endpoint: Option<String>,
    pub parameters: ModelParameters,
}

impl BenchmarkRunner {
    pub fn new(
        metric_registry: Arc<MetricRegistry>,
        executor: Arc<BenchmarkExecutor>,
        result_store: Arc<dyn BenchmarkResultStore>,
        event_publisher: Arc<dyn EventPublisher>,
        config: BenchmarkConfig,
    ) -> Self {
        Self {
            metric_registry,
            executor,
            result_store,
            event_publisher,
            config,
        }
    }

    /// Run a benchmark suite against a model
    pub async fn run_benchmark(
        &self,
        model: ModelRef,
        suite: BenchmarkSuite,
        run_id: Option<RunId>,
    ) -> Result<BenchmarkResults, MetricError> {
        let benchmark_id = Uuid::new_v4();
        let start_time = Utc::now();

        // Publish start event
        self.event_publisher
            .publish(Event::BenchmarkStarted {
                benchmark_id,
                model_id: model.model_id.clone(),
                suite_name: suite.name.clone(),
                timestamp: start_time,
            })
            .await
            .ok();

        // Resolve metrics from registry
        let metrics = self.resolve_metrics(&suite.metrics).await?;

        // Create evaluation tasks
        let mut evaluation_results = Vec::with_capacity(suite.test_cases.len() * metrics.len());
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_evaluations,
        ));

        // Process test cases with concurrency control
        let mut tasks = Vec::new();

        for test_case in &suite.test_cases {
            for (metric_config, metric) in metrics.iter() {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let model = model.clone();
                let test_case = test_case.clone();
                let metric = metric.clone();
                let metric_config = metric_config.clone();
                let executor = self.executor.clone();
                let timeout = self.config.timeout_per_evaluation;
                let retry_policy = self.config.retry_policy.clone();

                let task = tokio::spawn(async move {
                    let result = Self::run_single_evaluation(
                        executor,
                        model,
                        test_case,
                        metric,
                        metric_config,
                        timeout,
                        retry_policy,
                    )
                    .await;
                    drop(permit);
                    result
                });

                tasks.push(task);
            }
        }

        // Collect results
        for task in tasks {
            match task.await {
                Ok(Ok(result)) => evaluation_results.push(result),
                Ok(Err(e)) => {
                    tracing::warn!("Evaluation failed: {}", e);
                    // Continue with other evaluations
                }
                Err(e) => {
                    tracing::error!("Task panicked: {}", e);
                }
            }
        }

        // Aggregate results
        let aggregated = self.aggregate_results(&evaluation_results, &suite.aggregation_config)?;

        let end_time = Utc::now();
        let results = BenchmarkResults {
            id: benchmark_id,
            model: model.clone(),
            suite_id: suite.id,
            suite_name: suite.name.clone(),
            run_id,
            individual_results: evaluation_results,
            aggregated_metrics: aggregated.metrics,
            statistical_summary: aggregated.summary,
            started_at: start_time,
            completed_at: end_time,
            duration: end_time - start_time,
        };

        // Store results
        self.result_store.save(&results).await?;

        // Publish completion event
        self.event_publisher
            .publish(Event::BenchmarkCompleted {
                benchmark_id,
                model_id: model.model_id,
                duration: results.duration,
                timestamp: end_time,
            })
            .await
            .ok();

        Ok(results)
    }

    /// Compare multiple models on the same benchmark
    pub async fn compare_models(
        &self,
        models: Vec<ModelRef>,
        suite: BenchmarkSuite,
    ) -> Result<ModelComparison, MetricError> {
        if models.len() < 2 {
            return Err(MetricError::ValidationError {
                message: "At least 2 models required for comparison".to_string(),
            });
        }

        let mut model_results = Vec::with_capacity(models.len());

        for model in models {
            let results = self.run_benchmark(model, suite.clone(), None).await?;
            model_results.push(results);
        }

        // Build comparison matrix
        let metric_names: Vec<_> = model_results
            .first()
            .map(|r| r.aggregated_metrics.keys().cloned().collect())
            .unwrap_or_default();

        let mut comparison_matrix = HashMap::new();

        for metric_name in &metric_names {
            let values: Vec<_> = model_results
                .iter()
                .map(|r| r.aggregated_metrics.get(metric_name).cloned())
                .collect();

            // Run pairwise statistical tests
            let mut pairwise_tests = Vec::new();
            for i in 0..model_results.len() {
                for j in (i + 1)..model_results.len() {
                    if let (Some(a), Some(b)) = (&values[i], &values[j]) {
                        let test = self.run_significance_test(
                            &model_results[i],
                            &model_results[j],
                            metric_name,
                        )?;
                        pairwise_tests.push(PairwiseTest {
                            model_a_index: i,
                            model_b_index: j,
                            test_result: test,
                        });
                    }
                }
            }

            comparison_matrix.insert(
                metric_name.clone(),
                MetricComparisonEntry {
                    values,
                    pairwise_tests,
                    best_model_index: None, // Computed after all tests
                },
            );
        }

        // Determine winners for each metric
        for (metric_name, entry) in &mut comparison_matrix {
            let properties = self
                .metric_registry
                .get_properties(metric_name)
                .await
                .ok();

            let higher_is_better = properties
                .map(|p| p.higher_is_better)
                .unwrap_or(true);

            entry.best_model_index = entry
                .values
                .iter()
                .enumerate()
                .filter_map(|(i, v)| v.as_ref().map(|agg| (i, agg.mean)))
                .max_by(|(_, a), (_, b)| {
                    if higher_is_better {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)
                    }
                })
                .map(|(i, _)| i);
        }

        Ok(ModelComparison {
            models: model_results.iter().map(|r| r.model.clone()).collect(),
            suite_id: suite.id,
            suite_name: suite.name,
            comparison_matrix,
            generated_at: Utc::now(),
        })
    }

    async fn run_single_evaluation(
        executor: Arc<BenchmarkExecutor>,
        model: ModelRef,
        test_case: TestCase,
        metric: Arc<dyn Metric>,
        metric_config: MetricConfig,
        timeout: std::time::Duration,
        retry_policy: RetryPolicy,
    ) -> Result<EvaluationResult, MetricError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= retry_policy.max_retries {
            let result = tokio::time::timeout(timeout, async {
                // Get model response
                let response = executor.get_model_response(&model, &test_case).await?;

                // Build metric input
                let input = MetricInput {
                    prompt: test_case.prompt.clone(),
                    response: response.content.clone(),
                    expected: test_case.expected.clone(),
                    context: test_case.context.clone(),
                    model_info: Some(ModelInfo {
                        provider: model.provider.to_string(),
                        model_id: model.model_id.clone(),
                        parameters: HashMap::new(),
                    }),
                    latency_ms: Some(response.latency_ms),
                    token_counts: Some(response.usage),
                };

                // Compute metric
                let output = metric.compute(input).await?;

                Ok::<_, MetricError>(EvaluationResult {
                    test_case_id: test_case.id,
                    metric_name: metric.name().to_string(),
                    metric_version: metric.version(),
                    output,
                    model_response: response.content,
                    evaluated_at: Utc::now(),
                })
            })
            .await;

            match result {
                Ok(Ok(eval_result)) => return Ok(eval_result),
                Ok(Err(e)) => {
                    last_error = Some(e);
                    attempts += 1;
                }
                Err(_) => {
                    last_error = Some(MetricError::Timeout);
                    attempts += 1;
                }
            }

            if attempts <= retry_policy.max_retries {
                let delay = std::cmp::min(
                    retry_policy.initial_delay
                        * (retry_policy.exponential_base.powi(attempts as i32 - 1) as u32),
                    retry_policy.max_delay,
                );
                tokio::time::sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or(MetricError::Unknown))
    }

    async fn resolve_metrics(
        &self,
        configs: &[MetricConfig],
    ) -> Result<Vec<(MetricConfig, Arc<dyn Metric>)>, MetricError> {
        let mut resolved = Vec::with_capacity(configs.len());

        for config in configs {
            let metric = self
                .metric_registry
                .get(config.metric_id, &config.version)
                .await?;
            resolved.push((config.clone(), metric));
        }

        Ok(resolved)
    }

    fn aggregate_results(
        &self,
        results: &[EvaluationResult],
        config: &AggregationConfig,
    ) -> Result<AggregatedResults, MetricError> {
        let mut metrics_by_name: HashMap<String, Vec<f64>> = HashMap::new();

        for result in results {
            if let MetricValue::Scalar(value) = result.output.value {
                metrics_by_name
                    .entry(result.metric_name.clone())
                    .or_default()
                    .push(value);
            }
        }

        let mut aggregated_metrics = HashMap::new();

        for (name, values) in &metrics_by_name {
            let agg = compute_aggregations(&values, config)?;
            aggregated_metrics.insert(name.clone(), agg);
        }

        let summary = StatisticalSummary {
            total_evaluations: results.len(),
            successful_evaluations: results.len(), // TODO: track failures
            failed_evaluations: 0,
            metrics_computed: aggregated_metrics.len(),
        };

        Ok(AggregatedResults {
            metrics: aggregated_metrics,
            summary,
        })
    }

    fn run_significance_test(
        &self,
        results_a: &BenchmarkResults,
        results_b: &BenchmarkResults,
        metric_name: &str,
    ) -> Result<StatisticalTestResult, MetricError> {
        let values_a: Vec<f64> = results_a
            .individual_results
            .iter()
            .filter(|r| r.metric_name == metric_name)
            .filter_map(|r| match r.output.value {
                MetricValue::Scalar(v) => Some(v),
                _ => None,
            })
            .collect();

        let values_b: Vec<f64> = results_b
            .individual_results
            .iter()
            .filter(|r| r.metric_name == metric_name)
            .filter_map(|r| match r.output.value {
                MetricValue::Scalar(v) => Some(v),
                _ => None,
            })
            .collect();

        // Perform Mann-Whitney U test (non-parametric)
        let test_result = mann_whitney_u_test(&values_a, &values_b)?;

        Ok(test_result)
    }
}

/// Benchmark executor for getting model responses
pub struct BenchmarkExecutor {
    clients: HashMap<String, Arc<dyn LlmClient>>,
}

impl BenchmarkExecutor {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn register_client(&mut self, provider: &str, client: Arc<dyn LlmClient>) {
        self.clients.insert(provider.to_string(), client);
    }

    pub async fn get_model_response(
        &self,
        model: &ModelRef,
        test_case: &TestCase,
    ) -> Result<LlmResponse, MetricError> {
        let client = self
            .clients
            .get(&model.provider.to_string())
            .ok_or_else(|| MetricError::ProviderNotFound {
                provider: model.provider.to_string(),
            })?;

        let config = ModelConfig {
            model_id: model.model_id.clone(),
            provider: model.provider.clone(),
            variant: None,
            endpoint: model.endpoint.clone(),
            parameters: model.parameters.clone(),
            credentials_ref: None,
        };

        client.complete(&test_case.prompt, &config).await
    }
}

/// Result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub id: Uuid,
    pub model: ModelRef,
    pub suite_id: Uuid,
    pub suite_name: String,
    pub run_id: Option<RunId>,
    pub individual_results: Vec<EvaluationResult>,
    pub aggregated_metrics: HashMap<String, AggregatedMetric>,
    pub statistical_summary: StatisticalSummary,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration: chrono::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub test_case_id: Uuid,
    pub metric_name: String,
    pub metric_version: SemanticVersion,
    pub output: MetricOutput,
    pub model_response: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub mean: f64,
    pub std: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
    pub confidence_interval: Option<(f64, f64)>,
    pub percentiles: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalSummary {
    pub total_evaluations: usize,
    pub successful_evaluations: usize,
    pub failed_evaluations: usize,
    pub metrics_computed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelComparison {
    pub models: Vec<ModelRef>,
    pub suite_id: Uuid,
    pub suite_name: String,
    pub comparison_matrix: HashMap<String, MetricComparisonEntry>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparisonEntry {
    pub values: Vec<Option<AggregatedMetric>>,
    pub pairwise_tests: Vec<PairwiseTest>,
    pub best_model_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseTest {
    pub model_a_index: usize,
    pub model_b_index: usize,
    pub test_result: StatisticalTestResult,
}

struct AggregatedResults {
    metrics: HashMap<String, AggregatedMetric>,
    summary: StatisticalSummary,
}

fn compute_aggregations(
    values: &[f64],
    config: &AggregationConfig,
) -> Result<AggregatedMetric, MetricError> {
    if values.is_empty() {
        return Err(MetricError::InsufficientData);
    }

    let n = values.len();
    let sum: f64 = values.iter().sum();
    let mean = sum / n as f64;

    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std = variance.sqrt();

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };

    let percentiles = compute_percentiles(&sorted);

    let confidence_interval = if config.include_confidence_intervals {
        let bootstrap_samples = config.bootstrap_samples.unwrap_or(1000);
        Some(bootstrap_confidence_interval(values, bootstrap_samples, 0.95)?)
    } else {
        None
    };

    Ok(AggregatedMetric {
        mean,
        std,
        median,
        min: *sorted.first().unwrap(),
        max: *sorted.last().unwrap(),
        count: n,
        confidence_interval,
        percentiles,
    })
}

fn compute_percentiles(sorted: &[f64]) -> HashMap<String, f64> {
    let mut percentiles = HashMap::new();
    let n = sorted.len();

    for p in [25, 50, 75, 90, 95, 99] {
        let idx = (p as f64 / 100.0 * n as f64).ceil() as usize - 1;
        let idx = idx.min(n - 1);
        percentiles.insert(format!("p{}", p), sorted[idx]);
    }

    percentiles
}

fn bootstrap_confidence_interval(
    values: &[f64],
    n_samples: usize,
    confidence: f64,
) -> Result<(f64, f64), MetricError> {
    use rand::prelude::*;

    let mut rng = rand::thread_rng();
    let n = values.len();
    let mut bootstrap_means = Vec::with_capacity(n_samples);

    for _ in 0..n_samples {
        let sample_sum: f64 = (0..n)
            .map(|_| values[rng.gen_range(0..n)])
            .sum();
        bootstrap_means.push(sample_sum / n as f64);
    }

    bootstrap_means.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let alpha = 1.0 - confidence;
    let lower_idx = (alpha / 2.0 * n_samples as f64).floor() as usize;
    let upper_idx = ((1.0 - alpha / 2.0) * n_samples as f64).ceil() as usize - 1;

    Ok((bootstrap_means[lower_idx], bootstrap_means[upper_idx]))
}

fn mann_whitney_u_test(a: &[f64], b: &[f64]) -> Result<StatisticalTestResult, MetricError> {
    // Combine and rank
    let mut combined: Vec<(f64, bool)> = a.iter().map(|&v| (v, true)).collect();
    combined.extend(b.iter().map(|&v| (v, false)));
    combined.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());

    // Assign ranks (handling ties)
    let mut ranks: Vec<(f64, bool)> = Vec::with_capacity(combined.len());
    let mut i = 0;
    while i < combined.len() {
        let mut j = i;
        while j < combined.len() && combined[j].0 == combined[i].0 {
            j += 1;
        }
        let avg_rank = (i + j + 1) as f64 / 2.0;
        for k in i..j {
            ranks.push((avg_rank, combined[k].1));
        }
        i = j;
    }

    // Calculate U statistic
    let r1: f64 = ranks.iter().filter(|(_, is_a)| *is_a).map(|(r, _)| r).sum();
    let n1 = a.len() as f64;
    let n2 = b.len() as f64;
    let u1 = r1 - (n1 * (n1 + 1.0)) / 2.0;
    let u2 = n1 * n2 - u1;
    let u = u1.min(u2);

    // Normal approximation for large samples
    let mean_u = n1 * n2 / 2.0;
    let std_u = ((n1 * n2 * (n1 + n2 + 1.0)) / 12.0).sqrt();
    let z = (u - mean_u) / std_u;

    // Two-tailed p-value (using normal approximation)
    let p_value = 2.0 * (1.0 - normal_cdf(z.abs()));

    // Effect size (rank-biserial correlation)
    let effect_size = 1.0 - (2.0 * u) / (n1 * n2);

    Ok(StatisticalTestResult {
        test_type: StatisticalTest::WilcoxonRankSum,
        statistic: u,
        p_value,
        significant: p_value < 0.05,
        effect_size: Some(effect_size),
        confidence_interval: None,
    })
}

fn normal_cdf(x: f64) -> f64 {
    // Approximation of the standard normal CDF
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

fn erf(x: f64) -> f64 {
    // Approximation of the error function
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}
```

### 3.3 Metric Registry

```rust
/// Registry for managing metric definitions
pub struct MetricRegistry {
    metrics: Arc<RwLock<HashMap<MetricId, Vec<RegisteredMetric>>>>,
    store: Arc<dyn MetricDefinitionStore>,
}

#[derive(Clone)]
struct RegisteredMetric {
    id: MetricId,
    version: SemanticVersion,
    definition: MetricDefinition,
    implementation: Arc<dyn Metric>,
    registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub id: MetricId,
    pub name: String,
    pub description: String,
    pub version: SemanticVersion,
    pub properties: MetricProperties,
    pub parameters_schema: Option<serde_json::Value>,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub deprecated: bool,
    pub deprecation_message: Option<String>,
}

#[async_trait]
pub trait MetricDefinitionStore: Send + Sync {
    async fn save(&self, definition: &MetricDefinition) -> Result<(), MetricError>;
    async fn get(&self, id: MetricId, version: &SemanticVersion) -> Result<MetricDefinition, MetricError>;
    async fn list(&self, filter: MetricFilter) -> Result<Vec<MetricDefinition>, MetricError>;
    async fn get_latest_version(&self, id: MetricId) -> Result<MetricDefinition, MetricError>;
}

#[derive(Debug, Clone, Default)]
pub struct MetricFilter {
    pub metric_type: Option<MetricType>,
    pub tags: Option<Vec<String>>,
    pub include_deprecated: bool,
}

impl MetricRegistry {
    pub fn new(store: Arc<dyn MetricDefinitionStore>) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            store,
        }
    }

    /// Register a new metric implementation
    pub async fn register(
        &self,
        definition: MetricDefinition,
        implementation: Arc<dyn Metric>,
    ) -> Result<(), MetricError> {
        // Validate implementation matches definition
        if implementation.name() != definition.name {
            return Err(MetricError::ValidationError {
                message: format!(
                    "Implementation name '{}' doesn't match definition name '{}'",
                    implementation.name(),
                    definition.name
                ),
            });
        }

        if implementation.version() != definition.version {
            return Err(MetricError::ValidationError {
                message: format!(
                    "Implementation version '{}' doesn't match definition version '{}'",
                    implementation.version(),
                    definition.version
                ),
            });
        }

        // Save definition to store
        self.store.save(&definition).await?;

        // Register in memory
        let registered = RegisteredMetric {
            id: definition.id,
            version: definition.version.clone(),
            definition,
            implementation,
            registered_at: Utc::now(),
        };

        let mut metrics = self.metrics.write().await;
        metrics
            .entry(registered.id)
            .or_insert_with(Vec::new)
            .push(registered);

        Ok(())
    }

    /// Get a metric by ID and version
    pub async fn get(
        &self,
        id: MetricId,
        version: &SemanticVersion,
    ) -> Result<Arc<dyn Metric>, MetricError> {
        let metrics = self.metrics.read().await;

        let versions = metrics.get(&id).ok_or(MetricError::NotFound {
            metric: format!("{:?}", id),
        })?;

        let registered = versions
            .iter()
            .find(|m| m.version == *version)
            .ok_or(MetricError::VersionNotFound {
                metric: format!("{:?}", id),
                version: version.to_string(),
            })?;

        Ok(registered.implementation.clone())
    }

    /// Get the latest version of a metric
    pub async fn get_latest(&self, id: MetricId) -> Result<Arc<dyn Metric>, MetricError> {
        let metrics = self.metrics.read().await;

        let versions = metrics.get(&id).ok_or(MetricError::NotFound {
            metric: format!("{:?}", id),
        })?;

        let registered = versions
            .iter()
            .max_by(|a, b| a.version.cmp(&b.version))
            .ok_or(MetricError::NotFound {
                metric: format!("{:?}", id),
            })?;

        Ok(registered.implementation.clone())
    }

    /// Get metric properties
    pub async fn get_properties(&self, name: &str) -> Result<MetricProperties, MetricError> {
        let metrics = self.metrics.read().await;

        for versions in metrics.values() {
            if let Some(registered) = versions.iter().find(|m| m.definition.name == name) {
                return Ok(registered.definition.properties.clone());
            }
        }

        Err(MetricError::NotFound {
            metric: name.to_string(),
        })
    }

    /// List all registered metrics
    pub async fn list(&self, filter: MetricFilter) -> Result<Vec<MetricDefinition>, MetricError> {
        self.store.list(filter).await
    }

    /// Register built-in metrics
    pub async fn register_builtins(&self, user_id: UserId) -> Result<(), MetricError> {
        // Exact match
        let exact_match = ExactMatchMetric::new(false, true);
        self.register(
            MetricDefinition {
                id: MetricId(Uuid::new_v4()),
                name: "exact_match".to_string(),
                description: "Exact string match between response and expected".to_string(),
                version: SemanticVersion::new(1, 0, 0),
                properties: exact_match.properties(),
                parameters_schema: None,
                created_by: user_id,
                created_at: Utc::now(),
                deprecated: false,
                deprecation_message: None,
            },
            Arc::new(exact_match),
        )
        .await?;

        // BLEU
        let bleu = BleuMetric::new(4, SmoothingMethod::Add1);
        self.register(
            MetricDefinition {
                id: MetricId(Uuid::new_v4()),
                name: "bleu".to_string(),
                description: "BLEU score for text similarity".to_string(),
                version: SemanticVersion::new(1, 0, 0),
                properties: bleu.properties(),
                parameters_schema: None,
                created_by: user_id,
                created_at: Utc::now(),
                deprecated: false,
                deprecation_message: None,
            },
            Arc::new(bleu),
        )
        .await?;

        // ROUGE-L
        let rouge_l = RougeMetric::new(RougeType::RougeL);
        self.register(
            MetricDefinition {
                id: MetricId(Uuid::new_v4()),
                name: "rouge_l".to_string(),
                description: "ROUGE-L score using longest common subsequence".to_string(),
                version: SemanticVersion::new(1, 0, 0),
                properties: rouge_l.properties(),
                parameters_schema: None,
                created_by: user_id,
                created_at: Utc::now(),
                deprecated: false,
                deprecation_message: None,
            },
            Arc::new(rouge_l),
        )
        .await?;

        // Latency
        let latency = LatencyMetric {
            unit: TimeUnit::Milliseconds,
        };
        self.register(
            MetricDefinition {
                id: MetricId(Uuid::new_v4()),
                name: "latency".to_string(),
                description: "Response latency in milliseconds".to_string(),
                version: SemanticVersion::new(1, 0, 0),
                properties: latency.properties(),
                parameters_schema: None,
                created_by: user_id,
                created_at: Utc::now(),
                deprecated: false,
                deprecation_message: None,
            },
            Arc::new(latency),
        )
        .await?;

        Ok(())
    }
}

/// Metric error types
#[derive(Debug, thiserror::Error)]
pub enum MetricError {
    #[error("Metric not found: {metric}")]
    NotFound { metric: String },

    #[error("Version not found for metric {metric}: {version}")]
    VersionNotFound { metric: String, version: String },

    #[error("Missing reference for metric {metric}")]
    MissingReference { metric: String },

    #[error("Missing field: {field}")]
    MissingField { field: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Provider not found: {provider}")]
    ProviderNotFound { provider: String },

    #[error("Computation timeout")]
    Timeout,

    #[error("Insufficient data for computation")]
    InsufficientData,

    #[error("Storage error: {0}")]
    Storage(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Unknown error")]
    Unknown,
}

#[async_trait]
pub trait BenchmarkResultStore: Send + Sync {
    async fn save(&self, results: &BenchmarkResults) -> Result<(), MetricError>;
    async fn get(&self, id: Uuid) -> Result<BenchmarkResults, MetricError>;
    async fn list_for_model(&self, model_id: &str) -> Result<Vec<BenchmarkResults>, MetricError>;
    async fn list_for_suite(&self, suite_id: Uuid) -> Result<Vec<BenchmarkResults>, MetricError>;
}

impl std::fmt::Display for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelProvider::OpenAI => write!(f, "openai"),
            ModelProvider::Anthropic => write!(f, "anthropic"),
            ModelProvider::Google => write!(f, "google"),
            ModelProvider::Cohere => write!(f, "cohere"),
            ModelProvider::HuggingFace => write!(f, "huggingface"),
            ModelProvider::AzureOpenAI => write!(f, "azure_openai"),
            ModelProvider::AWSBedrock => write!(f, "aws_bedrock"),
            ModelProvider::Custom { name } => write!(f, "custom:{}", name),
        }
    }
}

// Additional event types for benchmarking
impl Event {
    pub fn benchmark_started(
        benchmark_id: Uuid,
        model_id: String,
        suite_name: String,
    ) -> Self {
        Event::BenchmarkStarted {
            benchmark_id,
            model_id,
            suite_name,
            timestamp: Utc::now(),
        }
    }

    pub fn benchmark_completed(
        benchmark_id: Uuid,
        model_id: String,
        duration: chrono::Duration,
    ) -> Self {
        Event::BenchmarkCompleted {
            benchmark_id,
            model_id,
            duration,
            timestamp: Utc::now(),
        }
    }
}

// Extend Event enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // ... previous variants ...
    BenchmarkStarted {
        benchmark_id: Uuid,
        model_id: String,
        suite_name: String,
        timestamp: DateTime<Utc>,
    },
    BenchmarkCompleted {
        benchmark_id: Uuid,
        model_id: String,
        duration: chrono::Duration,
        timestamp: DateTime<Utc>,
    },
    BenchmarkFailed {
        benchmark_id: Uuid,
        model_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
}
```

---

## 4. Dataset Versioning System

### 4.1 Dataset Manager

```rust
//! Dataset versioning system with content-addressable storage
//! Provides immutable versioning, lineage tracking, and efficient storage

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Dataset manager for versioned dataset operations
pub struct DatasetManager {
    content_store: Arc<ContentStore>,
    metadata_store: Arc<dyn DatasetMetadataStore>,
    lineage_graph: Arc<RwLock<DatasetLineageGraph>>,
    vault_client: Arc<dyn DataVaultClient>,
    config: DatasetConfig,
}

#[derive(Debug, Clone)]
pub struct DatasetConfig {
    pub chunk_size: usize,
    pub compression: CompressionType,
    pub deduplication: bool,
    pub max_inline_size: usize,
    pub default_retention_days: Option<u32>,
}

impl Default for DatasetConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024 * 1024, // 64MB chunks
            compression: CompressionType::Zstd,
            deduplication: true,
            max_inline_size: 1024 * 1024, // 1MB inline threshold
            default_retention_days: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
    Snappy,
}

/// Dataset entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: DatasetId,
    pub name: String,
    pub description: Option<String>,
    pub schema: DatasetSchema,
    pub tags: Vec<String>,
    pub owner_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub governance: GovernanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSchema {
    pub format: DataFormat,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: Option<Vec<String>>,
    pub partition_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataFormat {
    Csv,
    Json,
    Jsonl,
    Parquet,
    Arrow,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub description: Option<String>,
    pub constraints: Vec<ColumnConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Timestamp,
    Date,
    Binary,
    Array(Box<DataType>),
    Map { key: Box<DataType>, value: Box<DataType> },
    Struct(Vec<ColumnDefinition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnConstraint {
    NotNull,
    Unique,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Range { min: Option<f64>, max: Option<f64> },
    Enum(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub classification: DataClassification,
    pub retention_days: Option<u32>,
    pub access_control: AccessControl,
    pub pii_columns: Vec<String>,
    pub audit_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    pub read_roles: Vec<String>,
    pub write_roles: Vec<String>,
    pub admin_roles: Vec<String>,
}

/// Dataset version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetVersion {
    pub id: DatasetVersionId,
    pub dataset_id: DatasetId,
    pub version_number: u64,
    pub content_hash: ContentHash,
    pub parent_version_id: Option<DatasetVersionId>,
    pub transform: Option<Transform>,
    pub statistics: DatasetStatistics,
    pub manifest: DataManifest,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub message: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStatistics {
    pub row_count: u64,
    pub size_bytes: u64,
    pub column_statistics: HashMap<String, ColumnStatistics>,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatistics {
    pub null_count: u64,
    pub distinct_count: Option<u64>,
    pub min_value: Option<serde_json::Value>,
    pub max_value: Option<serde_json::Value>,
    pub mean: Option<f64>,
    pub std: Option<f64>,
    pub histogram: Option<Histogram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataManifest {
    pub chunks: Vec<ChunkInfo>,
    pub total_size: u64,
    pub compression: CompressionType,
    pub format: DataFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub chunk_id: String,
    pub content_hash: ContentHash,
    pub offset: u64,
    pub size_bytes: u64,
    pub row_count: u64,
    pub storage_uri: String,
}

/// Transform that created a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub transform_type: TransformType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub source_versions: Vec<DatasetVersionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformType {
    Import,
    Filter,
    Sample,
    Split,
    Join,
    Aggregate,
    Augment,
    Clean,
    Custom(String),
}

impl DatasetManager {
    pub fn new(
        content_store: Arc<ContentStore>,
        metadata_store: Arc<dyn DatasetMetadataStore>,
        vault_client: Arc<dyn DataVaultClient>,
        config: DatasetConfig,
    ) -> Self {
        Self {
            content_store,
            metadata_store,
            lineage_graph: Arc::new(RwLock::new(DatasetLineageGraph::new())),
            vault_client,
            config,
        }
    }

    /// Register a new dataset
    pub async fn register_dataset(
        &self,
        request: RegisterDatasetRequest,
    ) -> Result<Dataset, DatasetError> {
        // Validate schema
        self.validate_schema(&request.schema)?;

        let id = DatasetId(Uuid::new_v4());
        let now = Utc::now();

        let dataset = Dataset {
            id,
            name: request.name,
            description: request.description,
            schema: request.schema,
            tags: request.tags.unwrap_or_default(),
            owner_id: request.owner_id,
            created_at: now,
            updated_at: now,
            metadata: request.metadata.unwrap_or_default(),
            governance: request.governance.unwrap_or_else(|| GovernanceConfig {
                classification: DataClassification::Internal,
                retention_days: self.config.default_retention_days,
                access_control: AccessControl {
                    read_roles: vec!["researcher".to_string()],
                    write_roles: vec!["data_engineer".to_string()],
                    admin_roles: vec!["admin".to_string()],
                },
                pii_columns: Vec::new(),
                audit_enabled: true,
            }),
        };

        // Save to metadata store
        self.metadata_store.save_dataset(&dataset).await?;

        // Initialize lineage node
        let mut lineage = self.lineage_graph.write().await;
        lineage.add_dataset_node(dataset.id);

        Ok(dataset)
    }

    /// Create a new version of a dataset
    pub async fn create_version(
        &self,
        dataset_id: DatasetId,
        request: CreateVersionRequest,
    ) -> Result<DatasetVersion, DatasetError> {
        // Get dataset
        let dataset = self.metadata_store.get_dataset(dataset_id).await?;

        // Verify access
        self.vault_client
            .verify_access(dataset_id, &request.user_id, AccessType::Write)
            .await?;

        // Get next version number
        let version_number = self
            .metadata_store
            .get_next_version_number(dataset_id)
            .await?;

        // Store content
        let (manifest, content_hash) = match request.source {
            DataSource::Upload(data) => self.store_uploaded_data(data, &dataset.schema).await?,
            DataSource::Stream(stream) => {
                self.store_streamed_data(stream, &dataset.schema).await?
            }
            DataSource::Transform { source_version, transform } => {
                self.apply_transform(source_version, transform).await?
            }
            DataSource::External(uri) => self.import_external(uri, &dataset.schema).await?,
        };

        // Compute statistics
        let statistics = self.compute_statistics(&manifest, &dataset.schema).await?;

        let version_id = DatasetVersionId(Uuid::new_v4());
        let now = Utc::now();

        let version = DatasetVersion {
            id: version_id,
            dataset_id,
            version_number,
            content_hash,
            parent_version_id: request.parent_version_id,
            transform: request.transform,
            statistics,
            manifest,
            created_at: now,
            created_by: request.user_id,
            message: request.message,
            tags: request.tags.unwrap_or_default(),
        };

        // Save version
        self.metadata_store.save_version(&version).await?;

        // Update lineage
        let mut lineage = self.lineage_graph.write().await;
        lineage.add_version_node(version_id, dataset_id, request.parent_version_id);

        Ok(version)
    }

    /// Get a dataset version by reference
    pub async fn get_version(
        &self,
        dataset_id: DatasetId,
        selector: DatasetVersionSelector,
    ) -> Result<DatasetVersion, DatasetError> {
        match selector {
            DatasetVersionSelector::Latest => {
                self.metadata_store.get_latest_version(dataset_id).await
            }
            DatasetVersionSelector::Specific(version_id) => {
                self.metadata_store.get_version(version_id).await
            }
            DatasetVersionSelector::Tag(tag) => {
                self.metadata_store.get_version_by_tag(dataset_id, &tag).await
            }
            DatasetVersionSelector::ContentHash(hash) => {
                self.metadata_store.get_version_by_hash(dataset_id, &hash).await
            }
        }
    }

    /// Stream dataset content
    pub async fn stream_data(
        &self,
        version_id: DatasetVersionId,
        options: StreamOptions,
    ) -> Result<DataStream, DatasetError> {
        let version = self.metadata_store.get_version(version_id).await?;

        // Build stream from chunks
        let chunks = if let Some(ref columns) = options.columns {
            // Column projection - need to read only relevant chunks
            self.get_projected_chunks(&version.manifest, columns)?
        } else {
            version.manifest.chunks.clone()
        };

        let stream = self.content_store.stream_chunks(chunks, options).await?;

        Ok(stream)
    }

    /// Create dataset splits (train/val/test)
    pub async fn create_splits(
        &self,
        version_id: DatasetVersionId,
        config: SplitConfig,
    ) -> Result<DatasetSplits, DatasetError> {
        let version = self.metadata_store.get_version(version_id).await?;
        let dataset = self.metadata_store.get_dataset(version.dataset_id).await?;

        // Validate ratios
        let total: f64 = config.ratios.values().sum();
        if (total - 1.0).abs() > 0.001 {
            return Err(DatasetError::ValidationError {
                message: format!("Split ratios must sum to 1.0, got {}", total),
            });
        }

        // Load indices
        let total_rows = version.statistics.row_count;
        let mut indices: Vec<usize> = (0..total_rows as usize).collect();

        // Shuffle if needed
        if config.shuffle {
            use rand::prelude::*;
            let mut rng = match config.seed {
                Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
                None => rand::rngs::StdRng::from_entropy(),
            };
            indices.shuffle(&mut rng);
        }

        // Stratified splitting if requested
        let split_indices = if let Some(ref stratify_column) = config.stratify_by {
            self.stratified_split(&version, stratify_column, &config.ratios, config.seed)
                .await?
        } else {
            self.random_split(&indices, &config.ratios)
        };

        // Create split versions
        let mut splits = HashMap::new();

        for (split_name, indices) in split_indices {
            let split_version = self
                .create_split_version(
                    &version,
                    &dataset,
                    &split_name,
                    &indices,
                    config.user_id,
                )
                .await?;
            splits.insert(split_name, split_version.id);
        }

        Ok(DatasetSplits {
            source_version_id: version_id,
            splits,
            config,
            created_at: Utc::now(),
        })
    }

    /// Get dataset lineage
    pub async fn get_lineage(
        &self,
        dataset_id: DatasetId,
    ) -> Result<DatasetLineage, DatasetError> {
        let lineage = self.lineage_graph.read().await;
        lineage.get_dataset_lineage(dataset_id)
    }

    /// Compare two dataset versions
    pub async fn compare_versions(
        &self,
        version_a: DatasetVersionId,
        version_b: DatasetVersionId,
    ) -> Result<VersionComparison, DatasetError> {
        let a = self.metadata_store.get_version(version_a).await?;
        let b = self.metadata_store.get_version(version_b).await?;

        // Compare statistics
        let stats_diff = self.compare_statistics(&a.statistics, &b.statistics);

        // Compare schemas if from same dataset
        let schema_diff = if a.dataset_id == b.dataset_id {
            None
        } else {
            let dataset_a = self.metadata_store.get_dataset(a.dataset_id).await?;
            let dataset_b = self.metadata_store.get_dataset(b.dataset_id).await?;
            Some(self.compare_schemas(&dataset_a.schema, &dataset_b.schema))
        };

        // Compute content diff (sampling for large datasets)
        let content_diff = self.compute_content_diff(&a, &b).await?;

        Ok(VersionComparison {
            version_a: version_a,
            version_b: version_b,
            statistics_diff: stats_diff,
            schema_diff,
            content_diff,
            compared_at: Utc::now(),
        })
    }

    // Private helper methods

    fn validate_schema(&self, schema: &DatasetSchema) -> Result<(), DatasetError> {
        if schema.columns.is_empty() {
            return Err(DatasetError::ValidationError {
                message: "Schema must have at least one column".to_string(),
            });
        }

        // Check for duplicate column names
        let mut seen = std::collections::HashSet::new();
        for col in &schema.columns {
            if !seen.insert(&col.name) {
                return Err(DatasetError::ValidationError {
                    message: format!("Duplicate column name: {}", col.name),
                });
            }
        }

        // Validate primary key columns exist
        if let Some(ref pk) = schema.primary_key {
            for key_col in pk {
                if !schema.columns.iter().any(|c| &c.name == key_col) {
                    return Err(DatasetError::ValidationError {
                        message: format!("Primary key column not found: {}", key_col),
                    });
                }
            }
        }

        Ok(())
    }

    async fn store_uploaded_data(
        &self,
        data: Vec<u8>,
        schema: &DatasetSchema,
    ) -> Result<(DataManifest, ContentHash), DatasetError> {
        // Validate data against schema
        self.validate_data(&data, schema)?;

        // Chunk the data
        let chunks = self.chunk_data(&data)?;

        // Store chunks and build manifest
        let mut manifest_chunks = Vec::with_capacity(chunks.len());
        let mut hasher = sha2::Sha256::new();
        let mut offset = 0u64;

        for (idx, chunk) in chunks.into_iter().enumerate() {
            let chunk_hash = ContentHash::from_bytes(&chunk);
            hasher.update(chunk_hash.as_str().as_bytes());

            // Compress chunk
            let compressed = self.compress_chunk(&chunk)?;

            // Store chunk
            let storage_uri = self
                .content_store
                .store_chunk(&chunk_hash, &compressed)
                .await?;

            let chunk_info = ChunkInfo {
                chunk_id: format!("chunk_{:06}", idx),
                content_hash: chunk_hash,
                offset,
                size_bytes: chunk.len() as u64,
                row_count: 0, // Would be computed during chunking
                storage_uri,
            };

            offset += chunk.len() as u64;
            manifest_chunks.push(chunk_info);
        }

        let content_hash = ContentHash(hex::encode(hasher.finalize()));

        let manifest = DataManifest {
            chunks: manifest_chunks,
            total_size: offset,
            compression: self.config.compression,
            format: schema.format.clone(),
        };

        Ok((manifest, content_hash))
    }

    async fn store_streamed_data(
        &self,
        mut stream: DataInputStream,
        schema: &DatasetSchema,
    ) -> Result<(DataManifest, ContentHash), DatasetError> {
        let mut manifest_chunks = Vec::new();
        let mut hasher = sha2::Sha256::new();
        let mut offset = 0u64;
        let mut chunk_idx = 0;
        let mut buffer = Vec::with_capacity(self.config.chunk_size);

        while let Some(batch) = stream.next().await {
            let batch = batch?;
            buffer.extend_from_slice(&batch);

            // Flush when buffer reaches chunk size
            while buffer.len() >= self.config.chunk_size {
                let chunk: Vec<u8> = buffer.drain(..self.config.chunk_size).collect();
                let chunk_info = self
                    .store_single_chunk(&chunk, chunk_idx, offset, &mut hasher)
                    .await?;
                offset += chunk_info.size_bytes;
                chunk_idx += 1;
                manifest_chunks.push(chunk_info);
            }
        }

        // Store remaining data
        if !buffer.is_empty() {
            let chunk_info = self
                .store_single_chunk(&buffer, chunk_idx, offset, &mut hasher)
                .await?;
            offset += chunk_info.size_bytes;
            manifest_chunks.push(chunk_info);
        }

        let content_hash = ContentHash(hex::encode(hasher.finalize()));

        let manifest = DataManifest {
            chunks: manifest_chunks,
            total_size: offset,
            compression: self.config.compression,
            format: schema.format.clone(),
        };

        Ok((manifest, content_hash))
    }

    async fn store_single_chunk(
        &self,
        chunk: &[u8],
        idx: usize,
        offset: u64,
        hasher: &mut sha2::Sha256,
    ) -> Result<ChunkInfo, DatasetError> {
        use sha2::Digest;

        let chunk_hash = ContentHash::from_bytes(chunk);
        hasher.update(chunk_hash.as_str().as_bytes());

        let compressed = self.compress_chunk(chunk)?;
        let storage_uri = self
            .content_store
            .store_chunk(&chunk_hash, &compressed)
            .await?;

        Ok(ChunkInfo {
            chunk_id: format!("chunk_{:06}", idx),
            content_hash: chunk_hash,
            offset,
            size_bytes: chunk.len() as u64,
            row_count: 0,
            storage_uri,
        })
    }

    async fn apply_transform(
        &self,
        source_version: DatasetVersionId,
        transform: Transform,
    ) -> Result<(DataManifest, ContentHash), DatasetError> {
        let source = self.metadata_store.get_version(source_version).await?;

        match transform.transform_type {
            TransformType::Filter => {
                let predicate = transform
                    .parameters
                    .get("predicate")
                    .ok_or(DatasetError::MissingParameter {
                        parameter: "predicate".to_string(),
                    })?;
                self.apply_filter(&source, predicate).await
            }
            TransformType::Sample => {
                let fraction = transform
                    .parameters
                    .get("fraction")
                    .and_then(|v| v.as_f64())
                    .ok_or(DatasetError::MissingParameter {
                        parameter: "fraction".to_string(),
                    })?;
                let seed = transform.parameters.get("seed").and_then(|v| v.as_u64());
                self.apply_sample(&source, fraction, seed).await
            }
            TransformType::Split => {
                // Handled separately via create_splits
                Err(DatasetError::ValidationError {
                    message: "Use create_splits for split operations".to_string(),
                })
            }
            _ => Err(DatasetError::UnsupportedTransform {
                transform: format!("{:?}", transform.transform_type),
            }),
        }
    }

    async fn apply_filter(
        &self,
        source: &DatasetVersion,
        predicate: &serde_json::Value,
    ) -> Result<(DataManifest, ContentHash), DatasetError> {
        // Stream source data, apply filter, store results
        let stream = self
            .stream_data(source.id, StreamOptions::default())
            .await?;

        let filtered = self.filter_stream(stream, predicate).await?;

        let dataset = self.metadata_store.get_dataset(source.dataset_id).await?;
        self.store_streamed_data(filtered, &dataset.schema).await
    }

    async fn apply_sample(
        &self,
        source: &DatasetVersion,
        fraction: f64,
        seed: Option<u64>,
    ) -> Result<(DataManifest, ContentHash), DatasetError> {
        use rand::prelude::*;

        let total_rows = source.statistics.row_count;
        let sample_size = (total_rows as f64 * fraction).ceil() as usize;

        // Generate sample indices
        let mut rng = match seed {
            Some(s) => rand::rngs::StdRng::seed_from_u64(s),
            None => rand::rngs::StdRng::from_entropy(),
        };

        let mut indices: Vec<usize> = (0..total_rows as usize).collect();
        indices.shuffle(&mut rng);
        indices.truncate(sample_size);
        indices.sort();

        // Stream and filter by indices
        let stream = self
            .stream_data(source.id, StreamOptions::default())
            .await?;
        let sampled = self.sample_stream(stream, &indices).await?;

        let dataset = self.metadata_store.get_dataset(source.dataset_id).await?;
        self.store_streamed_data(sampled, &dataset.schema).await
    }

    async fn import_external(
        &self,
        uri: String,
        schema: &DatasetSchema,
    ) -> Result<(DataManifest, ContentHash), DatasetError> {
        // Fetch data from external URI
        let data = self.fetch_external_data(&uri).await?;
        self.store_uploaded_data(data, schema).await
    }

    async fn compute_statistics(
        &self,
        manifest: &DataManifest,
        schema: &DatasetSchema,
    ) -> Result<DatasetStatistics, DatasetError> {
        let mut row_count = 0u64;
        let mut column_stats: HashMap<String, ColumnStatsAccumulator> = schema
            .columns
            .iter()
            .map(|c| (c.name.clone(), ColumnStatsAccumulator::new(&c.data_type)))
            .collect();

        // Stream through chunks to compute statistics
        for chunk in &manifest.chunks {
            let data = self.content_store.read_chunk(&chunk.storage_uri).await?;
            let decompressed = self.decompress_chunk(&data)?;

            // Parse and accumulate stats
            let records = self.parse_chunk(&decompressed, &manifest.format)?;
            row_count += records.len() as u64;

            for record in records {
                for (col_name, accumulator) in &mut column_stats {
                    if let Some(value) = record.get(col_name) {
                        accumulator.add(value);
                    } else {
                        accumulator.add_null();
                    }
                }
            }
        }

        let column_statistics: HashMap<String, ColumnStatistics> = column_stats
            .into_iter()
            .map(|(name, acc)| (name, acc.finalize()))
            .collect();

        Ok(DatasetStatistics {
            row_count,
            size_bytes: manifest.total_size,
            column_statistics,
            computed_at: Utc::now(),
        })
    }

    fn chunk_data(&self, data: &[u8]) -> Result<Vec<Vec<u8>>, DatasetError> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let end = std::cmp::min(offset + self.config.chunk_size, data.len());
            chunks.push(data[offset..end].to_vec());
            offset = end;
        }

        Ok(chunks)
    }

    fn compress_chunk(&self, data: &[u8]) -> Result<Vec<u8>, DatasetError> {
        match self.config.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;

                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            CompressionType::Zstd => {
                let compressed = zstd::encode_all(data, 3)?;
                Ok(compressed)
            }
            CompressionType::Lz4 => {
                let compressed = lz4_flex::compress_prepend_size(data);
                Ok(compressed)
            }
            CompressionType::Snappy => {
                let mut encoder = snap::raw::Encoder::new();
                let compressed = encoder.compress_vec(data)?;
                Ok(compressed)
            }
        }
    }

    fn decompress_chunk(&self, data: &[u8]) -> Result<Vec<u8>, DatasetError> {
        match self.config.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::read::GzDecoder;
                use std::io::Read;

                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            CompressionType::Zstd => {
                let decompressed = zstd::decode_all(data)?;
                Ok(decompressed)
            }
            CompressionType::Lz4 => {
                let decompressed = lz4_flex::decompress_size_prepended(data)?;
                Ok(decompressed)
            }
            CompressionType::Snappy => {
                let mut decoder = snap::raw::Decoder::new();
                let decompressed = decoder.decompress_vec(data)?;
                Ok(decompressed)
            }
        }
    }

    fn validate_data(&self, _data: &[u8], _schema: &DatasetSchema) -> Result<(), DatasetError> {
        // Validate data format matches schema
        // This would involve parsing and type checking
        Ok(())
    }

    fn random_split(
        &self,
        indices: &[usize],
        ratios: &HashMap<String, f64>,
    ) -> HashMap<String, Vec<usize>> {
        let mut result = HashMap::new();
        let mut start = 0;

        let total = indices.len();
        for (name, ratio) in ratios {
            let count = (total as f64 * ratio).round() as usize;
            let end = std::cmp::min(start + count, total);
            result.insert(name.clone(), indices[start..end].to_vec());
            start = end;
        }

        result
    }

    async fn stratified_split(
        &self,
        version: &DatasetVersion,
        stratify_column: &str,
        ratios: &HashMap<String, f64>,
        seed: Option<u64>,
    ) -> Result<HashMap<String, Vec<usize>>, DatasetError> {
        use rand::prelude::*;

        // Group indices by stratification column value
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();

        // Stream data and build groups
        let stream = self
            .stream_data(version.id, StreamOptions {
                columns: Some(vec![stratify_column.to_string()]),
                ..Default::default()
            })
            .await?;

        let mut idx = 0;
        let mut stream_pin = Box::pin(stream);
        while let Some(batch) = stream_pin.next().await {
            let batch = batch?;
            for record in batch {
                let value = record
                    .get(stratify_column)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string());
                groups.entry(value).or_default().push(idx);
                idx += 1;
            }
        }

        // Shuffle each group
        let mut rng = match seed {
            Some(s) => rand::rngs::StdRng::seed_from_u64(s),
            None => rand::rngs::StdRng::from_entropy(),
        };

        for indices in groups.values_mut() {
            indices.shuffle(&mut rng);
        }

        // Split each group according to ratios
        let mut result: HashMap<String, Vec<usize>> = ratios
            .keys()
            .map(|k| (k.clone(), Vec::new()))
            .collect();

        for (_value, indices) in groups {
            let group_splits = self.random_split(&indices, ratios);
            for (split_name, split_indices) in group_splits {
                result.get_mut(&split_name).unwrap().extend(split_indices);
            }
        }

        Ok(result)
    }

    async fn create_split_version(
        &self,
        source: &DatasetVersion,
        dataset: &Dataset,
        split_name: &str,
        indices: &[usize],
        user_id: UserId,
    ) -> Result<DatasetVersion, DatasetError> {
        // Stream source and select only specified indices
        let stream = self
            .stream_data(source.id, StreamOptions::default())
            .await?;
        let filtered = self.select_indices_stream(stream, indices).await?;

        let (manifest, content_hash) = self
            .store_streamed_data(filtered, &dataset.schema)
            .await?;
        let statistics = self.compute_statistics(&manifest, &dataset.schema).await?;

        let version_number = self
            .metadata_store
            .get_next_version_number(dataset.id)
            .await?;

        let version = DatasetVersion {
            id: DatasetVersionId(Uuid::new_v4()),
            dataset_id: dataset.id,
            version_number,
            content_hash,
            parent_version_id: Some(source.id),
            transform: Some(Transform {
                transform_type: TransformType::Split,
                parameters: {
                    let mut p = HashMap::new();
                    p.insert("split_name".to_string(), serde_json::json!(split_name));
                    p.insert("indices_count".to_string(), serde_json::json!(indices.len()));
                    p
                },
                source_versions: vec![source.id],
            }),
            statistics,
            manifest,
            created_at: Utc::now(),
            created_by: user_id,
            message: Some(format!("Split: {}", split_name)),
            tags: vec![format!("split:{}", split_name)],
        };

        self.metadata_store.save_version(&version).await?;

        Ok(version)
    }

    fn compare_statistics(
        &self,
        a: &DatasetStatistics,
        b: &DatasetStatistics,
    ) -> StatisticsDiff {
        StatisticsDiff {
            row_count_diff: b.row_count as i64 - a.row_count as i64,
            size_diff: b.size_bytes as i64 - a.size_bytes as i64,
            column_diffs: HashMap::new(), // Would compute per-column diffs
        }
    }

    fn compare_schemas(&self, a: &DatasetSchema, b: &DatasetSchema) -> SchemaDiff {
        let a_cols: std::collections::HashSet<_> = a.columns.iter().map(|c| &c.name).collect();
        let b_cols: std::collections::HashSet<_> = b.columns.iter().map(|c| &c.name).collect();

        SchemaDiff {
            added_columns: b_cols.difference(&a_cols).map(|s| (*s).clone()).collect(),
            removed_columns: a_cols.difference(&b_cols).map(|s| (*s).clone()).collect(),
            type_changes: Vec::new(), // Would detect type changes for common columns
        }
    }

    async fn compute_content_diff(
        &self,
        _a: &DatasetVersion,
        _b: &DatasetVersion,
    ) -> Result<ContentDiff, DatasetError> {
        // For large datasets, sample and compare
        Ok(ContentDiff {
            sample_based: true,
            sample_size: 1000,
            added_rows_estimate: 0,
            removed_rows_estimate: 0,
            modified_rows_estimate: 0,
        })
    }

    fn get_projected_chunks(
        &self,
        manifest: &DataManifest,
        _columns: &[String],
    ) -> Result<Vec<ChunkInfo>, DatasetError> {
        // For columnar formats, return only relevant chunks
        // For row-based formats, return all chunks
        Ok(manifest.chunks.clone())
    }

    fn parse_chunk(
        &self,
        _data: &[u8],
        _format: &DataFormat,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>, DatasetError> {
        // Parse chunk based on format
        Ok(Vec::new())
    }

    async fn filter_stream(
        &self,
        _stream: DataStream,
        _predicate: &serde_json::Value,
    ) -> Result<DataInputStream, DatasetError> {
        // Apply filter predicate to stream
        unimplemented!()
    }

    async fn sample_stream(
        &self,
        _stream: DataStream,
        _indices: &[usize],
    ) -> Result<DataInputStream, DatasetError> {
        // Sample stream at specified indices
        unimplemented!()
    }

    async fn select_indices_stream(
        &self,
        _stream: DataStream,
        _indices: &[usize],
    ) -> Result<DataInputStream, DatasetError> {
        // Select specific indices from stream
        unimplemented!()
    }

    async fn fetch_external_data(&self, _uri: &str) -> Result<Vec<u8>, DatasetError> {
        // Fetch data from external source
        unimplemented!()
    }
}

// Type aliases and helper structs
pub type DataStream = tokio::sync::mpsc::Receiver<Result<Vec<HashMap<String, serde_json::Value>>, DatasetError>>;
pub type DataInputStream = tokio::sync::mpsc::Receiver<Result<Vec<u8>, DatasetError>>;

#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    pub columns: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitConfig {
    pub ratios: HashMap<String, f64>,
    pub shuffle: bool,
    pub seed: Option<u64>,
    pub stratify_by: Option<String>,
    pub user_id: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSplits {
    pub source_version_id: DatasetVersionId,
    pub splits: HashMap<String, DatasetVersionId>,
    pub config: SplitConfig,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetLineage {
    pub dataset_id: DatasetId,
    pub versions: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparison {
    pub version_a: DatasetVersionId,
    pub version_b: DatasetVersionId,
    pub statistics_diff: StatisticsDiff,
    pub schema_diff: Option<SchemaDiff>,
    pub content_diff: ContentDiff,
    pub compared_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsDiff {
    pub row_count_diff: i64,
    pub size_diff: i64,
    pub column_diffs: HashMap<String, ColumnStatsDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatsDiff {
    pub null_count_diff: i64,
    pub distinct_count_diff: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    pub added_columns: Vec<String>,
    pub removed_columns: Vec<String>,
    pub type_changes: Vec<TypeChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeChange {
    pub column: String,
    pub old_type: DataType,
    pub new_type: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDiff {
    pub sample_based: bool,
    pub sample_size: usize,
    pub added_rows_estimate: u64,
    pub removed_rows_estimate: u64,
    pub modified_rows_estimate: u64,
}

/// Request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDatasetRequest {
    pub name: String,
    pub description: Option<String>,
    pub schema: DatasetSchema,
    pub tags: Option<Vec<String>>,
    pub owner_id: UserId,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub governance: Option<GovernanceConfig>,
}

#[derive(Debug, Clone)]
pub struct CreateVersionRequest {
    pub source: DataSource,
    pub parent_version_id: Option<DatasetVersionId>,
    pub transform: Option<Transform>,
    pub message: Option<String>,
    pub tags: Option<Vec<String>>,
    pub user_id: UserId,
}

#[derive(Debug)]
pub enum DataSource {
    Upload(Vec<u8>),
    Stream(DataInputStream),
    Transform {
        source_version: DatasetVersionId,
        transform: Transform,
    },
    External(String),
}

/// Column statistics accumulator
struct ColumnStatsAccumulator {
    data_type: DataType,
    count: u64,
    null_count: u64,
    values: Vec<serde_json::Value>,
    sum: f64,
    sum_sq: f64,
}

impl ColumnStatsAccumulator {
    fn new(data_type: &DataType) -> Self {
        Self {
            data_type: data_type.clone(),
            count: 0,
            null_count: 0,
            values: Vec::new(),
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    fn add(&mut self, value: &serde_json::Value) {
        self.count += 1;
        self.values.push(value.clone());

        if let Some(n) = value.as_f64() {
            self.sum += n;
            self.sum_sq += n * n;
        }
    }

    fn add_null(&mut self) {
        self.count += 1;
        self.null_count += 1;
    }

    fn finalize(self) -> ColumnStatistics {
        let distinct_count = if self.values.len() < 10000 {
            let unique: std::collections::HashSet<_> =
                self.values.iter().map(|v| v.to_string()).collect();
            Some(unique.len() as u64)
        } else {
            None
        };

        let (mean, std) = if self.count > self.null_count {
            let n = (self.count - self.null_count) as f64;
            let mean = self.sum / n;
            let variance = (self.sum_sq / n) - (mean * mean);
            (Some(mean), Some(variance.sqrt()))
        } else {
            (None, None)
        };

        ColumnStatistics {
            null_count: self.null_count,
            distinct_count,
            min_value: self.values.iter().min_by(|a, b| {
                a.to_string().cmp(&b.to_string())
            }).cloned(),
            max_value: self.values.iter().max_by(|a, b| {
                a.to_string().cmp(&b.to_string())
            }).cloned(),
            mean,
            std,
            histogram: None,
        }
    }
}

/// Dataset error types
#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("Dataset not found: {id:?}")]
    NotFound { id: DatasetId },

    #[error("Version not found: {id:?}")]
    VersionNotFound { id: DatasetVersionId },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Missing parameter: {parameter}")]
    MissingParameter { parameter: String },

    #[error("Unsupported transform: {transform}")]
    UnsupportedTransform { transform: String },

    #[error("Access denied")]
    AccessDenied,

    #[error("Storage error: {0}")]
    Storage(#[from] std::io::Error),

    #[error("Compression error: {message}")]
    CompressionError { message: String },
}

/// Storage traits
#[async_trait]
pub trait DatasetMetadataStore: Send + Sync {
    async fn save_dataset(&self, dataset: &Dataset) -> Result<(), DatasetError>;
    async fn get_dataset(&self, id: DatasetId) -> Result<Dataset, DatasetError>;
    async fn save_version(&self, version: &DatasetVersion) -> Result<(), DatasetError>;
    async fn get_version(&self, id: DatasetVersionId) -> Result<DatasetVersion, DatasetError>;
    async fn get_latest_version(&self, dataset_id: DatasetId) -> Result<DatasetVersion, DatasetError>;
    async fn get_version_by_tag(&self, dataset_id: DatasetId, tag: &str) -> Result<DatasetVersion, DatasetError>;
    async fn get_version_by_hash(&self, dataset_id: DatasetId, hash: &ContentHash) -> Result<DatasetVersion, DatasetError>;
    async fn get_next_version_number(&self, dataset_id: DatasetId) -> Result<u64, DatasetError>;
}

#[async_trait]
pub trait DataVaultClient: Send + Sync {
    async fn verify_access(
        &self,
        dataset_id: DatasetId,
        user_id: &UserId,
        access_type: AccessType,
    ) -> Result<(), DatasetError>;
}

#[derive(Debug, Clone, Copy)]
pub enum AccessType {
    Read,
    Write,
    Admin,
}

/// Content store for chunk storage
pub struct ContentStore {
    storage: Arc<dyn ObjectStorage>,
    deduplication: bool,
}

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn put(&self, key: &str, data: &[u8]) -> Result<String, DatasetError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, DatasetError>;
    async fn exists(&self, key: &str) -> Result<bool, DatasetError>;
    async fn delete(&self, key: &str) -> Result<(), DatasetError>;
}

impl ContentStore {
    pub fn new(storage: Arc<dyn ObjectStorage>, deduplication: bool) -> Self {
        Self {
            storage,
            deduplication,
        }
    }

    pub async fn store_chunk(
        &self,
        hash: &ContentHash,
        data: &[u8],
    ) -> Result<String, DatasetError> {
        let key = format!("chunks/{}/{}", &hash.0[..2], hash.0);

        // Check for existing chunk if deduplication enabled
        if self.deduplication {
            if self.storage.exists(&key).await? {
                return Ok(key);
            }
        }

        self.storage.put(&key, data).await
    }

    pub async fn read_chunk(&self, uri: &str) -> Result<Vec<u8>, DatasetError> {
        self.storage.get(uri).await
    }

    pub async fn stream_chunks(
        &self,
        chunks: Vec<ChunkInfo>,
        _options: StreamOptions,
    ) -> Result<DataStream, DatasetError> {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let storage = self.storage.clone();

        tokio::spawn(async move {
            for chunk in chunks {
                match storage.get(&chunk.storage_uri).await {
                    Ok(data) => {
                        // Parse and send records
                        // Simplified - would actually parse the data
                        let _ = tx.send(Ok(Vec::new())).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Dataset lineage graph
pub struct DatasetLineageGraph {
    nodes: HashMap<String, LineageNode>,
    edges: Vec<LineageEdge>,
}

impl DatasetLineageGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_dataset_node(&mut self, dataset_id: DatasetId) {
        let node = LineageNode {
            id: format!("dataset:{}", dataset_id.0),
            node_type: LineageNodeType::Dataset,
            name: String::new(),
            version: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_version_node(
        &mut self,
        version_id: DatasetVersionId,
        dataset_id: DatasetId,
        parent_version_id: Option<DatasetVersionId>,
    ) {
        let node = LineageNode {
            id: format!("version:{}", version_id.0),
            node_type: LineageNodeType::DatasetVersion,
            name: String::new(),
            version: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        self.nodes.insert(node.id.clone(), node);

        // Edge from dataset to version
        self.edges.push(LineageEdge {
            source_id: format!("dataset:{}", dataset_id.0),
            target_id: format!("version:{}", version_id.0),
            edge_type: LineageEdgeType::Contains,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        });

        // Edge from parent version if exists
        if let Some(parent_id) = parent_version_id {
            self.edges.push(LineageEdge {
                source_id: format!("version:{}", parent_id.0),
                target_id: format!("version:{}", version_id.0),
                edge_type: LineageEdgeType::DerivedFrom,
                metadata: HashMap::new(),
                created_at: Utc::now(),
            });
        }
    }

    pub fn get_dataset_lineage(&self, dataset_id: DatasetId) -> Result<DatasetLineage, DatasetError> {
        let prefix = format!("dataset:{}", dataset_id.0);

        let nodes: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.id.starts_with(&prefix) || self.is_connected_to(&n.id, &prefix))
            .cloned()
            .collect();

        let node_ids: std::collections::HashSet<_> = nodes.iter().map(|n| &n.id).collect();

        let edges: Vec<_> = self
            .edges
            .iter()
            .filter(|e| node_ids.contains(&e.source_id) || node_ids.contains(&e.target_id))
            .cloned()
            .collect();

        Ok(DatasetLineage {
            dataset_id,
            versions: nodes,
            edges,
        })
    }

    fn is_connected_to(&self, node_id: &str, target_prefix: &str) -> bool {
        // BFS to find connection
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(node_id.to_string());

        while let Some(current) = queue.pop_front() {
            if current.starts_with(target_prefix) {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            for edge in &self.edges {
                if edge.source_id == current && !visited.contains(&edge.target_id) {
                    queue.push_back(edge.target_id.clone());
                }
                if edge.target_id == current && !visited.contains(&edge.source_id) {
                    queue.push_back(edge.source_id.clone());
                }
            }
        }

        false
    }
}
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Draft |
| **SPARC Phase** | Pseudocode (Part 2 of 3) |
| **Created** | 2025-11-28 |
| **Ecosystem** | LLM DevOps |
| **Previous Part** | Pseudocode Part 1: Core Data Models & Experiment Tracking |
| **Next Part** | Pseudocode Part 3: Integration APIs & Workflow Orchestration |

---

*This pseudocode document is part of the SPARC methodology. Part 3 covers Integration APIs, Reproducibility Engine, Workflow Orchestration, and Error Handling.*
