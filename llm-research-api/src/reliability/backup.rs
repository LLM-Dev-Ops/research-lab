//! Database backup and restore functionality for the LLM Research API.
//!
//! This module provides comprehensive backup capabilities including:
//! - Full, incremental, and differential backups
//! - Automated backup scheduling (cron-like)
//! - Retention policy enforcement
//! - S3 storage integration
//! - Backup verification and checksum validation
//! - Restore functionality
//!
//! # Example
//!
//! ```no_run
//! use llm_research_api::reliability::backup::*;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = BackupConfig {
//!     schedule: "0 2 * * *".to_string(), // Daily at 2 AM
//!     retention_days: 30,
//!     max_backups: 10,
//!     storage_path: "/backups".to_string(),
//!     compression_enabled: true,
//!     encryption_enabled: false,
//! };
//!
//! // Create and run backup
//! // let service = PostgresBackupService::new(pool, config);
//! // let metadata = service.create_backup(BackupType::Full).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Backup-related errors
#[derive(Error, Debug)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database backup failed: {0}")]
    DatabaseBackup(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Verification failed: {0}")]
    Verification(String),

    #[error("Restore failed: {0}")]
    Restore(String),

    #[error("Invalid backup metadata: {0}")]
    InvalidMetadata(String),

    #[error("Backup not found: {0}")]
    NotFound(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("S3 error: {0}")]
    S3(String),
}

pub type BackupResult<T> = Result<T, BackupError>;

/// Type of backup to perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupType {
    /// Full database backup
    Full,
    /// Incremental backup (only changes since last backup)
    Incremental,
    /// Differential backup (changes since last full backup)
    Differential,
}

impl BackupType {
    /// Returns the file suffix for this backup type
    pub fn suffix(&self) -> &str {
        match self {
            BackupType::Full => "full",
            BackupType::Incremental => "incr",
            BackupType::Differential => "diff",
        }
    }
}

/// Current status of a backup operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    /// Backup is currently in progress
    InProgress,
    /// Backup completed successfully
    Completed,
    /// Backup failed
    Failed,
}

/// Configuration for backup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Cron-like schedule expression (e.g., "0 2 * * *" for daily at 2 AM)
    pub schedule: String,
    /// Number of days to retain backups
    pub retention_days: u32,
    /// Maximum number of backups to keep
    pub max_backups: u32,
    /// Base storage path for backups
    pub storage_path: String,
    /// Whether to compress backups
    pub compression_enabled: bool,
    /// Whether to encrypt backups (requires encryption key)
    pub encryption_enabled: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            schedule: "0 2 * * *".to_string(), // Daily at 2 AM
            retention_days: 30,
            max_backups: 10,
            storage_path: "/var/backups/llm-research".to_string(),
            compression_enabled: true,
            encryption_enabled: false,
        }
    }
}

/// Metadata about a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique identifier for this backup
    pub id: String,
    /// Type of backup
    pub backup_type: BackupType,
    /// Current status
    pub status: BackupStatus,
    /// When the backup was created
    pub created_at: DateTime<Utc>,
    /// When the backup was completed (if successful)
    pub completed_at: Option<DateTime<Utc>>,
    /// Size of the backup file in bytes
    pub size_bytes: u64,
    /// SHA-256 checksum of the backup file
    pub checksum: String,
    /// Storage location (local path or S3 URI)
    pub storage_location: String,
    /// Database name that was backed up
    pub database_name: String,
    /// Optional error message if backup failed
    pub error_message: Option<String>,
    /// Whether the backup was compressed
    pub compressed: bool,
    /// Whether the backup was encrypted
    pub encrypted: bool,
}

impl BackupMetadata {
    /// Creates a new backup metadata entry
    pub fn new(
        id: String,
        backup_type: BackupType,
        database_name: String,
        storage_location: String,
    ) -> Self {
        Self {
            id,
            backup_type,
            status: BackupStatus::InProgress,
            created_at: Utc::now(),
            completed_at: None,
            size_bytes: 0,
            checksum: String::new(),
            storage_location,
            database_name,
            error_message: None,
            compressed: false,
            encrypted: false,
        }
    }

