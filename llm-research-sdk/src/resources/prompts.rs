//! Prompts resource client
//!
//! This module provides methods for managing prompt templates.

use crate::client::{HttpClient, PaginatedResponse, PaginationParams};
use crate::error::SdkResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Client for prompt template operations
#[derive(Debug, Clone)]
pub struct PromptsClient {
    client: Arc<HttpClient>,
}

impl PromptsClient {
    /// Create a new prompts client
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Create a new prompt template
    pub async fn create(&self, request: CreatePromptRequest) -> SdkResult<PromptTemplate> {
        self.client.post("/prompts", request).await
    }

    /// Get a prompt template by ID
    pub async fn get(&self, id: Uuid) -> SdkResult<PromptTemplate> {
        self.client.get(&format!("/prompts/{}", id)).await
    }

    /// List prompt templates with optional filtering and pagination
    pub async fn list(
        &self,
        params: Option<ListPromptsParams>,
    ) -> SdkResult<PaginatedResponse<PromptTemplate>> {
        match params {
            Some(p) => self.client.get_with_query("/prompts", &p).await,
            None => self.client.get("/prompts").await,
        }
    }

    /// Update a prompt template
    pub async fn update(
        &self,
        id: Uuid,
        request: UpdatePromptRequest,
    ) -> SdkResult<PromptTemplate> {
        self.client.put(&format!("/prompts/{}", id), request).await
    }

    /// Delete a prompt template
    pub async fn delete(&self, id: Uuid) -> SdkResult<()> {
        self.client.delete(&format!("/prompts/{}", id)).await
    }

    /// Create a new version of a prompt template
    pub async fn create_version(
        &self,
        prompt_id: Uuid,
        request: CreatePromptVersionRequest,
    ) -> SdkResult<PromptVersion> {
        self.client
            .post(&format!("/prompts/{}/versions", prompt_id), request)
            .await
    }

    /// List versions of a prompt template
    pub async fn list_versions(
        &self,
        prompt_id: Uuid,
        pagination: Option<PaginationParams>,
    ) -> SdkResult<PaginatedResponse<PromptVersion>> {
        let path = format!("/prompts/{}/versions", prompt_id);
        match pagination {
            Some(p) => self.client.get_with_query(&path, &p).await,
            None => self.client.get(&path).await,
        }
    }

    /// Get a specific version of a prompt template
    pub async fn get_version(
        &self,
        prompt_id: Uuid,
        version_id: Uuid,
    ) -> SdkResult<PromptVersion> {
        self.client
            .get(&format!("/prompts/{}/versions/{}", prompt_id, version_id))
            .await
    }

    /// Render a prompt template with variables
    pub async fn render(
        &self,
        prompt_id: Uuid,
        request: RenderPromptRequest,
    ) -> SdkResult<RenderPromptResponse> {
        self.client
            .post(&format!("/prompts/{}/render", prompt_id), request)
            .await
    }

    /// Validate a prompt template
    pub async fn validate(
        &self,
        request: ValidatePromptRequest,
    ) -> SdkResult<ValidatePromptResponse> {
        self.client.post("/prompts/validate", request).await
    }
}

/// Request to create a new prompt template
#[derive(Debug, Clone, Serialize)]
pub struct CreatePromptRequest {
    pub name: String,
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<PromptVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl CreatePromptRequest {
    /// Create a new prompt request
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            template: template.into(),
            description: None,
            system_prompt: None,
            variables: None,
            tags: None,
            metadata: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a system prompt
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Add variables
    pub fn with_variables(mut self, variables: Vec<PromptVariable>) -> Self {
        self.variables = Some(variables);
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

/// Prompt variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: VariableType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(default)]
    pub required: bool,
}

impl PromptVariable {
    pub fn new(name: impl Into<String>, variable_type: VariableType) -> Self {
        Self {
            name: name.into(),
            variable_type,
            description: None,
            default_value: None,
            required: true,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default(mut self, default_value: serde_json::Value) -> Self {
        self.default_value = Some(default_value);
        self.required = false;
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// Variable type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

impl std::fmt::Display for VariableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
            Self::Number => write!(f, "number"),
            Self::Boolean => write!(f, "boolean"),
            Self::Array => write!(f, "array"),
            Self::Object => write!(f, "object"),
        }
    }
}

/// Request to update a prompt template
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdatePromptRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl UpdatePromptRequest {
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

/// Parameters for listing prompts
#[derive(Debug, Clone, Serialize, Default)]
pub struct ListPromptsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

impl ListPromptsParams {
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

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags.join(","));
        self
    }

    pub fn with_search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
}

/// Request to create a prompt version
#[derive(Debug, Clone, Serialize)]
pub struct CreatePromptVersionRequest {
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<PromptVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

impl CreatePromptVersionRequest {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            system_prompt: None,
            variables: None,
            changelog: None,
        }
    }

    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    pub fn with_variables(mut self, variables: Vec<PromptVariable>) -> Self {
        self.variables = Some(variables);
        self
    }

    pub fn with_changelog(mut self, changelog: impl Into<String>) -> Self {
        self.changelog = Some(changelog.into());
        self
    }
}

/// Request to render a prompt
#[derive(Debug, Clone, Serialize)]
pub struct RenderPromptRequest {
    pub variables: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<Uuid>,
}

impl RenderPromptRequest {
    pub fn new(variables: HashMap<String, serde_json::Value>) -> Self {
        Self {
            variables,
            version_id: None,
        }
    }

