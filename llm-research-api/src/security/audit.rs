//! Audit logging system for tracking all significant operations
//!
//! This module provides comprehensive audit logging capabilities including:
//! - Structured audit events with rich metadata
//! - Multiple storage backends (database, file, tracing)
//! - Middleware for automatic request/response logging
//! - Retention and rotation support

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_research_core::domain::UserId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Result type for audit operations
pub type AuditResult<T> = Result<T, AuditError>;

/// Errors that can occur during audit logging
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Write failed: {0}")]
    WriteFailed(String),
}

/// Comprehensive audit event capturing all relevant operation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique identifier for this audit event
    pub id: Uuid,

    /// When the event occurred
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: AuditEventType,

    /// Who performed the action
    pub actor: AuditActor,

    /// What resource was affected
    pub resource: AuditResource,

    /// What action was performed
    pub action: AuditAction,

    /// Outcome of the operation
    pub outcome: AuditOutcome,

    /// Additional structured details
    pub details: Value,

    /// IP address of the requester
    pub ip_address: Option<IpAddr>,

    /// User agent string from the request
    pub user_agent: Option<String>,

    /// Request ID for correlation
    pub request_id: Option<String>,

    /// How long the operation took (milliseconds)
    pub duration_ms: Option<u64>,
}

impl AuditEvent {
    /// Create a new audit event with minimal required fields
    pub fn new(
        event_type: AuditEventType,
        actor: AuditActor,
        resource: AuditResource,
        action: AuditAction,
        outcome: AuditOutcome,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            actor,
            resource,
            action,
            outcome,
            details: Value::Null,
            ip_address: None,
            user_agent: None,
            request_id: None,
            duration_ms: None,
        }
    }

    /// Add structured details to the event
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }

    /// Add IP address
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Add user agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Add request ID for correlation
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Add duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// Classification of audit events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Authentication events (login, logout, etc.)
    Authentication,

    /// Authorization events (permission checks)
    Authorization,

    /// Data access events (reads)
    DataAccess,

    /// Data modification events (create, update, delete)
    DataModification,

    /// Configuration changes
    Configuration,

    /// Security-related events
    Security,

    /// System-level events
    SystemEvent,
}

/// Represents who performed an action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditActor {
    /// Authenticated user
    User {
        id: UserId,
        email: String,
    },

    /// API key authentication
    ApiKey {
        id: Uuid,
        name: String,
    },

    /// System-initiated action
    System,

    /// Unauthenticated request
    Anonymous {
        ip: IpAddr,
    },
}

/// Represents what resource was affected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditResource {
    Experiment {
        id: Uuid,
    },
    Run {
        id: Uuid,
        experiment_id: Uuid,
    },
    Model {
        id: Uuid,
    },
    Dataset {
        id: Uuid,
    },
    PromptTemplate {
        id: Uuid,
    },
    Evaluation {
        id: Uuid,
    },
    User {
        id: Uuid,
    },
    ApiKey {
        id: Uuid,
    },
    System,
}

/// Actions that can be audited
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // CRUD operations
    Create,
    Read,
    Update,
    Delete,

    // Authentication actions
    Login,
    Logout,
    LoginFailed,

    // Password management
    PasswordChange,
    PasswordReset,

    // API key management
    ApiKeyCreated,
    ApiKeyRevoked,

    // Permission management
    PermissionGranted,
    PermissionRevoked,

    // Data operations
    Export,
    Import,

    // Configuration
    ConfigChange,
}

/// Outcome of an audited operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Operation succeeded
    Success,

    /// Operation failed
    Failure {
        reason: String,
    },

    /// Access was denied
    Denied {
        reason: String,
    },
}

impl AuditOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, AuditOutcome::Success)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, AuditOutcome::Failure { .. })
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, AuditOutcome::Denied { .. })
    }
}

/// Trait for audit event writers
#[async_trait]
pub trait AuditWriter: Send + Sync {
    /// Write an audit event
    async fn write(&self, event: &AuditEvent) -> AuditResult<()>;

    /// Flush any pending writes
    async fn flush(&self) -> AuditResult<()> {
        Ok(())
    }
}

