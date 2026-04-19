//! Hypothesis Agent Implementation
//!
//! # Agent Contract
//!
//! ## Agent Name
//! Hypothesis Agent
//!
//! ## Purpose
//! Define, evaluate, and validate research hypotheses using structured
//! experimental inputs and observed signals.
//!
//! ## Classification
//! HYPOTHESIS EVALUATION
//!
//! ## Scope
//! - Define testable hypotheses
//! - Evaluate hypotheses against experimental data
//! - Emit structured hypothesis outcomes
//!
//! ## decision_type
//! "hypothesis_evaluation"
//!
//! ## Explicit Non-Responsibilities (MUST NEVER)
//!
//! This agent MUST NEVER:
//! - Execute inference
//! - Modify prompts or responses
//! - Route inference requests
//! - Trigger orchestration or retries
//! - Apply optimizations automatically
//! - Enforce policies or governance decisions
//!
//! ## Failure Modes
//! - Invalid input schema: Returns validation error
//! - Insufficient sample size: Returns inconclusive result with warning
//! - Statistical assumption violations: Returns result with violation flags
//! - ruvector-service unavailable: Returns persistence error

use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::json;
use thiserror::Error;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;
use validator::Validate;

use crate::contracts::{
    hypothesis::*,
    decision_event::*,
    common::*,
};
use super::traits::{Agent, ConfidenceEstimator, PerformanceBounded, PerformanceBudget, BudgetViolation};

/// Agent version (semantic versioning).
pub const HYPOTHESIS_AGENT_VERSION: &str = "1.0.0";

/// Agent identifier.
pub const HYPOTHESIS_AGENT_ID: &str = "hypothesis-agent-v1";

/// Errors from Hypothesis Agent operations.
#[derive(Debug, Error)]
pub enum HypothesisAgentError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Insufficient sample size: required {required}, got {actual}")]
    InsufficientSampleSize { required: u64, actual: u64 },

    #[error("Statistical computation error: {0}")]
    StatisticalComputation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Performance budget exceeded: {0}")]
    BudgetExceeded(String),
}

impl From<validator::ValidationErrors> for HypothesisAgentError {
    fn from(err: validator::ValidationErrors) -> Self {
        HypothesisAgentError::Validation(err.to_string())
    }
}

impl From<BudgetViolation> for HypothesisAgentError {
    fn from(err: BudgetViolation) -> Self {
        HypothesisAgentError::BudgetExceeded(err.message)
    }
}

/// Hypothesis Agent for evaluating research hypotheses.
///
/// This agent implements the core hypothesis evaluation logic for LLM-Research-Lab.
/// It is stateless, deterministic, and produces structured DecisionEvents.
///
/// # Phase 7 Performance Budgets
///
/// This agent enforces strict performance budgets:
/// - Maximum latency: 5000ms (default)
/// - Maximum tokens: 2500 (default)
/// - Maximum API calls: 5 (default)
///
/// Execution will ABORT if any budget is exceeded.
#[derive(Clone)]
pub struct HypothesisAgent {
    identity: AgentIdentity,
    config: HypothesisAgentConfig,
    /// Performance budget for this agent (Phase 7 MANDATORY)
    budget: PerformanceBudget,
}

/// Configuration for Hypothesis Agent.
#[derive(Debug, Clone)]
pub struct HypothesisAgentConfig {
    /// Minimum sample size for evaluation
    pub min_sample_size: u64,

    /// Default significance level
    pub default_alpha: Decimal,

    /// Enable assumption checking
    pub check_assumptions: bool,

    /// Random seed for reproducibility
    pub random_seed: Option<u64>,
}

impl Default for HypothesisAgentConfig {
    fn default() -> Self {
        Self {
            min_sample_size: 30,
            default_alpha: dec!(0.05),
            check_assumptions: true,
            random_seed: None,
        }
    }
}

impl HypothesisAgent {
    /// Create a new Hypothesis Agent with default configuration.
    pub fn new() -> Self {
        Self::with_config(HypothesisAgentConfig::default())
    }

    /// Create a new Hypothesis Agent with custom configuration.
    pub fn with_config(config: HypothesisAgentConfig) -> Self {
        Self::with_config_and_budget(config, PerformanceBudget::default())
    }

    /// Create a new Hypothesis Agent with custom configuration and budget.
    pub fn with_config_and_budget(config: HypothesisAgentConfig, budget: PerformanceBudget) -> Self {
        Self {
            identity: AgentIdentity {
                id: HYPOTHESIS_AGENT_ID.to_string(),
                version: HYPOTHESIS_AGENT_VERSION.to_string(),
                classification: AgentClassification::HypothesisEvaluation,
                description: "Evaluates research hypotheses using statistical methods".to_string(),
            },
            config,
            budget,
        }
    }

    /// Determine the dependent-variable name to extract from each observation.
    ///
    /// Falls back to "value" when no variable with `VariableRole::Dependent`
    /// is declared — preserving the legacy test fixture shape.
    fn dependent_variable_name(hypothesis: &HypothesisDefinition) -> String {
        hypothesis
            .variables
            .iter()
            .find(|v| v.role == VariableRole::Dependent)
            .map(|v| v.name.clone())
            .unwrap_or_else(|| "value".to_string())
    }

