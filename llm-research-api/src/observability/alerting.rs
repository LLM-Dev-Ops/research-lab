//! Alerting rules and SLO definitions for LLM Research API
//!
//! This module provides enterprise-grade alerting configuration including:
//! - Service Level Objectives (SLOs) for availability and latency
//! - Alert rule definitions for operational monitoring
//! - Error budget tracking and burn rate calculations
//! - Prometheus-compatible alerting rule generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Service Level Objectives (SLOs)
// ============================================================================

/// Represents a Service Level Objective configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLevelObjective {
    /// Name of the SLO (e.g., "api_availability", "experiment_latency")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Target percentage (e.g., 99.9 for 99.9% availability)
    pub target: f64,

    /// Window over which the SLO is measured
    pub window: SloWindow,

    /// The indicator used to measure this SLO
    pub indicator: ServiceLevelIndicator,

    /// Alert thresholds for error budget burn rate
    pub burn_rate_alerts: Vec<BurnRateAlert>,

    /// Labels to apply to alerts from this SLO
    pub labels: HashMap<String, String>,
}

/// Time window for SLO measurement
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SloWindow {
    /// Rolling 1-day window
    OneDay,
    /// Rolling 7-day window
    SevenDays,
    /// Rolling 28-day window
    TwentyEightDays,
    /// Rolling 30-day window
    ThirtyDays,
    /// Calendar month
    CalendarMonth,
}

impl SloWindow {
    /// Get duration in seconds
    pub fn duration_seconds(&self) -> u64 {
        match self {
            SloWindow::OneDay => 86_400,
            SloWindow::SevenDays => 604_800,
            SloWindow::TwentyEightDays => 2_419_200,
            SloWindow::ThirtyDays => 2_592_000,
            SloWindow::CalendarMonth => 2_592_000, // Approximate
        }
    }

    /// Get window as Duration
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs(self.duration_seconds())
    }
}

/// Service Level Indicator - what metric is being measured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceLevelIndicator {
    /// Availability: successful requests / total requests
    Availability {
        /// Metric name for total requests
        total_metric: String,
        /// Metric name for successful requests
        success_metric: String,
        /// Label filters (e.g., service="api")
        labels: HashMap<String, String>,
    },

    /// Latency: requests faster than threshold / total requests
    Latency {
        /// Histogram metric name
        histogram_metric: String,
        /// Threshold in seconds
        threshold_seconds: f64,
        /// Label filters
        labels: HashMap<String, String>,
    },

    /// Quality: requests meeting quality bar / total requests
    Quality {
        /// Metric for quality events
        quality_metric: String,
        /// Metric for total events
        total_metric: String,
        /// Label filters
        labels: HashMap<String, String>,
    },
}

/// Burn rate alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRateAlert {
    /// Name of this alert tier
    pub name: String,

    /// Severity level
    pub severity: AlertSeverity,

    /// Short window burn rate threshold (e.g., 14.4x for 1-hour window)
    pub short_burn_rate: f64,

    /// Short window duration
    pub short_window: Duration,

    /// Long window burn rate threshold (e.g., 14.4x for 1-hour equivalent)
    pub long_burn_rate: f64,

    /// Long window duration
    pub long_window: Duration,

    /// Percentage of error budget consumed at this rate over the SLO window
    pub budget_consumption_threshold: f64,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertSeverity {
    /// Critical: immediate action required (pages on-call)
    Critical,
    /// Warning: investigate soon but not emergency
    Warning,
    /// Info: informational, may indicate degradation
    Info,
    /// Notice: for visibility, no action typically needed
    Notice,
}

impl AlertSeverity {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Critical => "critical",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Info => "info",
            AlertSeverity::Notice => "notice",
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Alert Rules
// ============================================================================

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Name of the alert
    pub name: String,

    /// PromQL expression for the alert condition
    pub expression: String,

    /// Duration the condition must be true before firing
    pub for_duration: Duration,

    /// Severity of this alert
    pub severity: AlertSeverity,

    /// Summary annotation (supports templating)
    pub summary: String,

    /// Description annotation (supports templating)
    pub description: String,

    /// Runbook URL for responders
    pub runbook_url: Option<String>,

    /// Additional labels
    pub labels: HashMap<String, String>,

    /// Additional annotations
    pub annotations: HashMap<String, String>,
}

