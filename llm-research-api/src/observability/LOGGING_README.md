# Structured Logging and Request Correlation System

## Overview

The logging module provides enterprise-grade structured logging capabilities with request correlation, sensitive data redaction, and OpenTelemetry compatibility.

## Features

### 1. **Structured JSON Logging**
- JSON format for production (machine-readable)
- Pretty format for development (human-readable)
- Compact format for minimal output
- Configurable log levels per module
- Environment-based filter configuration

### 2. **Request Correlation IDs**
- Automatic UUID v4 generation for each request
- X-Request-ID header extraction and propagation
- Request ID included in all log entries
- Response header injection for tracing

### 3. **Request/Response Logging Middleware**
- Logs incoming requests with method, path, headers
- Logs response status and duration
- Automatic header sanitization
- Configurable body logging (disabled by default)
- Severity-based logging (ERROR for 5xx, WARN for 4xx, INFO for 2xx)

### 4. **Sensitive Data Redaction**
- Automatic redaction of sensitive headers:
  - Authorization
  - Cookie/Set-Cookie
  - X-API-Key
  - X-Auth-Token
  - Proxy-Authorization
- Pattern-based redaction for:
  - Passwords
  - Tokens
  - API keys
  - Secrets
  - Bearer tokens
- Custom redaction patterns support
- JSON field redaction (password, token, secret, key fields)

### 5. **Log Context Propagation**
- Thread-local storage for request context
- Includes request_id, user_id, experiment_id, run_id
- Custom fields support
- Automatic context propagation across async boundaries

## Usage

### Basic Initialization

```rust
use llm_research_api::observability::logging::{init_logging, LogConfig, LogFormat};

// Initialize with default configuration
init_default_logging()?;

// Or with custom configuration
let config = LogConfig {
    format: LogFormat::Json,
    level: "info".to_string(),
    filter: Some("llm_research_api=debug,sqlx=warn".to_string()),
    ..Default::default()
};
init_logging(config)?;
```

### Adding Request Logging Middleware

```rust
use axum::{Router, middleware};
use llm_research_api::observability::logging::{
    LogConfig, RequestLoggingState, request_logging_middleware
};

let log_config = LogConfig::default();
let log_state = RequestLoggingState::new(log_config);

let app = Router::new()
    .route("/api/health", get(health_handler))
    .layer(middleware::from_fn_with_state(
        (log_state,),
        request_logging_middleware
    ));
```

### Using Log Context

```rust
use llm_research_api::observability::logging::{LogContext, with_context};
use uuid::Uuid;

async fn process_request(request_id: String, user_id: String) {
    let context = LogContext::with_request_id(request_id)
        .with_user_id(user_id)
        .with_experiment_id(Uuid::new_v4())
        .with_field("custom_key", "custom_value");

    with_context(context, || async {
        // All logs within this scope will include the context
        tracing::info!("Processing request");
        // request_id, user_id, experiment_id automatically included
    }).await;
}
```

### Sensitive Data Redaction

```rust
use llm_research_api::observability::logging::SensitiveDataRedactor;
use axum::http::HeaderMap;

let redactor = SensitiveDataRedactor::new();

// Redact text
let safe_text = redactor.redact("password: secret123");
// Output: "password: [REDACTED]"

// Redact headers
let mut headers = HeaderMap::new();
headers.insert("authorization", "Bearer token123".parse().unwrap());
let safe_headers = redactor.redact_headers(&headers);
// authorization: "[REDACTED]"

// Redact JSON
let json = serde_json::json!({
    "username": "john",
    "password": "secret",
    "api_key": "sk-1234"
});
let safe_json = redactor.redact_json(&json);
// password and api_key fields will be "[REDACTED]"
```

### Custom Redaction Patterns

```rust
use llm_research_api::observability::logging::{LogConfig, SensitiveDataRedactor};

let mut config = LogConfig::default();
config.custom_redaction_patterns.push(r"ssn\s*[:=]\s*\d{3}-\d{2}-\d{4}".to_string());
config.custom_sensitive_headers.push("X-Custom-Secret".to_string());

let redactor = SensitiveDataRedactor::with_config(&config);
```

## Configuration

### LogConfig

```rust
pub struct LogConfig {
    /// Log format (Json, Pretty, Compact)
    pub format: LogFormat,

    /// Default log level (trace, debug, info, warn, error)
    pub level: String,

    /// Per-module log levels (e.g., "sqlx=warn,tower_http=debug")
    pub filter: Option<String>,

    /// Log rotation configuration
    pub rotation: LogRotationConfig,

    /// Enable request/response body logging
    pub log_bodies: bool,

    /// Maximum body size to log (bytes)
    pub max_body_size: usize,

    /// Enable OpenTelemetry integration
    pub opentelemetry_enabled: bool,

    /// Custom redaction patterns (regex)
    pub custom_redaction_patterns: Vec<String>,

    /// Additional sensitive headers to redact
    pub custom_sensitive_headers: Vec<String>,
}
```

### LogRotationConfig

```rust
pub struct LogRotationConfig {
    /// Enable log rotation
    pub enabled: bool,

    /// Maximum size per log file (bytes)
    pub max_size: u64,

    /// Maximum number of log files to keep
    pub max_files: usize,

    /// Directory for log files
    pub directory: String,

    /// File name prefix
    pub file_prefix: String,
}
```

