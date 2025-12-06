//! LLM-Data-Vault Consumption Adapter
//!
//! Thin adapter for consuming stored datasets, anonymized corpora, and
//! lineage-aware research artifacts from the LLM-Data-Vault service.
//! This adapter does not modify any existing research logic or data processing.
//!
//! # Consumed Data Types
//!
//! - Stored datasets (versioned, immutable data snapshots)
//! - Anonymized corpora (privacy-preserving text collections)
//! - Lineage-aware artifacts (with full provenance tracking)
//! - Data access audit logs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ConsumerResult, ConsumptionMetadata, ExternalServiceConfig, HealthCheckable};

/// Configuration specific to LLM-Data-Vault consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataVaultConfig {
    /// Base service configuration
    #[serde(flatten)]
    pub base: ExternalServiceConfig,
    /// Vault namespace
    pub vault_namespace: Option<String>,
    /// Whether to include lineage information
    pub include_lineage: bool,
    /// Whether to verify data integrity
    pub verify_integrity: bool,
    /// Access purpose (for audit logging)
    pub access_purpose: Option<String>,
}

impl Default for DataVaultConfig {
    fn default() -> Self {
        Self {
            base: ExternalServiceConfig {
                endpoint: "https://api.data-vault.local".to_string(),
                ..Default::default()
            },
            vault_namespace: None,
            include_lineage: true,
            verify_integrity: true,
            access_purpose: None,
        }
    }
}

/// A stored dataset from Data-Vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDataset {
    /// Dataset identifier
    pub dataset_id: Uuid,
    /// Dataset name
    pub name: String,
    /// Version identifier
    pub version: String,
    /// Description
    pub description: String,
    /// Schema definition
    pub schema: DatasetSchema,
    /// Dataset statistics
    pub statistics: DatasetStatistics,
    /// Storage location/reference
    pub storage: DatasetStorage,
    /// Lineage information
    pub lineage: Option<DataLineage>,
    /// Access control level
    pub access_level: AccessLevel,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Schema definition for a dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSchema {
    /// Schema format (json-schema, avro, protobuf, etc.)
    pub format: String,
    /// Schema definition
    pub definition: Value,
    /// Field descriptions
    pub field_descriptions: Option<Value>,
}

/// Statistics about a dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStatistics {
    /// Number of records
    pub record_count: u64,
    /// Size in bytes
    pub size_bytes: u64,
    /// Column/field statistics
    pub field_stats: Option<Value>,
    /// Last computed timestamp
    pub computed_at: String,
}

/// Storage information for a dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStorage {
    /// Storage type (s3, gcs, azure-blob, local)
    pub storage_type: String,
    /// Storage path/URI
    pub path: String,
    /// File format
    pub format: DataFormat,
    /// Compression (if any)
    pub compression: Option<String>,
    /// Partition columns (if partitioned)
    pub partitions: Vec<String>,
}

/// Data file format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    Parquet,
    Csv,
    Json,
    Jsonl,
    Arrow,
    Avro,
    Custom(String),
}

/// Data lineage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLineage {
    /// Lineage graph ID
    pub lineage_id: Uuid,
    /// Source datasets
    pub sources: Vec<LineageSource>,
    /// Transformations applied
    pub transformations: Vec<LineageTransformation>,
    /// Derived datasets
    pub derived: Vec<Uuid>,
    /// Full lineage graph (for complex lineages)
    pub graph: Option<Value>,
}

/// Source in lineage graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSource {
    /// Source dataset ID
    pub dataset_id: Uuid,
    /// Source version
    pub version: String,
    /// Relationship type
    pub relationship: String,
}

/// Transformation in lineage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageTransformation {
    /// Transformation ID
    pub transform_id: String,
    /// Transformation type
    pub transform_type: String,
    /// Description
    pub description: String,
    /// Parameters used
    pub parameters: Option<Value>,
    /// Timestamp
    pub applied_at: String,
}

/// Access level for data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Public,
    Internal,
    Restricted,
    Confidential,
    Secret,
}

/// An anonymized corpus from Data-Vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedCorpus {
    /// Corpus identifier
    pub corpus_id: Uuid,
    /// Corpus name
    pub name: String,
    /// Description
    pub description: String,
    /// Anonymization configuration used
    pub anonymization_config: AnonymizationConfig,
    /// Sample count
    pub sample_count: u64,
    /// Storage reference
    pub storage: DatasetStorage,
    /// Original corpus reference (if linked)
    pub original_corpus_id: Option<Uuid>,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Configuration for anonymization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    /// Anonymization method
    pub method: AnonymizationMethod,
    /// Fields anonymized
    pub anonymized_fields: Vec<String>,
    /// Privacy parameters
    pub privacy_params: Option<Value>,
    /// Verification status
    pub verified: bool,
}