/// Collection of alert rules organized by category
#[derive(Debug, Clone, Default)]
pub struct AlertRuleSet {
    /// SLO-based alerts
    pub slo_alerts: Vec<AlertRule>,

    /// Infrastructure alerts (CPU, memory, disk)
    pub infrastructure_alerts: Vec<AlertRule>,

    /// Database alerts (connections, query latency)
    pub database_alerts: Vec<AlertRule>,

    /// Application alerts (error rates, specific failures)
    pub application_alerts: Vec<AlertRule>,

    /// Security alerts (auth failures, suspicious activity)
    pub security_alerts: Vec<AlertRule>,

    /// Business metric alerts (experiment failures, etc.)
    pub business_alerts: Vec<AlertRule>,
}

impl AlertRuleSet {
    /// Create a new empty rule set
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an SLO-based alert
    pub fn add_slo_alert(mut self, rule: AlertRule) -> Self {
        self.slo_alerts.push(rule);
        self
    }

    /// Add an infrastructure alert
    pub fn add_infrastructure_alert(mut self, rule: AlertRule) -> Self {
        self.infrastructure_alerts.push(rule);
        self
    }

    /// Add a database alert
    pub fn add_database_alert(mut self, rule: AlertRule) -> Self {
        self.database_alerts.push(rule);
        self
    }

    /// Add an application alert
    pub fn add_application_alert(mut self, rule: AlertRule) -> Self {
        self.application_alerts.push(rule);
        self
    }

    /// Add a security alert
    pub fn add_security_alert(mut self, rule: AlertRule) -> Self {
        self.security_alerts.push(rule);
        self
    }

    /// Add a business alert
    pub fn add_business_alert(mut self, rule: AlertRule) -> Self {
        self.business_alerts.push(rule);
        self
    }

    /// Get all alerts as a flat list
    pub fn all_alerts(&self) -> Vec<&AlertRule> {
        let mut all = Vec::new();
        all.extend(self.slo_alerts.iter());
        all.extend(self.infrastructure_alerts.iter());
        all.extend(self.database_alerts.iter());
        all.extend(self.application_alerts.iter());
        all.extend(self.security_alerts.iter());
        all.extend(self.business_alerts.iter());
        all
    }

    /// Generate Prometheus alerting rules YAML
    pub fn to_prometheus_yaml(&self) -> String {
        let mut yaml = String::new();
        yaml.push_str("groups:\n");

        // SLO alerts group
        if !self.slo_alerts.is_empty() {
            yaml.push_str("  - name: slo_alerts\n");
            yaml.push_str("    rules:\n");
            for rule in &self.slo_alerts {
                yaml.push_str(&rule.to_prometheus_yaml(6));
            }
        }

        // Infrastructure alerts group
        if !self.infrastructure_alerts.is_empty() {
            yaml.push_str("  - name: infrastructure_alerts\n");
            yaml.push_str("    rules:\n");
            for rule in &self.infrastructure_alerts {
                yaml.push_str(&rule.to_prometheus_yaml(6));
            }
        }

        // Database alerts group
        if !self.database_alerts.is_empty() {
            yaml.push_str("  - name: database_alerts\n");
            yaml.push_str("    rules:\n");
            for rule in &self.database_alerts {
                yaml.push_str(&rule.to_prometheus_yaml(6));
            }
        }

        // Application alerts group
        if !self.application_alerts.is_empty() {
            yaml.push_str("  - name: application_alerts\n");
            yaml.push_str("    rules:\n");
            for rule in &self.application_alerts {
                yaml.push_str(&rule.to_prometheus_yaml(6));
            }
        }

        // Security alerts group
        if !self.security_alerts.is_empty() {
            yaml.push_str("  - name: security_alerts\n");
            yaml.push_str("    rules:\n");
            for rule in &self.security_alerts {
                yaml.push_str(&rule.to_prometheus_yaml(6));
            }
        }

        // Business alerts group
        if !self.business_alerts.is_empty() {
            yaml.push_str("  - name: business_alerts\n");
            yaml.push_str("    rules:\n");
            for rule in &self.business_alerts {
                yaml.push_str(&rule.to_prometheus_yaml(6));
            }
        }

        yaml
    }
}

