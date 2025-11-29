//! Comprehensive structured logging and request correlation system
//!
//! This module provides enterprise-grade logging capabilities including:
//! - Structured JSON logging with configurable formats
//! - Request correlation IDs for distributed tracing
//! - Request/response logging middleware
//! - Sensitive data redaction
//! - Thread-local log context propagation
//! - OpenTelemetry compatibility
//!
//! # Examples
//!
//! ```rust,no_run
//! use llm_research_api::observability::logging::{init_logging, LogConfig, LogFormat};
//!
//! // Initialize logging
//! let config = LogConfig::default();
//! init_logging(config).expect("Failed to initialize logging");
//! ```

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};
use tokio::task_local;
use tracing::{info, Span};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// HTTP header for request correlation ID
pub const REQUEST_ID_HEADER: &str = "X-Request-ID";

/// Default sensitive headers to redact
pub const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-csrf-token",
    "proxy-authorization",
];

/// Default sensitive patterns to redact in logs
pub const SENSITIVE_PATTERNS: &[&str] = &[
    r"password\s*[:=]\s*[^\s,}]+",
    r"token\s*[:=]\s*[^\s,}]+",
    r"api[_-]?key\s*[:=]\s*[^\s,}]+",
    r"secret\s*[:=]\s*[^\s,}]+",
    r"bearer\s+[^\s,}]+",
];

// ============================================================================
// Configuration
// ============================================================================

/// Log format configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON format for production (machine-readable)
    Json,
    /// Pretty format for development (human-readable)
    Pretty,
    /// Compact format for minimal output
    Compact,
}

impl Default for LogFormat {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        return Self::Pretty;

        #[cfg(not(debug_assertions))]
        return Self::Json;
    }
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Enable log rotation
    pub enabled: bool,
    /// Maximum size per log file (in bytes)
    pub max_size: u64,
    /// Maximum number of log files to keep
    pub max_files: usize,
    /// Directory for log files
    pub directory: String,
    /// File name prefix
    pub file_prefix: String,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size: 100 * 1024 * 1024, // 100 MB
            max_files: 10,
            directory: "logs".to_string(),
            file_prefix: "llm-research-api".to_string(),
        }
    }
}

/// Main logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log format
    pub format: LogFormat,
    /// Default log level
    pub level: String,
    /// Per-module log levels (e.g., "sqlx=warn,tower_http=debug")
    pub filter: Option<String>,
    /// Enable log rotation
    pub rotation: LogRotationConfig,
    /// Enable request/response body logging
    pub log_bodies: bool,
    /// Maximum body size to log (in bytes)
    pub max_body_size: usize,
    /// Enable OpenTelemetry integration
    pub opentelemetry_enabled: bool,
    /// Custom redaction patterns (regex)
    pub custom_redaction_patterns: Vec<String>,
    /// Sensitive headers to redact (in addition to defaults)
    pub custom_sensitive_headers: Vec<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::default(),
            level: "info".to_string(),
            filter: None,
            rotation: LogRotationConfig::default(),
            log_bodies: false,
            max_body_size: 4096, // 4 KB
            opentelemetry_enabled: false,
            custom_redaction_patterns: Vec::new(),
            custom_sensitive_headers: Vec::new(),
        }
    }
}

// ============================================================================
// Request Context
// ============================================================================

/// Log context that can be attached to requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
    /// Unique request correlation ID
    pub request_id: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Experiment ID (if applicable)
    pub experiment_id: Option<Uuid>,
    /// Run ID (if applicable)
    pub run_id: Option<Uuid>,
    /// Custom key-value pairs
    pub custom_fields: HashMap<String, String>,
}

