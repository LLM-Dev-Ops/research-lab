//! Extended health check functionality for the LLM Research API.
//!
//! This module extends the basic health check system with:
//! - Detailed dependency health tracking
//! - Health aggregation across multiple checks
//! - Deep/recursive dependency checking
//! - Periodic health check scheduling
//! - Health history tracking
//! - Alert integration hooks
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::reliability::health_ext::*;
//! use std::time::Duration;
//!
//! # async fn example() {
//! let mut aggregator = HealthAggregator::new();
//!
//! // Add dependency checks
//! // aggregator.add_dependency(db_health);
//! // aggregator.add_dependency(cache_health);
//!
//! // Get aggregated health status
//! // let health = aggregator.check_all().await;
//! # }
//! ```

use crate::observability::health::{
    ComponentHealth, HealthCheck, HealthCheckConfig, HealthStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Detailed health information for a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealth {
    /// Name of the dependency
    pub name: String,
    /// Current health status
    pub status: HealthStatus,
    /// Detailed message
    pub message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// When this check was performed
    pub checked_at: DateTime<Utc>,
    /// Dependency version or build info
    pub version: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl DependencyHealth {
    /// Creates a new dependency health entry
    pub fn new(name: impl Into<String>, status: HealthStatus) -> Self {
        Self {
            name: name.into(),
            status,
            message: None,
            response_time_ms: None,
            checked_at: Utc::now(),
            version: None,
            metadata: HashMap::new(),
        }
    }

    /// Sets the message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Sets the response time
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }

    /// Sets the version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Adds metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl From<ComponentHealth> for DependencyHealth {
    fn from(component: ComponentHealth) -> Self {
        Self {
            name: component.name,
            status: component.status,
            message: component.message,
            response_time_ms: component.latency_ms,
            checked_at: component.last_check.unwrap_or_else(Utc::now),
            version: None,
            metadata: HashMap::new(),
        }
    }
}

/// Aggregates health checks from multiple sources
pub struct HealthAggregator {
    dependencies: Vec<Arc<dyn HealthCheck>>,
    weights: HashMap<String, f64>,
}

impl HealthAggregator {
    /// Creates a new health aggregator
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
            weights: HashMap::new(),
        }
    }

    /// Adds a dependency to monitor
    pub fn add_dependency(mut self, check: Arc<dyn HealthCheck>) -> Self {
        let name = check.name().to_string();
        self.dependencies.push(check);
        self.weights.insert(name, 1.0);
        self
    }

    /// Sets the weight for a dependency (for weighted health calculation)
    pub fn with_weight(mut self, name: impl Into<String>, weight: f64) -> Self {
        self.weights.insert(name.into(), weight);
        self
    }

    /// Checks all dependencies and returns their health
    pub async fn check_all(&self) -> Vec<DependencyHealth> {
        let mut results = Vec::new();

        // Run all checks concurrently
        let futures: Vec<_> = self
            .dependencies
            .iter()
            .map(|check| {
                let check = Arc::clone(check);
                async move { check.check().await }
            })
            .collect();

        let component_healths = futures::future::join_all(futures).await;

        for component_health in component_healths {
            results.push(DependencyHealth::from(component_health));
        }

        results
    }

    /// Computes a weighted health score (0.0 = unhealthy, 1.0 = healthy)
    pub async fn health_score(&self) -> f64 {
        let healths = self.check_all().await;

        if healths.is_empty() {
            return 1.0;
        }

        let mut total_weight = 0.0;
        let mut weighted_score = 0.0;

        for health in &healths {
            let weight = self.weights.get(&health.name).copied().unwrap_or(1.0);
            total_weight += weight;

            let score = match health.status {
                HealthStatus::Healthy => 1.0,
                HealthStatus::Degraded => 0.5,
                HealthStatus::Unhealthy => 0.0,
            };

            weighted_score += score * weight;
        }

        if total_weight > 0.0 {
            weighted_score / total_weight
        } else {
            1.0
        }
    }

    /// Gets the overall health status based on aggregated checks
    pub async fn overall_status(&self) -> HealthStatus {
        let score = self.health_score().await;

        if score >= 0.9 {
            HealthStatus::Healthy
        } else if score >= 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }
}

impl Default for HealthAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Deep health check that recursively checks dependencies
pub struct DeepHealthCheck {
    name: String,
    config: HealthCheckConfig,
    check_fn: Arc<dyn Fn() -> futures::future::BoxFuture<'static, ComponentHealth> + Send + Sync>,
    dependencies: Vec<Arc<DeepHealthCheck>>,
}

