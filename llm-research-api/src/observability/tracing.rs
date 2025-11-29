//! OpenTelemetry Distributed Tracing Implementation
//!
//! This module provides comprehensive distributed tracing capabilities using OpenTelemetry,
//! supporting W3C Trace Context propagation, OTLP export, and integration with Jaeger, Tempo,
//! and other OTLP-compatible backends.
//!
//! # Features
//!
//! - Full OpenTelemetry setup with configurable exporters
//! - W3C Trace Context propagation (traceparent/tracestate)
//! - Automatic span creation for HTTP requests
//! - Database query tracing with SQLx integration
//! - Configurable sampling strategies
//! - Context propagation across async boundaries
//! - Integration with structured logging
//!
//! # Example
//!
//! ```rust,no_run
//! use llm_research_api::observability::tracing::{TracingConfig, init_tracing};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = TracingConfig::builder()
//!         .service_name("llm-research-api")
//!         .service_version("1.0.0")
//!         .environment("production")
//!         .otlp_endpoint("http://localhost:4317")
//!         .build();
//!
//!     init_tracing(config).await?;
//!     Ok(())
//! }
//! ```

use axum::{
    extract::{Request, MatchedPath},
    middleware::Next,
    response::Response,
    http::{HeaderMap, HeaderValue, StatusCode},
};
use opentelemetry::{
    global,
    trace::{
        SpanContext, SpanId, SpanKind, Status, TraceContextExt, TraceFlags, TraceId,
        TraceState, TracerProvider as _,
    },
    Context as OtelContext, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{Config, RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Encode, Executor, Type};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use tracing::{error, info, span, warn, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// OpenTelemetry semantic convention constants
const SERVICE_NAME: &str = "service.name";
const SERVICE_VERSION: &str = "service.version";
const DEPLOYMENT_ENVIRONMENT: &str = "deployment.environment";
const HTTP_REQUEST_METHOD: &str = "http.request.method";
const HTTP_RESPONSE_STATUS_CODE: &str = "http.response.status_code";
const HTTP_ROUTE: &str = "http.route";
const SERVER_ADDRESS: &str = "server.address";
const SERVER_PORT: &str = "server.port";
const USER_AGENT_ORIGINAL: &str = "user_agent.original";
const DB_SYSTEM: &str = "db.system";
const DB_OPERATION: &str = "db.operation";
const DB_STATEMENT: &str = "db.statement";

/// Errors that can occur during tracing operations
#[derive(Error, Debug)]
pub enum TracingError {
    #[error("Failed to initialize OpenTelemetry: {0}")]
    InitializationError(String),

    #[error("Failed to export traces: {0}")]
    ExportError(String),

    #[error("Invalid trace context: {0}")]
    InvalidTraceContext(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for tracing operations
pub type TracingResult<T> = Result<T, TracingError>;

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Service name (e.g., "llm-research-api")
    pub service_name: String,

    /// Service version (e.g., "1.0.0")
    pub service_version: String,

    /// Deployment environment (e.g., "production", "staging", "development")
    pub environment: String,

    /// OTLP exporter endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: String,

    /// Enable tracing
    pub enabled: bool,

    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,

    /// Always sample errors
    pub always_sample_errors: bool,

    /// Maximum spans per trace
    pub max_spans_per_trace: u32,

    /// Custom resource attributes
    pub resource_attributes: HashMap<String, String>,

    /// Enable database tracing
    pub enable_db_tracing: bool,

    /// Sanitize database statements (remove sensitive data)
    pub sanitize_db_statements: bool,

    /// Enable console logging
    pub enable_console_logging: bool,

    /// Log level filter
    pub log_level: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "llm-research-api".to_string(),
            service_version: env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
            environment: env::var("DEPLOYMENT_ENV").unwrap_or_else(|_| "development".to_string()),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            enabled: env::var("OTEL_TRACING_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            sampling_rate: env::var("OTEL_SAMPLING_RATE")
                .unwrap_or_else(|_| "1.0".to_string())
                .parse()
                .unwrap_or(1.0),
            always_sample_errors: true,
            max_spans_per_trace: 1000,
            resource_attributes: HashMap::new(),
            enable_db_tracing: true,
            sanitize_db_statements: true,
            enable_console_logging: true,
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }
}

impl TracingConfig {
    /// Create a new builder for TracingConfig
    pub fn builder() -> TracingConfigBuilder {
        TracingConfigBuilder::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> TracingResult<()> {
        if self.service_name.is_empty() {
            return Err(TracingError::ConfigError(
                "Service name cannot be empty".to_string(),
            ));
        }

        if self.sampling_rate < 0.0 || self.sampling_rate > 1.0 {
            return Err(TracingError::ConfigError(
                "Sampling rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        if self.otlp_endpoint.is_empty() && self.enabled {
            return Err(TracingError::ConfigError(
                "OTLP endpoint cannot be empty when tracing is enabled".to_string(),
            ));
        }

        Ok(())
    }
}

/// Builder for TracingConfig
#[derive(Default)]
pub struct TracingConfigBuilder {
    service_name: Option<String>,
    service_version: Option<String>,
    environment: Option<String>,
    otlp_endpoint: Option<String>,
    enabled: Option<bool>,
    sampling_rate: Option<f64>,
    always_sample_errors: Option<bool>,
    max_spans_per_trace: Option<u32>,
    resource_attributes: HashMap<String, String>,
    enable_db_tracing: Option<bool>,
    sanitize_db_statements: Option<bool>,
    enable_console_logging: Option<bool>,
    log_level: Option<String>,
}

impl TracingConfigBuilder {
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    pub fn otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = Some(rate);
        self
    }

    pub fn always_sample_errors(mut self, sample: bool) -> Self {
        self.always_sample_errors = Some(sample);
        self
    }

    pub fn max_spans_per_trace(mut self, max: u32) -> Self {
        self.max_spans_per_trace = Some(max);
        self
    }

    pub fn resource_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.resource_attributes.insert(key.into(), value.into());
        self
    }

    pub fn enable_db_tracing(mut self, enable: bool) -> Self {
        self.enable_db_tracing = Some(enable);
        self
    }

    pub fn sanitize_db_statements(mut self, sanitize: bool) -> Self {
        self.sanitize_db_statements = Some(sanitize);
        self
    }

    pub fn enable_console_logging(mut self, enable: bool) -> Self {
        self.enable_console_logging = Some(enable);
        self
    }

    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    pub fn build(self) -> TracingConfig {
        let default = TracingConfig::default();
        TracingConfig {
            service_name: self.service_name.unwrap_or(default.service_name),
            service_version: self.service_version.unwrap_or(default.service_version),
            environment: self.environment.unwrap_or(default.environment),
            otlp_endpoint: self.otlp_endpoint.unwrap_or(default.otlp_endpoint),
            enabled: self.enabled.unwrap_or(default.enabled),
            sampling_rate: self.sampling_rate.unwrap_or(default.sampling_rate),
            always_sample_errors: self.always_sample_errors.unwrap_or(default.always_sample_errors),
            max_spans_per_trace: self.max_spans_per_trace.unwrap_or(default.max_spans_per_trace),
            resource_attributes: self.resource_attributes,
            enable_db_tracing: self.enable_db_tracing.unwrap_or(default.enable_db_tracing),
            sanitize_db_statements: self
                .sanitize_db_statements
                .unwrap_or(default.sanitize_db_statements),
            enable_console_logging: self
                .enable_console_logging
                .unwrap_or(default.enable_console_logging),
            log_level: self.log_level.unwrap_or(default.log_level),
        }
    }
}

/// Initialize OpenTelemetry tracing with the provided configuration
pub async fn init_tracing(config: TracingConfig) -> TracingResult<()> {
    config.validate()?;

    if !config.enabled {
        info!("OpenTelemetry tracing is disabled");
        return Ok(());
    }

    // Set up global propagator for W3C Trace Context
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Build resource attributes
    let mut resource_kvs = vec![
        KeyValue::new(SERVICE_NAME, config.service_name.clone()),
        KeyValue::new(SERVICE_VERSION, config.service_version.clone()),
        KeyValue::new(DEPLOYMENT_ENVIRONMENT, config.environment.clone()),
    ];

    // Add custom resource attributes
    for (key, value) in &config.resource_attributes {
        resource_kvs.push(KeyValue::new(key.clone(), value.clone()));
    }

    let resource = Resource::new(resource_kvs);

    // Create OTLP exporter
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.otlp_endpoint)
        .build()
        .map_err(|e| TracingError::InitializationError(e.to_string()))?;

    // Create sampler based on configuration
    let sampler = create_sampler(&config);

    // Build tracer provider
    let tracer_provider = TracerProvider::builder()
        .with_config(
            Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(128)
                .with_max_links_per_span(32)
                .with_resource(resource),
        )
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    // Set global tracer provider
    global::set_tracer_provider(tracer_provider.clone());

    // Create tracing layer
    let tracer = tracer_provider.tracer(config.service_name.clone());
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create filter
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))
        .map_err(|e| TracingError::ConfigError(e.to_string()))?;

    // Initialize subscriber with layers
    if config.enable_console_logging {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(telemetry_layer)
            .with(fmt_layer)
            .with(filter)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(telemetry_layer)
            .with(filter)
            .init();
    }

    info!(
        service_name = %config.service_name,
        service_version = %config.service_version,
        environment = %config.environment,
        otlp_endpoint = %config.otlp_endpoint,
        "OpenTelemetry tracing initialized"
    );

    Ok(())
}

/// Create a sampler based on configuration
fn create_sampler(config: &TracingConfig) -> Sampler {
    if config.always_sample_errors {
        // Use parent-based sampler with trace ID ratio as default
        Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(config.sampling_rate)))
    } else {
        Sampler::TraceIdRatioBased(config.sampling_rate)
    }
}

