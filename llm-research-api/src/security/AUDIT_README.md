# Audit Logging System

A comprehensive audit logging system for tracking all significant operations in the llm-research-api crate.

## Overview

The audit logging system provides:

- **Structured audit events** with rich metadata
- **Multiple storage backends** (PostgreSQL, file, tracing)
- **Automatic request/response logging** via middleware
- **File rotation** support for long-running systems
- **Async-safe** implementation with tokio
- **Flexible composition** of multiple audit writers

## Components

### Core Types

#### AuditEvent

The main audit event structure that captures:
- `id`: Unique event identifier
- `timestamp`: When the event occurred
- `event_type`: Classification of the event
- `actor`: Who performed the action
- `resource`: What was affected
- `action`: What action was performed
- `outcome`: Success, failure, or denied
- `details`: Additional structured data
- `ip_address`: Requester's IP
- `user_agent`: Browser/client info
- `request_id`: For correlation
- `duration_ms`: Operation duration

#### AuditEventType

Classifies events into categories:
- `Authentication` - Login, logout events
- `Authorization` - Permission checks
- `DataAccess` - Read operations
- `DataModification` - Create, update, delete
- `Configuration` - Config changes
- `Security` - Security-related events
- `SystemEvent` - System-level operations

#### AuditActor

Represents who performed an action:
- `User { id, email }` - Authenticated user
- `ApiKey { id, name }` - API key authentication
- `System` - System-initiated
- `Anonymous { ip }` - Unauthenticated

#### AuditResource

Represents what was affected:
- `Experiment { id }`
- `Run { id, experiment_id }`
- `Model { id }`
- `Dataset { id }`
- `PromptTemplate { id }`
- `Evaluation { id }`
- `User { id }`
- `ApiKey { id }`
- `System`

#### AuditAction

Actions that can be audited:
- CRUD: `Create`, `Read`, `Update`, `Delete`
- Auth: `Login`, `Logout`, `LoginFailed`
- Security: `PasswordChange`, `PasswordReset`
- API Keys: `ApiKeyCreated`, `ApiKeyRevoked`
- Permissions: `PermissionGranted`, `PermissionRevoked`
- Data: `Export`, `Import`
- Config: `ConfigChange`

#### AuditOutcome

Result of the operation:
- `Success` - Operation completed successfully
- `Failure { reason }` - Operation failed
- `Denied { reason }` - Access was denied

### AuditLogger

Main interface for logging audit events:

```rust
use llm_research_api::security::*;

// Create logger with a writer
let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

// Log authentication success
logger.log_auth_success(&actor, ip_address).await?;

// Log authentication failure
logger.log_auth_failure("user@example.com", "Invalid password", ip_address).await?;

// Log resource access
logger.log_access(&actor, &resource, action, outcome).await?;

// Log data modification with before/after state
logger.log_modification(&actor, &resource, action, Some(before), Some(after)).await?;
```

### Audit Writers

#### DatabaseAuditWriter

Writes audit events to PostgreSQL:

```rust
use llm_research_api::security::*;

let writer = DatabaseAuditWriter::new(db_pool);
let logger = AuditLogger::new(Box::new(writer));
```

**Database Schema:**

Run the migration at `/workspaces/llm-research-lab/llm-research-storage/migrations/001_create_audit_log.sql`

#### FileAuditWriter

Writes audit events to JSON file with rotation:

```rust
use llm_research_api::security::*;
use std::path::PathBuf;

let writer = FileAuditWriter::new(PathBuf::from("/var/log/audit.log"))
    .with_max_size(100 * 1024 * 1024)  // 100 MB
    .with_max_files(10);  // Keep 10 rotated files

let logger = AuditLogger::new(Box::new(writer));
```

Features:
- Automatic file rotation when size limit is reached
- Configurable retention (number of files to keep)
- JSON format (one event per line)
- Async I/O with tokio

#### TracingAuditWriter

Writes audit events using the tracing crate:

```rust
use llm_research_api::security::*;

let writer = TracingAuditWriter::new();
let logger = AuditLogger::new(Box::new(writer));
```

Events are logged at:
- `WARN` level for denied/failed operations
- `INFO` level for successful operations

#### CompositeAuditWriter

Writes to multiple destinations simultaneously:

```rust
use llm_research_api::security::*;

let composite = CompositeAuditWriter::new()
    .add_writer(Box::new(DatabaseAuditWriter::new(db_pool)))
    .add_writer(Box::new(TracingAuditWriter::new()))
    .add_writer(Box::new(FileAuditWriter::new(path)));

let logger = AuditLogger::new(Box::new(composite));
```

Features:
- Writes to all configured writers
- Continues if individual writers fail
- Only returns error if ALL writers fail

### Middleware

Automatic audit logging for HTTP requests:

```rust
use axum::Router;
use llm_research_api::security::*;

let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

let app = Router::new()
    .route("/api/experiments", get(handler))
    .layer(middleware::from_fn_with_state(
        logger,
        audit_middleware
    ));
```

The middleware automatically:
- Extracts request metadata (method, path, IP, user agent)
- Determines resource and action from the request
- Measures operation duration
- Logs the outcome based on response status
- Adds request ID for correlation

## Usage Examples

### Basic Usage

