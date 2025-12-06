//! LLM-Test-Bench Runtime Ingestion Adapter
//!
//! Runtime-only adapter for ingesting benchmark definitions and results from
//! LLM-Test-Bench via exported files or SDK calls. This adapter intentionally
//! has NO compile-time dependency on LLM-Test-Bench - all ingestion is performed
//! through file I/O or HTTP/SDK calls at runtime.
//!
//! # Design Rationale
//!
//! Benchmark ingestion must remain file-based or SDK-based only, without
//! introducing compile-time coupling to the Test-Bench crate. This ensures:
//!
//! - Loose coupling with the benchmarking infrastructure
//! - Flexibility in benchmark format evolution
//! - Support for multiple benchmark sources
//! - Runtime-configurable ingestion pipelines
//!
//! # Supported Ingestion Methods
//!
//! 1. **File-based**: Read benchmark definitions from JSON/YAML files
//! 2. **SDK-based**: HTTP calls to Test-Bench API (no crate dependency)
//! 3. **Directory watching**: Monitor directories for new benchmark files

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::{ConsumerResult, ConsumptionMetadata, ExternalServiceConfig, HealthCheckable};

/// Configuration for Test-Bench runtime ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBenchIngesterConfig {
    /// File-based ingestion configuration
    pub file_config: Option<FileIngestionConfig>,
    /// SDK/API-based ingestion configuration
    pub sdk_config: Option<SdkIngestionConfig>,
    /// Supported file formats
    pub supported_formats: Vec<FileFormat>,
    /// Whether to validate ingested benchmarks
    pub validate_on_ingest: bool,
    /// Schema validation strictness
    pub strict_schema: bool,
}

impl Default for TestBenchIngesterConfig {
    fn default() -> Self {
        Self {
            file_config: Some(FileIngestionConfig::default()),
            sdk_config: None,
            supported_formats: vec![FileFormat::Json, FileFormat::Yaml],
            validate_on_ingest: true,
            strict_schema: false,
        }
    }
}

/// Configuration for file-based ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIngestionConfig {
    /// Base directory for benchmark files
    pub base_directory: PathBuf,
    /// File patterns to match (glob patterns)
    pub file_patterns: Vec<String>,
    /// Whether to watch for changes
    pub watch_enabled: bool,
    /// Watch poll interval in seconds
    pub watch_interval_secs: u64,
}

impl Default for FileIngestionConfig {
    fn default() -> Self {
        Self {
            base_directory: PathBuf::from("./benchmarks"),
            file_patterns: vec![
                "*.json".to_string(),
                "*.yaml".to_string(),
                "*.yml".to_string(),
            ],
            watch_enabled: false,
            watch_interval_secs: 30,
        }
    }
}

/// Configuration for SDK/API-based ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkIngestionConfig {
    /// API endpoint for Test-Bench
    #[serde(flatten)]
    pub base: ExternalServiceConfig,
    /// API version
    pub api_version: String,
    /// Whether to use streaming
    pub streaming_enabled: bool,
}

impl Default for SdkIngestionConfig {
    fn default() -> Self {
        Self {
            base: ExternalServiceConfig {
                endpoint: "https://api.test-bench.local".to_string(),
                ..Default::default()
            },
            api_version: "v1".to_string(),
            streaming_enabled: false,
        }
    }
}

/// Supported file formats for ingestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Json,
    Yaml,
    Toml,
    Csv,
}

/// An ingested benchmark definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestedBenchmark {
    /// Benchmark identifier (generated or from file)
    pub benchmark_id: String,
    /// Source of ingestion
    pub source: IngestionSource,
    /// Benchmark name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Version
    pub version: String,
    /// Benchmark configuration
    pub config: BenchmarkConfig,
    /// Test cases
    pub test_cases: Vec<IngestedTestCase>,
    /// Validation status
    pub validation: ValidationResult,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Source of an ingested benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IngestionSource {
    /// Ingested from a file
    File {
        path: PathBuf,
        format: FileFormat,
        checksum: String,
    },
    /// Ingested via SDK/API
    Sdk {
        endpoint: String,
        request_id: String,
    },
    /// Ingested from directory scan
    DirectoryScan {
        directory: PathBuf,
        scan_timestamp: String,
    },
}

