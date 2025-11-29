//! Health check endpoints and implementations for the LLM Research API.
//!
//! This module provides comprehensive health checking capabilities for Kubernetes
//! deployments and operational monitoring. It includes:
//!
//! - Liveness checks: Simple process health verification
//! - Readiness checks: Full dependency health verification
//! - Detailed health: Component-level diagnostics with metrics
//!
//! # Architecture
//!
//! The health check system is designed to be:
//! - Non-blocking: All checks run concurrently with timeouts
//! - Cached: Results are cached to avoid overwhelming dependencies
//! - Extensible: Custom health checks can be registered
//! - Production-ready: Handles failures gracefully with proper HTTP status codes
//!
//! # Kubernetes Integration
//!
//! - `/health/live` - Liveness probe (minimal checks)
//! - `/health/ready` - Readiness probe (full dependency checks)
//! - `/health` - Detailed diagnostics (not used by K8s)

use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

// Re-exports for convenience
pub use aws_sdk_s3::Client as S3Client;

/// Health status of a component or the overall system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Component is fully operational
    Healthy,
    /// Component is operational but with reduced functionality
    Degraded,
    /// Component is not operational
    Unhealthy,
}

impl HealthStatus {
    /// Returns true if the status is healthy or degraded
    pub fn is_available(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    /// Returns the HTTP status code for this health status
    pub fn http_status(&self) -> StatusCode {
        match self {
            HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Combines two health statuses, returning the worst status
    pub fn combine(&self, other: &HealthStatus) -> HealthStatus {
        match (self, other) {
            (HealthStatus::Unhealthy, _) | (_, HealthStatus::Unhealthy) => HealthStatus::Unhealthy,
            (HealthStatus::Degraded, _) | (_, HealthStatus::Degraded) => HealthStatus::Degraded,
            _ => HealthStatus::Healthy,
        }
    }
}

/// Health information for a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Name of the component
    pub name: String,
    /// Current health status
    pub status: HealthStatus,
    /// Optional message providing additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Time taken to perform the health check (in milliseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Timestamp of the last successful check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_check: Option<DateTime<Utc>>,
}

impl ComponentHealth {
    /// Creates a new healthy component
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            latency_ms: None,
            last_check: Some(Utc::now()),
        }
    }

    /// Creates a new degraded component
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            latency_ms: None,
            last_check: Some(Utc::now()),
        }
    }

    /// Creates a new unhealthy component
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            latency_ms: None,
            last_check: Some(Utc::now()),
        }
    }

    /// Sets the latency for this health check
    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.latency_ms = Some(latency.as_millis() as u64);
        self
    }
}

/// Overall health status including all components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallHealth {
    /// Aggregated health status
    pub status: HealthStatus,
    /// Health status of individual components
    pub components: HashMap<String, ComponentHealth>,
    /// Timestamp when this health check was performed
    pub timestamp: DateTime<Utc>,
    /// Application version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Application uptime in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
}

impl OverallHealth {
    /// Creates a new overall health status from components
    pub fn new(components: HashMap<String, ComponentHealth>) -> Self {
        let status = components
            .values()
            .fold(HealthStatus::Healthy, |acc, component| {
                acc.combine(&component.status)
            });

        Self {
            status,
            components,
            timestamp: Utc::now(),
            version: None,
            uptime_seconds: None,
        }
    }

    /// Sets the version information
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the uptime information
    pub fn with_uptime(mut self, uptime: Duration) -> Self {
        self.uptime_seconds = Some(uptime.as_secs());
        self
    }
}

impl IntoResponse for OverallHealth {
    fn into_response(self) -> Response {
        let status_code = self.status.http_status();
        (status_code, Json(self)).into_response()
    }
}

/// Configuration for health checks.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Timeout for individual health checks
    pub check_timeout: Duration,
    /// Cache TTL for health check results
    pub cache_ttl: Duration,
    /// Whether to mark the check as critical (affects readiness)
    pub is_critical: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_timeout: Duration::from_secs(5),
            cache_ttl: Duration::from_secs(30),
            is_critical: true,
        }
    }
}

