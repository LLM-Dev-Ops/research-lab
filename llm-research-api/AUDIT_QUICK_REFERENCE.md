# Audit Logging Quick Reference

## File Locations

```
/workspaces/llm-research-lab/llm-research-api/src/security/
├── audit.rs                    # Core audit types and logger
├── audit_middleware.rs         # Axum middleware for automatic logging
├── audit_query.rs              # Database query utilities
├── AUDIT_README.md            # Comprehensive documentation
└── ...

/workspaces/llm-research-lab/llm-research-api/
├── examples/audit_logging_example.rs      # Usage examples
├── AUDIT_INTEGRATION_GUIDE.md             # Integration guide
└── AUDIT_QUICK_REFERENCE.md               # This file

/workspaces/llm-research-lab/llm-research-storage/
└── migrations/001_create_audit_log.sql    # Database schema
```

## Core Components

### AuditEvent
```rust
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub actor: AuditActor,
    pub resource: AuditResource,
    pub action: AuditAction,
    pub outcome: AuditOutcome,
    pub details: Value,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub duration_ms: Option<u64>,
}
```

### AuditEventType
```rust
Authentication | Authorization | DataAccess | DataModification |
Configuration | Security | SystemEvent
```

### AuditActor
```rust
User { id, email } | ApiKey { id, name } | System | Anonymous { ip }
```

### AuditResource
```rust
Experiment { id } | Run { id, experiment_id } | Model { id } |
Dataset { id } | PromptTemplate { id } | Evaluation { id } |
User { id } | ApiKey { id } | System
```

### AuditAction
```rust
Create | Read | Update | Delete | Login | Logout | LoginFailed |
PasswordChange | PasswordReset | ApiKeyCreated | ApiKeyRevoked |
PermissionGranted | PermissionRevoked | Export | Import | ConfigChange
```

### AuditOutcome
```rust
Success | Failure { reason } | Denied { reason }
```

## Quick Start

### 1. Setup (main.rs)
```rust
use llm_research_api::security::*;

let logger = AuditLogger::new(Box::new(DatabaseAuditWriter::new(db_pool)));
```

### 2. Log Authentication
```rust
// Success
logger.log_auth_success(&actor, ip_address).await?;

// Failure
logger.log_auth_failure("email@example.com", "Invalid password", ip).await?;
```

### 3. Log Access
```rust
logger.log_access(&actor, &resource, action, outcome).await?;
```

### 4. Log Modification
```rust
logger.log_modification(&actor, &resource, action, Some(before), Some(after)).await?;
```

### 5. Custom Event
```rust
let event = AuditEvent::new(event_type, actor, resource, action, outcome)
    .with_ip(ip)
    .with_user_agent(ua)
    .with_details(details);
logger.log(event).await?;
```

## Writers

### Database
```rust
DatabaseAuditWriter::new(db_pool)
```

### File (with rotation)
```rust
FileAuditWriter::new(path)
    .with_max_size(100 * 1024 * 1024)  // 100 MB
    .with_max_files(10)
```

### Tracing
```rust
TracingAuditWriter::new()
```

### Composite
```rust
CompositeAuditWriter::new()
    .add_writer(Box::new(DatabaseAuditWriter::new(pool)))
    .add_writer(Box::new(FileAuditWriter::new(path)))
    .add_writer(Box::new(TracingAuditWriter::new()))
```

## Middleware

```rust
Router::new()
    .route("/api/path", handler)
    .layer(middleware::from_fn_with_state(
        logger,
        audit_middleware
    ))
```

## Queries

### Setup
```rust
let query = AuditLogQuery::new(db_pool);
```

### Common Queries
```rust
// Failed logins (last 24h)
query.get_failed_logins(Utc::now() - Duration::hours(24), 100).await?;

// User activity
query.get_user_activity(user_id, 100).await?;

// Resource history
query.get_resource_history("experiment", experiment_id, 100).await?;

// Denied access
query.get_denied_access(Utc::now() - Duration::hours(24), 100).await?;

// Events by IP
query.get_events_by_ip("192.168.1.1", 100).await?;

// Statistics
query.get_statistics(Utc::now() - Duration::days(7)).await?;
```

### Custom Query
```rust
query.query(&AuditLogFilter {
    event_type: Some("authentication".to_string()),
    after: Some(Utc::now() - Duration::hours(1)),
    limit: Some(50),
    ..Default::default()
}).await?;
```

