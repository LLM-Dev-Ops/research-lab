//! Observability module for distributed tracing and monitoring
//!
//! This module provides comprehensive observability features including:
//! - OpenTelemetry distributed tracing
//! - W3C Trace Context propagation
//! - Database query tracing
//! - HTTP request/response tracing
//! - Custom span creation and attributes
//! - Prometheus metrics collection and export
//! - Structured JSON logging with sensitive data redaction
//! - Request correlation IDs
//! - SLO definitions and alerting rules

pub mod tracing;
pub mod metrics;
pub mod logging;
pub mod health;
pub mod alerting;

pub use tracing::{
    create_span, current_span_id, current_trace_id, init_tracing, record_error, record_event,
    shutdown_tracing, tracing_middleware, DbSpan, SpanBuilder, TraceContextPropagation,
    TracingConfig, TracingConfigBuilder, TracingError, TracingResult,
};

pub use metrics::{
    init_metrics, metrics_handler, BusinessMetrics, DatabaseMetrics, DurationGuard,
    HttpMetrics, MetricsConfig, MetricsError, MetricsLayer, MetricsRecorder, SystemMetrics,
    increment_counter, observe_duration, set_gauge,
};

pub use logging::{
    // Configuration
    LogConfig, LogFormat, LogRotationConfig,
    // Context
    LogContext, current_context, with_context,
    // Redaction
    SensitiveDataRedactor,
    // Middleware
    RequestLoggingState, request_logging_middleware, create_request_logging_middleware,
    // Initialization
    init_logging, init_default_logging,
    // Constants
    REQUEST_ID_HEADER, SENSITIVE_HEADERS, SENSITIVE_PATTERNS,
};

pub use health::{
    // Core types
    HealthStatus, ComponentHealth, OverallHealth, HealthCheckConfig,
    // Health checks
    HealthCheck, PostgresHealthCheck, ClickHouseHealthCheck, S3HealthCheck,
    // Registry and state
    HealthCheckRegistry, HealthCheckState,
    // Handlers
    liveness_handler, readiness_handler, health_handler,
};

pub use alerting::{
    // SLO types
    ServiceLevelObjective, ServiceLevelIndicator, SloWindow,
    BurnRateAlert, AlertSeverity,
    // Alert rules
    AlertRule, AlertRuleSet,
    // Error budget
    ErrorBudget,
    // Defaults
    default_slos, default_alert_rules,
};
