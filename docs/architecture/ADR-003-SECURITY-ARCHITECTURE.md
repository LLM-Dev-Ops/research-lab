# ADR-003: Security Architecture

## Status
Accepted

## Date
2025-01-15

## Context

The LLM Research Lab platform handles sensitive data including:
- Proprietary experiment configurations and results
- Dataset contents (potentially including PII)
- API credentials and secrets
- User authentication data

Security requirements:
- **Confidentiality**: Protect data from unauthorized access
- **Integrity**: Prevent unauthorized modification
- **Availability**: Ensure authorized access is not blocked
- **Compliance**: Meet SOC2, GDPR requirements
- **Auditability**: Complete audit trail of all actions

## Decision

We implement a **defense-in-depth** security architecture with multiple layers:

### 1. Authentication Layer

**Primary: JWT (JSON Web Tokens)**
- Stateless authentication for API requests
- Short-lived access tokens (1 hour)
- Long-lived refresh tokens (7 days)
- RS256 signing algorithm

**Secondary: API Keys**
- For service-to-service communication
- For third-party integrations
- Scoped permissions
- Rate limit tiers

```rust
// JWT Configuration
pub struct JwtConfig {
    pub access_token_expiry: Duration,    // 1 hour
    pub refresh_token_expiry: Duration,   // 7 days
    pub issuer: String,                   // "llm-research-lab"
    pub audience: Vec<String>,            // ["api.llm-research-lab.io"]
}

// API Key Structure
pub struct ApiKey {
    pub id: Uuid,
    pub key_prefix: String,              // First 8 chars for identification
    pub key_hash: String,                // bcrypt hash of full key
    pub scopes: Vec<ApiScope>,           // Permitted operations
    pub rate_limit_tier: RateLimitTier,  // Rate limiting tier
    pub expires_at: Option<DateTime<Utc>>,
}
```

### 2. Authorization Layer

**Role-Based Access Control (RBAC)**

| Role | Permissions |
|------|-------------|
| Viewer | Read experiments, models, datasets |
| Researcher | Viewer + Create/Edit own resources |
| Lead | Researcher + Manage team resources |
| Admin | Full access + User management |
| System | Internal service operations |

**Resource Ownership Model**
```rust
pub struct ResourceOwnership {
    pub owner_id: UserId,
    pub collaborators: Vec<UserId>,
    pub team_id: Option<TeamId>,
    pub visibility: Visibility,  // Private, Team, Public
}
```

### 3. Rate Limiting Layer

**Tiered Rate Limiting**

| Tier | Requests/Hour | Burst | Use Case |
|------|---------------|-------|----------|
| Free | 100 | 10 | Trial users |
| Standard | 1,000 | 50 | Individual researchers |
| Professional | 10,000 | 200 | Teams |
| Enterprise | Custom | Custom | Large organizations |

**Implementation:**
- Sliding window algorithm
- Per-user and per-IP limiting
- Endpoint-specific limits for expensive operations
- Graceful degradation under load

### 4. Input Validation Layer

**Request Validation**
- Schema validation with `validator` crate
- SQL injection prevention via parameterized queries
- XSS prevention via output encoding
- Path traversal prevention
- Content-type validation

**Validation Rules:**
```rust
#[derive(Validate)]
pub struct CreateExperimentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(length(max = 10000))]
    pub description: Option<String>,

    #[validate(custom = "validate_identifier")]
    pub tags: Option<Vec<String>>,

    #[validate]
    pub config: ExperimentConfig,
}
```

### 5. Audit Logging Layer

**Audit Events Captured:**
- Authentication events (login, logout, token refresh)
- Authorization decisions (allowed, denied)
- Resource access (create, read, update, delete)
- Configuration changes
- Security events (rate limit hits, invalid tokens)

**Audit Log Schema:**
```rust
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub actor: AuditActor,          // User, System, Anonymous
    pub action: AuditAction,        // Create, Read, Update, Delete
    pub resource: AuditResource,    // Resource type and ID
    pub outcome: AuditOutcome,      // Success, Failure, Partial
    pub metadata: serde_json::Value,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
}
```

### 6. Transport Security

**TLS Configuration:**
- TLS 1.3 only (1.2 for legacy clients)
- Strong cipher suites only
- HSTS enabled (1 year, includeSubDomains)
- Certificate transparency logging

**Security Headers:**
```rust
pub fn security_headers() -> impl Layer<ServiceRequest> {
    SetResponseHeader::overriding(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    )
    .layer(SetResponseHeader::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeader::overriding(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeader::overriding(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    ))
}
```