impl DeepHealthCheck {
    /// Creates a new deep health check
    pub fn new<F, Fut>(name: impl Into<String>, config: HealthCheckConfig, check_fn: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: futures::Future<Output = ComponentHealth> + Send + 'static,
    {
        let check_fn = Arc::new(move || Box::pin(check_fn()) as futures::future::BoxFuture<'static, ComponentHealth>);

        Self {
            name: name.into(),
            config,
            check_fn,
            dependencies: Vec::new(),
        }
    }

    /// Adds a dependency to this health check
    pub fn with_dependency(mut self, dependency: Arc<DeepHealthCheck>) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Recursively checks this component and all its dependencies
    pub fn check_deep(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<DependencyHealth>> + Send + '_>> {
        Box::pin(async move {
            let mut results = Vec::new();

            // Check this component
            let component_health = (self.check_fn)().await;
            results.push(DependencyHealth::from(component_health));

            // Recursively check dependencies
            for dep in &self.dependencies {
                let dep_results = dep.check_deep().await;
                results.extend(dep_results);
            }

            results
        })
    }
}

#[async_trait]
impl HealthCheck for DeepHealthCheck {
    async fn check(&self) -> ComponentHealth {
        (self.check_fn)().await
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}

/// Historical health data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthHistoryEntry {
    /// When this check occurred
    pub timestamp: DateTime<Utc>,
    /// The health status at this time
    pub status: HealthStatus,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Optional message
    pub message: Option<String>,
}

/// Tracks health check history over time
pub struct HealthHistory {
    component_name: String,
    max_entries: usize,
    history: Arc<RwLock<VecDeque<HealthHistoryEntry>>>,
}

impl HealthHistory {
    /// Creates a new health history tracker
    pub fn new(component_name: impl Into<String>, max_entries: usize) -> Self {
        Self {
            component_name: component_name.into(),
            max_entries,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
        }
    }

    /// Records a health check result
    pub async fn record(&self, health: &ComponentHealth) {
        let entry = HealthHistoryEntry {
            timestamp: health.last_check.unwrap_or_else(Utc::now),
            status: health.status,
            response_time_ms: health.latency_ms,
            message: health.message.clone(),
        };

        let mut history = self.history.write().await;

        if history.len() >= self.max_entries {
            history.pop_front();
        }

        history.push_back(entry);
    }

    /// Gets all history entries
    pub async fn get_all(&self) -> Vec<HealthHistoryEntry> {
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }

    /// Gets the most recent N entries
    pub async fn get_recent(&self, n: usize) -> Vec<HealthHistoryEntry> {
        let history = self.history.read().await;
        history.iter().rev().take(n).cloned().collect()
    }

    /// Calculates uptime percentage over the history
    pub async fn uptime_percentage(&self) -> f64 {
        let history = self.history.read().await;

        if history.is_empty() {
            return 100.0;
        }

        let healthy_count = history
            .iter()
            .filter(|e| matches!(e.status, HealthStatus::Healthy))
            .count();

        (healthy_count as f64 / history.len() as f64) * 100.0
    }

    /// Gets average response time
    pub async fn average_response_time(&self) -> Option<f64> {
        let history = self.history.read().await;

        let times: Vec<u64> = history
            .iter()
            .filter_map(|e| e.response_time_ms)
            .collect();

        if times.is_empty() {
            None
        } else {
            let sum: u64 = times.iter().sum();
            Some(sum as f64 / times.len() as f64)
        }
    }

    /// Clears all history
    pub async fn clear(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning that requires attention
    Warning,
    /// Critical issue requiring immediate action
    Critical,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Component that triggered the alert
    pub component: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// When the alert was triggered
    pub triggered_at: DateTime<Utc>,
    /// Current health status
    pub status: HealthStatus,
}

/// Alert handler trait
#[async_trait]
pub trait AlertHandler: Send + Sync {
    /// Handles a health alert
    async fn handle_alert(&self, alert: HealthAlert);
}

/// Logs alerts using the tracing system
pub struct LoggingAlertHandler;

#[async_trait]
impl AlertHandler for LoggingAlertHandler {
    async fn handle_alert(&self, alert: HealthAlert) {
        match alert.severity {
            AlertSeverity::Info => {
                info!("Health alert for {}: {}", alert.component, alert.message);
            }
            AlertSeverity::Warning => {
                warn!("Health alert for {}: {}", alert.component, alert.message);
            }
            AlertSeverity::Critical => {
                error!("CRITICAL health alert for {}: {}", alert.component, alert.message);
            }
        }
    }
}

/// Scheduler for periodic health checks
pub struct HealthCheckScheduler {
    checks: Vec<Arc<dyn HealthCheck>>,
    interval: Duration,
    alert_handlers: Vec<Arc<dyn AlertHandler>>,
    history_trackers: HashMap<String, Arc<HealthHistory>>,
}

impl HealthCheckScheduler {
    /// Creates a new scheduler
    pub fn new(interval: Duration) -> Self {
        Self {
            checks: Vec::new(),
            interval,
            alert_handlers: Vec::new(),
            history_trackers: HashMap::new(),
        }
    }

    /// Adds a health check to schedule
    pub fn add_check(mut self, check: Arc<dyn HealthCheck>) -> Self {
        let name = check.name().to_string();
        self.checks.push(check);

        // Create history tracker for this check
        let history = Arc::new(HealthHistory::new(name.clone(), 100));
        self.history_trackers.insert(name, history);

        self
    }

    /// Adds an alert handler
    pub fn add_alert_handler(mut self, handler: Arc<dyn AlertHandler>) -> Self {
        self.alert_handlers.push(handler);
        self
    }

    /// Runs the scheduler (blocks until cancelled)
    pub async fn run(self: Arc<Self>) {
        let mut ticker = interval(self.interval);

        loop {
            ticker.tick().await;
            self.run_checks().await;
        }
    }

    /// Runs all checks once
    async fn run_checks(&self) {
        for check in &self.checks {
            let health = check.check().await;
            let name = check.name();

            // Record in history
            if let Some(history) = self.history_trackers.get(name) {
                history.record(&health).await;
            }

            // Check if we should alert
            if health.status != HealthStatus::Healthy {
                let severity = match health.status {
                    HealthStatus::Degraded => AlertSeverity::Warning,
                    HealthStatus::Unhealthy => AlertSeverity::Critical,
                    _ => AlertSeverity::Info,
                };

                let alert = HealthAlert {
                    component: name.to_string(),
                    severity,
                    message: health.message.unwrap_or_else(|| "Health check failed".to_string()),
                    triggered_at: Utc::now(),
                    status: health.status,
                };

                // Notify all alert handlers
                for handler in &self.alert_handlers {
                    handler.handle_alert(alert.clone()).await;
                }
            }
        }
    }

    /// Gets history for a component
    pub fn get_history(&self, component: &str) -> Option<Arc<HealthHistory>> {
        self.history_trackers.get(component).map(Arc::clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    // Mock health check for testing
    struct MockHealthCheck {
        name: String,
        status: HealthStatus,
        config: HealthCheckConfig,
    }

    #[async_trait]
    impl HealthCheck for MockHealthCheck {
        async fn check(&self) -> ComponentHealth {
            ComponentHealth {
                name: self.name.clone(),
                status: self.status,
                message: None,
                latency_ms: Some(10),
                last_check: Some(Utc::now()),
            }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn config(&self) -> &HealthCheckConfig {
            &self.config
        }
    }

    #[test]
    fn test_dependency_health_new() {
        let health = DependencyHealth::new("test", HealthStatus::Healthy);
        assert_eq!(health.name, "test");
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.message.is_none());
    }

    #[test]
    fn test_dependency_health_builders() {
        let health = DependencyHealth::new("test", HealthStatus::Healthy)
            .with_message("all good")
            .with_response_time(150)
            .with_version("1.0.0")
            .with_metadata("region", "us-east-1");

        assert_eq!(health.message, Some("all good".to_string()));
        assert_eq!(health.response_time_ms, Some(150));
        assert_eq!(health.version, Some("1.0.0".to_string()));
        assert_eq!(health.metadata.get("region"), Some(&"us-east-1".to_string()));
    }

    #[test]
    fn test_dependency_health_from_component() {
        let component = ComponentHealth {
            name: "test".to_string(),
            status: HealthStatus::Degraded,
            message: Some("slow".to_string()),
            latency_ms: Some(200),
            last_check: Some(Utc::now()),
        };

        let dep_health: DependencyHealth = component.into();
        assert_eq!(dep_health.name, "test");
        assert_eq!(dep_health.status, HealthStatus::Degraded);
        assert_eq!(dep_health.message, Some("slow".to_string()));
        assert_eq!(dep_health.response_time_ms, Some(200));
    }

    #[tokio::test]
    async fn test_health_aggregator_new() {
        let aggregator = HealthAggregator::new();
        let results = aggregator.check_all().await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_health_aggregator_add_dependency() {
        let check = Arc::new(MockHealthCheck {
            name: "test".to_string(),
            status: HealthStatus::Healthy,
            config: HealthCheckConfig::default(),
        });

        let aggregator = HealthAggregator::new().add_dependency(check);
        let results = aggregator.check_all().await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test");
        assert_eq!(results[0].status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_aggregator_score_all_healthy() {
        let check1 = Arc::new(MockHealthCheck {
            name: "check1".to_string(),
            status: HealthStatus::Healthy,
            config: HealthCheckConfig::default(),
        });

        let check2 = Arc::new(MockHealthCheck {
            name: "check2".to_string(),
            status: HealthStatus::Healthy,
            config: HealthCheckConfig::default(),
        });

        let aggregator = HealthAggregator::new()
            .add_dependency(check1)
            .add_dependency(check2);

        let score = aggregator.health_score().await;
        assert_eq!(score, 1.0);

        let status = aggregator.overall_status().await;
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_aggregator_score_degraded() {
        let check1 = Arc::new(MockHealthCheck {
            name: "check1".to_string(),
            status: HealthStatus::Healthy,
            config: HealthCheckConfig::default(),
        });

        let check2 = Arc::new(MockHealthCheck {
            name: "check2".to_string(),
            status: HealthStatus::Degraded,
            config: HealthCheckConfig::default(),
        });

        let aggregator = HealthAggregator::new()
            .add_dependency(check1)
            .add_dependency(check2);

        let score = aggregator.health_score().await;
        assert_eq!(score, 0.75); // (1.0 + 0.5) / 2

        let status = aggregator.overall_status().await;
        assert_eq!(status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_health_aggregator_weighted_score() {
        let check1 = Arc::new(MockHealthCheck {
            name: "critical".to_string(),
            status: HealthStatus::Unhealthy,
            config: HealthCheckConfig::default(),
        });

        let check2 = Arc::new(MockHealthCheck {
            name: "optional".to_string(),
            status: HealthStatus::Healthy,
            config: HealthCheckConfig::default(),
        });

        let aggregator = HealthAggregator::new()
            .add_dependency(check1)
            .add_dependency(check2)
            .with_weight("critical", 3.0)
            .with_weight("optional", 1.0);

        let score = aggregator.health_score().await;
        assert_eq!(score, 0.25); // (0.0 * 3.0 + 1.0 * 1.0) / 4.0
    }

    #[tokio::test]
    async fn test_health_history_new() {
        let history = HealthHistory::new("test", 10);
        let entries = history.get_all().await;
        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn test_health_history_record() {
        let history = HealthHistory::new("test", 10);

        let health = ComponentHealth {
            name: "test".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            latency_ms: Some(50),
            last_check: Some(Utc::now()),
        };

        history.record(&health).await;

        let entries = history.get_all().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, HealthStatus::Healthy);
        assert_eq!(entries[0].response_time_ms, Some(50));
    }

    #[tokio::test]
    async fn test_health_history_max_entries() {
        let history = HealthHistory::new("test", 3);

        for i in 0..5 {
            let health = ComponentHealth {
                name: "test".to_string(),
                status: HealthStatus::Healthy,
                message: Some(format!("check {}", i)),
                latency_ms: Some(i as u64),
                last_check: Some(Utc::now()),
            };
            history.record(&health).await;
        }

        let entries = history.get_all().await;
        assert_eq!(entries.len(), 3); // Should only keep last 3
    }

    #[tokio::test]
    async fn test_health_history_uptime_percentage() {
        let history = HealthHistory::new("test", 10);

        // 3 healthy, 1 degraded, 1 unhealthy
        for status in [
            HealthStatus::Healthy,
            HealthStatus::Healthy,
            HealthStatus::Healthy,
            HealthStatus::Degraded,
            HealthStatus::Unhealthy,
        ] {
            let health = ComponentHealth {
                name: "test".to_string(),
                status,
                message: None,
                latency_ms: None,
                last_check: Some(Utc::now()),
            };
            history.record(&health).await;
        }

        let uptime = history.uptime_percentage().await;
        assert_eq!(uptime, 60.0); // 3 out of 5 = 60%
    }

    #[tokio::test]
    async fn test_health_history_average_response_time() {
        let history = HealthHistory::new("test", 10);

        for latency in [10, 20, 30] {
            let health = ComponentHealth {
                name: "test".to_string(),
                status: HealthStatus::Healthy,
                message: None,
                latency_ms: Some(latency),
                last_check: Some(Utc::now()),
            };
            history.record(&health).await;
        }

        let avg = history.average_response_time().await;
        assert_eq!(avg, Some(20.0)); // (10 + 20 + 30) / 3 = 20
    }

    #[test]
    fn test_alert_severity() {
        assert_ne!(AlertSeverity::Info, AlertSeverity::Warning);
        assert_ne!(AlertSeverity::Warning, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_logging_alert_handler() {
        let handler = LoggingAlertHandler;

        let alert = HealthAlert {
            component: "test".to_string(),
            severity: AlertSeverity::Warning,
            message: "test alert".to_string(),
            triggered_at: Utc::now(),
            status: HealthStatus::Degraded,
        };

        // Should not panic
        handler.handle_alert(alert).await;
    }
}