    pub fn with_version(mut self, version_id: Uuid) -> Self {
        self.version_id = Some(version_id);
        self
    }
}

/// Response from rendering a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct RenderPromptResponse {
    pub rendered_template: String,
    pub rendered_system_prompt: Option<String>,
    pub token_count: Option<u32>,
}

/// Request to validate a prompt template
#[derive(Debug, Clone, Serialize)]
pub struct ValidatePromptRequest {
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<PromptVariable>>,
}

impl ValidatePromptRequest {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            variables: None,
        }
    }

    pub fn with_variables(mut self, variables: Vec<PromptVariable>) -> Self {
        self.variables = Some(variables);
        self
    }
}

/// Response from validating a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct ValidatePromptResponse {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub detected_variables: Vec<String>,
}

/// Validation error
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationError {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Validation warning
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationWarning {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Prompt template entity
#[derive(Debug, Clone, Deserialize)]
pub struct PromptTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub template: String,
    pub system_prompt: Option<String>,
    pub variables: Vec<PromptVariable>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub current_version_id: Option<Uuid>,
    pub version_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Prompt version entity
#[derive(Debug, Clone, Deserialize)]
pub struct PromptVersion {
    pub id: Uuid,
    pub prompt_id: Uuid,
    pub version_number: u32,
    pub template: String,
    pub system_prompt: Option<String>,
    pub variables: Vec<PromptVariable>,
    pub changelog: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_prompt_request_builder() {
        let request = CreatePromptRequest::new(
            "QA Prompt",
            "Answer the following question: {{question}}\n\nContext: {{context}}",
        )
        .with_description("A Q&A prompt template")
        .with_system_prompt("You are a helpful assistant.")
        .with_variables(vec![
            PromptVariable::new("question", VariableType::String),
            PromptVariable::new("context", VariableType::String).optional(),
        ])
        .with_tags(vec!["qa".to_string(), "general".to_string()]);

        assert_eq!(request.name, "QA Prompt");
        assert!(request.template.contains("{{question}}"));
        assert!(request.system_prompt.is_some());
        assert_eq!(request.variables.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_prompt_variable_builder() {
        let var = PromptVariable::new("name", VariableType::String)
            .with_description("The user's name")
            .with_default(serde_json::json!("World"));

        assert_eq!(var.name, "name");
        assert_eq!(var.variable_type, VariableType::String);
        assert!(!var.required); // Should be false when default is set
        assert!(var.default_value.is_some());
    }

    #[test]
    fn test_render_prompt_request() {
        let mut variables = HashMap::new();
        variables.insert("question".to_string(), serde_json::json!("What is Rust?"));
        variables.insert(
            "context".to_string(),
            serde_json::json!("Rust is a programming language."),
        );

        let request = RenderPromptRequest::new(variables);
        assert_eq!(request.variables.len(), 2);
    }

    #[test]
    fn test_validate_prompt_request() {
        let request = ValidatePromptRequest::new("Hello {{name}}!")
            .with_variables(vec![PromptVariable::new("name", VariableType::String)]);

        assert!(request.template.contains("{{name}}"));
        assert_eq!(request.variables.as_ref().unwrap().len(), 1);
    }
}