impl AlertRule {
    /// Generate Prometheus-format YAML for this rule
    pub fn to_prometheus_yaml(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let mut yaml = String::new();

        yaml.push_str(&format!("{pad}- alert: {}\n", self.name));
        yaml.push_str(&format!("{pad}  expr: {}\n", self.expression));

        if self.for_duration > Duration::ZERO {
            yaml.push_str(&format!("{pad}  for: {}s\n", self.for_duration.as_secs()));
        }

        // Labels
        yaml.push_str(&format!("{pad}  labels:\n"));
        yaml.push_str(&format!("{pad}    severity: {}\n", self.severity));
        for (key, value) in &self.labels {
            yaml.push_str(&format!("{pad}    {}: \"{}\"\n", key, value));
        }

        // Annotations
        yaml.push_str(&format!("{pad}  annotations:\n"));
        yaml.push_str(&format!("{pad}    summary: \"{}\"\n", self.summary));
        yaml.push_str(&format!("{pad}    description: \"{}\"\n", self.description));
        if let Some(ref runbook) = self.runbook_url {
            yaml.push_str(&format!("{pad}    runbook_url: \"{}\"\n", runbook));
        }
        for (key, value) in &self.annotations {
            yaml.push_str(&format!("{pad}    {}: \"{}\"\n", key, value));
        }

        yaml
    }
}

// ============================================================================
// Error Budget Tracking
// ============================================================================

/// Error budget state for an SLO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudget {
    /// Name of the associated SLO
    pub slo_name: String,

    /// Total error budget (1 - target) as a ratio
    pub total_budget: f64,

    /// Currently consumed budget as a ratio
    pub consumed_budget: f64,

    /// Remaining budget as a ratio
    pub remaining_budget: f64,

    /// Current burn rate (how fast budget is being consumed)
    pub current_burn_rate: f64,

    /// Time until budget exhaustion at current burn rate
    pub time_until_exhaustion: Option<Duration>,

    /// Start of the current window
    pub window_start: chrono::DateTime<chrono::Utc>,

    /// End of the current window
    pub window_end: chrono::DateTime<chrono::Utc>,
}

impl ErrorBudget {
    /// Create a new error budget from SLO and current metrics
    pub fn from_slo(
        slo: &ServiceLevelObjective,
        current_good_events: u64,
        current_total_events: u64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let target = slo.target / 100.0; // Convert percentage to ratio
        let total_budget = 1.0 - target;

        // Calculate current success rate
        let current_success_rate = if current_total_events > 0 {
            current_good_events as f64 / current_total_events as f64
        } else {
            1.0 // No events = perfect success
        };

        // Calculate consumed budget
        let consumed_budget = if current_success_rate < target {
            // We're under target, calculate how much budget consumed
            (target - current_success_rate) / total_budget
        } else {
            0.0 // Under budget
        };

        let remaining_budget = (total_budget - consumed_budget * total_budget).max(0.0);

        // Calculate burn rate (normalized to 1.0 = exact budget consumption rate)
        let window_seconds = slo.window.duration_seconds() as f64;
        let elapsed_seconds = (now - (now - chrono::Duration::seconds(slo.window.duration_seconds() as i64)))
            .num_seconds() as f64;

        let expected_consumption = elapsed_seconds / window_seconds * total_budget;
        let current_burn_rate = if expected_consumption > 0.0 {
            consumed_budget * total_budget / expected_consumption
        } else {
            0.0
        };

        // Time until exhaustion
        let time_until_exhaustion = if current_burn_rate > 0.0 && remaining_budget > 0.0 {
            let seconds_remaining = remaining_budget / (current_burn_rate * total_budget / window_seconds);
            Some(Duration::from_secs_f64(seconds_remaining))
        } else {
            None
        };

        let window_start = now - chrono::Duration::seconds(slo.window.duration_seconds() as i64);
        let window_end = now;

        Self {
            slo_name: slo.name.clone(),
            total_budget,
            consumed_budget,
            remaining_budget,
            current_burn_rate,
            time_until_exhaustion,
            window_start,
            window_end,
        }
    }

    /// Check if error budget is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.remaining_budget <= 0.0
    }

    /// Get remaining budget as a percentage
    pub fn remaining_percentage(&self) -> f64 {
        (self.remaining_budget / self.total_budget) * 100.0
    }
}

// ============================================================================
// Pre-configured Alert Rules
// ============================================================================

