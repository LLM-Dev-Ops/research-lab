use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Artifact metadata stored with S3 objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Size of the artifact in bytes
    pub size: u64,
    /// MIME content type
    pub content_type: String,
    /// SHA-256 checksum of the artifact data
    pub checksum: String,
    /// When the artifact was created
    pub created_at: DateTime<Utc>,
    /// Optional custom metadata
    #[serde(default)]
    pub custom: serde_json::Value,
}

impl ArtifactMetadata {
    /// Create new artifact metadata
    pub fn new(data: &[u8], content_type: String) -> Self {
        Self {
            size: data.len() as u64,
            content_type,
            checksum: Self::compute_checksum(data),
            created_at: Utc::now(),
            custom: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Create artifact metadata with custom fields
    pub fn with_custom(
        data: &[u8],
        content_type: String,
        custom: serde_json::Value,
    ) -> Self {
        Self {
            size: data.len() as u64,
            content_type,
            checksum: Self::compute_checksum(data),
            created_at: Utc::now(),
            custom,
        }
    }

    /// Compute SHA-256 checksum of data
    pub fn compute_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Verify that data matches the stored checksum
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        let computed = Self::compute_checksum(data);
        computed == self.checksum
    }

    /// Convert metadata to JSON string for S3 object metadata
    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Parse metadata from JSON string
    pub fn from_json_string(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Convert metadata to S3 metadata map
    pub fn to_s3_metadata(&self) -> std::collections::HashMap<String, String> {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("size".to_string(), self.size.to_string());
        metadata.insert("content-type".to_string(), self.content_type.clone());
        metadata.insert("checksum".to_string(), self.checksum.clone());
        metadata.insert("created-at".to_string(), self.created_at.to_rfc3339());

        // Store custom metadata as JSON
        if !self.custom.is_null() {
            if let Ok(custom_json) = serde_json::to_string(&self.custom) {
                metadata.insert("custom".to_string(), custom_json);
            }
        }

        metadata
    }

    /// Parse metadata from S3 metadata map
    pub fn from_s3_metadata(
        metadata: &std::collections::HashMap<String, String>,
    ) -> Result<Self> {
        let size = metadata
            .get("size")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let content_type = metadata
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let checksum = metadata
            .get("checksum")
            .cloned()
            .unwrap_or_default();

        let created_at = metadata
            .get("created-at")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let custom = metadata
            .get("custom")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        Ok(Self {
            size,
            content_type,
            checksum,
            created_at,
            custom,
        })
    }
}

/// Helper function to detect content type from file extension
pub fn detect_content_type(filename: &str) -> String {
    let extension = filename.split('.').last().unwrap_or("");

    match extension.to_lowercase().as_str() {
        "json" => "application/json",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "pt" | "pth" => "application/octet-stream", // PyTorch models
        "onnx" => "application/octet-stream",       // ONNX models
        "bin" => "application/octet-stream",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Helper function to format file size in human-readable format
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let data = b"test data";
        let metadata = ArtifactMetadata::new(data, "text/plain".to_string());

        assert_eq!(metadata.size, 9);
        assert_eq!(metadata.content_type, "text/plain");
        assert!(!metadata.checksum.is_empty());
    }

    #[test]
    fn test_checksum_verification() {
        let data = b"test data";
        let metadata = ArtifactMetadata::new(data, "text/plain".to_string());

        assert!(metadata.verify_checksum(data));
        assert!(!metadata.verify_checksum(b"different data"));
    }

    #[test]
    fn test_content_type_detection() {
        assert_eq!(detect_content_type("file.json"), "application/json");
        assert_eq!(detect_content_type("file.txt"), "text/plain");
        assert_eq!(detect_content_type("model.pt"), "application/octet-stream");
        assert_eq!(detect_content_type("image.png"), "image/png");
    }

    #[test]
    fn test_file_size_formatting() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(500), "500.00 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1048576), "1.00 MB");
        assert_eq!(format_file_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_s3_metadata_conversion() {
        let data = b"test data";
        let metadata = ArtifactMetadata::new(data, "text/plain".to_string());

        let s3_metadata = metadata.to_s3_metadata();
        assert_eq!(s3_metadata.get("size").unwrap(), "9");
        assert_eq!(s3_metadata.get("content-type").unwrap(), "text/plain");

        let parsed = ArtifactMetadata::from_s3_metadata(&s3_metadata).unwrap();
        assert_eq!(parsed.size, metadata.size);
        assert_eq!(parsed.content_type, metadata.content_type);
        assert_eq!(parsed.checksum, metadata.checksum);
    }
}
