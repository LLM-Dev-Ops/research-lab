//! Load shedding implementation for graceful degradation under high load.
//!
//! This module provides load shedding capabilities to prevent system overload by:
//! - Monitoring system resources (CPU, memory)
//! - Rejecting requests when thresholds are exceeded
//! - Supporting request priority classification
//! - Implementing graceful degradation strategies
//! - Providing Axum middleware integration
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::reliability::load_shedding::*;
//!
//! # async fn example() {
//! let config = LoadSheddingConfig {
//!     cpu_threshold: 0.8,
//!     memory_threshold: 0.85,
//!     queue_threshold: 1000,
//!     check_interval_ms: 1000,
//!     enabled: true,
//! };
//!
//! let shedder = LoadShedder::new(config);
//!
//! // Check if we should accept a request
//! if shedder.should_accept_request(RequestPriority::Normal).await {
//!     // Process request
//! } else {
//!     // Reject with 503
//! }
//! # }
//! ```

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use metrics::{counter, gauge};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, warn};

/// Load shedding errors
#[derive(Error, Debug, Clone, Serialize)]
pub enum LoadSheddingError {
    #[error("System overloaded: {reason}")]
    Overloaded { reason: String },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    #[error("Queue full")]
    QueueFull,

    #[error("Request priority too low")]
    PriorityTooLow,
}

impl IntoResponse for LoadSheddingError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            LoadSheddingError::Overloaded { reason } => {
                (StatusCode::SERVICE_UNAVAILABLE, format!("Service overloaded: {}", reason))
            }
            LoadSheddingError::ResourceExhausted { resource } => {
                (StatusCode::SERVICE_UNAVAILABLE, format!("Resource exhausted: {}", resource))
            }
            LoadSheddingError::QueueFull => {
                (StatusCode::SERVICE_UNAVAILABLE, "Queue is full".to_string())
            }
            LoadSheddingError::PriorityTooLow => {
                (StatusCode::SERVICE_UNAVAILABLE, "Request priority too low".to_string())
            }
        };

        let body = serde_json::json!({
            "error": message,
            "retry_after": 60,
        });

        (status, Json(body)).into_response()
    }
}

/// Load shedding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSheddingConfig {
    /// CPU utilization threshold (0.0 - 1.0)
    pub cpu_threshold: f64,
    /// Memory utilization threshold (0.0 - 1.0)
    pub memory_threshold: f64,
    /// Maximum queue size before shedding
    pub queue_threshold: usize,
    /// How often to check system resources (milliseconds)
    pub check_interval_ms: u64,
    /// Whether load shedding is enabled
    pub enabled: bool,
}

impl Default for LoadSheddingConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 0.85,
            memory_threshold: 0.90,
            queue_threshold: 1000,
            check_interval_ms: 1000,
            enabled: true,
        }
    }
}

impl LoadSheddingConfig {
    /// Creates a conservative configuration (shed load earlier)
    pub fn conservative() -> Self {
        Self {
            cpu_threshold: 0.70,
            memory_threshold: 0.75,
            queue_threshold: 500,
            check_interval_ms: 500,
            enabled: true,
        }
    }

    /// Creates an aggressive configuration (shed load later)
    pub fn aggressive() -> Self {
        Self {
            cpu_threshold: 0.95,
            memory_threshold: 0.95,
            queue_threshold: 2000,
            check_interval_ms: 2000,
            enabled: true,
        }
    }

    /// Creates a disabled configuration (no load shedding)
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// Request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RequestPriority {
    /// Background tasks (shed first)
    Background = 0,
    /// Low priority requests
    Low = 1,
    /// Normal priority requests
    Normal = 2,
    /// High priority requests
    High = 3,
    /// Critical requests (never shed)
    Critical = 4,
}

impl Default for RequestPriority {
    fn default() -> Self {
        RequestPriority::Normal
    }
}

impl RequestPriority {
    /// Returns true if this priority should be shed under the given load level
    pub fn should_shed(&self, load_level: LoadLevel) -> bool {
        match load_level {
            LoadLevel::Normal => false,
            LoadLevel::Moderate => matches!(self, RequestPriority::Background),
            LoadLevel::High => matches!(self, RequestPriority::Background | RequestPriority::Low),
            LoadLevel::Critical => matches!(
                self,
                RequestPriority::Background | RequestPriority::Low | RequestPriority::Normal
            ),
            LoadLevel::Emergency => !matches!(self, RequestPriority::Critical),
        }
    }
}

/// System load level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadLevel {
    /// Normal operations
    Normal = 0,
    /// Moderate load
    Moderate = 1,
    /// High load (start shedding)
    High = 2,
    /// Critical load (aggressive shedding)
    Critical = 3,
    /// Emergency (shed almost everything)
    Emergency = 4,
}

