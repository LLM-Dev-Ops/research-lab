//! LLM Research Lab SDK
//!
//! This crate provides a Rust SDK for interacting with the LLM Research Lab API.
//! It offers type-safe clients for managing experiments, models, datasets, prompts,
//! and evaluations.
//!
//! # Features
//!
//! - **Type-safe API clients**: Strongly-typed request and response models
//! - **Builder patterns**: Ergonomic request construction
//! - **Automatic retries**: Configurable retry logic with exponential backoff
//! - **Rate limiting**: Automatic handling of rate limits with retry-after
//! - **Multiple auth methods**: API key, bearer token, or basic auth
//! - **Pagination support**: Offset-based and cursor-based pagination
//! - **Comprehensive error handling**: Detailed error types with retryability
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use llm_research_sdk::{LlmResearchClient, SdkConfig, AuthConfig};
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client with API key authentication
//!     let config = SdkConfig::new("https://api.llm-research.example.com")
//!         .with_auth(AuthConfig::ApiKey("your-api-key".to_string()));
//!
//!     let client = LlmResearchClient::new(config)?;
//!
//!     // List experiments
//!     let experiments = client.experiments().list(None).await?;
//!     println!("Found {} experiments", experiments.data.len());
//!
//!     // Get a specific model
//!     let model_id = Uuid::parse_str("...")?;
//!     let model = client.models().get(model_id).await?;
//!     println!("Model: {}", model.name);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! The SDK can be configured with various options:
//!
//! ```rust,no_run
//! use llm_research_sdk::{SdkConfig, AuthConfig};
//! use std::time::Duration;
//!
//! let config = SdkConfig::new("https://api.llm-research.example.com")
//!     .with_auth(AuthConfig::BearerToken("token".to_string()))
//!     .with_timeout(Duration::from_secs(60))
//!     .with_max_retries(5)
//!     .with_logging(true);
//! ```
//!
//! # Error Handling
//!
//! The SDK provides detailed error types for different failure scenarios:
//!
//! ```rust,no_run
//! use llm_research_sdk::{LlmResearchClient, SdkConfig, SdkError};
//!
//! async fn handle_errors(client: &LlmResearchClient) {
//!     match client.experiments().list(None).await {
//!         Ok(experiments) => println!("Got {} experiments", experiments.data.len()),
//!         Err(SdkError::AuthenticationError(msg)) => eprintln!("Auth failed: {}", msg),
//!         Err(SdkError::NotFound { resource_type, resource_id }) => {
//!             eprintln!("Not found: {} with id {}", resource_type, resource_id)
//!         }
//!         Err(SdkError::RateLimited { retry_after, .. }) => {
//!             eprintln!("Rate limited, retry after {} seconds", retry_after)
//!         }
//!         Err(e) => eprintln!("Other error: {}", e),
//!     }
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![deny(unsafe_code)]

pub mod client;
pub mod config;
pub mod error;
pub mod resources;

// Re-export main types for convenience
pub use client::{HttpClient, PaginatedResponse, PaginationInfo, PaginationParams};
pub use config::{AuthConfig, SdkConfig, SdkConfigBuilder};
pub use error::{SdkError, SdkResult};

// Re-export resource clients
pub use resources::datasets::{
    CreateDatasetRequest, CreateVersionRequest as CreateDatasetVersionRequest, Dataset,
    DatasetFormat, DatasetVersion, DatasetsClient, DownloadResponse, ListDatasetsParams,
    UpdateDatasetRequest, UploadRequest, UploadResponse,
};
pub use resources::evaluations::{
    CompareEvaluationsRequest, ComparisonResult, CreateEvaluationRequest, Evaluation,
    EvaluationConfig, EvaluationResults, EvaluationRun, EvaluationType, EvaluationsClient,
    JudgeConfig, JudgeCriterion, JudgeScale, ListEvaluationsParams, MetricConfig, MetricResult,
    MetricType, MetricValue, RunEvaluationRequest, RunStatus, SampleResult, SubmitMetricsRequest,
    UpdateEvaluationRequest,
};
pub use resources::experiments::{
    CreateExperimentRequest, CreateRunRequest, Experiment, ExperimentConfig, ExperimentMetrics,
    ExperimentRun, ExperimentStatus, ExperimentsClient, ListExperimentsParams, MetricSummary,
    RunMetrics, UpdateExperimentRequest,
};
pub use resources::models::{
    CreateModelRequest, ListModelsParams, Model, ModelProvider, ModelsClient, UpdateModelRequest,
};
pub use resources::prompts::{
    CreatePromptRequest, CreatePromptVersionRequest, ListPromptsParams, PromptTemplate,
    PromptVariable, PromptVersion, PromptsClient, RenderPromptRequest, RenderPromptResponse,
    UpdatePromptRequest, ValidatePromptRequest, ValidatePromptResponse, VariableType,
};

use std::sync::Arc;

/// The main client for the LLM Research Lab API.
///
/// This client provides access to all API resources through dedicated sub-clients.
/// It handles authentication, request retries, and error handling automatically.
///
/// # Example
///
/// ```rust,no_run
/// use llm_research_sdk::{LlmResearchClient, SdkConfig, AuthConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SdkConfig::new("https://api.example.com")
///     .with_auth(AuthConfig::ApiKey("your-key".to_string()));
///
/// let client = LlmResearchClient::new(config)?;
///
/// // Access different resources
/// let experiments = client.experiments();
/// let models = client.models();
/// let datasets = client.datasets();
/// let prompts = client.prompts();
/// let evaluations = client.evaluations();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct LlmResearchClient {
    http_client: Arc<HttpClient>,
    experiments: ExperimentsClient,
    models: ModelsClient,
    datasets: DatasetsClient,
    prompts: PromptsClient,
    evaluations: EvaluationsClient,
}