impl HealthCheckConfig {
    /// Creates a configuration for a critical component
    pub fn critical() -> Self {
        Self {
            is_critical: true,
            ..Default::default()
        }
    }

    /// Creates a configuration for a non-critical component
    pub fn non_critical() -> Self {
        Self {
            is_critical: false,
            ..Default::default()
        }
    }

    /// Sets the timeout for this health check
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.check_timeout = timeout;
        self
    }

    /// Sets the cache TTL for this health check
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }
}

/// Trait for implementing custom health checks.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Performs the health check and returns the component health
    async fn check(&self) -> ComponentHealth;

    /// Returns the name of this health check
    fn name(&self) -> &str;

    /// Returns the configuration for this health check
    fn config(&self) -> &HealthCheckConfig;
}

/// PostgreSQL database health check.
pub struct PostgresHealthCheck {
    pool: PgPool,
    config: HealthCheckConfig,
}

impl PostgresHealthCheck {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            config: HealthCheckConfig::critical().with_timeout(Duration::from_secs(3)),
        }
    }

    pub fn with_config(pool: PgPool, config: HealthCheckConfig) -> Self {
        Self { pool, config }
    }
}

#[async_trait]
impl HealthCheck for PostgresHealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = Instant::now();

        match tokio::time::timeout(
            self.config.check_timeout,
            sqlx::query("SELECT 1").execute(&self.pool),
        )
        .await
        {
            Ok(Ok(_)) => {
                let latency = start.elapsed();
                debug!("PostgreSQL health check passed in {:?}", latency);
                ComponentHealth::healthy("postgres").with_latency(latency)
            }
            Ok(Err(e)) => {
                error!("PostgreSQL health check failed: {}", e);
                ComponentHealth::unhealthy("postgres", format!("Database error: {}", e))
            }
            Err(_) => {
                warn!("PostgreSQL health check timed out");
                ComponentHealth::unhealthy("postgres", "Health check timed out")
            }
        }
    }

    fn name(&self) -> &str {
        "postgres"
    }

    fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}

/// ClickHouse database health check.
pub struct ClickHouseHealthCheck {
    client: clickhouse::Client,
    config: HealthCheckConfig,
}

impl ClickHouseHealthCheck {
    pub fn new(client: clickhouse::Client) -> Self {
        Self {
            client,
            config: HealthCheckConfig::non_critical().with_timeout(Duration::from_secs(3)),
        }
    }

    pub fn with_config(client: clickhouse::Client, config: HealthCheckConfig) -> Self {
        Self { client, config }
    }
}

#[async_trait]
impl HealthCheck for ClickHouseHealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = Instant::now();

        match tokio::time::timeout(
            self.config.check_timeout,
            self.client.query("SELECT 1").execute(),
        )
        .await
        {
            Ok(Ok(_)) => {
                let latency = start.elapsed();
                debug!("ClickHouse health check passed in {:?}", latency);
                ComponentHealth::healthy("clickhouse").with_latency(latency)
            }
            Ok(Err(e)) => {
                warn!("ClickHouse health check failed: {}", e);
                if self.config.is_critical {
                    ComponentHealth::unhealthy("clickhouse", format!("Database error: {}", e))
                } else {
                    ComponentHealth::degraded("clickhouse", format!("Database error: {}", e))
                }
            }
            Err(_) => {
                warn!("ClickHouse health check timed out");
                if self.config.is_critical {
                    ComponentHealth::unhealthy("clickhouse", "Health check timed out")
                } else {
                    ComponentHealth::degraded("clickhouse", "Health check timed out")
                }
            }
        }
    }

    fn name(&self) -> &str {
        "clickhouse"
    }

    fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}

/// S3 storage health check.
pub struct S3HealthCheck {
    client: S3Client,
    bucket: String,
    config: HealthCheckConfig,
}

impl S3HealthCheck {
    pub fn new(client: S3Client, bucket: String) -> Self {
        Self {
            client,
            bucket,
            config: HealthCheckConfig::critical().with_timeout(Duration::from_secs(5)),
        }
    }

