//! Reliability module for the LLM Research API.
//!
//! This module provides comprehensive reliability patterns and practices including:
//! - Database backup and restore
//! - Bulkhead pattern for resource isolation
//! - Extended health checking
//! - Load shedding for graceful degradation
//!
//! # Module Organization
//!
//! - `backup`: Database backup and restore functionality
//! - `bulkhead`: Bulkhead pattern for fault isolation
//! - `health_ext`: Extended health check capabilities
//! - `load_shedding`: Load shedding for system protection
//!
//! # Example Usage
//!
//! ```no_run
//! use llm_research_api::reliability::*;
//! use std::sync::Arc;
//!
//! # async fn example() {
//! // Set up bulkhead for database operations
//! let db_bulkhead = Arc::new(Bulkhead::new(
//!     "database",
//!     BulkheadConfig::default(),
//! ));
//!
//! // Set up load shedding
//! let load_shedder = Arc::new(LoadShedder::new(LoadSheddingConfig::default()));
//! load_shedder.clone().start_monitoring();
//!
//! // Set up backup service
//! // let backup_service = PostgresBackupService::new(
//! //     db_url,
//! //     db_name,
//! //     BackupConfig::default(),
//! // );
//! # }
//! ```

pub mod backup;
pub mod bulkhead;
pub mod health_ext;
pub mod load_shedding;

// Re-export commonly used types for convenience

// Backup exports
pub use backup::{
    BackupConfig, BackupError, BackupMetadata, BackupResult, BackupService, BackupStatus,
    BackupType, PostgresBackupService, S3BackupStorage,
};

// Bulkhead exports
pub use bulkhead::{
    Bulkhead, BulkheadConfig, BulkheadError, BulkheadMetrics, BulkheadRegistry, RequestPriority,
    with_bulkhead,
};

// Health extensions exports
pub use health_ext::{
    AlertHandler, AlertSeverity, DeepHealthCheck, DependencyHealth, HealthAggregator,
    HealthAlert, HealthCheckScheduler, HealthHistory, HealthHistoryEntry, LoggingAlertHandler,
};

// Load shedding exports
pub use load_shedding::{
    create_load_shedding_layer, load_shedding_middleware, LoadLevel, LoadShedder,
    LoadSheddingConfig, LoadSheddingError, LoadSheddingMiddlewareState, LoadSheddingStats,
    RequestPriority as LoadSheddingPriority, ResourceMetrics,
};