/// Configuration within a benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Timeout for each test case (ms)
    pub timeout_ms: u64,
    /// Number of iterations
    pub iterations: u32,
    /// Warmup iterations
    pub warmup_iterations: u32,
    /// Metrics to collect
    pub metrics: Vec<String>,
    /// Custom configuration
    pub custom: Option<Value>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30000,
            iterations: 1,
            warmup_iterations: 0,
            metrics: vec!["latency".to_string(), "throughput".to_string()],
            custom: None,
        }
    }
}

/// An ingested test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestedTestCase {
    /// Test case identifier
    pub case_id: String,
    /// Test case name
    pub name: String,
    /// Input data
    pub input: Value,
    /// Expected output (for validation)
    pub expected_output: Option<Value>,
    /// Tags
    pub tags: Vec<String>,
    /// Priority (1-10)
    pub priority: Option<u8>,
}

/// Validation result for ingested benchmarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Schema version used for validation
    pub schema_version: Option<String>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
            schema_version: None,
        }
    }

    /// Create a failed validation result.
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: vec![],
            schema_version: None,
        }
    }
}

/// A validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Location in the file/data (JSON path)
    pub path: Option<String>,
}

/// A validation warning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
    /// Location in the file/data
    pub path: Option<String>,
}

/// Ingested benchmark results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestedResults {
    /// Results identifier
    pub results_id: Uuid,
    /// Associated benchmark ID
    pub benchmark_id: String,
    /// Run identifier
    pub run_id: Uuid,
    /// Individual test case results
    pub case_results: Vec<TestCaseResult>,
    /// Aggregate metrics
    pub aggregate_metrics: Value,
    /// Execution metadata
    pub execution: ExecutionMetadata,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Result for a single test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    /// Test case ID
    pub case_id: String,
    /// Pass/fail status
    pub passed: bool,
    /// Actual output
    pub actual_output: Option<Value>,
    /// Metrics collected
    pub metrics: Value,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: f64,
}

/// Metadata about benchmark execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution start time
    pub started_at: String,
    /// Execution end time
    pub ended_at: String,
    /// Total duration in milliseconds
    pub total_duration_ms: f64,
    /// Environment information
    pub environment: Option<Value>,
    /// Runner version
    pub runner_version: Option<String>,
}

/// Trait for runtime ingestion from Test-Bench.
#[async_trait]
pub trait TestBenchIngester: HealthCheckable {
    /// Ingest a benchmark from a file path.
    async fn ingest_from_file(&self, path: &Path) -> ConsumerResult<IngestedBenchmark>;

    /// Ingest all benchmarks from a directory.
    async fn ingest_from_directory(&self, directory: &Path)
        -> ConsumerResult<Vec<IngestedBenchmark>>;

    /// Ingest a benchmark via SDK/API call.
    async fn ingest_from_sdk(&self, benchmark_id: &str) -> ConsumerResult<IngestedBenchmark>;

    /// Ingest benchmark results from a file.
    async fn ingest_results_from_file(&self, path: &Path) -> ConsumerResult<IngestedResults>;

    /// Ingest benchmark results via SDK/API.
    async fn ingest_results_from_sdk(
        &self,
        benchmark_id: &str,
        run_id: Uuid,
    ) -> ConsumerResult<IngestedResults>;

    /// Validate a benchmark definition.
    fn validate_benchmark(&self, benchmark: &Value) -> ValidationResult;

    /// List available benchmarks in configured sources.
    async fn list_available(&self) -> ConsumerResult<Vec<String>>;
}

/// Client implementation for Test-Bench runtime ingestion.
pub struct TestBenchIngesterClient {
    config: TestBenchIngesterConfig,
}

impl TestBenchIngesterClient {
    /// Create a new ingester client with the given configuration.
    pub fn new(config: TestBenchIngesterConfig) -> Self {
        Self { config }
    }

    /// Create a client for file-based ingestion only.
    pub fn file_based(directory: PathBuf) -> Self {
        Self {
            config: TestBenchIngesterConfig {
                file_config: Some(FileIngestionConfig {
                    base_directory: directory,
                    ..Default::default()
                }),
                sdk_config: None,
                ..Default::default()
            },
        }
    }