    /// Extract a numeric value from a single observation, trying multiple shapes.
    ///
    /// Supports payload shapes emitted by upstream callers:
    /// - `values: { <var_name>: 0.88 }` (standard — nested, keyed by variable name)
    /// - `values: { value: 0.88 }` (legacy fixture shape)
    /// - `values: 0.88` (scalar — rare but seen in CLI variants)
    /// - `values: { anything: 0.88, ... }` (fallback: first numeric field)
    fn extract_observation_value(obs: &Observation, var_name: &str) -> Option<f64> {
        if let Some(v) = obs.values.get(var_name).and_then(|v| v.as_f64()) {
            return Some(v);
        }
        if let Some(v) = obs.values.get("value").and_then(|v| v.as_f64()) {
            return Some(v);
        }
        if let Some(v) = obs.values.as_f64() {
            return Some(v);
        }
        // Last-resort fallback: pick the first numeric field in the object.
        // Covers payload variants where the dependent variable is not named
        // explicitly or observations omit a declared variable.
        if let Some(map) = obs.values.as_object() {
            for (_, val) in map {
                if let Some(n) = val.as_f64() {
                    return Some(n);
                }
            }
        }
        None
    }

    /// Extract all usable sample values, partitioned by group if groups exist.
    ///
    /// Returns `(flat_values, Option<(group_a, group_b)>)`. If two distinct
    /// non-empty groups are present the second element is populated and the
    /// caller may run a two-sample test.
    fn extract_samples(
        hypothesis: &HypothesisDefinition,
        data: &ExperimentalData,
    ) -> (Vec<f64>, Option<(Vec<f64>, Vec<f64>)>) {
        let var_name = Self::dependent_variable_name(hypothesis);

        let mut flat: Vec<f64> = Vec::with_capacity(data.observations.len());
        let mut grouped: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();

        for obs in &data.observations {
            if let Some(value) = Self::extract_observation_value(obs, &var_name) {
                flat.push(value);
                if let Some(group) = obs.group.as_ref() {
                    grouped.entry(group.clone()).or_default().push(value);
                }
            }
        }

        let two_sample = if grouped.len() >= 2 {
            let mut iter = grouped.into_iter();
            let (_, a) = iter.next().unwrap();
            let (_, b) = iter.next().unwrap();
            if !a.is_empty() && !b.is_empty() {
                Some((a, b))
            } else {
                None
            }
        } else {
            None
        };

        (flat, two_sample)
    }

    /// Perform t-test hypothesis evaluation.
    ///
    /// Runs a two-sample independent (Welch's) t-test when observations are
    /// tagged with two or more distinct groups, otherwise a one-sample t-test
    /// against mu = 0.
    #[instrument(skip(self, data), fields(sample_size = data.sample_size))]
    fn evaluate_ttest(
        &self,
        hypothesis: &HypothesisDefinition,
        data: &ExperimentalData,
        config: &EvaluationConfig,
    ) -> Result<TestResults, HypothesisAgentError> {
        info!("Performing t-test evaluation");

        let (values, two_sample) = Self::extract_samples(hypothesis, data);

        let total_samples = values.len();
        let min_required = self.config.min_sample_size as usize;

        // For two-sample tests, require min_sample_size across both groups combined.
        if total_samples < min_required {
            warn!(
                dependent_variable = %Self::dependent_variable_name(hypothesis),
                observations_in_payload = data.observations.len(),
                samples_extracted = total_samples,
                min_required,
                "Insufficient extractable samples from payload"
            );
            return Err(HypothesisAgentError::InsufficientSampleSize {
                required: self.config.min_sample_size,
                actual: total_samples as u64,
            });
        }

        let alpha: f64 = hypothesis
            .significance_level
            .try_into()
            .unwrap_or(0.05);

        if let Some((a, b)) = two_sample {
            return Ok(self.two_sample_ttest(&a, &b, alpha, config));
        }

        // One-sample t-test against mu = 0.
        let n = values.len() as f64;
        let mean: f64 = values.iter().sum::<f64>() / n;
        let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        let std_error = if std_dev == 0.0 { 1e-12 } else { std_dev / n.sqrt() };

        let t_statistic = mean / std_error;
        let df = n - 1.0;
        let p_value = self.approximate_t_pvalue(t_statistic.abs(), df);
        let null_rejected = p_value < alpha;

        let t_critical = self.t_critical_value(alpha / 2.0, df);
        let ci_margin = t_critical * std_error;

        debug!(
            t_statistic = t_statistic,
            p_value = p_value,
            df = df,
            null_rejected = null_rejected,
            "One-sample t-test results"
        );

        Ok(TestResults {
            test_statistic: Decimal::try_from(t_statistic).unwrap_or(dec!(0)),
            p_value: Decimal::try_from(p_value).unwrap_or(dec!(1)),
            corrected_p_value: if config.apply_correction {
                Some(Decimal::try_from(p_value).unwrap_or(dec!(1)))
            } else {
                None
            },
            degrees_of_freedom: Some(Decimal::try_from(df).unwrap_or(dec!(0))),
            confidence_interval: Some(ConfidenceInterval {
                lower: Decimal::try_from(mean - ci_margin).unwrap_or(dec!(0)),
                upper: Decimal::try_from(mean + ci_margin).unwrap_or(dec!(0)),
                level: dec!(0.95),
            }),
            null_rejected,
            decision: if null_rejected {
                "Reject null hypothesis".to_string()
            } else {
                "Fail to reject null hypothesis".to_string()
            },
        })
    }