/// Shutdown the global tracer provider
pub async fn shutdown_tracing() -> TracingResult<()> {
    global::shutdown_tracer_provider();
    Ok(())
}

/// W3C Trace Context propagator for extracting and injecting trace context
pub struct TraceContextPropagation;

impl TraceContextPropagation {
    /// Extract trace context from HTTP headers
    pub fn extract(headers: &HeaderMap) -> Option<OtelContext> {
        use opentelemetry::propagation::TextMapPropagator;

        let propagator = TraceContextPropagator::new();
        let context = propagator.extract(&HeaderMapExtractor::new(headers));

        let span_context = context.span().span_context().clone();
        if span_context.is_valid() {
            Some(context)
        } else {
            None
        }
    }

    /// Inject trace context into HTTP headers
    pub fn inject(context: &OtelContext, headers: &mut HeaderMap) {
        use opentelemetry::propagation::TextMapPropagator;

        let propagator = TraceContextPropagator::new();
        propagator.inject_context(context, &mut HeaderMapInjector::new(headers));
    }

    /// Parse traceparent header manually
    pub fn parse_traceparent(traceparent: &str) -> Option<SpanContext> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        // version-traceid-parentid-traceflags
        let trace_id = TraceId::from_hex(parts[1]).ok()?;
        let span_id = SpanId::from_hex(parts[2]).ok()?;
        let trace_flags = TraceFlags::new(u8::from_str_radix(parts[3], 16).ok()?);