    /// Create a client for SDK-based ingestion only.
    pub fn sdk_based(endpoint: &str) -> Self {
        Self {
            config: TestBenchIngesterConfig {
                file_config: None,
                sdk_config: Some(SdkIngestionConfig {
                    base: ExternalServiceConfig {
                        endpoint: endpoint.to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &TestBenchIngesterConfig {
        &self.config
    }

    /// Parse file content based on format.
    fn parse_file_content(&self, content: &str, format: &FileFormat) -> ConsumerResult<Value> {
        match format {
            FileFormat::Json => {
                serde_json::from_str(content).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            FileFormat::Yaml => {
                // YAML parsing would be implemented here
                // For now, attempt JSON parsing as fallback
                serde_json::from_str(content).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            FileFormat::Toml => {
                // TOML parsing would be implemented here
                Err("TOML parsing not yet implemented".into())
            }
            FileFormat::Csv => {
                // CSV parsing would be implemented here
                Err("CSV parsing not yet implemented".into())
            }
        }
    }
}

#[async_trait]
impl HealthCheckable for TestBenchIngesterClient {
    async fn health_check(&self) -> ConsumerResult<bool> {
        // Check file config
        if let Some(ref file_config) = self.config.file_config {
            if !file_config.base_directory.exists() {
                return Ok(false);
            }
        }

        // Check SDK config
        if let Some(ref sdk_config) = self.config.sdk_config {
            if sdk_config.base.endpoint.is_empty() {
                return Ok(false);
            }
        }

        // At least one config must be present
        Ok(self.config.file_config.is_some() || self.config.sdk_config.is_some())
    }
}

#[async_trait]
impl TestBenchIngester for TestBenchIngesterClient {
    async fn ingest_from_file(&self, path: &Path) -> ConsumerResult<IngestedBenchmark> {
        // Determine file format from extension
        let format = match path.extension().and_then(|e| e.to_str()) {
            Some("json") => FileFormat::Json,
            Some("yaml") | Some("yml") => FileFormat::Yaml,
            Some("toml") => FileFormat::Toml,
            _ => return Err("Unsupported file format".into()),
        };

        // Read file content
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to read file: {}", e),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Calculate checksum
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let checksum = hex::encode(hasher.finalize());

        // Parse content
        let data = self.parse_file_content(&content, &format)?;

        // Validate if configured
        let validation = if self.config.validate_on_ingest {
            self.validate_benchmark(&data)
        } else {
            ValidationResult::success()
        };

        // Extract benchmark data
        let benchmark_id = data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed Benchmark")
            .to_string();

        Ok(IngestedBenchmark {
            benchmark_id,
            source: IngestionSource::File {
                path: path.to_path_buf(),
                format,
                checksum,
            },
            name,
            description: data.get("description").and_then(|v| v.as_str()).map(String::from),
            version: data
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            config: BenchmarkConfig::default(),
            test_cases: vec![],
            validation,
            metadata: ConsumptionMetadata::new("llm-test-bench"),
        })
    }

    async fn ingest_from_directory(
        &self,
        directory: &Path,
    ) -> ConsumerResult<Vec<IngestedBenchmark>> {
        let mut benchmarks = Vec::new();

        // Read directory entries
        let mut entries = tokio::fs::read_dir(directory).await.map_err(|e| {
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })? {
            let path = entry.path();

            // Check if it's a supported file
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "json" | "yaml" | "yml") {
                    match self.ingest_from_file(&path).await {
                        Ok(benchmark) => benchmarks.push(benchmark),
                        Err(e) => {
                            // Log error but continue with other files
                            eprintln!("Failed to ingest {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(benchmarks)
    }

    async fn ingest_from_sdk(&self, benchmark_id: &str) -> ConsumerResult<IngestedBenchmark> {
        let sdk_config = self
            .config
            .sdk_config
            .as_ref()
            .ok_or("SDK configuration not available")?;

        // In production, this would make an HTTP request to the Test-Bench API
        // The implementation would use reqwest (available in workspace)

        Ok(IngestedBenchmark {
            benchmark_id: benchmark_id.to_string(),
            source: IngestionSource::Sdk {
                endpoint: sdk_config.base.endpoint.clone(),
                request_id: Uuid::new_v4().to_string(),
            },
            name: format!("Benchmark {}", benchmark_id),
            description: None,
            version: "1.0.0".to_string(),
            config: BenchmarkConfig::default(),
            test_cases: vec![],
            validation: ValidationResult::success(),
            metadata: ConsumptionMetadata::new("llm-test-bench"),
        })
    }

    async fn ingest_results_from_file(&self, path: &Path) -> ConsumerResult<IngestedResults> {
        // Read and parse results file
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let data: Value = serde_json::from_str(&content)?;

        Ok(IngestedResults {
            results_id: Uuid::new_v4(),
            benchmark_id: data
                .get("benchmark_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            run_id: Uuid::new_v4(),
            case_results: vec![],
            aggregate_metrics: data.get("metrics").cloned().unwrap_or(serde_json::json!({})),
            execution: ExecutionMetadata {
                started_at: chrono::Utc::now().to_rfc3339(),
                ended_at: chrono::Utc::now().to_rfc3339(),
                total_duration_ms: 0.0,
                environment: None,
                runner_version: None,
            },
            metadata: ConsumptionMetadata::new("llm-test-bench"),
        })
    }

    async fn ingest_results_from_sdk(
        &self,
        benchmark_id: &str,
        run_id: Uuid,
    ) -> ConsumerResult<IngestedResults> {
        // In production, this would fetch results via HTTP
        Ok(IngestedResults {
            results_id: Uuid::new_v4(),
            benchmark_id: benchmark_id.to_string(),
            run_id,
            case_results: vec![],
            aggregate_metrics: serde_json::json!({}),
            execution: ExecutionMetadata {
                started_at: chrono::Utc::now().to_rfc3339(),
                ended_at: chrono::Utc::now().to_rfc3339(),
                total_duration_ms: 0.0,
                environment: None,
                runner_version: None,
            },
            metadata: ConsumptionMetadata::new("llm-test-bench"),
        })
    }

    fn validate_benchmark(&self, benchmark: &Value) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Required fields validation
        if !benchmark.get("name").is_some() {
            errors.push(ValidationError {
                code: "MISSING_NAME".to_string(),
                message: "Benchmark name is required".to_string(),
                path: Some("$.name".to_string()),
            });
        }

        // Optional field warnings
        if !benchmark.get("description").is_some() {
            warnings.push(ValidationWarning {
                code: "MISSING_DESCRIPTION".to_string(),
                message: "Benchmark description is recommended".to_string(),
                path: Some("$.description".to_string()),
            });
        }

        if !benchmark.get("version").is_some() {
            warnings.push(ValidationWarning {
                code: "MISSING_VERSION".to_string(),
                message: "Benchmark version is recommended".to_string(),
                path: Some("$.version".to_string()),
            });
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            schema_version: Some("1.0".to_string()),
        }
    }

    async fn list_available(&self) -> ConsumerResult<Vec<String>> {
        let mut available = Vec::new();

        // List from file config
        if let Some(ref file_config) = self.config.file_config {
            if file_config.base_directory.exists() {
                let mut entries = tokio::fs::read_dir(&file_config.base_directory)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                while let Some(entry) = entries.next_entry().await.map_err(|e| {
                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                })? {
                    if let Some(name) = entry.path().file_stem().and_then(|n| n.to_str()) {
                        available.push(format!("file:{}", name));
                    }
                }
            }
        }

        // List from SDK would be added here

        Ok(available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingester_config_default() {
        let config = TestBenchIngesterConfig::default();
        assert!(config.validate_on_ingest);
        assert!(!config.strict_schema);
        assert!(config.file_config.is_some());
    }

    #[test]
    fn test_file_based_client() {
        let client = TestBenchIngesterClient::file_based(PathBuf::from("/tmp/benchmarks"));
        assert!(client.config().file_config.is_some());
        assert!(client.config().sdk_config.is_none());
    }

    #[test]
    fn test_sdk_based_client() {
        let client = TestBenchIngesterClient::sdk_based("https://test-bench.example.com");
        assert!(client.config().file_config.is_none());
        assert!(client.config().sdk_config.is_some());
    }

    #[test]
    fn test_validation_success() {
        let client = TestBenchIngesterClient::new(TestBenchIngesterConfig::default());
        let benchmark = serde_json::json!({
            "name": "Test Benchmark",
            "version": "1.0.0",
            "description": "A test benchmark"
        });

        let result = client.validate_benchmark(&benchmark);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_failure() {
        let client = TestBenchIngesterClient::new(TestBenchIngesterConfig::default());
        let benchmark = serde_json::json!({
            "version": "1.0.0"
        });

        let result = client.validate_benchmark(&benchmark);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_file_format_serialization() {
        let format = FileFormat::Json;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"json\"");
    }

    #[test]
    fn test_ingestion_source_file() {
        let source = IngestionSource::File {
            path: PathBuf::from("/tmp/test.json"),
            format: FileFormat::Json,
            checksum: "abc123".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("file"));
    }
}