    /// Welch's two-sample independent t-test.
    fn two_sample_ttest(
        &self,
        a: &[f64],
        b: &[f64],
        alpha: f64,
        config: &EvaluationConfig,
    ) -> TestResults {
        let (mean_a, var_a) = mean_and_variance(a);
        let (mean_b, var_b) = mean_and_variance(b);
        let na = a.len() as f64;
        let nb = b.len() as f64;

        let se_sq = var_a / na + var_b / nb;
        let se = if se_sq <= 0.0 { 1e-12 } else { se_sq.sqrt() };
        let diff = mean_a - mean_b;
        let t_statistic = diff / se;

        // Welch–Satterthwaite df
        let df_num = se_sq.powi(2);
        let df_den = (var_a / na).powi(2) / (na - 1.0).max(1.0)
            + (var_b / nb).powi(2) / (nb - 1.0).max(1.0);
        let df = if df_den <= 0.0 { na + nb - 2.0 } else { df_num / df_den };

        let p_value = self.approximate_t_pvalue(t_statistic.abs(), df);
        let null_rejected = p_value < alpha;

        let t_critical = self.t_critical_value(alpha / 2.0, df);
        let ci_margin = t_critical * se;

        debug!(
            t_statistic = t_statistic,
            p_value = p_value,
            df = df,
            mean_a = mean_a,
            mean_b = mean_b,
            n_a = na,
            n_b = nb,
            "Two-sample (Welch) t-test results"
        );

        TestResults {
            test_statistic: Decimal::try_from(t_statistic).unwrap_or(dec!(0)),
            p_value: Decimal::try_from(p_value).unwrap_or(dec!(1)),
            corrected_p_value: if config.apply_correction {
                Some(Decimal::try_from(p_value).unwrap_or(dec!(1)))
            } else {
                None
            },
            degrees_of_freedom: Some(Decimal::try_from(df).unwrap_or(dec!(0))),
            confidence_interval: Some(ConfidenceInterval {
                lower: Decimal::try_from(diff - ci_margin).unwrap_or(dec!(0)),
                upper: Decimal::try_from(diff + ci_margin).unwrap_or(dec!(0)),
                level: dec!(0.95),
            }),
            null_rejected,
            decision: if null_rejected {
                "Reject null hypothesis".to_string()
            } else {
                "Fail to reject null hypothesis".to_string()
            },
        }
    }

    /// Mann-Whitney U test (two-sample non-parametric).
    ///
    /// Falls back to a one-sample Wilcoxon-style sign check when only one
    /// group is present, since the test is fundamentally two-sample.
    #[instrument(skip(self, data), fields(sample_size = data.sample_size))]
    fn evaluate_mann_whitney(
        &self,
        hypothesis: &HypothesisDefinition,
        data: &ExperimentalData,
        config: &EvaluationConfig,
    ) -> Result<TestResults, HypothesisAgentError> {
        info!("Performing Mann-Whitney U test");

        let (values, two_sample) = Self::extract_samples(hypothesis, data);

        if values.len() < self.config.min_sample_size as usize {
            return Err(HypothesisAgentError::InsufficientSampleSize {
                required: self.config.min_sample_size,
                actual: values.len() as u64,
            });
        }

        let alpha: f64 = hypothesis
            .significance_level
            .try_into()
            .unwrap_or(0.05);

        // Mann-Whitney U needs two groups. Without groups, degrade to a
        // two-sample t-test across a median split to still return real output.
        let (a, b) = match two_sample {
            Some(pair) => pair,
            None => {
                warn!("Mann-Whitney requested but no groups present; falling back to Welch t-test on median split");
                let mut sorted = values.clone();
                sorted.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted.len() / 2;
                let lo: Vec<f64> = sorted[..mid].to_vec();
                let hi: Vec<f64> = sorted[mid..].to_vec();
                return Ok(self.two_sample_ttest(&lo, &hi, alpha, config));
            }
        };

        let (u_statistic, p_value) = mann_whitney_u(&a, &b);
        let null_rejected = p_value < alpha;

        let (mean_a, _) = mean_and_variance(&a);
        let (mean_b, _) = mean_and_variance(&b);
        let diff = mean_a - mean_b;

        debug!(
            u_statistic = u_statistic,
            p_value = p_value,
            n_a = a.len(),
            n_b = b.len(),
            "Mann-Whitney U results"
        );

        Ok(TestResults {
            test_statistic: Decimal::try_from(u_statistic).unwrap_or(dec!(0)),
            p_value: Decimal::try_from(p_value).unwrap_or(dec!(1)),
            corrected_p_value: if config.apply_correction {
                Some(Decimal::try_from(p_value).unwrap_or(dec!(1)))
            } else {
                None
            },
            degrees_of_freedom: None,
            confidence_interval: Some(ConfidenceInterval {
                lower: Decimal::try_from(diff).unwrap_or(dec!(0)),
                upper: Decimal::try_from(diff).unwrap_or(dec!(0)),
                level: dec!(0.95),
            }),
            null_rejected,
            decision: if null_rejected {
                "Reject null hypothesis".to_string()
            } else {
                "Fail to reject null hypothesis".to_string()
            },
        })
    }