impl LogContext {
    /// Create a new log context with a generated request ID
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            user_id: None,
            experiment_id: None,
            run_id: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Create a log context with a specific request ID
    pub fn with_request_id(request_id: String) -> Self {
        Self {
            request_id,
            user_id: None,
            experiment_id: None,
            run_id: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Set the user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the experiment ID
    pub fn with_experiment_id(mut self, experiment_id: Uuid) -> Self {
        self.experiment_id = Some(experiment_id);
        self
    }

    /// Set the run ID
    pub fn with_run_id(mut self, run_id: Uuid) -> Self {
        self.run_id = Some(run_id);
        self
    }

    /// Add a custom field
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_fields.insert(key.into(), value.into());
        self
    }

    /// Add the context fields to a tracing span
    pub fn attach_to_span(&self, span: &Span) {
        span.record("request_id", &self.request_id.as_str());

        if let Some(ref user_id) = self.user_id {
            span.record("user_id", user_id.as_str());
        }

        if let Some(experiment_id) = self.experiment_id {
            span.record("experiment_id", experiment_id.to_string().as_str());
        }

        if let Some(run_id) = self.run_id {
            span.record("run_id", run_id.to_string().as_str());
        }

        for (key, value) in &self.custom_fields {
            span.record(key.as_str(), value.as_str());
        }
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}

// Task-local storage for log context
task_local! {
    pub static LOG_CONTEXT: LogContext;
}

/// Get the current log context, or create a new one if not set
pub fn current_context() -> LogContext {
    LOG_CONTEXT.try_with(|ctx| ctx.clone()).unwrap_or_default()
}

/// Execute a closure with a specific log context
pub async fn with_context<F, Fut, T>(context: LogContext, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    LOG_CONTEXT.scope(context, f()).await
}

// ============================================================================
// Sensitive Data Redaction
// ============================================================================

/// Utility for redacting sensitive data in logs
#[derive(Debug, Clone)]
pub struct SensitiveDataRedactor {
    patterns: Vec<Regex>,
    sensitive_headers: Vec<String>,
}

impl SensitiveDataRedactor {
    /// Create a new redactor with default patterns
    pub fn new() -> Self {
        let mut patterns = Vec::new();

        for pattern in SENSITIVE_PATTERNS {
            if let Ok(regex) = Regex::new(pattern) {
                patterns.push(regex);
            }
        }

        Self {
            patterns,
            sensitive_headers: SENSITIVE_HEADERS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a redactor with custom configuration
    pub fn with_config(config: &LogConfig) -> Self {
        let mut redactor = Self::new();

        // Add custom patterns
        for pattern in &config.custom_redaction_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                redactor.patterns.push(regex);
            }
        }

        // Add custom sensitive headers
        redactor.sensitive_headers.extend(
            config.custom_sensitive_headers.iter().cloned()
        );

        redactor
    }

    /// Redact sensitive data in a string
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();

        for pattern in &self.patterns {
            result = pattern.replace_all(&result, "[REDACTED]").to_string();
        }

        result
    }

    /// Redact sensitive headers
    pub fn redact_headers(&self, headers: &HeaderMap) -> HashMap<String, String> {
        let mut sanitized = HashMap::new();

        for (name, value) in headers.iter() {
            let name_str = name.as_str();
            let is_sensitive = self.sensitive_headers.iter()
                .any(|h| h.eq_ignore_ascii_case(name_str));

            let value_str = if is_sensitive {
                "[REDACTED]".to_string()
            } else {
                value.to_str().unwrap_or("[INVALID UTF-8]").to_string()
            };

            sanitized.insert(name_str.to_string(), value_str);
        }

        sanitized
    }

    /// Redact sensitive data in JSON
    pub fn redact_json(&self, json: &serde_json::Value) -> serde_json::Value {
        match json {
            serde_json::Value::Object(map) => {
                let mut redacted_map = serde_json::Map::new();

                for (key, value) in map {
                    let key_lower = key.to_lowercase();
                    let is_sensitive = key_lower.contains("password")
                        || key_lower.contains("token")
                        || key_lower.contains("secret")
                        || key_lower.contains("key");

                    let redacted_value = if is_sensitive {
                        serde_json::Value::String("[REDACTED]".to_string())
                    } else {
                        self.redact_json(value)
                    };

                    redacted_map.insert(key.clone(), redacted_value);
                }

                serde_json::Value::Object(redacted_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(
                    arr.iter().map(|v| self.redact_json(v)).collect()
                )
            }
            _ => json.clone(),
        }
    }
}

impl Default for SensitiveDataRedactor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request/Response Logging
// ============================================================================

/// Request metadata for logging
#[derive(Debug, Serialize)]
struct RequestLogEntry {
    request_id: String,
    method: String,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
}

/// Response metadata for logging
#[derive(Debug, Serialize)]
struct ResponseLogEntry {
    request_id: String,
    status: u16,
    duration_ms: u64,
    headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

/// Middleware state for request logging
#[derive(Clone)]
pub struct RequestLoggingState {
    config: Arc<LogConfig>,
    redactor: Arc<SensitiveDataRedactor>,
}

impl RequestLoggingState {
    /// Create a new request logging state
    pub fn new(config: LogConfig) -> Self {
        let redactor = SensitiveDataRedactor::with_config(&config);

        Self {
            config: Arc::new(config),
            redactor: Arc::new(redactor),
        }
    }
}

/// Extract or generate request ID from headers
fn extract_request_id(headers: &HeaderMap) -> String {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Middleware for request/response logging with correlation IDs
pub async fn request_logging_middleware(
    State(state): State<RequestLoggingState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();

    // Extract or generate request ID
    let request_id = extract_request_id(request.headers());

    // Create log context
    let context = LogContext::with_request_id(request_id.clone());

    // Extract request details
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let version = format!("{:?}", request.version());
    let headers = state.redactor.redact_headers(request.headers());

    // Log the incoming request (entry prepared for structured logging if needed)
    let _request_entry = RequestLogEntry {
        request_id: request_id.clone(),
        method: method.clone(),
        uri: uri.clone(),
        version,
        headers,
        body: None, // Body logging can be added if needed
        user_id: None,
    };

    info!(
        request.method = %method,
        request.uri = %uri,
        request.id = %request_id,
        "Incoming request"
    );

    // Process the request within the context
    let response = LOG_CONTEXT.scope(context, async move {
        match next.run(request).await {
            response => {
                let duration = start.elapsed();
                let status = response.status();

                // Extract response headers
                let response_headers = state.redactor.redact_headers(response.headers());

                // Log the response (entry prepared for structured logging if needed)
                let _response_entry = ResponseLogEntry {
                    request_id: request_id.clone(),
                    status: status.as_u16(),
                    duration_ms: duration.as_millis() as u64,
                    headers: response_headers,
                    body: None,
                };

                // Log based on status code
                if status.is_server_error() {
                    tracing::error!(
                        response.status = status.as_u16(),
                        response.duration_ms = duration.as_millis() as u64,
                        request.id = %request_id,
                        "Request completed with server error"
                    );
                } else if status.is_client_error() {
                    tracing::warn!(
                        response.status = status.as_u16(),
                        response.duration_ms = duration.as_millis() as u64,
                        request.id = %request_id,
                        "Request completed with client error"
                    );
                } else {
                    tracing::info!(
                        response.status = status.as_u16(),
                        response.duration_ms = duration.as_millis() as u64,
                        request.id = %request_id,
                        "Request completed successfully"
                    );
                }

                // Add request ID to response headers
                let (mut parts, body) = response.into_parts();
                parts.headers.insert(
                    REQUEST_ID_HEADER,
                    request_id.parse().unwrap(),
                );

                Ok(Response::from_parts(parts, body))
            }
        }
    }).await;

    response
}

/// Create a request logging middleware with the given configuration
///
/// # Example
///
/// ```rust,ignore
/// use llm_research_api::observability::logging::{LogConfig, request_logging_middleware, RequestLoggingState};
/// use axum::Router;
///
/// let config = LogConfig::default();
/// let state = RequestLoggingState::new(config);
/// let app = Router::new().layer(axum::middleware::from_fn_with_state(state, request_logging_middleware));
/// ```
pub fn create_request_logging_middleware(config: LogConfig) -> RequestLoggingState {
    RequestLoggingState::new(config)
}

// ============================================================================
// Logging Initialization
// ============================================================================

/// Initialize the logging system with the given configuration
pub fn init_logging(config: LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Build the environment filter
    let env_filter = if let Some(ref filter) = config.filter {
        EnvFilter::try_new(filter)?
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&config.level))
    };

    // Create the subscriber based on format
    match config.format {
        LogFormat::Json => {
            let json_layer = fmt::layer()
                .json()
                .with_span_events(FmtSpan::CLOSE)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_filter(env_filter);

            tracing_subscriber::registry()
                .with(json_layer)
                .init();
        }
        LogFormat::Pretty => {
            let pretty_layer = fmt::layer()
                .pretty()
                .with_span_events(FmtSpan::CLOSE)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_filter(env_filter);

            tracing_subscriber::registry()
                .with(pretty_layer)
                .init();
        }
        LogFormat::Compact => {
            let compact_layer = fmt::layer()
                .compact()
                .with_span_events(FmtSpan::CLOSE)
                .with_filter(env_filter);

            tracing_subscriber::registry()
                .with(compact_layer)
                .init();
        }
    }

    info!("Logging system initialized with format: {:?}", config.format);
    Ok(())
}

/// Initialize logging with default configuration
pub fn init_default_logging() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(LogConfig::default())
}

// ============================================================================
// Utility Macros
// ============================================================================

/// Log with the current request context
#[macro_export]
macro_rules! log_with_context {
    ($level:expr, $($arg:tt)*) => {{
        let context = $crate::observability::logging::current_context();
        tracing::event!(
            $level,
            request_id = %context.request_id,
            user_id = ?context.user_id,
            experiment_id = ?context.experiment_id,
            run_id = ?context.run_id,
            $($arg)*
        );
    }};
}

/// Info log with context
#[macro_export]
macro_rules! info_ctx {
    ($($arg:tt)*) => {
        $crate::log_with_context!(tracing::Level::INFO, $($arg)*)
    };
}

/// Error log with context
#[macro_export]
macro_rules! error_ctx {
    ($($arg:tt)*) => {
        $crate::log_with_context!(tracing::Level::ERROR, $($arg)*)
    };
}

/// Warn log with context
#[macro_export]
macro_rules! warn_ctx {
    ($($arg:tt)*) => {
        $crate::log_with_context!(tracing::Level::WARN, $($arg)*)
    };
}

/// Debug log with context
#[macro_export]
macro_rules! debug_ctx {
    ($($arg:tt)*) => {
        $crate::log_with_context!(tracing::Level::DEBUG, $($arg)*)
    };
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[test]
    fn test_log_context_creation() {
        let context = LogContext::new();
        assert!(!context.request_id.is_empty());
        assert!(context.user_id.is_none());
        assert!(context.experiment_id.is_none());
        assert!(context.run_id.is_none());
    }

    #[test]
    fn test_log_context_builder() {
        let context = LogContext::new()
            .with_user_id("user-123")
            .with_experiment_id(Uuid::new_v4())
            .with_field("custom", "value");

        assert_eq!(context.user_id, Some("user-123".to_string()));
        assert!(context.experiment_id.is_some());
        assert_eq!(context.custom_fields.get("custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_sensitive_data_redaction() {
        let redactor = SensitiveDataRedactor::new();

        // Test password redaction
        let text = "password: secret123";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("secret123"));

        // Test token redaction
        let text = "token=abc123def";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[REDACTED]"));

        // Test API key redaction
        let text = "api_key: sk-1234567890";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_header_redaction() {
        let redactor = SensitiveDataRedactor::new();
        let mut headers = HeaderMap::new();

        headers.insert("authorization", "Bearer token123".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-api-key", "secret-key".parse().unwrap());

        let redacted = redactor.redact_headers(&headers);

        assert_eq!(redacted.get("authorization"), Some(&"[REDACTED]".to_string()));
        assert_eq!(redacted.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(redacted.get("x-api-key"), Some(&"[REDACTED]".to_string()));
    }

    #[test]
    fn test_json_redaction() {
        let redactor = SensitiveDataRedactor::new();

        let json = serde_json::json!({
            "username": "john",
            "password": "secret123",
            "api_key": "sk-1234",
            "data": {
                "token": "abc123",
                "value": 42
            }
        });

        let redacted = redactor.redact_json(&json);

        // Check that sensitive fields are redacted
        assert_eq!(redacted["username"], "john");
        assert_eq!(redacted["password"], "[REDACTED]");
        assert_eq!(redacted["api_key"], "[REDACTED]");
        assert_eq!(redacted["data"]["token"], "[REDACTED]");
        assert_eq!(redacted["data"]["value"], 42);
    }

    #[test]
    fn test_request_id_extraction() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID_HEADER, "custom-request-id".parse().unwrap());

        let request_id = extract_request_id(&headers);
        assert_eq!(request_id, "custom-request-id");
    }

    #[test]
    fn test_request_id_generation() {
        let headers = HeaderMap::new();
        let request_id = extract_request_id(&headers);

        // Should generate a valid UUID
        assert!(Uuid::parse_str(&request_id).is_ok());
    }

    #[tokio::test]
    async fn test_context_propagation() {
        let context = LogContext::new().with_user_id("test-user");
        let request_id = context.request_id.clone();

        with_context(context, || async move {
            let current = current_context();
            assert_eq!(current.request_id, request_id);
            assert_eq!(current.user_id, Some("test-user".to_string()));
        }).await;
    }

    #[tokio::test]
    async fn test_request_logging_middleware() {
        // Create a simple handler
        async fn handler() -> &'static str {
            "OK"
        }

        let config = LogConfig::default();
        let state = RequestLoggingState::new(config);

        let app = Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn_with_state(
                state,
                request_logging_middleware,
            ));

        // Make a request
        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Check that response has request ID header
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_id_propagation() {
        async fn handler() -> &'static str {
            "OK"
        }

        let config = LogConfig::default();
        let state = RequestLoggingState::new(config);

        let app = Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn_with_state(
                state,
                request_logging_middleware,
            ));

        // Make a request with custom request ID
        let request = Request::builder()
            .uri("/test")
            .header(REQUEST_ID_HEADER, "custom-id-123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Check that the same request ID is returned
        let response_id = response.headers()
            .get(REQUEST_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(response_id, "custom-id-123");
    }

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();

        #[cfg(debug_assertions)]
        assert_eq!(config.format, LogFormat::Pretty);

        #[cfg(not(debug_assertions))]
        assert_eq!(config.format, LogFormat::Json);

        assert_eq!(config.level, "info");
        assert!(!config.log_bodies);
        assert_eq!(config.max_body_size, 4096);
    }

    #[test]
    fn test_custom_redaction_patterns() {
        let mut config = LogConfig::default();
        // Use case-insensitive flag (?i) to match SSN in any case
        config.custom_redaction_patterns.push(r"(?i)ssn\s*[:=]\s*\d{3}-\d{2}-\d{4}".to_string());

        let redactor = SensitiveDataRedactor::with_config(&config);

        let text = "User SSN: 123-45-6789";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("123-45-6789"));
    }

