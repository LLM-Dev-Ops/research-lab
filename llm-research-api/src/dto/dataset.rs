use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateDatasetRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    pub s3_path: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateDatasetRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub s3_path: String,
    pub sample_count: i64,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateDatasetVersionRequest {
    pub s3_path: String,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetVersionResponse {
    pub version: i32,
    pub dataset_id: Uuid,
    pub s3_path: String,
    pub sample_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadUrlResponse {
    pub upload_url: String,
    pub s3_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadUrlResponse {
    pub download_url: String,
}