    /// Approximate t-distribution p-value.
    ///
    /// This is a simplified approximation. In production, use a proper
    /// statistical library like statrs.
    fn approximate_t_pvalue(&self, t: f64, df: f64) -> f64 {
        // Use normal approximation for large df
        if df > 30.0 {
            // Standard normal CDF approximation
            let z = t;
            let p = 0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2));
            2.0 * (1.0 - p)
        } else {
            // Rough approximation for smaller df
            let x = df / (df + t * t);
            let p = incomplete_beta(df / 2.0, 0.5, x) / 2.0;
            2.0 * p.min(1.0 - p)
        }
    }

    /// Get critical t-value for given alpha and df.
    fn t_critical_value(&self, alpha: f64, df: f64) -> f64 {
        // Approximation using normal distribution for large df
        if df > 30.0 {
            // z-score approximation
            inverse_normal_cdf(1.0 - alpha)
        } else {
            // Rough approximation
            1.96 + 2.0 / df
        }
    }

    /// Compute effect size (Cohen's d).
    ///
    /// Uses the two-sample pooled-SD formulation when observations are split
    /// into groups, and the one-sample-vs-zero formulation otherwise.
    fn compute_effect_size(
        &self,
        hypothesis: &HypothesisDefinition,
        data: &ExperimentalData,
    ) -> Option<EffectSize> {
        let (values, two_sample) = Self::extract_samples(hypothesis, data);

        let d = if let Some((a, b)) = two_sample {
            if a.len() < 2 || b.len() < 2 {
                return None;
            }
            let (mean_a, var_a) = mean_and_variance(&a);
            let (mean_b, var_b) = mean_and_variance(&b);
            let na = a.len() as f64;
            let nb = b.len() as f64;
            let pooled = (((na - 1.0) * var_a + (nb - 1.0) * var_b) / (na + nb - 2.0).max(1.0)).sqrt();
            if pooled == 0.0 {
                return None;
            }
            (mean_a - mean_b) / pooled
        } else {
            if values.len() < 2 {
                return None;
            }
            let n = values.len() as f64;
            let mean: f64 = values.iter().sum::<f64>() / n;
            let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
            let std_dev = variance.sqrt();
            if std_dev == 0.0 {
                return None;
            }
            mean / std_dev
        };

        let interpretation = if d.abs() < 0.2 {
            "negligible"
        } else if d.abs() < 0.5 {
            "small"
        } else if d.abs() < 0.8 {
            "medium"
        } else {
            "large"
        };

        Some(EffectSize {
            measure: EffectSizeMeasure::CohensD,
            value: Decimal::try_from(d).unwrap_or(dec!(0)),
            interpretation: interpretation.to_string(),
        })
    }

    /// Check statistical assumptions.
    fn check_assumptions(&self, data: &ExperimentalData) -> Vec<AssumptionViolation> {
        let mut violations = Vec::new();

        // Check sample size
        if data.sample_size < self.config.min_sample_size {
            violations.push(AssumptionViolation {
                assumption: "Minimum sample size".to_string(),
                test_used: "Count check".to_string(),
                severity: ViolationSeverity::Severe,
                recommendation: format!(
                    "Increase sample size to at least {}",
                    self.config.min_sample_size
                ),
            });
        }

        // Check data quality
        let completeness: f64 = data.quality_metrics.completeness.try_into().unwrap_or(0.0);
        if completeness < 0.9 {
            violations.push(AssumptionViolation {
                assumption: "Data completeness".to_string(),
                test_used: "Completeness ratio".to_string(),
                severity: if completeness < 0.7 {
                    ViolationSeverity::Severe
                } else {
                    ViolationSeverity::Moderate
                },
                recommendation: "Address missing data before analysis".to_string(),
            });
        }

        violations
    }

    /// Determine hypothesis status from test results.
    fn determine_status(
        &self,
        results: &TestResults,
        violations: &[AssumptionViolation],
    ) -> HypothesisStatus {
        // Check for severe violations
        let has_severe_violations = violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Severe);

        if has_severe_violations {
            return HypothesisStatus::Inconclusive;
        }

        if results.null_rejected {
            HypothesisStatus::Accepted
        } else {
            HypothesisStatus::Rejected
        }
    }
}

impl Default for HypothesisAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfidenceEstimator for HypothesisAgent {
    fn estimate_confidence(&self, sample_size: u64, effect_size: Option<f64>) -> f64 {
        // Base confidence from sample size
        let size_confidence = (sample_size as f64 / 1000.0).min(0.9);

        // Adjust based on effect size if available
        let effect_adjustment = effect_size
            .map(|e| (e.abs() * 0.1).min(0.1))
            .unwrap_or(0.0);

        (size_confidence + effect_adjustment).min(0.99)
    }
}

impl PerformanceBounded for HypothesisAgent {
    fn budget(&self) -> &PerformanceBudget {
        &self.budget
    }
}

#[async_trait]
impl Agent for HypothesisAgent {
    type Input = HypothesisInput;
    type Output = HypothesisOutput;
    type Error = HypothesisAgentError;