## SQL Examples

### Recent Failed Logins
```sql
SELECT timestamp, actor, details->>'email' as email, outcome
FROM audit_log
WHERE event_type->>'type' = 'authentication'
  AND action = '"login_failed"'
  AND timestamp > NOW() - INTERVAL '1 hour'
ORDER BY timestamp DESC;
```

### Resource Access History
```sql
SELECT timestamp, actor, action, outcome, duration_ms
FROM audit_log
WHERE resource->>'id' = '123e4567-e89b-12d3-a456-426614174000'
ORDER BY timestamp DESC
LIMIT 50;
```

### Suspicious Activity (same IP, multiple failures)
```sql
SELECT ip_address, COUNT(*) as attempts
FROM audit_log
WHERE outcome->>'status' = 'failure'
  AND timestamp > NOW() - INTERVAL '15 minutes'
GROUP BY ip_address
HAVING COUNT(*) >= 5
ORDER BY attempts DESC;
```

## Handler Integration Patterns

### Create Operation
```rust
async fn create_handler(State(state): State<AppState>) -> Result<Json<Response>, ApiError> {
    let result = create_entity().await?;

    state.audit_logger.log_access(
        &actor,
        &AuditResource::Entity { id: result.id },
        AuditAction::Create,
        AuditOutcome::Success,
    ).await.ok();  // Don't fail request if audit fails

    Ok(Json(result))
}
```

### Update with Before/After
```rust
async fn update_handler(State(state): State<AppState>) -> Result<Json<Response>, ApiError> {
    let before = get_entity(id).await?;
    let after = update_entity(id, changes).await?;

    state.audit_logger.log_modification(
        &actor,
        &resource,
        AuditAction::Update,
        Some(serde_json::to_value(&before)?),
        Some(serde_json::to_value(&after)?),
    ).await.ok();

    Ok(Json(after))
}
```

### Permission Denial
```rust
async fn delete_handler(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    if !has_permission(user_id, resource_id).await? {
        state.audit_logger.log_access(
            &actor,
            &resource,
            AuditAction::Delete,
            AuditOutcome::Denied {
                reason: "Insufficient permissions".to_string(),
            },
        ).await.ok();

        return Err(ApiError::Forbidden);
    }

    delete_entity(id).await?;

    state.audit_logger.log_access(
        &actor,
        &resource,
        AuditAction::Delete,
        AuditOutcome::Success,
    ).await.ok();

    Ok(StatusCode::NO_CONTENT)
}
```

## Best Practices Checklist

- [ ] Always log authentication attempts (success and failure)
- [ ] Log all authorization denials
- [ ] Log data modifications with before/after state
- [ ] Include IP address when available
- [ ] Add request ID for correlation
- [ ] Don't fail requests if audit logging fails (use `.ok()`)
- [ ] Never log passwords or secrets
- [ ] Use composite writers in production
- [ ] Set up file rotation for file-based logging
- [ ] Implement regular log archiving
- [ ] Monitor for suspicious patterns
- [ ] Test audit logging in integration tests

## Testing

### Run Example
```bash
cargo run --example audit_logging_example
```

### Run Tests
```bash
cargo test -p llm-research-api --lib security::audit
```

## Common Troubleshooting

| Issue | Solution |
|-------|----------|
| Events not in database | Check migration was applied |
| File not created | Verify directory exists and permissions |
| Performance slow | Add database indexes, implement archiving |
| Disk full | Set up rotation, implement cleanup policies |

## Environment Variables

```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost/llm_research

# Audit file location (optional)
AUDIT_LOG_PATH=/var/log/llm-research/audit.log

# Audit log max size (bytes)
AUDIT_MAX_SIZE=104857600  # 100 MB

# Audit log retention (number of files)
AUDIT_MAX_FILES=365
```

## Related Documentation

- **Comprehensive Guide**: [AUDIT_README.md](src/security/AUDIT_README.md)
- **Integration Guide**: [AUDIT_INTEGRATION_GUIDE.md](AUDIT_INTEGRATION_GUIDE.md)
- **Example Code**: [examples/audit_logging_example.rs](examples/audit_logging_example.rs)
- **Database Schema**: [migrations/001_create_audit_log.sql](../llm-research-storage/migrations/001_create_audit_log.sql)