## Request Correlation

The middleware automatically:

1. Extracts request ID from `X-Request-ID` header if present
2. Generates a new UUID v4 if not present
3. Adds request ID to all log entries during request processing
4. Injects `X-Request-ID` header in the response

Example log output (JSON format):

```json
{
  "timestamp": "2025-11-28T12:34:56.789Z",
  "level": "INFO",
  "message": "Incoming request",
  "request": {
    "method": "GET",
    "uri": "/api/experiments/123",
    "id": "550e8400-e29b-41d4-a716-446655440000"
  }
}

{
  "timestamp": "2025-11-28T12:34:56.891Z",
  "level": "INFO",
  "message": "Request completed successfully",
  "request": {
    "id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "response": {
    "status": 200,
    "duration_ms": 102
  }
}
```

## Log Levels

The module uses the standard Rust tracing levels:

- **ERROR**: Server errors (5xx), critical failures
- **WARN**: Client errors (4xx), deprecated features
- **INFO**: Normal operations, request/response logging
- **DEBUG**: Detailed information for debugging
- **TRACE**: Very detailed information

### Per-Module Configuration

```rust
let config = LogConfig {
    filter: Some(concat!(
        "llm_research_api=debug,",
        "sqlx=warn,",
        "tower_http=info,",
        "hyper=warn"
    ).to_string()),
    ..Default::default()
};
```

## Security Considerations

### Default Sensitive Headers

The following headers are automatically redacted:
- `authorization`
- `cookie`
- `set-cookie`
- `x-api-key`
- `x-auth-token`
- `x-csrf-token`
- `proxy-authorization`

### Default Sensitive Patterns

The following patterns are automatically redacted from log messages:
- `password\s*[:=]\s*[^\s,}]+`
- `token\s*[:=]\s*[^\s,}]+`
- `api[_-]?key\s*[:=]\s*[^\s,}]+`
- `secret\s*[:=]\s*[^\s,}]+`
- `bearer\s+[^\s,}]+`

### JSON Field Redaction

JSON fields with these names are automatically redacted:
- Fields containing "password"
- Fields containing "token"
- Fields containing "secret"
- Fields containing "key"

## Testing

The module includes comprehensive tests for:

- Request ID generation and propagation
- Context creation and propagation
- Sensitive data redaction (text, headers, JSON)
- Middleware functionality
- Custom redaction patterns
- Nested JSON redaction

Run tests with:

```bash
cargo test --package llm-research-api observability::logging
```

## Integration with OpenTelemetry

The logging system is designed to be compatible with OpenTelemetry:

```rust
let config = LogConfig {
    opentelemetry_enabled: true,
    ..Default::default()
};
```

When enabled, log entries will be correlated with OpenTelemetry traces and spans.

## Performance Considerations

1. **Regex Compilation**: Redaction patterns are compiled once during `SensitiveDataRedactor` initialization
2. **Arc<T>**: Configuration and redactor are wrapped in `Arc` for efficient cloning in middleware
3. **Lazy Evaluation**: Body logging is disabled by default to avoid performance overhead
4. **Async Context**: Uses `tokio::task_local!` for efficient context propagation

## Best Practices

1. **Production vs Development**: Use JSON format in production, Pretty format in development
2. **Log Levels**: Set appropriate log levels to avoid noise
3. **Body Logging**: Keep disabled unless debugging specific issues
4. **Custom Patterns**: Add business-specific sensitive patterns
5. **Request IDs**: Always propagate X-Request-ID from upstream services
6. **Context Fields**: Add custom fields for domain-specific tracking

## Example: Complete Setup

```rust
use axum::{Router, routing::get};
use llm_research_api::observability::logging::{
    init_logging, LogConfig, LogFormat, RequestLoggingState, request_logging_middleware
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let log_config = LogConfig {
        format: LogFormat::Json,
        level: "info".to_string(),
        filter: Some("llm_research_api=debug,sqlx=warn".to_string()),
        log_bodies: false,
        max_body_size: 4096,
        ..Default::default()
    };

    init_logging(log_config.clone())?;

    // Create middleware state
    let log_state = RequestLoggingState::new(log_config);

    // Build app with logging middleware
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .layer(axum::middleware::from_fn_with_state(
            (log_state,),
            request_logging_middleware
        ));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

## Troubleshooting

### Logs Not Appearing

1. Check log level configuration
2. Verify filter settings
3. Ensure `init_logging` is called before any logging

### Request IDs Not Propagating

1. Ensure middleware is applied to routes
2. Check middleware ordering (logging should be early in the stack)
3. Verify X-Request-ID header is being sent by clients

### Performance Issues

1. Disable body logging if enabled
2. Reduce log level to WARN or ERROR
3. Check custom redaction patterns (complex regex can be slow)

## Future Enhancements

- [ ] Log rotation implementation
- [ ] Structured log export to files
- [ ] Automatic sampling for high-volume endpoints
- [ ] Integration with log aggregation services (Datadog, Splunk, etc.)
- [ ] Request body logging with size limits
- [ ] GraphQL query logging
- [ ] Correlation with distributed tracing spans