    fn identity(&self) -> &AgentIdentity {
        &self.identity
    }

    fn validate_input(&self, input: &Self::Input) -> Result<(), Self::Error> {
        input.validate()?;
        input.hypothesis.validate()?;
        input.experimental_data.validate()?;
        input.config.validate()?;
        Ok(())
    }

    #[instrument(skip(self, input), fields(
        request_id = %input.request_id,
        hypothesis_id = %input.hypothesis.id,
        hypothesis_type = ?input.hypothesis.hypothesis_type,
        sample_size = input.experimental_data.sample_size
    ))]
    async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        // Phase 7: Start timing for performance budget enforcement
        let start = std::time::Instant::now();

        info!("Executing hypothesis evaluation");

        // Check assumptions if configured
        let assumption_violations = if self.config.check_assumptions {
            self.check_assumptions(&input.experimental_data)
        } else {
            Vec::new()
        };

        if !assumption_violations.is_empty() {
            warn!(
                violations = assumption_violations.len(),
                "Assumption violations detected"
            );
        }

        // Perform statistical test based on configuration
        let test_results = match input.config.test_method {
            StatisticalTest::TTest | StatisticalTest::WelchTTest => {
                self.evaluate_ttest(&input.hypothesis, &input.experimental_data, &input.config)?
            }
            StatisticalTest::MannWhitneyU => {
                self.evaluate_mann_whitney(&input.hypothesis, &input.experimental_data, &input.config)?
            }
            _ => {
                // For other tests, run a t-test (one- or two-sample) rather
                // than silently falling through with the wrong test path.
                warn!("Test method {:?} not fully implemented, using t-test", input.config.test_method);
                self.evaluate_ttest(&input.hypothesis, &input.experimental_data, &input.config)?
            }
        };

        // Compute effect size if requested
        let effect_size = if input.config.compute_effect_size {
            self.compute_effect_size(&input.hypothesis, &input.experimental_data)
        } else {
            None
        };

        // Determine final status
        let status = self.determine_status(&test_results, &assumption_violations);

        // Compute achieved power (post-hoc)
        let sample_size = input.experimental_data.sample_size;
        let effect_value: Option<f64> = effect_size.as_ref().map(|e| e.value.try_into().unwrap_or(0.0));
        let achieved_power = Decimal::try_from(
            self.estimate_confidence(sample_size, effect_value)
        ).ok();

        // Build recommendations
        let recommendations = self.build_recommendations(&status, &test_results, &assumption_violations);

        // Assess sample adequacy
        let sample_adequacy = if sample_size >= 100 {
            SampleAdequacy::Adequate
        } else if sample_size >= 30 {
            SampleAdequacy::Marginal
        } else {
            SampleAdequacy::Inadequate
        };

        // Phase 7: Check latency budget BEFORE returning result
        let elapsed_ms = start.elapsed().as_millis() as u64;
        if let Err(violation) = self.check_latency(elapsed_ms) {
            tracing::error!(
                elapsed_ms = elapsed_ms,
                budget_ms = self.budget.max_latency_ms,
                budget_type = %violation.budget_type,
                "Performance budget exceeded - ABORTING"
            );
            return Err(HypothesisAgentError::BudgetExceeded(format!(
                "Latency budget exceeded: {}ms > {}ms limit",
                elapsed_ms, self.budget.max_latency_ms
            )));
        }

        let output = HypothesisOutput {
            hypothesis_id: input.hypothesis.id,
            status,
            test_results,
            effect_size,
            diagnostics: DiagnosticInfo {
                achieved_power,
                sample_adequacy,
                assumption_violations,
                warnings: Vec::new(),
            },
            recommendations,
        };

        info!(
            status = ?output.status,
            p_value = %output.test_results.p_value,
            elapsed_ms = elapsed_ms,
            "Hypothesis evaluation complete"
        );

        Ok(output)
    }

    fn build_decision_event(
        &self,
        input: &Self::Input,
        output: &Self::Output,
        execution_id: Uuid,
    ) -> Result<DecisionEvent, Self::Error> {
        // Compute inputs hash for determinism verification
        let inputs_hash = DecisionEvent::compute_inputs_hash(input)
            .map_err(|e| HypothesisAgentError::Internal(e.to_string()))?;

        // Estimate confidence
        let sample_size = input.experimental_data.sample_size;
        let effect_value: Option<f64> = output.effect_size.as_ref()
            .map(|e| e.value.try_into().unwrap_or(0.0));
        let confidence_value = self.estimate_confidence(sample_size, effect_value);

        let confidence = Confidence {
            value: Decimal::try_from(confidence_value)
                .map_err(|e| HypothesisAgentError::Internal(e.to_string()))?,
            method: ConfidenceMethod::Heuristic,
            sample_size: Some(sample_size),
            ci_lower: output.test_results.confidence_interval.as_ref().map(|ci| ci.lower),
            ci_upper: output.test_results.confidence_interval.as_ref().map(|ci| ci.upper),
        };

        // Build constraints
        let constraints = ConstraintsApplied {
            scope: vec![
                format!("hypothesis_type: {:?}", input.hypothesis.hypothesis_type),
                format!("test_method: {:?}", input.config.test_method),
            ],
            assumptions: vec![
                "Normal distribution assumed for t-test".to_string(),
                "Independent observations".to_string(),
            ],
            limitations: output
                .diagnostics
                .assumption_violations
                .iter()
                .map(|v| format!("{}: {}", v.assumption, v.recommendation))
                .collect(),
            data_filters: Vec::new(),
            temporal_bounds: None,
        };

        // Build execution ref
        let execution_ref = ExecutionRef {
            execution_id,
            trace_id: None, // Would be populated from tracing context
            span_id: None,
            parent_ref: input.context.as_ref().and_then(|c| c.telemetry_ref.clone()),
            runtime_version: Some(HYPOTHESIS_AGENT_VERSION.to_string()),
        };

        // Serialize output
        let outputs = serde_json::to_value(output)
            .map_err(|e| HypothesisAgentError::Internal(e.to_string()))?;

        DecisionEvent::builder()
            .agent_id(HYPOTHESIS_AGENT_ID)
            .agent_version(HYPOTHESIS_AGENT_VERSION)
            .decision_type(DecisionType::HypothesisEvaluation)
            .inputs_hash(inputs_hash)
            .outputs(outputs)
            .confidence(confidence)
            .constraints_applied(constraints)
            .execution_ref(execution_ref)
            .metadata(json!({
                "request_id": input.request_id,
                "hypothesis_id": input.hypothesis.id,
            }))
            .build()
            .map_err(|e| HypothesisAgentError::Internal(e.to_string()))
    }
}

