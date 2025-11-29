mod common;

use common::*;
use uuid::Uuid;

#[cfg(test)]
mod s3_config_tests {
    use super::*;
    use llm_research_storage::s3::S3Config;

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();

        assert_eq!(config.bucket, "llm-research-artifacts");
        assert_eq!(config.region, "us-east-1");
        assert!(config.endpoint.is_none());
        assert!(config.access_key.is_none());
        assert!(config.secret_key.is_none());
    }

    #[test]
    fn test_s3_config_with_credentials() {
        let config = S3Config {
            endpoint: Some("http://localhost:9000".to_string()),
            bucket: "test-bucket".to_string(),
            region: "us-west-2".to_string(),
            access_key: Some("test-key".to_string()),
            secret_key: Some("test-secret".to_string()),
        };

        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.region, "us-west-2");
        assert!(config.access_key.is_some());
        assert!(config.secret_key.is_some());
    }

    #[test]
    fn test_s3_config_serialization() {
        let config = S3Config::default();

        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.is_empty());

        let deserialized: S3Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.bucket, config.bucket);
        assert_eq!(deserialized.region, config.region);
    }
}

#[cfg(test)]
mod artifact_path_tests {
    use super::*;

    #[test]
    fn test_artifact_path_generation() {
        let experiment_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let artifact_name = "model.bin";

        let path = create_artifact_path(&experiment_id, &run_id, artifact_name);

        assert!(path.contains("experiments"));
        assert!(path.contains(&experiment_id.to_string()));
        assert!(path.contains("runs"));
        assert!(path.contains(&run_id.to_string()));
        assert!(path.contains("artifacts"));
        assert!(path.ends_with(artifact_name));
    }

    #[test]
    fn test_artifact_path_with_subdirectory() {
        let experiment_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let artifact_name = "checkpoints/model-epoch-10.bin";

        let path = create_artifact_path(&experiment_id, &run_id, artifact_name);

        assert!(path.contains("checkpoints"));
        assert!(path.ends_with("model-epoch-10.bin"));
    }

    #[test]
    fn test_dataset_path_generation() {
        let dataset_id = Uuid::new_v4();
        let path = create_dataset_path(&dataset_id);

        assert!(path.contains("datasets"));
        assert!(path.contains(&dataset_id.to_string()));
        assert!(path.ends_with(".parquet"));
    }

    #[test]
    fn test_path_normalization() {
        let experiment_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        // Test with various artifact names
        let names = vec![
            "model.bin",
            "data/train.csv",
            "logs/output.txt",
            "metrics.json",
        ];

        for name in names {
            let path = create_artifact_path(&experiment_id, &run_id, name);
            // Path should not have double slashes
            assert!(!path.contains("//"));
            // Path should not start with slash (for S3)
            assert!(!path.starts_with('/'));
        }
    }
}

#[cfg(test)]
mod content_hash_tests {
    use super::*;