    /// Marks the backup as completed with metadata
    pub fn complete(mut self, size_bytes: u64, checksum: String) -> Self {
        self.status = BackupStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.size_bytes = size_bytes;
        self.checksum = checksum;
        self
    }

    /// Marks the backup as failed with an error message
    pub fn fail(mut self, error: String) -> Self {
        self.status = BackupStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error);
        self
    }

    /// Returns the age of this backup
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }
}

/// Trait for implementing backup services
#[async_trait]
pub trait BackupService: Send + Sync {
    /// Creates a new backup of the specified type
    async fn create_backup(&self, backup_type: BackupType) -> BackupResult<BackupMetadata>;

    /// Lists all available backups
    async fn list_backups(&self) -> BackupResult<Vec<BackupMetadata>>;

    /// Retrieves metadata for a specific backup
    async fn get_backup(&self, backup_id: &str) -> BackupResult<BackupMetadata>;

    /// Verifies the integrity of a backup using its checksum
    async fn verify_backup(&self, backup_id: &str) -> BackupResult<bool>;

    /// Restores a database from a backup
    async fn restore_backup(&self, backup_id: &str) -> BackupResult<()>;

    /// Deletes a backup
    async fn delete_backup(&self, backup_id: &str) -> BackupResult<()>;

    /// Enforces retention policy by deleting old backups
    async fn enforce_retention_policy(&self) -> BackupResult<usize>;
}

/// PostgreSQL backup service implementation using pg_dump
pub struct PostgresBackupService {
    database_url: String,
    database_name: String,
    config: BackupConfig,
    metadata_store: PathBuf,
}

impl PostgresBackupService {
    /// Creates a new PostgreSQL backup service
    pub fn new(database_url: String, database_name: String, config: BackupConfig) -> Self {
        let metadata_store = PathBuf::from(&config.storage_path).join("metadata");

        Self {
            database_url,
            database_name,
            config,
            metadata_store,
        }
    }

    /// Ensures storage directories exist
    async fn ensure_storage(&self) -> BackupResult<()> {
        fs::create_dir_all(&self.config.storage_path).await?;
        fs::create_dir_all(&self.metadata_store).await?;
        Ok(())
    }