/// Main audit logger that coordinates event writing
pub struct AuditLogger {
    writer: Arc<dyn AuditWriter>,
}

impl AuditLogger {
    /// Create a new audit logger with the specified writer
    pub fn new(writer: Box<dyn AuditWriter>) -> Self {
        Self {
            writer: Arc::from(writer),
        }
    }

    /// Log an audit event
    pub async fn log(&self, event: AuditEvent) -> AuditResult<()> {
        self.writer.write(&event).await
    }

    /// Log a successful authentication
    pub async fn log_auth_success(
        &self,
        actor: &AuditActor,
        ip: IpAddr,
    ) -> AuditResult<()> {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            actor.clone(),
            AuditResource::System,
            AuditAction::Login,
            AuditOutcome::Success,
        )
        .with_ip(ip);

        self.log(event).await
    }

    /// Log a failed authentication attempt
    pub async fn log_auth_failure(
        &self,
        email: &str,
        reason: &str,
        ip: IpAddr,
    ) -> AuditResult<()> {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            AuditActor::Anonymous { ip },
            AuditResource::System,
            AuditAction::LoginFailed,
            AuditOutcome::Failure {
                reason: reason.to_string(),
            },
        )
        .with_ip(ip)
        .with_details(serde_json::json!({
            "email": email,
        }));

        self.log(event).await
    }

    /// Log a resource access event
    pub async fn log_access(
        &self,
        actor: &AuditActor,
        resource: &AuditResource,
        action: AuditAction,
        outcome: AuditOutcome,
    ) -> AuditResult<()> {
        let event_type = if matches!(action, AuditAction::Read) {
            AuditEventType::DataAccess
        } else {
            AuditEventType::DataModification
        };

        let event = AuditEvent::new(
            event_type,
            actor.clone(),
            resource.clone(),
            action,
            outcome,
        );

        self.log(event).await
    }

    /// Log a data modification event with before/after state
    pub async fn log_modification(
        &self,
        actor: &AuditActor,
        resource: &AuditResource,
        action: AuditAction,
        before: Option<Value>,
        after: Option<Value>,
    ) -> AuditResult<()> {
        let mut details = serde_json::Map::new();

        if let Some(before_val) = before {
            details.insert("before".to_string(), before_val);
        }

        if let Some(after_val) = after {
            details.insert("after".to_string(), after_val);
        }

        let event = AuditEvent::new(
            AuditEventType::DataModification,
            actor.clone(),
            resource.clone(),
            action,
            AuditOutcome::Success,
        )
        .with_details(Value::Object(details));

        self.log(event).await
    }

    /// Flush any pending writes
    pub async fn flush(&self) -> AuditResult<()> {
        self.writer.flush().await
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
        }
    }
}

/// Writes audit events to PostgreSQL database
pub struct DatabaseAuditWriter {
    pool: PgPool,
}

