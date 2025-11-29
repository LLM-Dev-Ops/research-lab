//! Models resource client
//!
//! This module provides methods for managing LLM models.

use crate::client::{HttpClient, PaginatedResponse, PaginationParams};
use crate::error::SdkResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Client for model operations
#[derive(Debug, Clone)]
pub struct ModelsClient {
    client: Arc<HttpClient>,
}

impl ModelsClient {
    /// Create a new models client
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Create a new model
    pub async fn create(&self, request: CreateModelRequest) -> SdkResult<Model> {
        self.client.post("/models", request).await
    }

    /// Get a model by ID
    pub async fn get(&self, id: Uuid) -> SdkResult<Model> {
        self.client.get(&format!("/models/{}", id)).await
    }

    /// List models with optional filtering and pagination
    pub async fn list(
        &self,
        params: Option<ListModelsParams>,
    ) -> SdkResult<PaginatedResponse<Model>> {
        match params {
            Some(p) => self.client.get_with_query("/models", &p).await,
            None => self.client.get("/models").await,
        }
    }

    /// Update a model
    pub async fn update(&self, id: Uuid, request: UpdateModelRequest) -> SdkResult<Model> {
        self.client.put(&format!("/models/{}", id), request).await
    }

    /// Delete a model
    pub async fn delete(&self, id: Uuid) -> SdkResult<()> {
        self.client.delete(&format!("/models/{}", id)).await
    }

    /// List available model providers
    pub async fn list_providers(&self) -> SdkResult<Vec<ModelProvider>> {
        self.client.get("/models/providers").await
    }
}

/// Request to create a new model
#[derive(Debug, Clone, Serialize)]
pub struct CreateModelRequest {
    pub name: String,
    pub provider: String,
    pub model_identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub config: serde_json::Value,
}

impl CreateModelRequest {
    /// Create a new model request
    pub fn new(
        name: impl Into<String>,
        provider: impl Into<String>,
        model_identifier: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider: provider.into(),
            model_identifier: model_identifier.into(),
            version: None,
            config: serde_json::json!({}),
        }
    }

    /// Add a version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Add configuration
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

/// Request to update a model
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateModelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl UpdateModelRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}

/// Parameters for listing models
#[derive(Debug, Clone, Serialize, Default)]
pub struct ListModelsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ListModelsParams {
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

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Model entity
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub model_identifier: String,
    pub version: Option<String>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model provider information
#[derive(Debug, Clone, Deserialize)]
pub struct ModelProvider {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub supported_models: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_model_request_builder() {
        let request = CreateModelRequest::new("GPT-4", "openai", "gpt-4-turbo")
            .with_version("2024-01-01")
            .with_config(serde_json::json!({
                "max_tokens": 4096,
                "temperature": 0.7
            }));

        assert_eq!(request.name, "GPT-4");
        assert_eq!(request.provider, "openai");
        assert_eq!(request.model_identifier, "gpt-4-turbo");
        assert_eq!(request.version, Some("2024-01-01".to_string()));
    }

    #[test]
    fn test_update_model_request_builder() {
        let request = UpdateModelRequest::new()
            .with_name("GPT-4 Updated")
            .with_version("2024-02-01");

        assert_eq!(request.name, Some("GPT-4 Updated".to_string()));
        assert_eq!(request.version, Some("2024-02-01".to_string()));
    }

    #[test]
    fn test_list_params_builder() {
        let params = ListModelsParams::new()
            .with_limit(10)
            .with_provider("openai");

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.provider, Some("openai".to_string()));
    }
}
