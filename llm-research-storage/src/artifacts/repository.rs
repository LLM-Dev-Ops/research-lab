use anyhow::Result;
use aws_sdk_s3::{
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::metadata::{ArtifactMetadata, detect_content_type};

/// Artifact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: Uuid,
    pub experiment_id: Uuid,
    pub run_id: Uuid,
    pub filename: String,
    pub s3_key: String,
    pub metadata: ArtifactMetadata,
}

impl Artifact {
    /// Create a new artifact
    pub fn new(
        experiment_id: Uuid,
        run_id: Uuid,
        filename: String,
        data: &[u8],
    ) -> Self {
        let artifact_id = Uuid::new_v4();
        let content_type = detect_content_type(&filename);
        let metadata = ArtifactMetadata::new(data, content_type);
        let s3_key = Self::build_s3_key(&experiment_id, &run_id, &artifact_id, &filename);

        Self {
            artifact_id,
            experiment_id,
            run_id,
            filename,
            s3_key,
            metadata,
        }
    }

    /// Build S3 key for an artifact
    pub fn build_s3_key(
        experiment_id: &Uuid,
        run_id: &Uuid,
        artifact_id: &Uuid,
        filename: &str,
    ) -> String {
        format!(
            "experiments/{}/runs/{}/artifacts/{}/{}",
            experiment_id, run_id, artifact_id, filename
        )
    }
}

/// Repository for managing artifacts in S3
pub struct ArtifactRepository {
    client: Client,
    bucket: String,
}

impl ArtifactRepository {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    /// Upload an artifact with metadata
    pub async fn upload_artifact(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        filename: String,
        data: Vec<u8>,
    ) -> Result<Artifact> {
        let artifact = Artifact::new(experiment_id, run_id, filename, &data);

        // Upload to S3 with metadata
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&artifact.s3_key)
            .body(ByteStream::from(data))
            .content_type(&artifact.metadata.content_type)
            .set_metadata(Some(artifact.metadata.to_s3_metadata()))
            .send()
            .await?;

        tracing::info!(
            "Uploaded artifact: id={}, key={}",
            artifact.artifact_id,
            artifact.s3_key
        );
        Ok(artifact)
    }

    /// Upload an artifact with custom metadata
    pub async fn upload_artifact_with_metadata(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        filename: String,
        data: Vec<u8>,
        custom_metadata: serde_json::Value,
    ) -> Result<Artifact> {
        let artifact_id = Uuid::new_v4();
        let content_type = detect_content_type(&filename);
        let metadata = ArtifactMetadata::with_custom(&data, content_type, custom_metadata);
        let s3_key = Artifact::build_s3_key(&experiment_id, &run_id, &artifact_id, &filename);

        let artifact = Artifact {
            artifact_id,
            experiment_id,
            run_id,
            filename,
            s3_key: s3_key.clone(),
            metadata: metadata.clone(),
        };

        // Upload to S3 with metadata
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .body(ByteStream::from(data))
            .content_type(&metadata.content_type)
            .set_metadata(Some(metadata.to_s3_metadata()))
            .send()
            .await?;

        tracing::info!(
            "Uploaded artifact with custom metadata: id={}, key={}",
            artifact.artifact_id,
            artifact.s3_key
        );
        Ok(artifact)
    }

    /// Download an artifact by its S3 key
    pub async fn download_artifact(&self, s3_key: &str) -> Result<Vec<u8>> {
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .send()
            .await?;

        let data = resp.body.collect().await?.into_bytes().to_vec();

        tracing::info!("Downloaded artifact: key={}", s3_key);
        Ok(data)
    }

    /// Download an artifact by ID
    pub async fn download_artifact_by_id(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        artifact_id: Uuid,
        filename: &str,
    ) -> Result<Vec<u8>> {
        let s3_key = Artifact::build_s3_key(&experiment_id, &run_id, &artifact_id, filename);
        self.download_artifact(&s3_key).await
    }

    /// Get artifact metadata without downloading the full artifact
    pub async fn get_artifact_metadata(&self, s3_key: &str) -> Result<ArtifactMetadata> {
        let resp = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .send()
            .await?;

        let metadata = resp.metadata().cloned().unwrap_or_default();
        ArtifactMetadata::from_s3_metadata(&metadata)
    }

    /// Delete an artifact
    pub async fn delete_artifact(&self, s3_key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .send()
            .await?;

        tracing::info!("Deleted artifact: key={}", s3_key);
        Ok(())
    }

    /// Delete an artifact by ID
    pub async fn delete_artifact_by_id(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        artifact_id: Uuid,
        filename: &str,
    ) -> Result<()> {
        let s3_key = Artifact::build_s3_key(&experiment_id, &run_id, &artifact_id, filename);
        self.delete_artifact(&s3_key).await
    }

    /// Generate a presigned URL for uploading an artifact (expires in 1 hour)
    pub async fn get_presigned_upload_url(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        artifact_id: Uuid,
        filename: &str,
    ) -> Result<String> {
        let s3_key = Artifact::build_s3_key(&experiment_id, &run_id, &artifact_id, filename);
        let content_type = detect_content_type(filename);
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

        let presigned_request = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .content_type(&content_type)
            .presigned(presigning_config)
            .await?;

        tracing::debug!("Generated presigned upload URL for: {}", s3_key);
        Ok(presigned_request.uri().to_string())
    }

    /// Generate a presigned URL for downloading an artifact (expires in 1 hour)
    pub async fn get_presigned_download_url(&self, s3_key: &str) -> Result<String> {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

        let presigned_request = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .presigned(presigning_config)
            .await?;

        tracing::debug!("Generated presigned download URL for: {}", s3_key);
        Ok(presigned_request.uri().to_string())
    }

    /// Generate a presigned download URL by artifact ID
    pub async fn get_presigned_download_url_by_id(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
        artifact_id: Uuid,
        filename: &str,
    ) -> Result<String> {
        let s3_key = Artifact::build_s3_key(&experiment_id, &run_id, &artifact_id, filename);
        self.get_presigned_download_url(&s3_key).await
    }

    /// List all artifacts for a specific run
    pub async fn list_artifacts(&self, experiment_id: Uuid, run_id: Uuid) -> Result<Vec<String>> {
        let prefix = format!("experiments/{}/runs/{}/artifacts/", experiment_id, run_id);

        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let keys = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        tracing::debug!(
            "Listed {} artifacts for experiment={}, run={}",
            resp.contents().len(),
            experiment_id,
            run_id
        );
        Ok(keys)
    }

    /// List all artifacts for an experiment
    pub async fn list_experiment_artifacts(&self, experiment_id: Uuid) -> Result<Vec<String>> {
        let prefix = format!("experiments/{}/runs/", experiment_id);

        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let keys = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        tracing::debug!(
            "Listed {} artifacts for experiment={}",
            resp.contents().len(),
            experiment_id
        );
        Ok(keys)
    }

    /// Check if an artifact exists
    pub async fn artifact_exists(&self, s3_key: &str) -> bool {
        self.client
            .head_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .send()
            .await
            .is_ok()
    }

    /// Delete all artifacts for a run
    pub async fn delete_run_artifacts(&self, experiment_id: Uuid, run_id: Uuid) -> Result<()> {
        let artifacts = self.list_artifacts(experiment_id, run_id).await?;

        for key in artifacts {
            self.delete_artifact(&key).await?;
        }

        tracing::info!(
            "Deleted all artifacts for experiment={}, run={}",
            experiment_id,
            run_id
        );
        Ok(())
    }

    /// Delete all artifacts for an experiment
    pub async fn delete_experiment_artifacts(&self, experiment_id: Uuid) -> Result<()> {
        let artifacts = self.list_experiment_artifacts(experiment_id).await?;

        for key in artifacts {
            self.delete_artifact(&key).await?;
        }

        tracing::info!(
            "Deleted all artifacts for experiment={}",
            experiment_id
        );
        Ok(())
    }

    /// Get total size of artifacts for a run
    pub async fn get_run_artifacts_size(&self, experiment_id: Uuid, run_id: Uuid) -> Result<u64> {
        let prefix = format!("experiments/{}/runs/{}/artifacts/", experiment_id, run_id);

        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let total_size: i64 = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.size())
            .sum();

        Ok(total_size as u64)
    }
}