impl DatabaseAuditWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditWriter for DatabaseAuditWriter {
    async fn write(&self, event: &AuditEvent) -> AuditResult<()> {
        let event_json = serde_json::to_value(event)?;

        sqlx::query(
            r#"
            INSERT INTO audit_log (
                id, timestamp, event_type, actor, resource, action, outcome,
                details, ip_address, user_agent, request_id, duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(event.id)
        .bind(event.timestamp)
        .bind(serde_json::to_value(&event.event_type)?)
        .bind(serde_json::to_value(&event.actor)?)
        .bind(serde_json::to_value(&event.resource)?)
        .bind(serde_json::to_value(&event.action)?)
        .bind(serde_json::to_value(&event.outcome)?)
        .bind(&event.details)
        .bind(event.ip_address.map(|ip| ip.to_string()))
        .bind(event.user_agent.as_ref())
        .bind(event.request_id.as_ref())
        .bind(event.duration_ms.map(|d| d as i64))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Writes audit events to JSON file with rotation support
pub struct FileAuditWriter {
    file_path: PathBuf,
    current_file: Arc<RwLock<Option<File>>>,
    max_size_bytes: u64,
    max_files: usize,
}

impl FileAuditWriter {
    /// Create a new file audit writer
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            current_file: Arc::new(RwLock::new(None)),
            max_size_bytes: 100 * 1024 * 1024, // 100 MB default
            max_files: 10, // Keep 10 rotated files
        }
    }

    /// Set maximum file size before rotation
    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.max_size_bytes = max_size_bytes;
        self
    }

    /// Set maximum number of rotated files to keep
    pub fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /// Ensure file is open and ready for writing
    async fn ensure_file_open(&self) -> AuditResult<()> {
        let mut file_lock = self.current_file.write().await;

        if file_lock.is_none() {
            // Create parent directories if they don't exist
            if let Some(parent) = self.file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .await?;

            *file_lock = Some(file);
        }

        Ok(())
    }

    /// Check if rotation is needed and perform it
    async fn check_rotation(&self) -> AuditResult<()> {
        let metadata = tokio::fs::metadata(&self.file_path).await?;

        if metadata.len() >= self.max_size_bytes {
            self.rotate_files().await?;
        }

        Ok(())
    }

    /// Rotate log files
    async fn rotate_files(&self) -> AuditResult<()> {
        // Close current file
        let mut file_lock = self.current_file.write().await;
        *file_lock = None;
        drop(file_lock);

        // Rotate existing files
        for i in (1..self.max_files).rev() {
            let old_path = if i == 1 {
                self.file_path.clone()
            } else {
                self.file_path.with_extension(format!("{}", i - 1))
            };

            let new_path = self.file_path.with_extension(format!("{}", i));

            if tokio::fs::metadata(&old_path).await.is_ok() {
                tokio::fs::rename(&old_path, &new_path).await?;
            }
        }

        // Delete oldest file if it exists
        let oldest = self.file_path.with_extension(format!("{}", self.max_files));
        let _ = tokio::fs::remove_file(&oldest).await; // Ignore error if file doesn't exist

        Ok(())
    }
}

#[async_trait]
impl AuditWriter for FileAuditWriter {
    async fn write(&self, event: &AuditEvent) -> AuditResult<()> {
        self.ensure_file_open().await?;

        // Check if rotation is needed
        if let Err(e) = self.check_rotation().await {
            error!("Failed to check/perform log rotation: {}", e);
        }

        let json = serde_json::to_string(event)?;
        let line = format!("{}\n", json);

        let mut file_lock = self.current_file.write().await;
        if let Some(ref mut file) = *file_lock {
            file.write_all(line.as_bytes()).await?;
        }

        Ok(())
    }

    async fn flush(&self) -> AuditResult<()> {
        let mut file_lock = self.current_file.write().await;
        if let Some(ref mut file) = *file_lock {
            file.flush().await?;
        }
        Ok(())
    }
}

/// Writes audit events using the tracing crate
pub struct TracingAuditWriter;

impl TracingAuditWriter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TracingAuditWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditWriter for TracingAuditWriter {
    async fn write(&self, event: &AuditEvent) -> AuditResult<()> {
        let level = if event.outcome.is_denied() || event.outcome.is_failure() {
            tracing::Level::WARN
        } else {
            tracing::Level::INFO
        };

        match level {
            tracing::Level::WARN => {
                warn!(
                    audit_id = %event.id,
                    timestamp = %event.timestamp,
                    event_type = ?event.event_type,
                    actor = ?event.actor,
                    resource = ?event.resource,
                    action = ?event.action,
                    outcome = ?event.outcome,
                    ip_address = ?event.ip_address,
                    duration_ms = ?event.duration_ms,
                    "Audit event"
                );
            }
            _ => {
                info!(
                    audit_id = %event.id,
                    timestamp = %event.timestamp,
                    event_type = ?event.event_type,
                    actor = ?event.actor,
                    resource = ?event.resource,
                    action = ?event.action,
                    outcome = ?event.outcome,
                    ip_address = ?event.ip_address,
                    duration_ms = ?event.duration_ms,
                    "Audit event"
                );
            }
        }

        Ok(())
    }
}

/// Writes audit events to multiple destinations
pub struct CompositeAuditWriter {
    writers: Vec<Arc<dyn AuditWriter>>,
}

impl CompositeAuditWriter {
    pub fn new() -> Self {
        Self {
            writers: Vec::new(),
        }
    }