        Some(SpanContext::new(
            trace_id,
            span_id,
            trace_flags,
            true, // is_remote
            TraceState::default(),
        ))
    }
}

/// Extractor for reading trace context from HTTP headers
struct HeaderMapExtractor<'a> {
    headers: &'a HeaderMap,
}

impl<'a> HeaderMapExtractor<'a> {
    fn new(headers: &'a HeaderMap) -> Self {
        Self { headers }
    }
}

impl<'a> opentelemetry::propagation::Extractor for HeaderMapExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|k| k.as_str()).collect()
    }
}

/// Injector for writing trace context to HTTP headers
struct HeaderMapInjector<'a> {
    headers: &'a mut HeaderMap,
}

impl<'a> HeaderMapInjector<'a> {
    fn new(headers: &'a mut HeaderMap) -> Self {
        Self { headers }
    }
}

impl<'a> opentelemetry::propagation::Injector for HeaderMapInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            if let Ok(header_name) = axum::http::HeaderName::from_str(key) {
                self.headers.insert(header_name, header_value);
            }
        }
    }
}

/// Axum middleware for automatic HTTP request tracing
pub async fn tracing_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Clone necessary header values before moving request
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let host = request
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Extract path template if available
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    // Extract trace context from headers
    let parent_context = TraceContextPropagation::extract(request.headers());

    // Create span for the request
    let span = if let Some(ctx) = parent_context {
        let span = tracing::info_span!(
            "http_request",
            otel.kind = "server",
            http.method = %method,
            http.route = %path,
            http.target = %uri,
        );
        span.set_parent(ctx);
        span
    } else {
        tracing::info_span!(
            "http_request",
            otel.kind = "server",
            http.method = %method,
            http.route = %path,
            http.target = %uri,
        )
    };

    // Execute request within span
    let response = async {
        // Add HTTP semantic conventions
        if let Some(ua) = user_agent {
            Span::current().record("http.user_agent", ua.as_str());
        }

        if let Some(h) = host {
            Span::current().record("http.host", h.as_str());
        }

        // Process request
        let response = next.run(request).await;

        // Record response status
        let status = response.status();
        Span::current().record("http.status_code", status.as_u16());

        // Set span status based on HTTP status code
        if status.is_server_error() {
            tracing::error!("Request failed with server error: {}", status);
        } else if status.is_client_error() {
            tracing::warn!("Request failed with client error: {}", status);
        }

        response
    }
    .instrument(span)
    .await;

    Ok(response)
}