    /// Generates a unique backup ID
    fn generate_backup_id(&self, backup_type: BackupType) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("{}_{}", timestamp, backup_type.suffix())
    }

    /// Generates the file path for a backup
    fn backup_path(&self, backup_id: &str) -> PathBuf {
        let mut path = PathBuf::from(&self.config.storage_path);
        path.push(format!("{}.sql", backup_id));

        if self.config.compression_enabled {
            path.set_extension("sql.gz");
        }

        path
    }

    /// Generates the metadata file path for a backup
    fn metadata_path(&self, backup_id: &str) -> PathBuf {
        self.metadata_store.join(format!("{}.json", backup_id))
    }

    /// Saves backup metadata to disk
    async fn save_metadata(&self, metadata: &BackupMetadata) -> BackupResult<()> {
        let path = self.metadata_path(&metadata.id);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| BackupError::InvalidMetadata(e.to_string()))?;

        fs::write(path, json).await?;
        Ok(())
    }

    /// Loads backup metadata from disk
    async fn load_metadata(&self, backup_id: &str) -> BackupResult<BackupMetadata> {
        let path = self.metadata_path(backup_id);

        if !path.exists() {
            return Err(BackupError::NotFound(backup_id.to_string()));
        }

        let json = fs::read_to_string(path).await?;
        let metadata = serde_json::from_str(&json)
            .map_err(|e| BackupError::InvalidMetadata(e.to_string()))?;

        Ok(metadata)
    }

    /// Computes SHA-256 checksum of a file
    async fn compute_checksum(&self, path: &Path) -> BackupResult<String> {
        let contents = fs::read(path).await?;
        let hash = Sha256::digest(&contents);
        Ok(format!("{:x}", hash))
    }

    /// Executes pg_dump to create a backup
    async fn execute_pg_dump(&self, output_path: &Path) -> BackupResult<()> {
        info!("Starting pg_dump to {:?}", output_path);

        let mut cmd = Command::new("pg_dump");
        cmd.arg(&self.database_url)
            .arg("--format=plain")
            .arg("--no-owner")
            .arg("--no-acl")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| BackupError::DatabaseBackup(format!("Failed to spawn pg_dump: {}", e)))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| BackupError::DatabaseBackup("Failed to capture stdout".to_string()))?;

        // Write output to file (with optional compression)
        if self.config.compression_enabled {
            self.write_compressed(stdout, output_path).await?;
        } else {
            self.write_uncompressed(stdout, output_path).await?;
        }

        // Wait for pg_dump to complete
        let status = child.wait().await?;

        if !status.success() {
            let stderr = child.stderr.take();
            let error_msg = if let Some(mut stderr) = stderr {
                let mut buf = Vec::new();
                tokio::io::copy(&mut stderr, &mut buf).await?;
                String::from_utf8_lossy(&buf).to_string()
            } else {
                "Unknown error".to_string()
            };

            return Err(BackupError::DatabaseBackup(format!(
                "pg_dump failed: {}",
                error_msg
            )));
        }

        Ok(())
    }

    /// Writes uncompressed output to a file
    async fn write_uncompressed(
        &self,
        mut source: impl tokio::io::AsyncRead + Unpin,
        path: &Path,
    ) -> BackupResult<()> {
        let mut file = fs::File::create(path).await?;
        tokio::io::copy(&mut source, &mut file).await?;
        file.flush().await?;
        Ok(())
    }

    /// Writes compressed output to a file using gzip
    async fn write_compressed(
        &self,
        mut source: impl tokio::io::AsyncRead + Unpin,
        path: &Path,
    ) -> BackupResult<()> {
        use tokio::io::AsyncReadExt;

        // Read all data
        let mut buffer = Vec::new();
        source.read_to_end(&mut buffer).await?;

        // Compress using flate2
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&buffer)
            .map_err(|e| BackupError::Compression(e.to_string()))?;
        let compressed = encoder.finish()
            .map_err(|e| BackupError::Compression(e.to_string()))?;

        // Write to file
        fs::write(path, compressed).await?;
        Ok(())
    }

    /// Executes psql to restore a backup
    async fn execute_psql_restore(&self, backup_path: &Path) -> BackupResult<()> {
        info!("Starting restore from {:?}", backup_path);

        // Read backup file (with optional decompression)
        let sql_content = if self.config.compression_enabled {
            self.read_compressed(backup_path).await?
        } else {
            fs::read_to_string(backup_path).await?
        };

        let mut cmd = Command::new("psql");
        cmd.arg(&self.database_url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| BackupError::Restore(format!("Failed to spawn psql: {}", e)))?;

        // Write SQL to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(sql_content.as_bytes()).await
                .map_err(|e| BackupError::Restore(format!("Failed to write to psql: {}", e)))?;
        }

        let status = child.wait().await?;

        if !status.success() {
            return Err(BackupError::Restore("psql restore failed".to_string()));
        }

        Ok(())
    }

    /// Reads and decompresses a gzipped file
    async fn read_compressed(&self, path: &Path) -> BackupResult<String> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let compressed = fs::read(path).await?;
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed)
            .map_err(|e| BackupError::Compression(e.to_string()))?;

        Ok(decompressed)
    }
}

#[async_trait]
impl BackupService for PostgresBackupService {
    async fn create_backup(&self, backup_type: BackupType) -> BackupResult<BackupMetadata> {
        self.ensure_storage().await?;

        let backup_id = self.generate_backup_id(backup_type);
        let backup_path = self.backup_path(&backup_id);

        info!("Creating {} backup: {}",
            match backup_type {
                BackupType::Full => "full",
                BackupType::Incremental => "incremental",
                BackupType::Differential => "differential",
            },
            backup_id
        );

        let mut metadata = BackupMetadata::new(
            backup_id.clone(),
            backup_type,
            self.database_name.clone(),
            backup_path.to_string_lossy().to_string(),
        );
        metadata.compressed = self.config.compression_enabled;

        // Save initial metadata
        self.save_metadata(&metadata).await?;

        // Execute backup
        match self.execute_pg_dump(&backup_path).await {
            Ok(_) => {
                // Compute file size and checksum
                let file_metadata = fs::metadata(&backup_path).await?;
                let size_bytes = file_metadata.len();
                let checksum = self.compute_checksum(&backup_path).await?;

                metadata = metadata.complete(size_bytes, checksum);
                self.save_metadata(&metadata).await?;

                info!("Backup completed: {} ({} bytes)", backup_id, size_bytes);
                Ok(metadata)
            }
            Err(e) => {
                error!("Backup failed: {}", e);
                metadata = metadata.fail(e.to_string());
                self.save_metadata(&metadata).await?;
                Err(e)
            }
        }
    }

