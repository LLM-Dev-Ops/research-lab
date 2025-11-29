# Audit Logging Integration Guide

This guide shows how to integrate the audit logging system into your LLM Research Lab API application.

## Table of Contents

1. [Database Setup](#database-setup)
2. [Basic Integration](#basic-integration)
3. [Production Setup](#production-setup)
4. [Middleware Integration](#middleware-integration)
5. [Custom Event Logging](#custom-event-logging)
6. [Querying Audit Logs](#querying-audit-logs)
7. [Best Practices](#best-practices)

## Database Setup

### 1. Run the Migration

Apply the audit log migration to your PostgreSQL database:

```bash
psql -U postgres -d llm_research < llm-research-storage/migrations/001_create_audit_log.sql
```

Or using sqlx:

```bash
sqlx migrate run --database-url postgresql://user:pass@localhost/llm_research
```

### 2. Verify the Table

```sql
\d audit_log
```

You should see the `audit_log` table with all indexes.

## Basic Integration

### 1. Initialize Audit Logger in main.rs

```rust
use llm_research_api::security::*;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Connect to database
    let db_pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;

    // Create audit logger with database writer
    let audit_logger = AuditLogger::new(
        Box::new(DatabaseAuditWriter::new(db_pool.clone()))
    );

    // Use the logger throughout your application
    // ...

    Ok(())
}
```

### 2. Add Audit Logger to Application State

```rust
use axum::extract::State;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub s3_client: S3Client,
    pub s3_bucket: String,
    pub audit_logger: AuditLogger,  // Add this
}

impl AppState {
    pub fn new(
        db_pool: PgPool,
        s3_client: S3Client,
        s3_bucket: String,
        audit_logger: AuditLogger,
    ) -> Self {
        Self {
            db_pool,
            s3_client,
            s3_bucket,
            audit_logger,
        }
    }
}
```

## Production Setup

### Composite Writer Configuration

For production, use multiple writers for redundancy:

```rust
use std::path::PathBuf;

async fn setup_production_audit_logger(db_pool: PgPool) -> AuditLogger {
    let composite = CompositeAuditWriter::new()
        // Primary: Database for queryable storage
        .add_writer(Box::new(DatabaseAuditWriter::new(db_pool)))

        // Secondary: File for backup and compliance
        .add_writer(Box::new(
            FileAuditWriter::new(PathBuf::from("/var/log/llm-research/audit.log"))
                .with_max_size(100 * 1024 * 1024)  // 100 MB
                .with_max_files(365)  // Keep 1 year of daily logs
        ))

        // Tertiary: Tracing for real-time monitoring
        .add_writer(Box::new(TracingAuditWriter::new()));

    AuditLogger::new(Box::new(composite))
}
```

### Environment-Based Configuration

```rust
fn setup_audit_logger(db_pool: PgPool, env: &str) -> AuditLogger {
    match env {
        "production" => {
            let composite = CompositeAuditWriter::new()
                .add_writer(Box::new(DatabaseAuditWriter::new(db_pool)))
                .add_writer(Box::new(
                    FileAuditWriter::new(PathBuf::from("/var/log/audit.log"))
                        .with_max_size(100 * 1024 * 1024)
                        .with_max_files(365)
                ));
            AuditLogger::new(Box::new(composite))
        }
        "development" => {
            AuditLogger::new(Box::new(TracingAuditWriter::new()))
        }
        _ => {
            AuditLogger::new(Box::new(TracingAuditWriter::new()))
        }
    }
}
```

## Middleware Integration

### 1. Add Audit Middleware to Router

```rust
use axum::{Router, middleware};
use llm_research_api::security::audit_middleware;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/experiments", post(handlers::experiments::create))
        .route("/experiments/:id", get(handlers::experiments::get))
        // ... other routes

        // Add audit middleware
        .layer(middleware::from_fn_with_state(
            state.audit_logger.clone(),
            audit_middleware
        ))

        // Other middleware layers
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

### 2. Custom Middleware with Authentication Context

```rust
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn audit_with_auth_middleware(
    State(logger): State<AuditLogger>,
    Extension(claims): Extension<Claims>,  // From auth middleware
    request: Request,
    next: Next,
) -> Response {
    let actor = AuditActor::User {
        id: claims.user_id,
        email: claims.email.clone(),
    };

    // Extract IP and other metadata
    // Log the event
    // ...

    next.run(request).await
}
```

## Custom Event Logging

### 1. In Handler Functions

```rust
use axum::{Json, extract::{State, Path}};
use llm_research_api::security::*;

pub async fn create_experiment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    // Create the experiment
    let experiment = create_experiment_logic(&state.db_pool, request).await?;

    // Log the audit event
    let actor = AuditActor::User {
        id: claims.user_id,
        email: claims.email.clone(),
    };

    state.audit_logger.log_access(
        &actor,
        &AuditResource::Experiment { id: experiment.id },
        AuditAction::Create,
        AuditOutcome::Success,
    ).await.ok();  // Don't fail request if audit logging fails

    Ok(Json(experiment))
}
```

### 2. Logging Data Modifications

```rust
pub async fn update_experiment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(updates): Json<UpdateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    // Get current state
    let before = get_experiment(&state.db_pool, id).await?;
    let before_json = serde_json::to_value(&before)?;

    // Apply updates
    let after = apply_updates(&state.db_pool, id, updates).await?;
    let after_json = serde_json::to_value(&after)?;

    // Log the modification
    let actor = AuditActor::User {
        id: claims.user_id,
        email: claims.email.clone(),
    };

    state.audit_logger.log_modification(
        &actor,
        &AuditResource::Experiment { id },
        AuditAction::Update,
        Some(before_json),
        Some(after_json),
    ).await.ok();

    Ok(Json(after))
}
```

### 3. Logging Authentication Events

```rust
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(credentials): Json<LoginRequest>,
) -> Result<Json<TokenPair>, ApiError> {
    let ip = addr.ip();

    match authenticate(&state.db_pool, &credentials).await {
        Ok(user) => {
            let actor = AuditActor::User {
                id: user.id,
                email: user.email.clone(),
            };

            state.audit_logger
                .log_auth_success(&actor, ip)
                .await
                .ok();

            // Generate tokens
            Ok(Json(generate_tokens(&user)?))
        }
        Err(e) => {
            state.audit_logger
                .log_auth_failure(&credentials.email, &e.to_string(), ip)
                .await
                .ok();

            Err(ApiError::Unauthorized)
        }
    }
}
```

### 4. Logging Permission Denials

```rust
pub async fn delete_experiment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let actor = AuditActor::User {
        id: claims.user_id,
        email: claims.email.clone(),
    };

    // Check permissions
    if !has_delete_permission(&state.db_pool, claims.user_id, id).await? {
        // Log denial
        state.audit_logger.log_access(
            &actor,
            &AuditResource::Experiment { id },
            AuditAction::Delete,
            AuditOutcome::Denied {
                reason: "User lacks delete permission".to_string(),
            },
        ).await.ok();

        return Err(ApiError::Forbidden);
    }

    // Perform deletion
    delete_experiment_logic(&state.db_pool, id).await?;

    // Log success
    state.audit_logger.log_access(
        &actor,
        &AuditResource::Experiment { id },
        AuditAction::Delete,
        AuditOutcome::Success,
    ).await.ok();

    Ok(StatusCode::NO_CONTENT)
}
```

## Querying Audit Logs

### 1. Create Query Service

```rust
use llm_research_api::security::*;

pub async fn setup_audit_query(db_pool: PgPool) -> AuditLogQuery {
    AuditLogQuery::new(db_pool)
}
```

### 2. Query Recent Failed Logins

```rust
pub async fn get_security_alerts(
    State(query): State<AuditLogQuery>,
) -> Result<Json<Vec<AuditEvent>>, ApiError> {
    let since = Utc::now() - chrono::Duration::hours(24);
    let failed_logins = query.get_failed_logins(since, 100).await?;

    Ok(Json(failed_logins))
}
```

### 3. Query User Activity

```rust
pub async fn get_user_audit_log(
    State(query): State<AuditLogQuery>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<AuditEvent>>, ApiError> {
    let events = query.get_user_activity(user_id, 100).await?;

    Ok(Json(events))
}
```

### 4. Get Audit Statistics

```rust
pub async fn get_audit_statistics(
    State(query): State<AuditLogQuery>,
) -> Result<Json<AuditStatistics>, ApiError> {
    let since = Utc::now() - chrono::Duration::days(7);
    let stats = query.get_statistics(since).await?;

    Ok(Json(stats))
}
```

## Best Practices

### 1. Don't Fail Requests on Audit Errors

```rust
// Good - log errors but continue
state.audit_logger.log(event).await.ok();

// Bad - could break the application
state.audit_logger.log(event).await?;
```

### 2. Log Sensitive Operations

Always audit:
- Authentication attempts (success and failure)
- Authorization denials
- Data modifications (with before/after state)
- Configuration changes
- Security events (password changes, API key creation)
- Data exports

### 3. Include Relevant Context

```rust
let event = AuditEvent::new(...)
    .with_ip(ip_address)
    .with_user_agent(user_agent)
    .with_request_id(request_id)
    .with_duration(operation_duration_ms)
    .with_details(json!({
        "additional": "context"
    }));
```

### 4. Don't Log Sensitive Data

```rust
// Bad - logs password
.with_details(json!({
    "password": credentials.password  // Never do this!
}))

// Good - logs metadata only
.with_details(json!({
    "email": credentials.email,
    "login_method": "password"
}))
```

### 5. Regular Maintenance

```sql
-- Archive old audit logs (quarterly)
CREATE TABLE audit_log_archive_2024_q1 AS
SELECT * FROM audit_log
WHERE timestamp >= '2024-01-01' AND timestamp < '2024-04-01';

-- Delete archived data
DELETE FROM audit_log
WHERE timestamp >= '2024-01-01' AND timestamp < '2024-04-01';

-- Or use partitioning for automatic management
```

### 6. Monitor Audit System Health

```rust
// Periodic health check
async fn check_audit_health(logger: &AuditLogger) -> Result<(), String> {
    let test_event = AuditEvent::new(
        AuditEventType::SystemEvent,
        AuditActor::System,
        AuditResource::System,
        AuditAction::Read,
        AuditOutcome::Success,
    );

    logger.log(test_event).await
        .map_err(|e| format!("Audit system unhealthy: {}", e))
}
```

### 7. Alerting on Suspicious Activity

```rust
async fn check_for_suspicious_activity(query: &AuditLogQuery) -> Vec<Alert> {
    let mut alerts = Vec::new();
    let since = Utc::now() - chrono::Duration::minutes(15);

    // Check for repeated failed logins
    if let Ok(failed) = query.get_failed_logins(since, 10).await {
        if failed.len() >= 5 {
            alerts.push(Alert::RepeatedFailedLogins(failed));
        }
    }

    // Check for denied access attempts
    if let Ok(denied) = query.get_denied_access(since, 10).await {
        if denied.len() >= 3 {
            alerts.push(Alert::RepeatedAccessDenials(denied));
        }
    }

    alerts
}
```

## Testing

### Integration Test Example

```rust
#[tokio::test]
async fn test_audit_logging() {
    let db_pool = setup_test_db().await;
    let logger = AuditLogger::new(Box::new(DatabaseAuditWriter::new(db_pool.clone())));

    let event = AuditEvent::new(
        AuditEventType::DataAccess,
        AuditActor::System,
        AuditResource::System,
        AuditAction::Read,
        AuditOutcome::Success,
    );

    logger.log(event.clone()).await.unwrap();

    // Query and verify
    let query = AuditLogQuery::new(db_pool);
    let events = query.query(&AuditLogFilter {
        event_type: Some("data_access".to_string()),
        limit: Some(1),
        ..Default::default()
    }).await.unwrap();

    assert_eq!(events.len(), 1);
}
```

## Troubleshooting

### Audit Logs Not Appearing

1. Check database connection
2. Verify migration was applied
3. Check file permissions (for FileAuditWriter)
4. Review application logs for errors

### Performance Issues

1. Add database indexes on frequently queried columns
2. Implement log archiving/partitioning
3. Use async batch writes for high-volume scenarios
4. Consider a dedicated audit database

### File Rotation Not Working

1. Check disk space
2. Verify write permissions
3. Review file path configuration
4. Check rotation size/count settings

## Additional Resources

- [Audit System README](src/security/AUDIT_README.md)
- [Example Application](examples/audit_logging_example.rs)
- [Database Schema](llm-research-storage/migrations/001_create_audit_log.sql)
