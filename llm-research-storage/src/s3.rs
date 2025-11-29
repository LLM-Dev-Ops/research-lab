use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{
    config::Credentials,
    presigning::PresigningConfig,
    Client,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub bucket: String,
    pub region: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: None,
            bucket: "llm-research-artifacts".to_string(),
            region: "us-east-1".to_string(),
            access_key: None,
            secret_key: None,
        }
    }
}

/// Create an S3 client using default AWS configuration
pub async fn create_client() -> Result<Client> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    tracing::info!("S3 client created with default configuration");
    Ok(client)
}

/// Create an S3 client with custom configuration
pub async fn create_client_with_config(config: &S3Config) -> Result<Client> {
    let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(config.region.clone()));

    // Set credentials if provided
    if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "s3-config",
        );
        aws_config_builder = aws_config_builder.credentials_provider(credentials);
    }

    let aws_config = aws_config_builder.load().await;
    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config);

    // Set custom endpoint if provided (useful for MinIO, LocalStack, etc.)
    if let Some(endpoint) = &config.endpoint {
        s3_config_builder = s3_config_builder
            .endpoint_url(endpoint)
            .force_path_style(true);
    }

    let client = Client::from_conf(s3_config_builder.build());

    tracing::info!(
        "S3 client created with custom configuration: bucket={}, region={}",
        config.bucket,
        config.region
    );
    Ok(client)
}

/// Storage abstraction for S3 operations
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await?;

        tracing::info!("Uploaded object to S3: {}", key);
        Ok(())
    }

    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let data = resp.body.collect().await?.into_bytes().to_vec();
        tracing::info!("Downloaded object from S3: {}", key);
        Ok(data)
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        tracing::info!("Deleted object from S3: {}", key);
        Ok(())
    }

    pub async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;

        let keys = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        Ok(keys)
    }
}

/// Artifact storage with presigned URL support
pub struct ArtifactStorage {
    client: Client,
    bucket: String,
}

impl ArtifactStorage {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    /// Upload artifact data
    pub async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()> {
        let mut request = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into());

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        request.send().await?;

        tracing::info!("Uploaded artifact to S3: {}", key);
        Ok(())
    }

    /// Download artifact data
    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let data = resp.body.collect().await?.into_bytes().to_vec();
        tracing::info!("Downloaded artifact from S3: {}", key);
        Ok(data)
    }

    /// Delete artifact
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        tracing::info!("Deleted artifact from S3: {}", key);
        Ok(())
    }

    /// Generate presigned URL for uploading (expires in 1 hour)
    pub async fn get_presigned_upload_url(&self, key: &str) -> Result<String> {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

        let presigned_request = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await?;

        tracing::debug!("Generated presigned upload URL for: {}", key);
        Ok(presigned_request.uri().to_string())
    }

    /// Generate presigned URL for downloading (expires in 1 hour)
    pub async fn get_presigned_download_url(&self, key: &str) -> Result<String> {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

        let presigned_request = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await?;

        tracing::debug!("Generated presigned download URL for: {}", key);
        Ok(presigned_request.uri().to_string())
    }

    /// List artifacts with a given prefix
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;

        let keys: Vec<String> = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        tracing::debug!("Listed {} artifacts with prefix: {}", keys.len(), prefix);
        Ok(keys)
    }

    /// Check if an object exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