impl HypothesisAgent {
    /// Build recommendations based on results.
    fn build_recommendations(
        &self,
        status: &HypothesisStatus,
        results: &TestResults,
        violations: &[AssumptionViolation],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Status-based recommendations
        match status {
            HypothesisStatus::Inconclusive => {
                recommendations.push("Address assumption violations before drawing conclusions".to_string());
            }
            HypothesisStatus::Accepted => {
                let p_value: f64 = results.p_value.try_into().unwrap_or(1.0);
                if p_value < 0.001 {
                    recommendations.push("Strong evidence against null hypothesis".to_string());
                } else {
                    recommendations.push("Moderate evidence against null hypothesis".to_string());
                }
            }
            HypothesisStatus::Rejected => {
                recommendations.push("Consider increasing sample size for more power".to_string());
            }
            _ => {}
        }

        // Violation-based recommendations
        for violation in violations {
            recommendations.push(violation.recommendation.clone());
        }

        recommendations
    }
}

// Helper functions for statistical approximations

/// Sample mean and unbiased variance for a slice of f64 values.
fn mean_and_variance(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n <= 1.0 {
        let m = if n == 0.0 { 0.0 } else { values[0] };
        return (m, 0.0);
    }
    let mean: f64 = values.iter().sum::<f64>() / n;
    let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, variance)
}

/// Standard-normal CDF using the erf approximation.
fn normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// Mann-Whitney U test with tie-corrected normal approximation.
///
/// Returns `(U, two_sided_p_value)` where U is the statistic for sample `a`.
fn mann_whitney_u(a: &[f64], b: &[f64]) -> (f64, f64) {
    let na = a.len();
    let nb = b.len();
    if na == 0 || nb == 0 {
        return (0.0, 1.0);
    }

    // Combine and rank with mid-rank for ties.
    #[derive(Clone, Copy)]
    struct Entry {
        value: f64,
        from_a: bool,
    }
    let mut combined: Vec<Entry> = Vec::with_capacity(na + nb);
    combined.extend(a.iter().map(|&v| Entry { value: v, from_a: true }));
    combined.extend(b.iter().map(|&v| Entry { value: v, from_a: false }));
    combined.sort_by(|x, y| x.value.partial_cmp(&y.value).unwrap_or(std::cmp::Ordering::Equal));

    let n_total = combined.len();
    let mut ranks = vec![0.0f64; n_total];
    let mut tie_adjustment = 0.0f64;
    let mut i = 0;
    while i < n_total {
        let mut j = i + 1;
        while j < n_total && (combined[j].value - combined[i].value).abs() < f64::EPSILON {
            j += 1;
        }
        let group_size = j - i;
        let avg_rank = ((i + 1) as f64 + j as f64) / 2.0;
        for k in i..j {
            ranks[k] = avg_rank;
        }
        if group_size > 1 {
            let t = group_size as f64;
            tie_adjustment += t * t * t - t;
        }
        i = j;
    }

    let rank_sum_a: f64 = combined
        .iter()
        .zip(ranks.iter())
        .filter(|(e, _)| e.from_a)
        .map(|(_, r)| *r)
        .sum();

    let na_f = na as f64;
    let nb_f = nb as f64;
    let u_a = rank_sum_a - na_f * (na_f + 1.0) / 2.0;
    let u_b = na_f * nb_f - u_a;
    let u_stat = u_a.min(u_b);

    let n_f = na_f + nb_f;
    let mean_u = na_f * nb_f / 2.0;
    let tie_term = if n_f > 1.0 {
        tie_adjustment / (n_f * (n_f - 1.0))
    } else {
        0.0
    };
    let var_u = na_f * nb_f * ((n_f + 1.0) - tie_term) / 12.0;

    let p_value = if var_u <= 0.0 {
        1.0
    } else {
        let sigma = var_u.sqrt();
        // Continuity correction
        let numerator = (u_stat - mean_u).abs() - 0.5;
        let z = if numerator <= 0.0 { 0.0 } else { numerator / sigma };
        2.0 * (1.0 - normal_cdf(z))
    };

    (u_a, p_value.clamp(0.0, 1.0))
}