impl LlmResearchClient {
    /// Create a new LLM Research client with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The SDK configuration including base URL and authentication
    ///
    /// # Returns
    ///
    /// Returns a new client instance or an error if configuration is invalid.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_research_sdk::{LlmResearchClient, SdkConfig, AuthConfig};
    ///
    /// let config = SdkConfig::new("https://api.example.com")
    ///     .with_auth(AuthConfig::ApiKey("your-api-key".to_string()));
    ///
    /// let client = LlmResearchClient::new(config)?;
    /// # Ok::<(), llm_research_sdk::SdkError>(())
    /// ```
    pub fn new(config: SdkConfig) -> SdkResult<Self> {
        let http_client = Arc::new(HttpClient::new(config)?);

        Ok(Self {
            experiments: ExperimentsClient::new(Arc::clone(&http_client)),
            models: ModelsClient::new(Arc::clone(&http_client)),
            datasets: DatasetsClient::new(Arc::clone(&http_client)),
            prompts: PromptsClient::new(Arc::clone(&http_client)),
            evaluations: EvaluationsClient::new(Arc::clone(&http_client)),
            http_client,
        })
    }

    /// Create a new client using a builder pattern.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the API
    ///
    /// # Returns
    ///
    /// Returns a configuration builder for fluent configuration.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use llm_research_sdk::{LlmResearchClient, AuthConfig};
    /// use std::time::Duration;
    ///
    /// let client = LlmResearchClient::builder("https://api.example.com")
    ///     .with_auth(AuthConfig::ApiKey("key".to_string()))
    ///     .with_timeout(Duration::from_secs(30))
    ///     .build()?;
    /// # Ok::<(), llm_research_sdk::SdkError>(())
    /// ```
    pub fn builder(base_url: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(base_url)
    }

    /// Get the experiments client for managing experiments.
    ///
    /// # Returns
    ///
    /// A reference to the experiments client.
    pub fn experiments(&self) -> &ExperimentsClient {
        &self.experiments
    }

    /// Get the models client for managing models.
    ///
    /// # Returns
    ///
    /// A reference to the models client.
    pub fn models(&self) -> &ModelsClient {
        &self.models
    }

    /// Get the datasets client for managing datasets.
    ///
    /// # Returns
    ///
    /// A reference to the datasets client.
    pub fn datasets(&self) -> &DatasetsClient {
        &self.datasets
    }

    /// Get the prompts client for managing prompt templates.
    ///
    /// # Returns
    ///
    /// A reference to the prompts client.
    pub fn prompts(&self) -> &PromptsClient {
        &self.prompts
    }

    /// Get the evaluations client for managing evaluations.
    ///
    /// # Returns
    ///
    /// A reference to the evaluations client.
    pub fn evaluations(&self) -> &EvaluationsClient {
        &self.evaluations
    }

    /// Get a reference to the underlying HTTP client.
    ///
    /// This is useful for making custom requests not covered by the resource clients.
    ///
    /// # Returns
    ///
    /// A reference to the HTTP client.
    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    /// Get the base URL of the API.
    ///
    /// # Returns
    ///
    /// The base URL string.
    pub fn base_url(&self) -> &str {
        &self.http_client.config().base_url
    }
}

/// Builder for creating an LlmResearchClient with fluent configuration.
#[derive(Debug)]
pub struct ClientBuilder {
    config_builder: SdkConfigBuilder,
}

impl ClientBuilder {
    /// Create a new client builder with the given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            config_builder: SdkConfig::builder(base_url),
        }
    }

    /// Set the authentication configuration.
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.config_builder = self.config_builder.with_auth(auth);
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config_builder = self.config_builder.with_timeout(timeout);
        self
    }

    /// Set the connection timeout.
    pub fn with_connect_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config_builder = self.config_builder.with_connect_timeout(timeout);
        self
    }

    /// Set the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.config_builder = self.config_builder.with_max_retries(max_retries);
        self
    }

    /// Enable or disable request/response logging.
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.config_builder = self.config_builder.with_logging(enable);
        self
    }

    /// Add a custom header to all requests.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.with_header(name, value);
        self
    }

    /// Build the client.
    ///
    /// # Returns
    ///
    /// Returns the configured client or an error if configuration is invalid.
    pub fn build(self) -> SdkResult<LlmResearchClient> {
        let config = self.config_builder.build();
        LlmResearchClient::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let result = LlmResearchClient::builder("https://api.example.com")
            .with_auth(AuthConfig::ApiKey("test-key".to_string()))
            .with_timeout(std::time::Duration::from_secs(30))
            .with_max_retries(3)
            .with_logging(true)
            .build();

        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.base_url(), "https://api.example.com");
    }

    #[test]
    fn test_client_new() {
        let config = SdkConfig::new("https://api.example.com")
            .with_auth(AuthConfig::BearerToken("token".to_string()));

        let result = LlmResearchClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_resource_access() {
        let config = SdkConfig::new("https://api.example.com");
        let client = LlmResearchClient::new(config).unwrap();

        // Just verify we can access all resource clients
        let _ = client.experiments();
        let _ = client.models();
        let _ = client.datasets();
        let _ = client.prompts();
        let _ = client.evaluations();
    }
}