/// System resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    /// CPU utilization (0.0 - 1.0)
    pub cpu_usage: f64,
    /// Memory utilization (0.0 - 1.0)
    pub memory_usage: f64,
    /// Current queue size
    pub queue_size: usize,
    /// Number of active requests
    pub active_requests: usize,
    /// When these metrics were captured
    pub timestamp: Instant,
}

impl ResourceMetrics {
    /// Returns the current load level based on metrics
    pub fn load_level(&self, config: &LoadSheddingConfig) -> LoadLevel {
        let cpu_ratio = self.cpu_usage / config.cpu_threshold;
        let mem_ratio = self.memory_usage / config.memory_threshold;
        let queue_ratio = self.queue_size as f64 / config.queue_threshold as f64;

        let max_ratio = cpu_ratio.max(mem_ratio).max(queue_ratio);

        if max_ratio >= 1.5 {
            LoadLevel::Emergency
        } else if max_ratio >= 1.2 {
            LoadLevel::Critical
        } else if max_ratio >= 1.0 {
            LoadLevel::High
        } else if max_ratio >= 0.8 {
            LoadLevel::Moderate
        } else {
            LoadLevel::Normal
        }
    }
}

/// Load shedder implementation
pub struct LoadShedder {
    config: LoadSheddingConfig,
    system: Arc<RwLock<System>>,
    metrics: Arc<RwLock<ResourceMetrics>>,
    active_requests: Arc<AtomicUsize>,
    queue_size: Arc<AtomicUsize>,
    total_shed: Arc<AtomicU64>,
    total_accepted: Arc<AtomicU64>,
}