    #[test]
    fn test_custom_sensitive_headers() {
        let mut config = LogConfig::default();
        config.custom_sensitive_headers.push("x-custom-secret".to_string());

        let redactor = SensitiveDataRedactor::with_config(&config);

        let mut headers = HeaderMap::new();
        headers.insert("x-custom-secret", "secret-value".parse().unwrap());

        let redacted = redactor.redact_headers(&headers);
        assert_eq!(redacted.get("x-custom-secret"), Some(&"[REDACTED]".to_string()));
    }

    #[test]
    fn test_nested_json_redaction() {
        let redactor = SensitiveDataRedactor::new();

        let json = serde_json::json!({
            "user": {
                "name": "John",
                "credentials": {
                    "password": "secret",
                    "api_token": "token123"
                }
            },
            "settings": {
                "theme": "dark"
            }
        });

        let redacted = redactor.redact_json(&json);

        assert_eq!(redacted["user"]["name"], "John");
        assert_eq!(redacted["user"]["credentials"]["password"], "[REDACTED]");
        assert_eq!(redacted["user"]["credentials"]["api_token"], "[REDACTED]");
        assert_eq!(redacted["settings"]["theme"], "dark");
    }

    #[test]
    fn test_array_json_redaction() {
        let redactor = SensitiveDataRedactor::new();

        let json = serde_json::json!([
            {"name": "user1", "password": "pass1"},
            {"name": "user2", "api_key": "key2"}
        ]);

        let redacted = redactor.redact_json(&json);

        assert_eq!(redacted[0]["name"], "user1");
        assert_eq!(redacted[0]["password"], "[REDACTED]");
        assert_eq!(redacted[1]["name"], "user2");
        assert_eq!(redacted[1]["api_key"], "[REDACTED]");
    }
}