    pub fn with_config(client: S3Client, bucket: String, config: HealthCheckConfig) -> Self {
        Self {
            client,
            bucket,
            config,
        }
    }
}

#[async_trait]
impl HealthCheck for S3HealthCheck {
    async fn check(&self) -> ComponentHealth {
        let start = Instant::now();

        match tokio::time::timeout(
            self.config.check_timeout,
            self.client.head_bucket().bucket(&self.bucket).send(),
        )
        .await
        {
            Ok(Ok(_)) => {
                let latency = start.elapsed();
                debug!("S3 health check passed in {:?}", latency);
                ComponentHealth::healthy("s3").with_latency(latency)
            }
            Ok(Err(e)) => {
                error!("S3 health check failed: {}", e);
                ComponentHealth::unhealthy("s3", format!("S3 error: {}", e))
            }
            Err(_) => {
                warn!("S3 health check timed out");
                ComponentHealth::unhealthy("s3", "Health check timed out")
            }
        }
    }

    fn name(&self) -> &str {
        "s3"
    }

    fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}

/// Cached health check result.
#[derive(Debug, Clone)]
struct CachedHealth {
    result: ComponentHealth,
    cached_at: Instant,
}

impl CachedHealth {
    fn is_expired(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() > ttl
    }
}

/// Health check registry that manages multiple health checks.
pub struct HealthCheckRegistry {
    checks: Vec<Arc<dyn HealthCheck>>,
    cache: Arc<RwLock<HashMap<String, CachedHealth>>>,
    start_time: Instant,
    version: String,
}

impl HealthCheckRegistry {
    /// Creates a new health check registry
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            checks: Vec::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            version: version.into(),
        }
    }

    /// Registers a new health check
    pub fn register(mut self, check: Arc<dyn HealthCheck>) -> Self {
        self.checks.push(check);
        self
    }

    /// Performs all health checks concurrently
    pub async fn check_all(&self) -> OverallHealth {
        let mut results = HashMap::new();

        // Run all checks concurrently
        let check_futures: Vec<_> = self
            .checks
            .iter()
            .map(|check| {
                let check = Arc::clone(check);
                let cache = Arc::clone(&self.cache);
                async move {
                    let name = check.name().to_string();
                    let config = check.config();

                    // Check cache first
                    {
                        let cache_read = cache.read().await;
                        if let Some(cached) = cache_read.get(&name) {
                            if !cached.is_expired(config.cache_ttl) {
                                debug!("Using cached health check result for {}", name);
                                return (name, cached.result.clone());
                            }
                        }
                    }

                    // Perform the check
                    let result = check.check().await;

                    // Update cache
                    {
                        let mut cache_write = cache.write().await;
                        cache_write.insert(
                            name.clone(),
                            CachedHealth {
                                result: result.clone(),
                                cached_at: Instant::now(),
                            },
                        );
                    }

                    (name, result)
                }
            })
            .collect();

        // Wait for all checks to complete
        let check_results = futures::future::join_all(check_futures).await;

        for (name, health) in check_results {
            results.insert(name, health);
        }

        OverallHealth::new(results)
            .with_version(&self.version)
            .with_uptime(self.start_time.elapsed())
    }

    /// Checks only critical components for readiness
    pub async fn check_readiness(&self) -> OverallHealth {
        let mut results = HashMap::new();

        // Run only critical checks concurrently
        let check_futures: Vec<_> = self
            .checks
            .iter()
            .filter(|check| check.config().is_critical)
            .map(|check| {
                let check = Arc::clone(check);
                async move {
                    let name = check.name().to_string();
                    let result = check.check().await;
                    (name, result)
                }
            })
            .collect();

        let check_results = futures::future::join_all(check_futures).await;

        for (name, health) in check_results {
            results.insert(name, health);
        }

        OverallHealth::new(results)
    }

    /// Simple liveness check (always returns healthy if process is running)
    pub async fn check_liveness(&self) -> OverallHealth {
        let mut results = HashMap::new();
        results.insert(
            "application".to_string(),
            ComponentHealth::healthy("application"),
        );

        OverallHealth::new(results)
            .with_version(&self.version)
            .with_uptime(self.start_time.elapsed())
    }
}