/// Artifact listing entry with details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactListEntry {
    pub s3_key: String,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
    pub e_tag: Option<String>,
}

impl ArtifactRepository {
    /// List artifacts with detailed information
    pub async fn list_artifacts_detailed(
        &self,
        experiment_id: Uuid,
        run_id: Uuid,
    ) -> Result<Vec<ArtifactListEntry>> {
        let prefix = format!("experiments/{}/runs/{}/artifacts/", experiment_id, run_id);

        let resp = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let entries = resp
            .contents()
            .iter()
            .filter_map(|obj| {
                let key = obj.key()?.to_string();
                let size = obj.size().unwrap_or(0) as u64;
                let last_modified = obj.last_modified()?;

                // Convert AWS DateTime to chrono DateTime
                let last_modified_dt = {
                    let seconds = last_modified.secs();
                    let nanos = last_modified.subsec_nanos();
                    DateTime::<Utc>::from_timestamp(seconds, nanos)?
                };

                let e_tag = obj.e_tag().map(String::from);

                Some(ArtifactListEntry {
                    s3_key: key,
                    size,
                    last_modified: last_modified_dt,
                    e_tag,
                })
            })
            .collect();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_key_building() {
        let experiment_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let artifact_id = Uuid::new_v4();
        let filename = "model.pt";

        let key = Artifact::build_s3_key(&experiment_id, &run_id, &artifact_id, filename);

        assert!(key.contains(&experiment_id.to_string()));
        assert!(key.contains(&run_id.to_string()));
        assert!(key.contains(&artifact_id.to_string()));
        assert!(key.ends_with("model.pt"));
        assert!(key.starts_with("experiments/"));
    }

    #[test]
    fn test_artifact_creation() {
        let experiment_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let data = b"test model data";

        let artifact = Artifact::new(
            experiment_id,
            run_id,
            "model.pt".to_string(),
            data,
        );

        assert_eq!(artifact.experiment_id, experiment_id);
        assert_eq!(artifact.run_id, run_id);
        assert_eq!(artifact.filename, "model.pt");
        assert_eq!(artifact.metadata.size, 15);
    }
}