impl LoadShedder {
    /// Creates a new load shedder
    pub fn new(config: LoadSheddingConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let initial_metrics = ResourceMetrics {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            queue_size: 0,
            active_requests: 0,
            timestamp: Instant::now(),
        };

        Self {
            config,
            system: Arc::new(RwLock::new(system)),
            metrics: Arc::new(RwLock::new(initial_metrics)),
            active_requests: Arc::new(AtomicUsize::new(0)),
            queue_size: Arc::new(AtomicUsize::new(0)),
            total_shed: Arc::new(AtomicU64::new(0)),
            total_accepted: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Starts the background metrics collection task
    pub fn start_monitoring(self: Arc<Self>) {
        let interval_duration = Duration::from_millis(self.config.check_interval_ms);
        let mut ticker = interval(interval_duration);

        tokio::spawn(async move {
            loop {
                ticker.tick().await;
                self.collect_metrics().await;
            }
        });
    }

    /// Collects current system metrics
    async fn collect_metrics(&self) {
        let mut system = self.system.write().await;

        // Refresh CPU and memory
        system.refresh_cpu_all();
        system.refresh_memory();

        // Calculate CPU usage (average across all cores)
        let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage() as f64).sum::<f64>()
            / system.cpus().len() as f64
            / 100.0; // Convert percentage to 0.0-1.0

        // Calculate memory usage
        let memory_usage = system.used_memory() as f64 / system.total_memory() as f64;

        let metrics = ResourceMetrics {
            cpu_usage,
            memory_usage,
            queue_size: self.queue_size.load(Ordering::Relaxed),
            active_requests: self.active_requests.load(Ordering::Relaxed),
            timestamp: Instant::now(),
        };

        // Update stored metrics
        *self.metrics.write().await = metrics.clone();

        // Emit metrics
        gauge!("load_shedding_cpu_usage").set(cpu_usage);
        gauge!("load_shedding_memory_usage").set(memory_usage);
        gauge!("load_shedding_queue_size").set(metrics.queue_size as f64);
        gauge!("load_shedding_active_requests").set(metrics.active_requests as f64);

        debug!(
            "System metrics: CPU={:.2}%, Memory={:.2}%, Queue={}, Active={}",
            cpu_usage * 100.0,
            memory_usage * 100.0,
            metrics.queue_size,
            metrics.active_requests
        );
    }

    /// Checks if a request should be accepted
    pub async fn should_accept_request(&self, priority: RequestPriority) -> bool {
        if !self.config.enabled {
            return true;
        }

        let metrics = self.metrics.read().await.clone();
        let load_level = metrics.load_level(&self.config);

        if priority.should_shed(load_level) {
            self.total_shed.fetch_add(1, Ordering::Relaxed);
            counter!("load_shedding_requests_shed", &[("priority", format!("{:?}", priority))])
                .increment(1);

            warn!(
                "Shedding request: priority={:?}, load_level={:?}, cpu={:.2}%, mem={:.2}%",
                priority,
                load_level,
                metrics.cpu_usage * 100.0,
                metrics.memory_usage * 100.0
            );

            false
        } else {
            self.total_accepted.fetch_add(1, Ordering::Relaxed);
            counter!("load_shedding_requests_accepted", &[("priority", format!("{:?}", priority))])
                .increment(1);
            true
        }
    }

    /// Increments the active request count
    pub fn increment_active(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the active request count
    pub fn decrement_active(&self) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increments the queue size
    pub fn increment_queue(&self) {
        self.queue_size.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the queue size
    pub fn decrement_queue(&self) {
        self.queue_size.fetch_sub(1, Ordering::Relaxed);
    }

    /// Gets current resource metrics
    pub async fn get_metrics(&self) -> ResourceMetrics {
        self.metrics.read().await.clone()
    }

    /// Gets the current load level
    pub async fn get_load_level(&self) -> LoadLevel {
        let metrics = self.get_metrics().await;
        metrics.load_level(&self.config)
    }

    /// Gets statistics
    pub fn get_stats(&self) -> LoadSheddingStats {
        LoadSheddingStats {
            total_shed: self.total_shed.load(Ordering::Relaxed),
            total_accepted: self.total_accepted.load(Ordering::Relaxed),
            active_requests: self.active_requests.load(Ordering::Relaxed),
            queue_size: self.queue_size.load(Ordering::Relaxed),
        }
    }
}

/// Load shedding statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSheddingStats {
    /// Total requests shed
    pub total_shed: u64,
    /// Total requests accepted
    pub total_accepted: u64,
    /// Currently active requests
    pub active_requests: usize,
    /// Current queue size
    pub queue_size: usize,
}

/// Axum middleware state
#[derive(Clone)]
pub struct LoadSheddingMiddlewareState {
    pub shedder: Arc<LoadShedder>,
    pub default_priority: RequestPriority,
}

/// Axum middleware for load shedding
pub async fn load_shedding_middleware(
    State(state): State<LoadSheddingMiddlewareState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, LoadSheddingError> {
    // Extract priority from request headers or use default
    let priority = request
        .headers()
        .get("X-Request-Priority")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| match s.to_lowercase().as_str() {
            "background" => Some(RequestPriority::Background),
            "low" => Some(RequestPriority::Low),
            "normal" => Some(RequestPriority::Normal),
            "high" => Some(RequestPriority::High),
            "critical" => Some(RequestPriority::Critical),
            _ => None,
        })
        .unwrap_or(state.default_priority);

    // Check if we should accept this request
    if !state.shedder.should_accept_request(priority).await {
        return Err(LoadSheddingError::Overloaded {
            reason: "System is overloaded".to_string(),
        });
    }

    // Track active request
    state.shedder.increment_active();

    // Process request
    let response = next.run(request).await;

    // Decrement after processing
    state.shedder.decrement_active();

    Ok(response)
}

/// Creates load shedding middleware layer
pub fn create_load_shedding_layer(
    shedder: Arc<LoadShedder>,
    default_priority: RequestPriority,
) -> LoadSheddingMiddlewareState {
    LoadSheddingMiddlewareState {
        shedder,
        default_priority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_shedding_config_default() {
        let config = LoadSheddingConfig::default();
        assert_eq!(config.cpu_threshold, 0.85);
        assert_eq!(config.memory_threshold, 0.90);
        assert_eq!(config.queue_threshold, 1000);
        assert!(config.enabled);
    }

    #[test]
    fn test_load_shedding_config_conservative() {
        let config = LoadSheddingConfig::conservative();
        assert_eq!(config.cpu_threshold, 0.70);
        assert_eq!(config.memory_threshold, 0.75);
        assert_eq!(config.queue_threshold, 500);
    }

    #[test]
    fn test_load_shedding_config_aggressive() {
        let config = LoadSheddingConfig::aggressive();
        assert_eq!(config.cpu_threshold, 0.95);
        assert_eq!(config.memory_threshold, 0.95);
        assert_eq!(config.queue_threshold, 2000);
    }

    #[test]
    fn test_load_shedding_config_disabled() {
        let config = LoadSheddingConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_request_priority_ordering() {
        assert!(RequestPriority::Critical > RequestPriority::High);
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
        assert!(RequestPriority::Low > RequestPriority::Background);
    }

    #[test]
    fn test_request_priority_should_shed_normal() {
        let priority = RequestPriority::Normal;
        assert!(!priority.should_shed(LoadLevel::Normal));
        assert!(!priority.should_shed(LoadLevel::Moderate));
        assert!(!priority.should_shed(LoadLevel::High));
        assert!(priority.should_shed(LoadLevel::Critical));
        assert!(priority.should_shed(LoadLevel::Emergency));
    }

    #[test]
    fn test_request_priority_should_shed_critical() {
        let priority = RequestPriority::Critical;
        assert!(!priority.should_shed(LoadLevel::Normal));
        assert!(!priority.should_shed(LoadLevel::Moderate));
        assert!(!priority.should_shed(LoadLevel::High));
        assert!(!priority.should_shed(LoadLevel::Critical));
        assert!(!priority.should_shed(LoadLevel::Emergency));
    }

    #[test]
    fn test_request_priority_should_shed_background() {
        let priority = RequestPriority::Background;
        assert!(!priority.should_shed(LoadLevel::Normal));
        assert!(priority.should_shed(LoadLevel::Moderate));
        assert!(priority.should_shed(LoadLevel::High));
        assert!(priority.should_shed(LoadLevel::Critical));
        assert!(priority.should_shed(LoadLevel::Emergency));
    }

    #[test]
    fn test_load_level_ordering() {
        assert!(LoadLevel::Emergency > LoadLevel::Critical);
        assert!(LoadLevel::Critical > LoadLevel::High);
        assert!(LoadLevel::High > LoadLevel::Moderate);
        assert!(LoadLevel::Moderate > LoadLevel::Normal);
    }

    #[test]
    fn test_resource_metrics_load_level_normal() {
        let config = LoadSheddingConfig::default();
        let metrics = ResourceMetrics {
            cpu_usage: 0.5,
            memory_usage: 0.6,
            queue_size: 100,
            active_requests: 10,
            timestamp: Instant::now(),
        };

        assert_eq!(metrics.load_level(&config), LoadLevel::Normal);
    }

    #[test]
    fn test_resource_metrics_load_level_high() {
        let config = LoadSheddingConfig::default();
        let metrics = ResourceMetrics {
            cpu_usage: 0.9, // Above threshold
            memory_usage: 0.6,
            queue_size: 100,
            active_requests: 50,
            timestamp: Instant::now(),
        };

        assert_eq!(metrics.load_level(&config), LoadLevel::High);
    }

    #[test]
    fn test_resource_metrics_load_level_critical() {
        let config = LoadSheddingConfig::default();
        // With default cpu_threshold=0.85, need cpu_usage > 0.85 * 1.2 = 1.02 for Critical
        // So use a ratio >= 1.2 by setting cpu=1.02 (which gives ratio 1.2)
        // Or just set values that result in max_ratio >= 1.2
        let metrics = ResourceMetrics {
            cpu_usage: 1.0,
            memory_usage: 1.1, // 1.1 / 0.90 = 1.22 >= 1.2 threshold for Critical
            queue_size: 100,
            active_requests: 100,
            timestamp: Instant::now(),
        };

        assert_eq!(metrics.load_level(&config), LoadLevel::Critical);
    }

    #[tokio::test]
    async fn test_load_shedder_new() {
        let config = LoadSheddingConfig::default();
        let shedder = LoadShedder::new(config);

        let stats = shedder.get_stats();
        assert_eq!(stats.total_shed, 0);
        assert_eq!(stats.total_accepted, 0);
        assert_eq!(stats.active_requests, 0);
    }

    #[tokio::test]
    async fn test_load_shedder_disabled() {
        let config = LoadSheddingConfig::disabled();
        let shedder = LoadShedder::new(config);

        // Should always accept when disabled
        assert!(shedder.should_accept_request(RequestPriority::Background).await);
        assert!(shedder.should_accept_request(RequestPriority::Low).await);
        assert!(shedder.should_accept_request(RequestPriority::Normal).await);
    }

    #[tokio::test]
    async fn test_load_shedder_increment_decrement() {
        let config = LoadSheddingConfig::default();
        let shedder = LoadShedder::new(config);

        assert_eq!(shedder.active_requests.load(Ordering::Relaxed), 0);

        shedder.increment_active();
        assert_eq!(shedder.active_requests.load(Ordering::Relaxed), 1);

        shedder.increment_active();
        assert_eq!(shedder.active_requests.load(Ordering::Relaxed), 2);

        shedder.decrement_active();
        assert_eq!(shedder.active_requests.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_load_shedder_queue_tracking() {
        let config = LoadSheddingConfig::default();
        let shedder = LoadShedder::new(config);

        shedder.increment_queue();
        shedder.increment_queue();
        assert_eq!(shedder.queue_size.load(Ordering::Relaxed), 2);

        shedder.decrement_queue();
        assert_eq!(shedder.queue_size.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_load_shedder_stats() {
        let config = LoadSheddingConfig::default();
        let shedder = LoadShedder::new(config);

        shedder.increment_active();
        shedder.increment_queue();

        let stats = shedder.get_stats();
        assert_eq!(stats.active_requests, 1);
        assert_eq!(stats.queue_size, 1);
    }

    #[test]
    fn test_create_load_shedding_layer() {
        let config = LoadSheddingConfig::default();
        let shedder = Arc::new(LoadShedder::new(config));

        let state = create_load_shedding_layer(shedder, RequestPriority::Normal);
        assert_eq!(state.default_priority, RequestPriority::Normal);
    }
}