/// Span utilities for manual span creation
pub struct SpanBuilder {
    name: String,
    kind: SpanKind,
    attributes: Vec<KeyValue>,
}

impl SpanBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: SpanKind::Internal,
            attributes: Vec::new(),
        }
    }

    pub fn kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(KeyValue::new(key.into(), value.into()));
        self
    }

    pub fn user_id(self, user_id: impl fmt::Display) -> Self {
        self.attribute("user.id", user_id.to_string())
    }

    pub fn experiment_id(self, experiment_id: impl fmt::Display) -> Self {
        self.attribute("experiment.id", experiment_id.to_string())
    }

    pub fn model_id(self, model_id: impl fmt::Display) -> Self {
        self.attribute("model.id", model_id.to_string())
    }

    pub fn dataset_id(self, dataset_id: impl fmt::Display) -> Self {
        self.attribute("dataset.id", dataset_id.to_string())
    }

    pub fn create(self) -> tracing::Span {
        let span = tracing::info_span!(target: "llm_research_api", "operation");

        // Use span.record for all fields since we can't use dynamic names in the macro
        span.record("otel.name", self.name.as_str());
        span.record("otel.kind", format!("{:?}", self.kind).as_str());

        for attr in self.attributes {
            span.record(attr.key.as_str(), attr.value.as_str().as_ref());
        }

        span
    }
}

/// Create a new span with the given name and attributes
pub fn create_span(name: impl Into<String>) -> SpanBuilder {
    SpanBuilder::new(name)
}

/// Record an error on the current span
pub fn record_error(error: &dyn std::error::Error) {
    let span = Span::current();
    span.record("error", true);
    span.record("error.message", error.to_string().as_str());

    // Get the OpenTelemetry span context and set status
    let context = span.context();
    context.span().set_status(Status::Error {
        description: error.to_string().into(),
    });
}

/// Record an event on the current span
pub fn record_event(name: &str, attributes: Vec<(&str, &str)>) {
    let span = Span::current();
    let kvs: Vec<KeyValue> = attributes
        .into_iter()
        .map(|(k, v)| KeyValue::new(k.to_string(), v.to_string()))
        .collect();

    span.context()
        .span()
        .add_event(name.to_string(), kvs);
}

/// Database query tracing wrapper
pub struct DbSpan {
    operation: String,
    statement: String,
    sanitize: bool,
}