```rust
use llm_research_api::security::*;
use std::net::IpAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a logger
    let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

    // Log a successful login
    let actor = AuditActor::User {
        id: UserId::new(),
        email: "alice@example.com".to_string(),
    };
    let ip: IpAddr = "192.168.1.100".parse()?;
    logger.log_auth_success(&actor, ip).await?;

    Ok(())
}
```

### Logging Data Modifications

```rust
use llm_research_api::security::*;
use serde_json::json;

async fn update_experiment(logger: &AuditLogger) -> AuditResult<()> {
    let actor = AuditActor::System;
    let resource = AuditResource::Experiment { id: experiment_id };

    let before = json!({
        "name": "Old Name",
        "status": "draft"
    });

    let after = json!({
        "name": "New Name",
        "status": "running"
    });

    logger.log_modification(
        &actor,
        &resource,
        AuditAction::Update,
        Some(before),
        Some(after),
    ).await
}
```

### Custom Audit Events

```rust
use llm_research_api::security::*;
use serde_json::json;

async fn log_custom_event(logger: &AuditLogger) -> AuditResult<()> {
    let event = AuditEvent::new(
        AuditEventType::Security,
        AuditActor::System,
        AuditResource::System,
        AuditAction::ConfigChange,
        AuditOutcome::Success,
    )
    .with_details(json!({
        "setting": "max_connections",
        "old_value": 100,
        "new_value": 200
    }))
    .with_duration(50);

    logger.log(event).await
}
```

### Production Setup

```rust
use llm_research_api::security::*;
use sqlx::PgPool;
use std::path::PathBuf;

async fn setup_audit_logging(db_pool: PgPool) -> AuditLogger {
    // Create composite writer for production
    let composite = CompositeAuditWriter::new()
        // Write to database for permanent storage
        .add_writer(Box::new(DatabaseAuditWriter::new(db_pool)))
        // Write to file for backup
        .add_writer(Box::new(
            FileAuditWriter::new(PathBuf::from("/var/log/llm-research/audit.log"))
                .with_max_size(100 * 1024 * 1024)
                .with_max_files(30)
        ))
        // Write to tracing for real-time monitoring
        .add_writer(Box::new(TracingAuditWriter::new()));

    AuditLogger::new(Box::new(composite))
}
```

## Database Queries

### Recent Failed Logins

```sql
SELECT
    timestamp,
    actor->>'ip' as ip_address,
    details->>'email' as email,
    outcome->>'reason' as reason
FROM audit_log
WHERE
    event_type->>'type' = 'authentication'
    AND action = '"login_failed"'
    AND timestamp > NOW() - INTERVAL '1 hour'
ORDER BY timestamp DESC;
```

### Access to Specific Resource

```sql
SELECT
    timestamp,
    actor,
    action,
    outcome,
    duration_ms
FROM audit_log
WHERE
    resource->>'type' = 'experiment'
    AND resource->>'id' = '123e4567-e89b-12d3-a456-426614174000'
ORDER BY timestamp DESC
LIMIT 100;
```

### Failed Operations by User

```sql
SELECT
    actor->>'email' as user_email,
    COUNT(*) as failed_count,
    array_agg(DISTINCT action) as attempted_actions
FROM audit_log
WHERE
    outcome->>'status' = 'failure'
    AND actor->>'type' = 'user'
    AND timestamp > NOW() - INTERVAL '24 hours'
GROUP BY actor->>'email'
ORDER BY failed_count DESC;
```

### Audit Events by IP Address

```sql
SELECT
    ip_address,
    COUNT(*) as total_events,
    COUNT(*) FILTER (WHERE outcome->>'status' = 'success') as successful,
    COUNT(*) FILTER (WHERE outcome->>'status' = 'failure') as failed,
    COUNT(*) FILTER (WHERE outcome->>'status' = 'denied') as denied
FROM audit_log
WHERE timestamp > NOW() - INTERVAL '1 day'
GROUP BY ip_address
HAVING COUNT(*) > 100  -- High activity IPs
ORDER BY total_events DESC;
```

## Performance Considerations

1. **Async Operations**: All writers use async I/O to avoid blocking
2. **Indexing**: Database has indexes on common query fields
3. **Batching**: Consider implementing batch writes for high-volume systems
4. **Retention**: Implement data retention policies to manage table size
5. **Partitioning**: For very high volumes, partition the audit_log table by timestamp

## Security Considerations

1. **Sensitive Data**: Be careful not to log passwords or tokens in details
2. **Access Control**: Restrict access to audit logs
3. **Immutability**: Audit logs should never be modified or deleted
4. **Encryption**: Consider encrypting audit logs at rest
5. **Compliance**: Ensure logging meets regulatory requirements (GDPR, HIPAA, etc.)

## Testing

Run the tests:

```bash
cargo test -p llm-research-api --lib security::audit
```

## Future Enhancements

Potential improvements:
- [ ] Batch writer for high-volume scenarios
- [ ] Compression for archived audit files
- [ ] Elasticsearch/OpenSearch writer for advanced queries
- [ ] Anomaly detection based on audit patterns
- [ ] Real-time alerting for security events
- [ ] Compliance report generation
- [ ] Audit log integrity verification (hash chains)