    #[test]
    fn test_content_hash_calculation() {
        let data = b"test content";
        let hash = calculate_content_hash(data);

        // SHA256 produces 64 hex characters
        assert_eq!(hash.len(), 64);

        // Hash should be deterministic
        let hash2 = calculate_content_hash(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_content_hash_different_data() {
        let data1 = b"content 1";
        let data2 = b"content 2";

        let hash1 = calculate_content_hash(data1);
        let hash2 = calculate_content_hash(data2);

        // Different data should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_empty_data() {
        let data = b"";
        let hash = calculate_content_hash(data);

        // Empty data should still produce a valid hash
        assert_eq!(hash.len(), 64);

        // SHA256 of empty string
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_content_hash_large_data() {
        let data = generate_test_data(1024 * 1024); // 1 MB
        let hash = calculate_content_hash(&data);

        assert_eq!(hash.len(), 64);

        // Same data should produce same hash
        let hash2 = calculate_content_hash(&data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_content_hash_hex_format() {
        let data = b"test";
        let hash = calculate_content_hash(data);

        // All characters should be valid hex (0-9, a-f)
        for c in hash.chars() {
            assert!(c.is_ascii_hexdigit());
        }
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct ArtifactMetadata {
        content_type: String,
        size_bytes: u64,
        hash: String,
        uploaded_at: String,
        tags: Vec<String>,
    }

    #[test]
    fn test_artifact_metadata_creation() {
        let metadata = ArtifactMetadata {
            content_type: "application/octet-stream".to_string(),
            size_bytes: 1024,
            hash: "abc123".to_string(),
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            tags: vec!["model".to_string(), "checkpoint".to_string()],
        };

        assert_eq!(metadata.content_type, "application/octet-stream");
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.tags.len(), 2);
    }

    #[test]
    fn test_artifact_metadata_serialization() {
        let metadata = ArtifactMetadata {
            content_type: "text/plain".to_string(),
            size_bytes: 512,
            hash: "hash123".to_string(),
            uploaded_at: "2024-01-01T00:00:00Z".to_string(),
            tags: vec!["log".to_string()],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(!json.is_empty());

        let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, metadata);
    }

    #[test]
    fn test_content_type_detection() {
        let extensions = vec![
            ("model.bin", "application/octet-stream"),
            ("data.json", "application/json"),
            ("log.txt", "text/plain"),
            ("data.csv", "text/csv"),
            ("image.png", "image/png"),
        ];

        for (filename, expected_type) in extensions {
            let detected_type = match filename.split('.').last() {
                Some("json") => "application/json",
                Some("txt") => "text/plain",
                Some("csv") => "text/csv",
                Some("png") => "image/png",
                _ => "application/octet-stream",
            };

            assert_eq!(detected_type, expected_type);
        }
    }
}

#[cfg(test)]
mod presigned_url_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_presigned_url_duration() {
        // Standard duration: 1 hour
        let duration = Duration::from_secs(3600);
        assert_eq!(duration.as_secs(), 3600);
    }

    #[test]
    fn test_presigned_url_custom_duration() {
        // Custom durations
        let one_hour = Duration::from_secs(3600);
        let six_hours = Duration::from_secs(6 * 3600);
        let one_day = Duration::from_secs(24 * 3600);

        assert_eq!(one_hour.as_secs(), 3600);
        assert_eq!(six_hours.as_secs(), 21600);
        assert_eq!(one_day.as_secs(), 86400);
    }

    #[test]
    fn test_presigned_url_max_duration() {
        // AWS S3 max presigned URL duration is 7 days
        let max_duration = Duration::from_secs(7 * 24 * 3600);
        assert_eq!(max_duration.as_secs(), 604800);
    }

    #[test]
    fn test_url_path_encoding() {
        // Test that special characters would need encoding
        let paths = vec![
            "simple.txt",
            "path with spaces.txt",
            "path/with/slashes.txt",
            "special-chars_file.bin",
        ];

        for path in paths {
            // Simple validation that path is not empty
            assert!(!path.is_empty());

            // Paths with spaces would need URL encoding
            if path.contains(' ') {
                let encoded = path.replace(' ', "%20");
                assert!(encoded.contains("%20"));
            }
        }
    }
}

#[cfg(test)]
mod s3_key_validation_tests {
    use super::*;

    fn is_valid_s3_key(key: &str) -> bool {
        // S3 key validation rules:
        // - Not empty
        // - Max 1024 bytes
        // - Should not start with /
        !key.is_empty() && key.len() <= 1024 && !key.starts_with('/')
    }

    #[test]
    fn test_valid_s3_keys() {
        let valid_keys = vec![
            "simple.txt",
            "path/to/file.bin",
            "experiments/123/runs/456/artifact.json",
            "datasets/uuid/data.parquet",
        ];

        for key in valid_keys {
            assert!(is_valid_s3_key(key), "Key should be valid: {}", key);
        }
    }

    #[test]
    fn test_invalid_s3_keys() {
        let invalid_keys = vec![
            "",           // Empty
            "/leading/slash.txt", // Starts with /
        ];

        for key in invalid_keys {
            assert!(!is_valid_s3_key(key), "Key should be invalid: {}", key);
        }
    }

    #[test]
    fn test_s3_key_length_limit() {
        let short_key = "a".repeat(1024);
        assert!(is_valid_s3_key(&short_key));

        let long_key = "a".repeat(1025);
        assert!(!is_valid_s3_key(&long_key));
    }

    #[test]
    fn test_s3_key_special_characters() {
        // S3 allows most characters, but some require encoding
        let keys_with_special_chars = vec![
            "file-with-dashes.txt",
            "file_with_underscores.txt",
            "file.with.dots.txt",
            "file(with)parens.txt",
        ];

        for key in keys_with_special_chars {
            assert!(is_valid_s3_key(key));
        }
    }
}

#[cfg(test)]
mod artifact_listing_tests {
    use super::*;

    #[test]
    fn test_prefix_filtering() {
        let all_keys = vec![
            "experiments/exp1/runs/run1/artifact1.bin",
            "experiments/exp1/runs/run2/artifact2.bin",
            "experiments/exp2/runs/run1/artifact3.bin",
            "datasets/dataset1/data.parquet",
        ];

        let prefix = "experiments/exp1/";
        let filtered: Vec<_> = all_keys
            .iter()
            .filter(|key| key.starts_with(prefix))
            .collect();

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_prefix_matching_hierarchy() {
        let keys = vec![
            "experiments/exp1/runs/run1/model.bin",
            "experiments/exp1/runs/run1/logs/output.txt",
            "experiments/exp1/runs/run2/model.bin",
        ];

        // Prefix for specific run
        let run1_prefix = "experiments/exp1/runs/run1/";
        let run1_keys: Vec<_> = keys
            .iter()
            .filter(|key| key.starts_with(run1_prefix))
            .collect();

        assert_eq!(run1_keys.len(), 2);

        // Prefix for logs
        let logs_prefix = "experiments/exp1/runs/run1/logs/";
        let log_keys: Vec<_> = keys
            .iter()
            .filter(|key| key.starts_with(logs_prefix))
            .collect();

        assert_eq!(log_keys.len(), 1);
    }

    #[test]
    fn test_empty_prefix_returns_all() {
        let keys = vec!["key1", "key2", "key3"];

        let prefix = "";
        let filtered: Vec<_> = keys
            .iter()
            .filter(|key| key.starts_with(prefix))
            .collect();

        assert_eq!(filtered.len(), keys.len());
    }
}

#[cfg(test)]
mod data_generation_tests {
    use super::*;

    #[test]
    fn test_generate_test_data_size() {
        let sizes = vec![0, 1, 100, 1024, 10240];

        for size in sizes {
            let data = generate_test_data(size);
            assert_eq!(data.len(), size);
        }
    }

    #[test]
    fn test_generate_test_data_randomness() {
        let data1 = generate_test_data(100);
        let data2 = generate_test_data(100);

        // Different calls should produce different data (with very high probability)
        // Note: This could theoretically fail, but probability is extremely low
        let all_same = data1.iter().zip(data2.iter()).all(|(a, b)| a == b);
        assert!(!all_same, "Generated data should be random");
    }

    #[test]
    fn test_generate_test_data_empty() {
        let data = generate_test_data(0);
        assert_eq!(data.len(), 0);
        assert!(data.is_empty());
    }
}

#[cfg(test)]
mod artifact_size_tests {
    use super::*;

    #[test]
    fn test_size_calculations() {
        let data_1kb = generate_test_data(1024);
        assert_eq!(data_1kb.len(), 1024);

        let data_1mb = generate_test_data(1024 * 1024);
        assert_eq!(data_1mb.len(), 1024 * 1024);
    }

    #[test]
    fn test_size_formatting() {
        fn format_size(bytes: u64) -> String {
            const KB: u64 = 1024;
            const MB: u64 = KB * 1024;
            const GB: u64 = MB * 1024;

            if bytes >= GB {
                format!("{:.2} GB", bytes as f64 / GB as f64)
            } else if bytes >= MB {
                format!("{:.2} MB", bytes as f64 / MB as f64)
            } else if bytes >= KB {
                format!("{:.2} KB", bytes as f64 / KB as f64)
            } else {
                format!("{} B", bytes)
            }
        }

        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1536), "1.50 KB");
    }
}

#[cfg(test)]
mod multipart_upload_tests {
    use super::*;

    const MIN_PART_SIZE: usize = 5 * 1024 * 1024; // 5 MB

    #[test]
    fn test_should_use_multipart() {
        fn should_use_multipart(size: usize) -> bool {
            size > MIN_PART_SIZE
        }

        assert!(!should_use_multipart(1024)); // 1 KB
        assert!(!should_use_multipart(1024 * 1024)); // 1 MB
        assert!(!should_use_multipart(MIN_PART_SIZE)); // 5 MB exactly
        assert!(should_use_multipart(MIN_PART_SIZE + 1)); // 5 MB + 1 byte
        assert!(should_use_multipart(100 * 1024 * 1024)); // 100 MB
    }

    #[test]
    fn test_calculate_part_count() {
        fn calculate_parts(total_size: usize, part_size: usize) -> usize {
            (total_size + part_size - 1) / part_size
        }

        assert_eq!(calculate_parts(10 * 1024 * 1024, MIN_PART_SIZE), 2);
        assert_eq!(calculate_parts(5 * 1024 * 1024, MIN_PART_SIZE), 1);
        assert_eq!(calculate_parts(15 * 1024 * 1024, MIN_PART_SIZE), 3);
    }

    #[test]
    fn test_part_ranges() {
        fn get_part_range(part_number: usize, part_size: usize, total_size: usize) -> (usize, usize) {
            let start = part_number * part_size;
            let end = std::cmp::min(start + part_size, total_size);
            (start, end)
        }

        let total_size = 15 * 1024 * 1024;
        let part_size = 5 * 1024 * 1024;

        assert_eq!(get_part_range(0, part_size, total_size), (0, 5 * 1024 * 1024));
        assert_eq!(get_part_range(1, part_size, total_size), (5 * 1024 * 1024, 10 * 1024 * 1024));
        assert_eq!(get_part_range(2, part_size, total_size), (10 * 1024 * 1024, 15 * 1024 * 1024));
    }
}