/// Create default SLOs for the LLM Research API
pub fn default_slos() -> Vec<ServiceLevelObjective> {
    vec![
        // API Availability SLO: 99.9% availability over 30 days
        ServiceLevelObjective {
            name: "api_availability".to_string(),
            description: "API should be available for 99.9% of requests".to_string(),
            target: 99.9,
            window: SloWindow::ThirtyDays,
            indicator: ServiceLevelIndicator::Availability {
                total_metric: "http_requests_total".to_string(),
                success_metric: "http_requests_total{status!~\"5..\"}".to_string(),
                labels: [("service".to_string(), "llm-research-api".to_string())]
                    .into_iter()
                    .collect(),
            },
            burn_rate_alerts: vec![
                // Page immediately: 14.4x burn rate over 1 hour
                BurnRateAlert {
                    name: "api_availability_critical".to_string(),
                    severity: AlertSeverity::Critical,
                    short_burn_rate: 14.4,
                    short_window: Duration::from_secs(3600), // 1 hour
                    long_burn_rate: 14.4,
                    long_window: Duration::from_secs(3600),
                    budget_consumption_threshold: 2.0, // 2% in 1 hour
                },
                // Warning: 6x burn rate over 6 hours
                BurnRateAlert {
                    name: "api_availability_warning".to_string(),
                    severity: AlertSeverity::Warning,
                    short_burn_rate: 6.0,
                    short_window: Duration::from_secs(21600), // 6 hours
                    long_burn_rate: 6.0,
                    long_window: Duration::from_secs(21600),
                    budget_consumption_threshold: 5.0, // 5% in 6 hours
                },
            ],
            labels: [
                ("team".to_string(), "platform".to_string()),
                ("tier".to_string(), "1".to_string()),
            ]
            .into_iter()
            .collect(),
        },
        // API Latency SLO: 99% of requests under 200ms over 7 days
        ServiceLevelObjective {
            name: "api_latency_p99".to_string(),
            description: "99% of API requests should complete in under 200ms".to_string(),
            target: 99.0,
            window: SloWindow::SevenDays,
            indicator: ServiceLevelIndicator::Latency {
                histogram_metric: "http_request_duration_seconds_bucket".to_string(),
                threshold_seconds: 0.2,
                labels: [("service".to_string(), "llm-research-api".to_string())]
                    .into_iter()
                    .collect(),
            },
            burn_rate_alerts: vec![
                BurnRateAlert {
                    name: "api_latency_critical".to_string(),
                    severity: AlertSeverity::Critical,
                    short_burn_rate: 14.4,
                    short_window: Duration::from_secs(3600),
                    long_burn_rate: 14.4,
                    long_window: Duration::from_secs(3600),
                    budget_consumption_threshold: 2.0,
                },
                BurnRateAlert {
                    name: "api_latency_warning".to_string(),
                    severity: AlertSeverity::Warning,
                    short_burn_rate: 6.0,
                    short_window: Duration::from_secs(21600),
                    long_burn_rate: 6.0,
                    long_window: Duration::from_secs(21600),
                    budget_consumption_threshold: 5.0,
                },
            ],
            labels: [
                ("team".to_string(), "platform".to_string()),
                ("tier".to_string(), "1".to_string()),
            ]
            .into_iter()
            .collect(),
        },
        // Database Query Latency SLO: 99.5% of queries under 50ms
        ServiceLevelObjective {
            name: "db_query_latency".to_string(),
            description: "99.5% of database queries should complete in under 50ms".to_string(),
            target: 99.5,
            window: SloWindow::SevenDays,
            indicator: ServiceLevelIndicator::Latency {
                histogram_metric: "db_query_duration_seconds_bucket".to_string(),
                threshold_seconds: 0.05,
                labels: [("db".to_string(), "postgres".to_string())]
                    .into_iter()
                    .collect(),
            },
            burn_rate_alerts: vec![
                BurnRateAlert {
                    name: "db_latency_warning".to_string(),
                    severity: AlertSeverity::Warning,
                    short_burn_rate: 6.0,
                    short_window: Duration::from_secs(3600),
                    long_burn_rate: 6.0,
                    long_window: Duration::from_secs(3600),
                    budget_consumption_threshold: 2.0,
                },
            ],
            labels: [("team".to_string(), "platform".to_string())]
                .into_iter()
                .collect(),
        },
        // Experiment Success Rate SLO: 95% of experiments should complete successfully
        ServiceLevelObjective {
            name: "experiment_success_rate".to_string(),
            description: "95% of experiments should complete successfully".to_string(),
            target: 95.0,
            window: SloWindow::SevenDays,
            indicator: ServiceLevelIndicator::Quality {
                quality_metric: "experiments_completed_total{status=\"success\"}".to_string(),
                total_metric: "experiments_completed_total".to_string(),
                labels: HashMap::new(),
            },
            burn_rate_alerts: vec![
                BurnRateAlert {
                    name: "experiment_success_warning".to_string(),
                    severity: AlertSeverity::Warning,
                    short_burn_rate: 3.0,
                    short_window: Duration::from_secs(21600),
                    long_burn_rate: 3.0,
                    long_window: Duration::from_secs(21600),
                    budget_consumption_threshold: 10.0,
                },
            ],
            labels: [("team".to_string(), "ml-platform".to_string())]
                .into_iter()
                .collect(),
        },
    ]
}