/// Method of anonymization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnonymizationMethod {
    Redaction,
    Pseudonymization,
    Generalization,
    Suppression,
    DifferentialPrivacy,
    KAnonymity,
    LDiversity,
    Custom(String),
}

/// A lineage-aware research artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchArtifact {
    /// Artifact identifier
    pub artifact_id: Uuid,
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// Name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Artifact content (or reference)
    pub content: ArtifactContent,
    /// Full lineage information
    pub lineage: DataLineage,
    /// Tags
    pub tags: Vec<String>,
    /// Consumption metadata
    pub metadata: ConsumptionMetadata,
}

/// Types of research artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Model,
    Dataset,
    Evaluation,
    Experiment,
    Report,
    Configuration,
    Checkpoint,
    Custom(String),
}

/// Artifact content - either inline or reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArtifactContent {
    /// Inline content for small artifacts
    Inline(Value),
    /// Reference to stored content
    Reference {
        storage_path: String,
        checksum: String,
        size_bytes: u64,
    },
}

/// Query parameters for Data-Vault consumption.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataVaultQuery {
    /// Filter by namespace
    pub namespace: Option<String>,
    /// Filter by access level
    pub access_level: Option<AccessLevel>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Include lineage
    pub include_lineage: bool,
    /// Version filter
    pub version: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
}

/// Trait for consuming data from LLM-Data-Vault.
#[async_trait]
pub trait DataVaultConsumer: HealthCheckable {
    /// Consume a stored dataset by ID.
    async fn consume_dataset(&self, dataset_id: Uuid) -> ConsumerResult<StoredDataset>;

    /// Consume a specific version of a dataset.
    async fn consume_dataset_version(
        &self,
        dataset_id: Uuid,
        version: &str,
    ) -> ConsumerResult<StoredDataset>;

    /// Search and consume datasets.
    async fn consume_datasets(&self, query: &DataVaultQuery)
        -> ConsumerResult<Vec<StoredDataset>>;

    /// Consume an anonymized corpus.
    async fn consume_anonymized_corpus(&self, corpus_id: Uuid)
        -> ConsumerResult<AnonymizedCorpus>;

    /// Consume a research artifact with lineage.
    async fn consume_artifact(&self, artifact_id: Uuid) -> ConsumerResult<ResearchArtifact>;

    /// Get lineage for a dataset.
    async fn get_dataset_lineage(&self, dataset_id: Uuid) -> ConsumerResult<DataLineage>;

    /// List available datasets.
    async fn list_available_datasets(&self, query: &DataVaultQuery)
        -> ConsumerResult<Vec<Uuid>>;

    /// Get dataset schema without full data.
    async fn get_dataset_schema(&self, dataset_id: Uuid) -> ConsumerResult<DatasetSchema>;
}

/// Client implementation for consuming from LLM-Data-Vault.
pub struct DataVaultClient {
    config: DataVaultConfig,
}

impl DataVaultClient {
    /// Create a new data vault client with the given configuration.
    pub fn new(config: DataVaultConfig) -> Self {
        Self { config }
    }

