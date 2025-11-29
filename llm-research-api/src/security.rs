//! Security module for llm-research-api
//!
//! Provides comprehensive security features including:
//! - JWT authentication and refresh tokens
//! - Role-based access control (RBAC)
//! - API key authentication for service accounts
//! - Rate limiting per endpoint and user
//! - Audit logging for compliance
//! - Request validation
//! - Security headers (CORS, CSP, HSTS)

pub mod auth;
pub mod rbac;
pub mod api_key;
pub mod rate_limit;
pub mod audit;
pub mod audit_middleware;
pub mod audit_query;
pub mod validation;
pub mod headers;

pub use auth::{
    AuthError, AuthResult, Claims, JwtConfig, JwtService, RefreshClaims, TokenPair, TokenType,
};
pub use rbac::{
    Role, Permission, RolePermissions, ResourceOwnership, PermissionGuard, helpers,
};
pub use rate_limit::{
    RateLimitConfig, RateLimitError, RateLimitInfo, RateLimitKey, RateLimitLayer,
    RateLimiter, rate_limit_middleware, UserId,
};
pub use audit::{
    AuditAction, AuditActor, AuditError, AuditEvent, AuditEventType, AuditLogger,
    AuditOutcome, AuditResource, AuditResult, AuditWriter, CompositeAuditWriter,
    DatabaseAuditWriter, FileAuditWriter, TracingAuditWriter, AuditMiddlewareState,
};
pub use audit_middleware::{audit_middleware, AuditMiddlewareError};
pub use audit_query::{AuditLogFilter, AuditLogQuery, AuditStatistics};
pub use api_key::{
    ApiKey, ApiKeyService, ApiKeyUser, ApiScope,
    ExperimentPermission, ModelPermission, DatasetPermission, MetricPermission,
    RateLimitTier,
    get_api_key_user, require_role, require_any_role, require_scope_permission,
    api_key_auth_middleware, optional_api_key_auth_middleware,
};
pub use validation::{
    ValidatedJson, ValidationRejection, FieldError,
    validate_identifier, validate_slug, validate_json_schema, validate_s3_path,
    validate_safe_filename, validate_uuid_string, validate_no_script_tags,
    sanitize,
};
pub use headers::{
    SecurityHeadersConfig, ContentSecurityPolicy, FrameOptions, ReferrerPolicy,
    CorsConfig, AllowedOrigins,
    security_headers_middleware, security_headers_with_config, create_security_headers_layer,
};