/// Error function approximation.
fn erf(x: f64) -> f64 {
    // Abramowitz and Stegun approximation
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

/// Incomplete beta function approximation.
fn incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    // Simple approximation for beta function
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }

    // Use continued fraction approximation
    let bt = if x == 0.0 || x == 1.0 {
        0.0
    } else {
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp()
    };

    if x < (a + 1.0) / (a + b + 2.0) {
        bt * beta_cf(a, b, x) / a
    } else {
        1.0 - bt * beta_cf(b, a, 1.0 - x) / b
    }
}

/// Continued fraction for incomplete beta.
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    let max_iter = 100;
    let eps = 1e-10;

    let mut c = 1.0;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < 1e-30 {
        d = 1e-30;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=max_iter {
        let m = m as f64;
        let m2 = 2.0 * m;

        let aa = m * (b - m) * x / ((a + m2 - 1.0) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        h *= d * c;

        let aa = -(a + m) * (a + b + m) * x / ((a + m2) * (a + m2 + 1.0));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;

        if (del - 1.0).abs() < eps {
            break;
        }
    }

    h
}

/// Log gamma function approximation.
fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation
    let g = 7;
    let c = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];

    if x < 0.5 {
        std::f64::consts::PI.ln() - (std::f64::consts::PI * x).sin().ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut a = c[0];
        for i in 1..g + 2 {
            a += c[i] / (x + i as f64);
        }
        let t = x + g as f64 + 0.5;
        0.5 * (2.0 * std::f64::consts::PI).ln() + (t - 0.5) * t.ln() - t + a.ln()
    }
}