/// Application state for health check handlers
#[derive(Clone)]
pub struct HealthCheckState {
    pub registry: Arc<HealthCheckRegistry>,
}

impl HealthCheckState {
    pub fn new(registry: Arc<HealthCheckRegistry>) -> Self {
        Self { registry }
    }
}

/// Liveness probe handler for Kubernetes
///
/// This endpoint is used by Kubernetes to determine if the pod should be restarted.
/// It performs minimal checks and should always return 200 if the process is running.
///
/// GET /health/live
pub async fn liveness_handler(
    State(state): State<HealthCheckState>,
) -> impl IntoResponse {
    let health = state.registry.check_liveness().await;
    (StatusCode::OK, Json(health))
}

/// Readiness probe handler for Kubernetes
///
/// This endpoint is used by Kubernetes to determine if the pod is ready to receive traffic.
/// It checks all critical dependencies and returns 503 if any are unhealthy.
///
/// GET /health/ready
pub async fn readiness_handler(
    State(state): State<HealthCheckState>,
) -> impl IntoResponse {
    let health = state.registry.check_readiness().await;
    health
}

/// Detailed health check handler
///
/// This endpoint provides comprehensive health information for all components,
/// including latency metrics and detailed error messages.
///
/// GET /health
pub async fn health_handler(
    State(state): State<HealthCheckState>,
) -> impl IntoResponse {
    let health = state.registry.check_all().await;
    health
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_combine() {
        assert_eq!(
            HealthStatus::Healthy.combine(&HealthStatus::Healthy),
            HealthStatus::Healthy
        );
        assert_eq!(
            HealthStatus::Healthy.combine(&HealthStatus::Degraded),
            HealthStatus::Degraded
        );
        assert_eq!(
            HealthStatus::Healthy.combine(&HealthStatus::Unhealthy),
            HealthStatus::Unhealthy
        );
        assert_eq!(
            HealthStatus::Degraded.combine(&HealthStatus::Unhealthy),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn test_health_status_is_available() {
        assert!(HealthStatus::Healthy.is_available());
        assert!(HealthStatus::Degraded.is_available());
        assert!(!HealthStatus::Unhealthy.is_available());
    }

    #[test]
    fn test_health_status_http_status() {
        assert_eq!(HealthStatus::Healthy.http_status(), StatusCode::OK);
        assert_eq!(HealthStatus::Degraded.http_status(), StatusCode::OK);
        assert_eq!(
            HealthStatus::Unhealthy.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_component_health_constructors() {
        let healthy = ComponentHealth::healthy("test");
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.name, "test");
        assert!(healthy.message.is_none());

        let degraded = ComponentHealth::degraded("test", "warning");
        assert_eq!(degraded.status, HealthStatus::Degraded);
        assert_eq!(degraded.message, Some("warning".to_string()));

        let unhealthy = ComponentHealth::unhealthy("test", "error");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.message, Some("error".to_string()));
    }

    #[test]
    fn test_component_health_with_latency() {
        let health = ComponentHealth::healthy("test")
            .with_latency(Duration::from_millis(150));
        assert_eq!(health.latency_ms, Some(150));
    }

    #[test]
    fn test_overall_health_aggregation() {
        let mut components = HashMap::new();
        components.insert(
            "db".to_string(),
            ComponentHealth::healthy("db"),
        );
        components.insert(
            "cache".to_string(),
            ComponentHealth::degraded("cache", "slow"),
        );

        let overall = OverallHealth::new(components);
        assert_eq!(overall.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_overall_health_all_healthy() {
        let mut components = HashMap::new();
        components.insert(
            "db".to_string(),
            ComponentHealth::healthy("db"),
        );
        components.insert(
            "api".to_string(),
            ComponentHealth::healthy("api"),
        );

        let overall = OverallHealth::new(components);
        assert_eq!(overall.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_overall_health_with_unhealthy() {
        let mut components = HashMap::new();
        components.insert(
            "db".to_string(),
            ComponentHealth::healthy("db"),
        );
        components.insert(
            "api".to_string(),
            ComponentHealth::unhealthy("api", "down"),
        );

        let overall = OverallHealth::new(components);
        assert_eq!(overall.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_overall_health_with_metadata() {
        let components = HashMap::new();
        let overall = OverallHealth::new(components)
            .with_version("1.0.0")
            .with_uptime(Duration::from_secs(3600));

        assert_eq!(overall.version, Some("1.0.0".to_string()));
        assert_eq!(overall.uptime_seconds, Some(3600));
    }

    #[test]
    fn test_health_check_config_defaults() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.check_timeout, Duration::from_secs(5));
        assert_eq!(config.cache_ttl, Duration::from_secs(30));
        assert!(config.is_critical);
    }

    #[test]
    fn test_health_check_config_builders() {
        let critical = HealthCheckConfig::critical();
        assert!(critical.is_critical);

        let non_critical = HealthCheckConfig::non_critical();
        assert!(!non_critical.is_critical);

        let custom = HealthCheckConfig::default()
            .with_timeout(Duration::from_secs(10))
            .with_cache_ttl(Duration::from_secs(60));
        assert_eq!(custom.check_timeout, Duration::from_secs(10));
        assert_eq!(custom.cache_ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_cached_health_expiration() {
        let cached = CachedHealth {
            result: ComponentHealth::healthy("test"),
            cached_at: Instant::now() - Duration::from_secs(60),
        };

        assert!(cached.is_expired(Duration::from_secs(30)));
        assert!(!cached.is_expired(Duration::from_secs(120)));
    }

    #[tokio::test]
    async fn test_health_check_registry_liveness() {
        let registry = HealthCheckRegistry::new("1.0.0");
        let health = registry.check_liveness().await;

        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.components.contains_key("application"));
        assert_eq!(health.version, Some("1.0.0".to_string()));
    }

    // Mock health check for testing
    struct MockHealthCheck {
        name: String,
        result: ComponentHealth,
        config: HealthCheckConfig,
    }

    #[async_trait]
    impl HealthCheck for MockHealthCheck {
        async fn check(&self) -> ComponentHealth {
            self.result.clone()
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn config(&self) -> &HealthCheckConfig {
            &self.config
        }
    }

    #[tokio::test]
    async fn test_health_check_registry_with_checks() {
        let mock_check = Arc::new(MockHealthCheck {
            name: "mock".to_string(),
            result: ComponentHealth::healthy("mock"),
            config: HealthCheckConfig::default(),
        });

        let registry = HealthCheckRegistry::new("1.0.0")
            .register(mock_check);

        let health = registry.check_all().await;
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.components.contains_key("mock"));
    }

    #[tokio::test]
    async fn test_health_check_registry_readiness_critical_only() {
        let critical_check = Arc::new(MockHealthCheck {
            name: "critical".to_string(),
            result: ComponentHealth::healthy("critical"),
            config: HealthCheckConfig::critical(),
        });

        let non_critical_check = Arc::new(MockHealthCheck {
            name: "non_critical".to_string(),
            result: ComponentHealth::degraded("non_critical", "slow"),
            config: HealthCheckConfig::non_critical(),
        });

        let registry = HealthCheckRegistry::new("1.0.0")
            .register(critical_check)
            .register(non_critical_check);

        let health = registry.check_readiness().await;
        assert!(health.components.contains_key("critical"));
        assert!(!health.components.contains_key("non_critical"));
    }

    #[tokio::test]
    async fn test_health_check_registry_caching() {
        let mock_check = Arc::new(MockHealthCheck {
            name: "cached".to_string(),
            result: ComponentHealth::healthy("cached"),
            config: HealthCheckConfig::default().with_cache_ttl(Duration::from_secs(60)),
        });

        let registry = HealthCheckRegistry::new("1.0.0")
            .register(mock_check);

        // First call should execute the check
        let health1 = registry.check_all().await;
        assert!(health1.components.contains_key("cached"));

        // Second call should use cached result
        let health2 = registry.check_all().await;
        assert!(health2.components.contains_key("cached"));
    }
}