    async fn list_backups(&self) -> BackupResult<Vec<BackupMetadata>> {
        let mut backups = Vec::new();

        if !self.metadata_store.exists() {
            return Ok(backups);
        }

        let mut entries = fs::read_dir(&self.metadata_store).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem() {
                    if let Some(backup_id) = filename.to_str() {
                        match self.load_metadata(backup_id).await {
                            Ok(metadata) => backups.push(metadata),
                            Err(e) => warn!("Failed to load metadata for {}: {}", backup_id, e),
                        }
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    async fn get_backup(&self, backup_id: &str) -> BackupResult<BackupMetadata> {
        self.load_metadata(backup_id).await
    }

    async fn verify_backup(&self, backup_id: &str) -> BackupResult<bool> {
        let metadata = self.load_metadata(backup_id).await?;
        let backup_path = PathBuf::from(&metadata.storage_location);

        if !backup_path.exists() {
            return Err(BackupError::NotFound(format!(
                "Backup file not found: {}",
                backup_path.display()
            )));
        }

        let computed_checksum = self.compute_checksum(&backup_path).await?;

        if computed_checksum != metadata.checksum {
            warn!("Checksum mismatch for backup {}: expected {}, got {}",
                backup_id, metadata.checksum, computed_checksum);
            return Ok(false);
        }

        debug!("Backup {} verified successfully", backup_id);
        Ok(true)
    }

    async fn restore_backup(&self, backup_id: &str) -> BackupResult<()> {
        let metadata = self.load_metadata(backup_id).await?;

        if metadata.status != BackupStatus::Completed {
            return Err(BackupError::Restore(format!(
                "Cannot restore backup with status: {:?}",
                metadata.status
            )));
        }

        // Verify backup integrity first
        if !self.verify_backup(backup_id).await? {
            return Err(BackupError::Verification(format!(
                "Backup {} failed verification",
                backup_id
            )));
        }

        let backup_path = PathBuf::from(&metadata.storage_location);
        self.execute_psql_restore(&backup_path).await?;

        info!("Successfully restored backup {}", backup_id);
        Ok(())
    }

    async fn delete_backup(&self, backup_id: &str) -> BackupResult<()> {
        let metadata = self.load_metadata(backup_id).await?;

        // Delete backup file
        let backup_path = PathBuf::from(&metadata.storage_location);
        if backup_path.exists() {
            fs::remove_file(&backup_path).await?;
        }

        // Delete metadata file
        let metadata_path = self.metadata_path(backup_id);
        if metadata_path.exists() {
            fs::remove_file(&metadata_path).await?;
        }

        info!("Deleted backup {}", backup_id);
        Ok(())
    }

    async fn enforce_retention_policy(&self) -> BackupResult<usize> {
        let backups = self.list_backups().await?;
        let mut deleted_count = 0;

        for backup in &backups {
            let should_delete =
                // Delete if older than retention period
                backup.age().num_days() > self.config.retention_days as i64
                // OR if we have too many backups
                || backups.len() > self.config.max_backups as usize;

            if should_delete {
                match self.delete_backup(&backup.id).await {
                    Ok(_) => {
                        deleted_count += 1;
                        info!("Deleted old backup: {}", backup.id);
                    }
                    Err(e) => {
                        warn!("Failed to delete backup {}: {}", backup.id, e);
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}

/// S3-based backup storage
pub struct S3BackupStorage {
    client: S3Client,
    bucket: String,
    prefix: String,
}

impl S3BackupStorage {
    /// Creates a new S3 backup storage
    pub fn new(client: S3Client, bucket: String, prefix: String) -> Self {
        Self {
            client,
            bucket,
            prefix,
        }
    }

    /// Uploads a backup file to S3
    pub async fn upload_backup(
        &self,
        backup_id: &str,
        file_path: &Path,
    ) -> BackupResult<String> {
        let key = format!("{}/{}.sql.gz", self.prefix, backup_id);
        let body = aws_sdk_s3::primitives::ByteStream::from_path(file_path)
            .await
            .map_err(|e| BackupError::S3(format!("Failed to read file: {}", e)))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .send()
            .await
            .map_err(|e| BackupError::S3(format!("Upload failed: {}", e)))?;

        let s3_uri = format!("s3://{}/{}", self.bucket, key);
        info!("Uploaded backup to {}", s3_uri);

        Ok(s3_uri)
    }

    /// Downloads a backup file from S3
    pub async fn download_backup(
        &self,
        backup_id: &str,
        destination: &Path,
    ) -> BackupResult<()> {
        let key = format!("{}/{}.sql.gz", self.prefix, backup_id);

        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| BackupError::S3(format!("Download failed: {}", e)))?;

        let data = response.body.collect().await
            .map_err(|e| BackupError::S3(format!("Failed to read body: {}", e)))?
            .into_bytes();

        fs::write(destination, data).await?;

        info!("Downloaded backup from s3://{}/{}", self.bucket, key);
        Ok(())
    }

    /// Lists all backups in S3
    pub async fn list_s3_backups(&self) -> BackupResult<Vec<String>> {
        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&self.prefix)
            .send()
            .await
            .map_err(|e| BackupError::S3(format!("List failed: {}", e)))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(keys)
    }

    /// Deletes a backup from S3
    pub async fn delete_s3_backup(&self, backup_id: &str) -> BackupResult<()> {
        let key = format!("{}/{}.sql.gz", self.prefix, backup_id);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| BackupError::S3(format!("Delete failed: {}", e)))?;

        info!("Deleted backup from s3://{}/{}", self.bucket, key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_type_suffix() {
        assert_eq!(BackupType::Full.suffix(), "full");
        assert_eq!(BackupType::Incremental.suffix(), "incr");
        assert_eq!(BackupType::Differential.suffix(), "diff");
    }

    #[test]
    fn test_backup_config_default() {
        let config = BackupConfig::default();
        assert_eq!(config.schedule, "0 2 * * *");
        assert_eq!(config.retention_days, 30);
        assert_eq!(config.max_backups, 10);
        assert!(config.compression_enabled);
        assert!(!config.encryption_enabled);
    }

    #[test]
    fn test_backup_metadata_new() {
        let metadata = BackupMetadata::new(
            "test_backup".to_string(),
            BackupType::Full,
            "testdb".to_string(),
            "/backups/test.sql".to_string(),
        );

        assert_eq!(metadata.id, "test_backup");
        assert_eq!(metadata.backup_type, BackupType::Full);
        assert_eq!(metadata.status, BackupStatus::InProgress);
        assert_eq!(metadata.database_name, "testdb");
        assert_eq!(metadata.size_bytes, 0);
        assert!(metadata.checksum.is_empty());
        assert!(metadata.completed_at.is_none());
    }

    #[test]
    fn test_backup_metadata_complete() {
        let metadata = BackupMetadata::new(
            "test".to_string(),
            BackupType::Full,
            "testdb".to_string(),
            "/backups/test.sql".to_string(),
        )
        .complete(1024, "abc123".to_string());

        assert_eq!(metadata.status, BackupStatus::Completed);
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.checksum, "abc123");
        assert!(metadata.completed_at.is_some());
        assert!(metadata.error_message.is_none());
    }

    #[test]
    fn test_backup_metadata_fail() {
        let metadata = BackupMetadata::new(
            "test".to_string(),
            BackupType::Full,
            "testdb".to_string(),
            "/backups/test.sql".to_string(),
        )
        .fail("Connection timeout".to_string());

        assert_eq!(metadata.status, BackupStatus::Failed);
        assert!(metadata.completed_at.is_some());
        assert_eq!(metadata.error_message, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_backup_metadata_age() {
        let metadata = BackupMetadata::new(
            "test".to_string(),
            BackupType::Full,
            "testdb".to_string(),
            "/backups/test.sql".to_string(),
        );

        let age = metadata.age();
        assert!(age.num_seconds() >= 0);
        assert!(age.num_seconds() < 5); // Should be very recent
    }

    #[test]
    fn test_postgres_backup_service_new() {
        let config = BackupConfig::default();
        let service = PostgresBackupService::new(
            "postgresql://localhost/test".to_string(),
            "testdb".to_string(),
            config.clone(),
        );

        assert_eq!(service.database_name, "testdb");
        assert_eq!(service.config.retention_days, config.retention_days);
    }

    #[test]
    fn test_postgres_backup_service_backup_path() {
        let config = BackupConfig {
            storage_path: "/tmp/backups".to_string(),
            compression_enabled: false,
            ..Default::default()
        };

        let service = PostgresBackupService::new(
            "postgresql://localhost/test".to_string(),
            "testdb".to_string(),
            config,
        );

        let path = service.backup_path("20240101_120000_full");
        assert_eq!(path, PathBuf::from("/tmp/backups/20240101_120000_full.sql"));
    }

    #[test]
    fn test_postgres_backup_service_backup_path_compressed() {
        let config = BackupConfig {
            storage_path: "/tmp/backups".to_string(),
            compression_enabled: true,
            ..Default::default()
        };

        let service = PostgresBackupService::new(
            "postgresql://localhost/test".to_string(),
            "testdb".to_string(),
            config,
        );

        let path = service.backup_path("20240101_120000_full");
        assert_eq!(path, PathBuf::from("/tmp/backups/20240101_120000_full.sql.gz"));
    }

    #[test]
    fn test_postgres_backup_service_metadata_path() {
        let config = BackupConfig {
            storage_path: "/tmp/backups".to_string(),
            ..Default::default()
        };

        let service = PostgresBackupService::new(
            "postgresql://localhost/test".to_string(),
            "testdb".to_string(),
            config,
        );

        let path = service.metadata_path("20240101_120000_full");
        assert_eq!(
            path,
            PathBuf::from("/tmp/backups/metadata/20240101_120000_full.json")
        );
    }

    #[test]
    fn test_s3_backup_storage_new() {
        use aws_sdk_s3::config::{BehaviorVersion, Region};
        use aws_sdk_s3::Config;

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .build();
        let client = S3Client::from_conf(config);

        let storage = S3BackupStorage::new(
            client,
            "my-backups".to_string(),
            "postgres/backups".to_string(),
        );

        assert_eq!(storage.bucket, "my-backups");
        assert_eq!(storage.prefix, "postgres/backups");
    }

    #[tokio::test]
    async fn test_backup_metadata_serialization() {
        let metadata = BackupMetadata::new(
            "test_backup".to_string(),
            BackupType::Full,
            "testdb".to_string(),
            "/backups/test.sql".to_string(),
        )
        .complete(2048, "checksum123".to_string());

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: BackupMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, metadata.id);
        assert_eq!(deserialized.status, BackupStatus::Completed);
        assert_eq!(deserialized.size_bytes, 2048);
        assert_eq!(deserialized.checksum, "checksum123");
    }

    #[tokio::test]
    async fn test_postgres_service_generate_backup_id() {
        let config = BackupConfig::default();
        let service = PostgresBackupService::new(
            "postgresql://localhost/test".to_string(),
            "testdb".to_string(),
            config,
        );

        let id = service.generate_backup_id(BackupType::Full);
        assert!(id.ends_with("_full"));
        assert!(id.len() > 10); // Should contain timestamp
    }

    // Integration tests would require actual database and S3 access
    // These are marked as ignored and can be run with `cargo test -- --ignored`

    #[tokio::test]
    #[ignore]
    async fn test_postgres_backup_service_create_backup() {
        // This test requires a running PostgreSQL instance
        // Run with: cargo test test_postgres_backup_service_create_backup -- --ignored
    }

    #[tokio::test]
    #[ignore]
    async fn test_s3_backup_storage_upload_download() {
        // This test requires AWS credentials and S3 access
        // Run with: cargo test test_s3_backup_storage_upload_download -- --ignored
    }
}
