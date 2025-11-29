# Logging Quick Reference

## Quick Start

```rust
// 1. Initialize logging
use llm_research_api::observability::logging::init_default_logging;
init_default_logging()?;

// 2. Add middleware to your app
use llm_research_api::observability::logging::{RequestLoggingState, request_logging_middleware};
let log_state = RequestLoggingState::new(LogConfig::default());
let app = Router::new()
    .layer(axum::middleware::from_fn_with_state((log_state,), request_logging_middleware));

// 3. Use logging
tracing::info!("Hello, world!");
```

## Configuration Presets

### Development
```rust
LogConfig {
    format: LogFormat::Pretty,
    level: "debug".to_string(),
    filter: Some("llm_research_api=debug,sqlx=warn".to_string()),
    ..Default::default()
}
```

### Production
```rust
LogConfig {
    format: LogFormat::Json,
    level: "info".to_string(),
    filter: Some("llm_research_api=info,sqlx=warn,tower_http=warn".to_string()),
    log_bodies: false,
    ..Default::default()
}
```

## Common Operations

### Log with Context
```rust
use llm_research_api::observability::logging::{LogContext, with_context};

let context = LogContext::new()
    .with_user_id("user-123")
    .with_experiment_id(experiment_id);

with_context(context, || async {
    tracing::info!("Processing request");
}).await;
```

### Redact Sensitive Data
```rust
use llm_research_api::observability::logging::SensitiveDataRedactor;

let redactor = SensitiveDataRedactor::new();
let safe_text = redactor.redact("password: secret123");
let safe_headers = redactor.redact_headers(&headers);
let safe_json = redactor.redact_json(&json);
```

### Custom Request ID
```rust
// Client sends:
// X-Request-ID: custom-request-id-123

// Server automatically:
// - Extracts and uses the ID
// - Includes in all logs
// - Returns in response header
```

## Log Levels

| Level | When to Use |
|-------|------------|
| ERROR | Server errors (5xx), critical failures |
| WARN  | Client errors (4xx), deprecated features |
| INFO  | Normal operations, requests |
| DEBUG | Detailed debugging information |
| TRACE | Very detailed trace information |

## Sensitive Data Patterns

### Auto-Redacted Headers
- `authorization`
- `cookie` / `set-cookie`
- `x-api-key`
- `x-auth-token`

### Auto-Redacted Patterns
- `password: xxx`
- `token: xxx`
- `api_key: xxx`
- `secret: xxx`
- `Bearer xxx`

### Auto-Redacted JSON Fields
- Fields containing "password"
- Fields containing "token"
- Fields containing "secret"
- Fields containing "key"

## Testing

```bash
# Run all logging tests
cargo test --package llm-research-api observability::logging

# Run specific test
cargo test --package llm-research-api test_sensitive_data_redaction

# Run with output
cargo test --package llm-research-api observability::logging -- --nocapture
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| No logs appearing | Check `init_logging()` called, verify log level |
| Request IDs missing | Ensure middleware is applied before routes |
| Sensitive data exposed | Verify redactor config, add custom patterns |
| Performance slow | Disable body logging, increase log level to WARN |

## Environment Variables

```bash
# Override log level
RUST_LOG=debug cargo run

# Per-module levels
RUST_LOG=llm_research_api=debug,sqlx=warn cargo run

# Maximum log level (for performance)
RUST_LOG=info cargo run
```

## Integration Examples

### With API Key Middleware
```rust
use llm_research_api::observability::logging::{RequestLoggingState, request_logging_middleware};
use llm_research_api::security::api_key_auth_middleware;

let app = Router::new()
    .route("/api/protected", get(handler))
    .layer(axum::middleware::from_fn_with_state((api_key_state,), api_key_auth_middleware))
    .layer(axum::middleware::from_fn_with_state((log_state,), request_logging_middleware));
```

### With Audit Middleware
```rust
use llm_research_api::observability::logging::{RequestLoggingState, request_logging_middleware};
use llm_research_api::security::audit_middleware;

let app = Router::new()
    .route("/api/admin", post(handler))
    .layer(axum::middleware::from_fn_with_state((audit_state,), audit_middleware))
    .layer(axum::middleware::from_fn_with_state((log_state,), request_logging_middleware));
```

## Performance Tips

1. Use JSON format in production (faster parsing)
2. Set appropriate log level (INFO or WARN in prod)
3. Disable body logging unless debugging
4. Use filter to reduce noise from dependencies
5. Consider async log writers for high-throughput

## Example Log Output

### Pretty Format (Development)
```
2025-11-28T12:34:56.789Z  INFO request{method=GET uri=/api/experiments/123 id=550e8400-e29b-41d4-a716-446655440000}: Incoming request
2025-11-28T12:34:56.891Z  INFO request{method=GET uri=/api/experiments/123 id=550e8400-e29b-41d4-a716-446655440000}: Request completed successfully response.status=200 response.duration_ms=102
```

### JSON Format (Production)
```json
{"timestamp":"2025-11-28T12:34:56.789Z","level":"INFO","message":"Incoming request","request":{"method":"GET","uri":"/api/experiments/123","id":"550e8400-e29b-41d4-a716-446655440000"}}
{"timestamp":"2025-11-28T12:34:56.891Z","level":"INFO","message":"Request completed successfully","request":{"id":"550e8400-e29b-41d4-a716-446655440000"},"response":{"status":200,"duration_ms":102}}
```