impl DbSpan {
    pub fn new(operation: impl Into<String>, statement: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            statement: statement.into(),
            sanitize: true,
        }
    }

    pub fn sanitize(mut self, sanitize: bool) -> Self {
        self.sanitize = sanitize;
        self
    }

    fn sanitize_statement(&self, statement: &str) -> String {
        if !self.sanitize {
            return statement.to_string();
        }

        // Simple sanitization: replace values in WHERE clauses
        // More sophisticated sanitization would use a SQL parser
        let mut sanitized = statement.to_string();

        // Replace string literals
        let re = regex::Regex::new(r"'[^']*'").unwrap();
        sanitized = re.replace_all(&sanitized, "'?'").to_string();

        // Replace numbers (simple approach)
        let re = regex::Regex::new(r"\b\d+\b").unwrap();
        sanitized = re.replace_all(&sanitized, "?").to_string();

        sanitized
    }

    pub fn create_span(&self) -> tracing::Span {
        let statement = if self.sanitize {
            self.sanitize_statement(&self.statement)
        } else {
            self.statement.clone()
        };

        tracing::info_span!(
            "db_query",
            otel.kind = "client",
            db.system = "postgresql",
            db.operation = %self.operation,
            db.statement = %statement,
        )
    }

    /// Execute a database query within a traced span
    pub async fn execute<T, F>(
        self,
        query_fn: F,
    ) -> Result<T, sqlx::Error>
    where
        F: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        let span = self.create_span();
        query_fn.instrument(span).await
    }
}

/// Get the current trace ID for logging correlation
pub fn current_trace_id() -> Option<String> {
    let context = Span::current().context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.trace_id().to_string())
    } else {
        None
    }
}

/// Get the current span ID
pub fn current_span_id() -> Option<String> {
    let context = Span::current().context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.span_id().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "llm-research-api");
        assert!(config.enabled);
        assert_eq!(config.sampling_rate, 1.0);
        assert!(config.always_sample_errors);
    }

    #[test]
    fn test_tracing_config_builder() {
        let config = TracingConfig::builder()
            .service_name("test-service")
            .service_version("2.0.0")
            .environment("test")
            .otlp_endpoint("http://test:4317")
            .sampling_rate(0.5)
            .build();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "2.0.0");
        assert_eq!(config.environment, "test");
        assert_eq!(config.sampling_rate, 0.5);
    }

    #[test]
    fn test_config_validation_success() {
        let config = TracingConfig::builder()
            .service_name("test")
            .otlp_endpoint("http://localhost:4317")
            .build();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_service_name() {
        let config = TracingConfig {
            service_name: String::new(),
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_sampling_rate() {
        let config = TracingConfig {
            sampling_rate: 1.5,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_parse_traceparent_valid() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let span_context = TraceContextPropagation::parse_traceparent(traceparent);

        assert!(span_context.is_some());
        let ctx = span_context.unwrap();
        assert!(ctx.is_valid());
        assert!(ctx.is_remote());
    }

    #[test]
    fn test_parse_traceparent_invalid() {
        let traceparent = "invalid-traceparent";
        let span_context = TraceContextPropagation::parse_traceparent(traceparent);

        assert!(span_context.is_none());
    }

    #[test]
    fn test_span_builder() {
        let span = create_span("test_operation")
            .user_id("user123")
            .experiment_id("exp456")
            .attribute("custom", "value")
            .create();

        // The span name is now static "operation" with dynamic name in attributes
        assert_eq!(span.metadata().unwrap().name(), "operation");
    }

    #[test]
    fn test_db_span_sanitization() {
        let db_span = DbSpan::new("SELECT", "SELECT * FROM users WHERE id = 123");
        let sanitized = db_span.sanitize_statement("SELECT * FROM users WHERE id = 123");

        // Should replace numbers with ?
        assert!(sanitized.contains('?'));
    }

    #[tokio::test]
    async fn test_trace_context_extraction() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );

        let context = TraceContextPropagation::extract(&headers);
        assert!(context.is_some());
    }

    #[test]
    fn test_resource_attributes() {
        let config = TracingConfig::builder()
            .service_name("test")
            .resource_attribute("deployment.region", "us-west-2")
            .resource_attribute("k8s.namespace", "production")
            .build();

        assert_eq!(
            config.resource_attributes.get("deployment.region"),
            Some(&"us-west-2".to_string())
        );
        assert_eq!(
            config.resource_attributes.get("k8s.namespace"),
            Some(&"production".to_string())
        );
    }
}