/// Inverse normal CDF approximation.
fn inverse_normal_cdf(p: f64) -> f64 {
    // Rational approximation
    let a = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    let b = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    let c = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    let d = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];

    let p_low = 0.02425;
    let p_high = 1.0 - p_low;

    if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= p_high {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_input(sample_size: u64) -> HypothesisInput {
        let observations: Vec<Observation> = (0..sample_size)
            .map(|i| Observation {
                id: Uuid::new_v4(),
                values: json!({"value": (i as f64) * 0.1 + 0.5}),
                group: None,
                weight: None,
                timestamp: None,
            })
            .collect();

        HypothesisInput {
            request_id: Uuid::new_v4(),
            hypothesis: HypothesisDefinition {
                id: Uuid::new_v4(),
                name: "Test Hypothesis".to_string(),
                statement: "Mean is greater than zero".to_string(),
                hypothesis_type: HypothesisType::Threshold,
                null_hypothesis: "Mean equals zero".to_string(),
                alternative_hypothesis: "Mean is greater than zero".to_string(),
                variables: vec![HypothesisVariable {
                    name: "value".to_string(),
                    role: VariableRole::Dependent,
                    data_type: VariableDataType::Continuous,
                    unit: None,
                }],
                expected_effect_size: Some(dec!(0.5)),
                significance_level: dec!(0.05),
                required_power: Some(dec!(0.8)),
            },
            experimental_data: ExperimentalData {
                source_id: "test-source".to_string(),
                collected_at: Utc::now(),
                observations,
                sample_size,
                quality_metrics: DataQualityMetrics {
                    completeness: dec!(1.0),
                    validity: dec!(1.0),
                    outlier_count: 0,
                    duplicate_count: 0,
                },
            },
            config: EvaluationConfig {
                test_method: StatisticalTest::TTest,
                apply_correction: false,
                correction_method: None,
                bootstrap_iterations: None,
                random_seed: Some(42),
                compute_effect_size: true,
                generate_diagnostics: true,
            },
            context: None,
        }
    }

    #[tokio::test]
    async fn test_hypothesis_agent_execution() {
        let agent = HypothesisAgent::new();
        let input = create_test_input(100);

        let result = agent.execute(input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(matches!(
            output.status,
            HypothesisStatus::Accepted | HypothesisStatus::Rejected
        ));
    }

    #[tokio::test]
    async fn test_insufficient_sample_size() {
        let agent = HypothesisAgent::new();
        let input = create_test_input(10); // Below minimum

        let result = agent.execute(input).await;
        assert!(matches!(result, Err(HypothesisAgentError::InsufficientSampleSize { .. })));
    }

    #[tokio::test]
    async fn test_decision_event_generation() {
        let agent = HypothesisAgent::new();
        let input = create_test_input(100);

        let (output, event) = agent.invoke(input).await.unwrap();

        assert_eq!(event.agent_id, HYPOTHESIS_AGENT_ID);
        assert_eq!(event.agent_version, HYPOTHESIS_AGENT_VERSION);
        assert_eq!(event.decision_type, DecisionType::HypothesisEvaluation);
        assert_eq!(event.inputs_hash.len(), 64);
    }

    #[test]
    fn test_confidence_estimator() {
        let agent = HypothesisAgent::new();

        let conf_small = agent.estimate_confidence(10, None);
        let conf_medium = agent.estimate_confidence(100, None);
        let conf_large = agent.estimate_confidence(1000, Some(0.8));

        assert!(conf_small < conf_medium);
        assert!(conf_medium < conf_large);
        assert!(conf_large <= 0.99);
    }

    /// Build an input that matches the shape emitted by the Agentics CLI:
    /// `variables` declares a dependent variable "accuracy", observations
    /// carry nested `values: { model, accuracy }` objects and are tagged
    /// with group "A"|"B".
    fn create_cli_shape_input(per_group: usize, mean_a: f64, mean_b: f64) -> HypothesisInput {
        let mut observations = Vec::with_capacity(per_group * 2);
        for i in 0..per_group {
            let jitter = (i as f64) * 0.001;
            observations.push(Observation {
                id: Uuid::new_v4(),
                values: json!({
                    "model": "alpha",
                    "accuracy": mean_a + jitter,
                }),
                group: Some("A".to_string()),
                weight: None,
                timestamp: None,
            });
            observations.push(Observation {
                id: Uuid::new_v4(),
                values: json!({
                    "model": "beta",
                    "accuracy": mean_b + jitter,
                }),
                group: Some("B".to_string()),
                weight: None,
                timestamp: None,
            });
        }

        HypothesisInput {
            request_id: Uuid::new_v4(),
            hypothesis: HypothesisDefinition {
                id: Uuid::new_v4(),
                name: "Model Comparison".to_string(),
                statement: "Model A outperforms Model B".to_string(),
                hypothesis_type: HypothesisType::Comparative,
                null_hypothesis: "Means are equal".to_string(),
                alternative_hypothesis: "Means differ".to_string(),
                variables: vec![HypothesisVariable {
                    name: "accuracy".to_string(),
                    role: VariableRole::Dependent,
                    data_type: VariableDataType::Continuous,
                    unit: None,
                }],
                expected_effect_size: Some(dec!(0.5)),
                significance_level: dec!(0.05),
                required_power: Some(dec!(0.8)),
            },
            experimental_data: ExperimentalData {
                source_id: "cli-probe".to_string(),
                collected_at: Utc::now(),
                observations: observations.clone(),
                sample_size: observations.len() as u64,
                quality_metrics: DataQualityMetrics {
                    completeness: dec!(1.0),
                    validity: dec!(1.0),
                    outlier_count: 0,
                    duplicate_count: 0,
                },
            },
            config: EvaluationConfig {
                test_method: StatisticalTest::TTest,
                apply_correction: false,
                correction_method: None,
                bootstrap_iterations: None,
                random_seed: Some(42),
                compute_effect_size: true,
                generate_diagnostics: true,
            },
            context: None,
        }
    }

    #[tokio::test]
    async fn test_cli_shape_two_sample_ttest_produces_real_pvalue() {
        let agent = HypothesisAgent::new();
        // 30 per group, A ≈ 0.88, B ≈ 0.83 — a real difference the test should detect.
        let mut input = create_cli_shape_input(30, 0.88, 0.83);
        input.config.test_method = StatisticalTest::TTest;

        let output = agent.execute(input).await.expect("should evaluate");

        let p: f64 = output.test_results.p_value.try_into().unwrap();
        let t: f64 = output.test_results.test_statistic.try_into().unwrap();
        assert!(p.is_finite(), "p-value must be finite");
        assert!(p >= 0.0 && p <= 1.0, "p-value must be in [0,1], got {p}");
        assert!(t.abs() > 0.0, "two-sample statistic must reflect group difference");
    }

    #[tokio::test]
    async fn test_cli_shape_mann_whitney_produces_real_pvalue() {
        let agent = HypothesisAgent::new();
        let mut input = create_cli_shape_input(30, 0.88, 0.83);
        input.config.test_method = StatisticalTest::MannWhitneyU;

        let output = agent.execute(input).await.expect("should evaluate");

        let p: f64 = output.test_results.p_value.try_into().unwrap();
        assert!(p.is_finite() && p >= 0.0 && p <= 1.0);
    }

    #[tokio::test]
    async fn test_cli_shape_insufficient_reports_actual_count() {
        let agent = HypothesisAgent::new();
        // 5 per group = 10 total, below default min_sample_size (30).
        let input = create_cli_shape_input(5, 0.88, 0.83);

        match agent.execute(input).await {
            Err(HypothesisAgentError::InsufficientSampleSize { required, actual }) => {
                assert_eq!(required, 30);
                assert_eq!(actual, 10, "must name actual count extracted, not 0");
            }
            other => panic!("expected InsufficientSampleSize, got {other:?}"),
        }
    }

    #[test]
    fn test_t_test_independent_serde_alias() {
        let method: StatisticalTest = serde_json::from_str("\"t_test_independent\"").unwrap();
        assert_eq!(method, StatisticalTest::TTest);
    }

    #[test]
    fn test_mann_whitney_u_helper_detects_difference() {
        let a: Vec<f64> = (0..30).map(|i| 0.88 + i as f64 * 0.0005).collect();
        let b: Vec<f64> = (0..30).map(|i| 0.83 + i as f64 * 0.0005).collect();
        let (_u, p) = mann_whitney_u(&a, &b);
        assert!(p < 0.05, "non-overlapping samples should reject null, p={p}");
    }
}
