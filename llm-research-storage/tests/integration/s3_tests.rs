//! S3 integration tests
//!
//! These tests require LocalStack or MinIO and are run with:
//! ```sh
//! cargo test --test integration_tests --features integration-tests
//! ```

#![cfg(feature = "integration-tests")]

use aws_sdk_s3::Client as S3Client;
use uuid::Uuid;

/// Create a test S3 client pointing to LocalStack
async fn create_test_s3_client() -> S3Client {
    use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url("http://localhost:4566") // LocalStack default
        .credentials_provider(Credentials::new(
            "test",
            "test",
            None,
            None,
            "test",
        ))
        .force_path_style(true)
        .build();

    S3Client::from_conf(config)
}

/// Create a unique test bucket name
fn unique_bucket_name() -> String {
    format!(
        "test-bucket-{}",
        Uuid::new_v4().to_string().split('-').next().unwrap()
    )
}

#[tokio::test]
#[ignore = "Requires LocalStack or MinIO"]
async fn test_s3_bucket_creation() {
    let client = create_test_s3_client().await;
    let bucket_name = unique_bucket_name();

    // Create bucket
    client
        .create_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to create bucket");

    // List buckets
    let buckets = client
        .list_buckets()
        .send()
        .await
        .expect("Failed to list buckets");

    let bucket_names: Vec<String> = buckets
        .buckets()
        .iter()
        .filter_map(|b| b.name().map(|s| s.to_string()))
        .collect();

    assert!(bucket_names.contains(&bucket_name));

    // Cleanup
    client
        .delete_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to delete bucket");
}

#[tokio::test]
#[ignore = "Requires LocalStack or MinIO"]
async fn test_s3_object_upload_download() {
    let client = create_test_s3_client().await;
    let bucket_name = unique_bucket_name();
    let object_key = format!("test-artifact-{}.json", Uuid::new_v4());

    // Create bucket
    client
        .create_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to create bucket");

    // Upload object
    let content = b"{ \"test\": \"data\" }";
    client
        .put_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .body(content.to_vec().into())
        .content_type("application/json")
        .send()
        .await
        .expect("Failed to upload object");

    // Download object
    let response = client
        .get_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .send()
        .await
        .expect("Failed to download object");

    let body = response
        .body
        .collect()
        .await
        .expect("Failed to read body")
        .into_bytes();

    assert_eq!(body.as_ref(), content);

    // Cleanup
    client
        .delete_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .send()
        .await
        .expect("Failed to delete object");

    client
        .delete_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to delete bucket");
}

#[tokio::test]
#[ignore = "Requires LocalStack or MinIO"]
async fn test_s3_object_listing() {
    let client = create_test_s3_client().await;
    let bucket_name = unique_bucket_name();

    // Create bucket
    client
        .create_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to create bucket");

    // Upload multiple objects
    let experiment_id = Uuid::new_v4();
    for i in 0..5 {
        let key = format!("experiments/{}/artifacts/file-{}.json", experiment_id, i);
        client
            .put_object()
            .bucket(&bucket_name)
            .key(&key)
            .body(b"test content".to_vec().into())
            .send()
            .await
            .expect("Failed to upload object");
    }

    // List objects with prefix
    let response = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(format!("experiments/{}/artifacts/", experiment_id))
        .send()
        .await
        .expect("Failed to list objects");

    let contents = response.contents();
    assert_eq!(contents.len(), 5);

    // Cleanup
    for obj in contents {
        if let Some(key) = obj.key() {
            client
                .delete_object()
                .bucket(&bucket_name)
                .key(key)
                .send()
                .await
                .expect("Failed to delete object");
        }
    }

    client
        .delete_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to delete bucket");
}

#[tokio::test]
#[ignore = "Requires LocalStack or MinIO"]
async fn test_s3_presigned_url() {
    use aws_sdk_s3::presigning::PresigningConfig;
    use std::time::Duration;

    let client = create_test_s3_client().await;
    let bucket_name = unique_bucket_name();
    let object_key = format!("test-presigned-{}.json", Uuid::new_v4());

    // Create bucket
    client
        .create_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to create bucket");

    // Upload object
    client
        .put_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .body(b"test content".to_vec().into())
        .send()
        .await
        .expect("Failed to upload object");

    // Generate presigned URL
    let presigning_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(3600))
        .build()
        .expect("Failed to create presigning config");

    let presigned = client
        .get_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .presigned(presigning_config)
        .await
        .expect("Failed to generate presigned URL");

    let url = presigned.uri();
    assert!(url.contains(&bucket_name));
    assert!(url.contains(&object_key));

    // Cleanup
    client
        .delete_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .send()
        .await
        .expect("Failed to delete object");

    client
        .delete_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .expect("Failed to delete bucket");
}