    /// Create a client with default configuration and custom endpoint.
    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            config: DataVaultConfig {
                base: ExternalServiceConfig {
                    endpoint: endpoint.to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &DataVaultConfig {
        &self.config
    }
}

#[async_trait]
impl HealthCheckable for DataVaultClient {
    async fn health_check(&self) -> ConsumerResult<bool> {
        Ok(!self.config.base.endpoint.is_empty())
    }
}

#[async_trait]
impl DataVaultConsumer for DataVaultClient {
    async fn consume_dataset(&self, dataset_id: Uuid) -> ConsumerResult<StoredDataset> {
        // Implementation would fetch dataset from Vault API
        Ok(StoredDataset {
            dataset_id,
            name: "Placeholder Dataset".to_string(),
            version: "1.0.0".to_string(),
            description: "Placeholder".to_string(),
            schema: DatasetSchema {
                format: "json-schema".to_string(),
                definition: serde_json::json!({}),
                field_descriptions: None,
            },
            statistics: DatasetStatistics {
                record_count: 0,
                size_bytes: 0,
                field_stats: None,
                computed_at: chrono::Utc::now().to_rfc3339(),
            },
            storage: DatasetStorage {
                storage_type: "s3".to_string(),
                path: "s3://placeholder/path".to_string(),
                format: DataFormat::Parquet,
                compression: Some("snappy".to_string()),
                partitions: vec![],
            },
            lineage: None,
            access_level: AccessLevel::Internal,
            metadata: ConsumptionMetadata::new("llm-data-vault"),
        })
    }

    async fn consume_dataset_version(
        &self,
        dataset_id: Uuid,
        version: &str,
    ) -> ConsumerResult<StoredDataset> {
        let mut dataset = self.consume_dataset(dataset_id).await?;
        dataset.version = version.to_string();
        Ok(dataset)
    }

    async fn consume_datasets(
        &self,
        _query: &DataVaultQuery,
    ) -> ConsumerResult<Vec<StoredDataset>> {
        // Implementation would query datasets with filters
        Ok(vec![])
    }

    async fn consume_anonymized_corpus(
        &self,
        corpus_id: Uuid,
    ) -> ConsumerResult<AnonymizedCorpus> {
        // Implementation would fetch anonymized corpus
        Ok(AnonymizedCorpus {
            corpus_id,
            name: "Placeholder Corpus".to_string(),
            description: "Placeholder".to_string(),
            anonymization_config: AnonymizationConfig {
                method: AnonymizationMethod::Pseudonymization,
                anonymized_fields: vec!["email".to_string(), "name".to_string()],
                privacy_params: None,
                verified: true,
            },
            sample_count: 0,
            storage: DatasetStorage {
                storage_type: "s3".to_string(),
                path: "s3://placeholder/corpus".to_string(),
                format: DataFormat::Jsonl,
                compression: Some("gzip".to_string()),
                partitions: vec![],
            },
            original_corpus_id: None,
            metadata: ConsumptionMetadata::new("llm-data-vault"),
        })
    }

    async fn consume_artifact(&self, artifact_id: Uuid) -> ConsumerResult<ResearchArtifact> {
        // Implementation would fetch artifact with lineage
        Ok(ResearchArtifact {
            artifact_id,
            artifact_type: ArtifactType::Experiment,
            name: "Placeholder Artifact".to_string(),
            version: "1.0.0".to_string(),
            description: "Placeholder".to_string(),
            content: ArtifactContent::Inline(serde_json::json!({})),
            lineage: DataLineage {
                lineage_id: Uuid::new_v4(),
                sources: vec![],
                transformations: vec![],
                derived: vec![],
                graph: None,
            },
            tags: vec![],
            metadata: ConsumptionMetadata::new("llm-data-vault"),
        })
    }

    async fn get_dataset_lineage(&self, dataset_id: Uuid) -> ConsumerResult<DataLineage> {
        // Implementation would fetch lineage graph
        Ok(DataLineage {
            lineage_id: Uuid::new_v4(),
            sources: vec![LineageSource {
                dataset_id,
                version: "1.0.0".to_string(),
                relationship: "derived_from".to_string(),
            }],
            transformations: vec![],
            derived: vec![],
            graph: None,
        })
    }

    async fn list_available_datasets(
        &self,
        _query: &DataVaultQuery,
    ) -> ConsumerResult<Vec<Uuid>> {
        // Implementation would list dataset IDs
        Ok(vec![])
    }

    async fn get_dataset_schema(&self, _dataset_id: Uuid) -> ConsumerResult<DatasetSchema> {
        // Implementation would fetch only schema
        Ok(DatasetSchema {
            format: "json-schema".to_string(),
            definition: serde_json::json!({}),
            field_descriptions: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_vault_config_default() {
        let config = DataVaultConfig::default();
        assert!(config.include_lineage);
        assert!(config.verify_integrity);
    }

    #[test]
    fn test_client_creation() {
        let client = DataVaultClient::with_endpoint("https://vault.example.com");
        assert_eq!(client.config().base.endpoint, "https://vault.example.com");
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = DataVaultClient::with_endpoint("https://vault.example.com");
        let result = client.health_check().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_access_level_serialization() {
        let level = AccessLevel::Confidential;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"confidential\"");
    }

    #[test]
    fn test_data_format_serialization() {
        let format = DataFormat::Parquet;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"parquet\"");
    }

    #[test]
    fn test_anonymization_method_serialization() {
        let method = AnonymizationMethod::DifferentialPrivacy;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, "\"differential_privacy\"");
    }

    #[test]
    fn test_artifact_content_inline() {
        let content = ArtifactContent::Inline(serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("key"));
    }
}