    /// Add a writer to the composite
    pub fn add_writer(mut self, writer: Box<dyn AuditWriter>) -> Self {
        self.writers.push(Arc::from(writer));
        self
    }
}

impl Default for CompositeAuditWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditWriter for CompositeAuditWriter {
    async fn write(&self, event: &AuditEvent) -> AuditResult<()> {
        let mut errors = Vec::new();

        for writer in &self.writers {
            if let Err(e) = writer.write(event).await {
                error!("Audit writer failed: {}", e);
                errors.push(e);
            }
        }

        // Return error only if all writers failed
        if !errors.is_empty() && errors.len() == self.writers.len() {
            return Err(AuditError::WriteFailed(
                "All audit writers failed".to_string(),
            ));
        }

        Ok(())
    }

    async fn flush(&self) -> AuditResult<()> {
        for writer in &self.writers {
            writer.flush().await?;
        }
        Ok(())
    }
}

/// Middleware state for audit logging
#[derive(Clone)]
pub struct AuditMiddlewareState {
    pub logger: AuditLogger,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            AuditActor::System,
            AuditResource::System,
            AuditAction::Login,
            AuditOutcome::Success,
        )
        .with_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
        .with_duration(100);

        assert_eq!(event.event_type, AuditEventType::Authentication);
        assert_eq!(event.action, AuditAction::Login);
        assert!(event.outcome.is_success());
        assert_eq!(event.duration_ms, Some(100));
    }

    #[tokio::test]
    async fn test_tracing_audit_writer() {
        let writer = TracingAuditWriter::new();

        let event = AuditEvent::new(
            AuditEventType::DataAccess,
            AuditActor::System,
            AuditResource::System,
            AuditAction::Read,
            AuditOutcome::Success,
        );

        let result = writer.write(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_composite_writer() {
        let mut composite = CompositeAuditWriter::new();
        composite = composite.add_writer(Box::new(TracingAuditWriter::new()));

        let event = AuditEvent::new(
            AuditEventType::DataModification,
            AuditActor::System,
            AuditResource::System,
            AuditAction::Create,
            AuditOutcome::Success,
        );

        let result = composite.write(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logger_auth_success() {
        let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

        let actor = AuditActor::User {
            id: UserId::new(),
            email: "test@example.com".to_string(),
        };

        let result = logger.log_auth_success(
            &actor,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logger_auth_failure() {
        let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

        let result = logger.log_auth_failure(
            "test@example.com",
            "Invalid password",
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logger_modification() {
        let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

        let actor = AuditActor::System;
        let resource = AuditResource::Experiment { id: Uuid::new_v4() };

        let before = serde_json::json!({"status": "draft"});
        let after = serde_json::json!({"status": "running"});

        let result = logger.log_modification(
            &actor,
            &resource,
            AuditAction::Update,
            Some(before),
            Some(after),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_outcome_checks() {
        assert!(AuditOutcome::Success.is_success());
        assert!(!AuditOutcome::Success.is_failure());
        assert!(!AuditOutcome::Success.is_denied());

        let failure = AuditOutcome::Failure {
            reason: "Error".to_string(),
        };
        assert!(failure.is_failure());
        assert!(!failure.is_success());

        let denied = AuditOutcome::Denied {
            reason: "Forbidden".to_string(),
        };
        assert!(denied.is_denied());
        assert!(!denied.is_success());
    }

    #[tokio::test]
    async fn test_file_writer_creation() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("audit.log");

        let writer = FileAuditWriter::new(file_path.clone())
            .with_max_size(1024 * 1024)
            .with_max_files(5);

        let event = AuditEvent::new(
            AuditEventType::SystemEvent,
            AuditActor::System,
            AuditResource::System,
            AuditAction::ConfigChange,
            AuditOutcome::Success,
        );

        let result = writer.write(&event).await;
        assert!(result.is_ok());

        // Verify file was created
        assert!(tokio::fs::metadata(&file_path).await.is_ok());

        // Verify content
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("config_change"));
    }
}