/// Create default alert rules for the LLM Research API
pub fn default_alert_rules() -> AlertRuleSet {
    AlertRuleSet::new()
        // Infrastructure alerts
        .add_infrastructure_alert(AlertRule {
            name: "HighCPUUsage".to_string(),
            expression: "avg(rate(process_cpu_seconds_total[5m])) > 0.8".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            summary: "High CPU usage detected".to_string(),
            description: "CPU usage has been above 80% for more than 5 minutes".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/high-cpu".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        .add_infrastructure_alert(AlertRule {
            name: "HighMemoryUsage".to_string(),
            expression: "process_resident_memory_bytes / system_memory_total_bytes > 0.85".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            summary: "High memory usage detected".to_string(),
            description: "Memory usage has been above 85% for more than 5 minutes".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/high-memory".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        // Database alerts
        .add_database_alert(AlertRule {
            name: "DatabaseConnectionPoolExhausted".to_string(),
            expression: "db_pool_connections_idle / db_pool_connections_max < 0.1".to_string(),
            for_duration: Duration::from_secs(60),
            severity: AlertSeverity::Critical,
            summary: "Database connection pool nearly exhausted".to_string(),
            description: "Less than 10% of database connections are available".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/db-pool".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        .add_database_alert(AlertRule {
            name: "DatabaseQueryLatencyHigh".to_string(),
            expression: "histogram_quantile(0.99, rate(db_query_duration_seconds_bucket[5m])) > 0.5".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            summary: "Database query latency is high".to_string(),
            description: "P99 database query latency is above 500ms".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/db-latency".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        // Application alerts
        .add_application_alert(AlertRule {
            name: "HighErrorRate".to_string(),
            expression: "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m])) > 0.01".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            summary: "High HTTP error rate".to_string(),
            description: "More than 1% of requests are resulting in 5xx errors".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/high-error-rate".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        .add_application_alert(AlertRule {
            name: "HighRequestLatency".to_string(),
            expression: "histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 1".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            summary: "High request latency".to_string(),
            description: "P99 request latency is above 1 second".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/high-latency".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        .add_application_alert(AlertRule {
            name: "ServiceDown".to_string(),
            expression: "up{job=\"llm-research-api\"} == 0".to_string(),
            for_duration: Duration::from_secs(60),
            severity: AlertSeverity::Critical,
            summary: "Service is down".to_string(),
            description: "The LLM Research API service is not responding".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/service-down".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        // Security alerts
        .add_security_alert(AlertRule {
            name: "HighAuthFailureRate".to_string(),
            expression: "sum(rate(auth_failures_total[5m])) > 10".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            summary: "High authentication failure rate".to_string(),
            description: "More than 10 authentication failures per second over 5 minutes".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/auth-failures".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        .add_security_alert(AlertRule {
            name: "RateLimitExceeded".to_string(),
            expression: "sum(rate(rate_limit_exceeded_total[5m])) > 100".to_string(),
            for_duration: Duration::from_secs(60),
            severity: AlertSeverity::Info,
            summary: "Rate limit frequently exceeded".to_string(),
            description: "Rate limits are being hit frequently, may indicate abuse".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/rate-limit".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        // Business alerts
        .add_business_alert(AlertRule {
            name: "ExperimentFailureSpike".to_string(),
            expression: "sum(rate(experiments_completed_total{status=\"failed\"}[1h])) / sum(rate(experiments_completed_total[1h])) > 0.1".to_string(),
            for_duration: Duration::from_secs(900),
            severity: AlertSeverity::Warning,
            summary: "Experiment failure rate is elevated".to_string(),
            description: "More than 10% of experiments are failing".to_string(),
            runbook_url: Some("https://docs.example.com/runbooks/experiment-failures".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
        .add_business_alert(AlertRule {
            name: "NoExperimentsRunning".to_string(),
            expression: "sum(experiments_running) == 0".to_string(),
            for_duration: Duration::from_secs(3600),
            severity: AlertSeverity::Info,
            summary: "No experiments running".to_string(),
            description: "No experiments have been running for over an hour".to_string(),
            runbook_url: None,
            labels: HashMap::new(),
            annotations: HashMap::new(),
        })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slo_window_duration() {
        assert_eq!(SloWindow::OneDay.duration_seconds(), 86_400);
        assert_eq!(SloWindow::SevenDays.duration_seconds(), 604_800);
        assert_eq!(SloWindow::ThirtyDays.duration_seconds(), 2_592_000);
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Critical.as_str(), "critical");
        assert_eq!(AlertSeverity::Warning.as_str(), "warning");
        assert_eq!(AlertSeverity::Info.as_str(), "info");
        assert_eq!(AlertSeverity::Notice.as_str(), "notice");
    }

    #[test]
    fn test_default_slos() {
        let slos = default_slos();
        assert!(!slos.is_empty());

        let availability_slo = slos.iter().find(|s| s.name == "api_availability");
        assert!(availability_slo.is_some());
        assert_eq!(availability_slo.unwrap().target, 99.9);
    }

    #[test]
    fn test_default_alert_rules() {
        let rules = default_alert_rules();
        assert!(!rules.infrastructure_alerts.is_empty());
        assert!(!rules.database_alerts.is_empty());
        assert!(!rules.application_alerts.is_empty());
        assert!(!rules.security_alerts.is_empty());
        assert!(!rules.business_alerts.is_empty());
    }

    #[test]
    fn test_alert_rule_to_yaml() {
        let rule = AlertRule {
            name: "TestAlert".to_string(),
            expression: "up == 0".to_string(),
            for_duration: Duration::from_secs(60),
            severity: AlertSeverity::Critical,
            summary: "Test alert".to_string(),
            description: "This is a test".to_string(),
            runbook_url: Some("https://example.com/runbook".to_string()),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };

        let yaml = rule.to_prometheus_yaml(0);
        assert!(yaml.contains("alert: TestAlert"));
        assert!(yaml.contains("expr: up == 0"));
        assert!(yaml.contains("for: 60s"));
        assert!(yaml.contains("severity: critical"));
    }

    #[test]
    fn test_alert_rule_set_to_yaml() {
        let rules = default_alert_rules();
        let yaml = rules.to_prometheus_yaml();

        assert!(yaml.contains("groups:"));
        assert!(yaml.contains("infrastructure_alerts"));
        assert!(yaml.contains("database_alerts"));
        assert!(yaml.contains("application_alerts"));
    }

    #[test]
    fn test_error_budget_calculation() {
        let slo = ServiceLevelObjective {
            name: "test_slo".to_string(),
            description: "Test SLO".to_string(),
            target: 99.0,
            window: SloWindow::SevenDays,
            indicator: ServiceLevelIndicator::Availability {
                total_metric: "total".to_string(),
                success_metric: "success".to_string(),
                labels: HashMap::new(),
            },
            burn_rate_alerts: vec![],
            labels: HashMap::new(),
        };

        let now = chrono::Utc::now();
        let budget = ErrorBudget::from_slo(&slo, 990, 1000, now);

        // 99% success rate, target is 99%, so we're exactly on budget
        // Use approximate comparison for floating point
        assert!((budget.total_budget - 0.01).abs() < 1e-10); // 1% error budget
        assert!(!budget.is_exhausted());
    }

    #[test]
    fn test_error_budget_exhausted() {
        let slo = ServiceLevelObjective {
            name: "test_slo".to_string(),
            description: "Test SLO".to_string(),
            target: 99.9,
            window: SloWindow::ThirtyDays,
            indicator: ServiceLevelIndicator::Availability {
                total_metric: "total".to_string(),
                success_metric: "success".to_string(),
                labels: HashMap::new(),
            },
            burn_rate_alerts: vec![],
            labels: HashMap::new(),
        };

        let now = chrono::Utc::now();
        // 98% success rate with 99.9% target = budget exceeded
        let budget = ErrorBudget::from_slo(&slo, 980, 1000, now);

        assert!(budget.consumed_budget > 0.0);
        assert!(budget.remaining_percentage() < 100.0);
    }
}
