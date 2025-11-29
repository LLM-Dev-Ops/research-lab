//! Datasets resource client
//!
//! This module provides methods for managing datasets.

use crate::client::{HttpClient, PaginatedResponse, PaginationParams};
use crate::error::SdkResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Client for dataset operations
#[derive(Debug, Clone)]
pub struct DatasetsClient {
    client: Arc<HttpClient>,
}

impl DatasetsClient {
    /// Create a new datasets client
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Create a new dataset
    pub async fn create(&self, request: CreateDatasetRequest) -> SdkResult<Dataset> {
        self.client.post("/datasets", request).await
    }

    /// Get a dataset by ID
    pub async fn get(&self, id: Uuid) -> SdkResult<Dataset> {
        self.client.get(&format!("/datasets/{}", id)).await
    }

    /// List datasets with optional filtering and pagination
    pub async fn list(
        &self,
        params: Option<ListDatasetsParams>,
    ) -> SdkResult<PaginatedResponse<Dataset>> {
        match params {
            Some(p) => self.client.get_with_query("/datasets", &p).await,
            None => self.client.get("/datasets").await,
        }
    }

    /// Update a dataset
    pub async fn update(&self, id: Uuid, request: UpdateDatasetRequest) -> SdkResult<Dataset> {
        self.client.put(&format!("/datasets/{}", id), request).await
    }

    /// Delete a dataset
    pub async fn delete(&self, id: Uuid) -> SdkResult<()> {
        self.client.delete(&format!("/datasets/{}", id)).await
    }

    /// Create a new version of a dataset
    pub async fn create_version(
        &self,
        dataset_id: Uuid,
        request: CreateVersionRequest,
    ) -> SdkResult<DatasetVersion> {
        self.client
            .post(&format!("/datasets/{}/versions", dataset_id), request)
            .await
    }

    /// List versions of a dataset
    pub async fn list_versions(
        &self,
        dataset_id: Uuid,
        pagination: Option<PaginationParams>,
    ) -> SdkResult<PaginatedResponse<DatasetVersion>> {
        let path = format!("/datasets/{}/versions", dataset_id);
        match pagination {
            Some(p) => self.client.get_with_query(&path, &p).await,
            None => self.client.get(&path).await,
        }
    }

    /// Get an upload URL for a dataset
    pub async fn get_upload_url(
        &self,
        dataset_id: Uuid,
        request: UploadRequest,
    ) -> SdkResult<UploadResponse> {
        self.client
            .post(&format!("/datasets/{}/upload", dataset_id), request)
            .await
    }

    /// Get a download URL for a dataset
    pub async fn get_download_url(&self, dataset_id: Uuid) -> SdkResult<DownloadResponse> {
        self.client
            .get(&format!("/datasets/{}/download", dataset_id))
            .await
    }
}

/// Request to create a new dataset
#[derive(Debug, Clone, Serialize)]
pub struct CreateDatasetRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub format: DatasetFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl CreateDatasetRequest {
    /// Create a new dataset request
    pub fn new(name: impl Into<String>, format: DatasetFormat) -> Self {
        Self {
            name: name.into(),
            description: None,
            format,
            schema: None,
            tags: None,
            metadata: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a schema
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Request to update a dataset
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateDatasetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl UpdateDatasetRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Parameters for listing datasets
#[derive(Debug, Clone, Serialize, Default)]
pub struct ListDatasetsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

impl ListDatasetsParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_format(mut self, format: DatasetFormat) -> Self {
        self.format = Some(format.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags.join(","));
        self
    }
}

/// Request to create a dataset version
#[derive(Debug, Clone, Serialize)]
pub struct CreateVersionRequest {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

impl CreateVersionRequest {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            description: None,
            changelog: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_changelog(mut self, changelog: impl Into<String>) -> Self {
        self.changelog = Some(changelog.into());
        self
    }
}

/// Request for upload URL
#[derive(Debug, Clone, Serialize)]
pub struct UploadRequest {
    pub filename: String,
    pub content_type: String,
}

impl UploadRequest {
    pub fn new(filename: impl Into<String>, content_type: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
        }
    }
}

/// Response with upload URL
#[derive(Debug, Clone, Deserialize)]
pub struct UploadResponse {
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
}

/// Response with download URL
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadResponse {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
}

/// Dataset format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasetFormat {
    Json,
    Jsonl,
    Csv,
    Parquet,
    Text,
}

impl std::fmt::Display for DatasetFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Jsonl => write!(f, "jsonl"),
            Self::Csv => write!(f, "csv"),
            Self::Parquet => write!(f, "parquet"),
            Self::Text => write!(f, "text"),
        }
    }
}

impl std::str::FromStr for DatasetFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "jsonl" => Ok(Self::Jsonl),
            "csv" => Ok(Self::Csv),
            "parquet" => Ok(Self::Parquet),
            "text" | "txt" => Ok(Self::Text),
            _ => Err(format!("Unknown dataset format: {}", s)),
        }
    }
}

/// Dataset entity
#[derive(Debug, Clone, Deserialize)]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub format: DatasetFormat,
    pub schema: Option<serde_json::Value>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub size_bytes: Option<u64>,
    pub row_count: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dataset version
#[derive(Debug, Clone, Deserialize)]
pub struct DatasetVersion {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub version: String,
    pub description: Option<String>,
    pub changelog: Option<String>,
    pub size_bytes: Option<u64>,
    pub row_count: Option<u64>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dataset_request_builder() {
        let request = CreateDatasetRequest::new("Test Dataset", DatasetFormat::Jsonl)
            .with_description("A test dataset")
            .with_tags(vec!["test".to_string(), "benchmark".to_string()])
            .with_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {"type": "string"},
                    "answer": {"type": "string"}
                }
            }));

        assert_eq!(request.name, "Test Dataset");
        assert_eq!(request.format, DatasetFormat::Jsonl);
        assert!(request.schema.is_some());
    }

    #[test]
    fn test_dataset_format_parsing() {
        assert_eq!("json".parse::<DatasetFormat>().unwrap(), DatasetFormat::Json);
        assert_eq!("jsonl".parse::<DatasetFormat>().unwrap(), DatasetFormat::Jsonl);
        assert_eq!("csv".parse::<DatasetFormat>().unwrap(), DatasetFormat::Csv);
        assert!("unknown".parse::<DatasetFormat>().is_err());
    }

    #[test]
    fn test_upload_request() {
        let request = UploadRequest::new("data.jsonl", "application/jsonl");
        assert_eq!(request.filename, "data.jsonl");
        assert_eq!(request.content_type, "application/jsonl");
    }
}