### 7. Secrets Management

**Secrets Storage:**
- AWS Secrets Manager for production secrets
- Kubernetes Secrets for runtime configuration
- Never in source code or environment variables
- Automatic rotation where supported

**Secret Types:**
| Secret | Rotation Period | Storage |
|--------|-----------------|---------|
| JWT Signing Key | 90 days | AWS Secrets Manager |
| Database Password | 30 days | AWS Secrets Manager |
| API Key Salt | Never (immutable) | AWS Secrets Manager |
| S3 Access Keys | IAM Role (no keys) | IAM |

## Alternatives Considered

### OAuth 2.0 / OpenID Connect
- **Pro**: Industry standard, SSO support
- **Con**: Complexity for API-first platform
- **Decision**: Deferred; will add for SSO integration

### Session-Based Authentication
- **Pro**: Simpler revocation
- **Con**: Requires session storage, not stateless
- **Decision**: Rejected for API; acceptable for web UI

### Mutual TLS (mTLS)
- **Pro**: Strong client authentication
- **Con**: Certificate management complexity
- **Decision**: Implement for internal service mesh only

## Consequences

### Positive
- **Defense in Depth**: Multiple security layers
- **Auditability**: Complete action history
- **Scalability**: Stateless auth scales horizontally
- **Flexibility**: Multiple auth methods for different use cases

### Negative
- **Complexity**: Multiple systems to maintain
- **Token Revocation**: JWT revocation requires blocklist
- **Key Management**: Rotation procedures needed

### Mitigations

**Token Revocation:**
```rust
// Redis-backed token blocklist
pub struct TokenBlocklist {
    redis: RedisClient,
    ttl: Duration,  // Match token expiry
}

impl TokenBlocklist {
    pub async fn is_blocked(&self, jti: &str) -> bool {
        self.redis.exists(format!("blocked:{}", jti)).await
    }

    pub async fn block(&self, jti: &str, exp: DateTime<Utc>) {
        let ttl = exp - Utc::now();
        self.redis.setex(format!("blocked:{}", jti), ttl, "1").await;
    }
}
```

**Key Rotation:**
- Automated rotation scripts
- Grace period for old keys
- Monitoring for rotation failures

## Security Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Internet                                   │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │   WAF / DDoS Protection │
                    │   (AWS Shield/WAF)      │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │   Load Balancer (ALB)   │
                    │   TLS Termination       │
                    └────────────┬────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
┌───────▼───────┐      ┌────────▼────────┐      ┌────────▼────────┐
│ Rate Limiter  │      │ Rate Limiter    │      │ Rate Limiter    │
│ (per-user)    │      │ (per-user)      │      │ (per-user)      │
└───────┬───────┘      └────────┬────────┘      └────────┬────────┘
        │                       │                        │
┌───────▼───────┐      ┌────────▼────────┐      ┌────────▼────────┐
│ Auth Layer    │      │ Auth Layer      │      │ Auth Layer      │
│ (JWT/API Key) │      │ (JWT/API Key)   │      │ (JWT/API Key)   │
└───────┬───────┘      └────────┬────────┘      └────────┬────────┘
        │                       │                        │
┌───────▼───────┐      ┌────────▼────────┐      ┌────────▼────────┐
│ RBAC Layer    │      │ RBAC Layer      │      │ RBAC Layer      │
│ (Permissions) │      │ (Permissions)   │      │ (Permissions)   │
└───────┬───────┘      └────────┬────────┘      └────────┬────────┘
        │                       │                        │
        └───────────────────────┼────────────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │    Application Logic   │
                    │    (Input Validation)  │
                    └───────────┬───────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
┌───────▼───────┐      ┌───────▼───────┐      ┌────────▼────────┐
│  PostgreSQL   │      │  ClickHouse   │      │    Amazon S3    │
│  (Encrypted)  │      │  (Encrypted)  │      │   (Encrypted)   │
└───────────────┘      └───────────────┘      └─────────────────┘
```

## Implementation Checklist

- [x] JWT authentication
- [x] API key authentication
- [x] Role-based access control
- [x] Rate limiting
- [x] Input validation
- [x] Audit logging
- [x] Security headers
- [ ] WAF rules (AWS WAF)
- [ ] Secrets rotation automation
- [ ] Penetration testing
- [ ] Security audit

## References
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [JWT Best Practices (RFC 8725)](https://datatracker.ietf.org/doc/html/rfc8725)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
