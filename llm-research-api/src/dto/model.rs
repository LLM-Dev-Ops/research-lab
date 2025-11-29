use chrono::{DateTime, Utc};
use llm_research_core::domain::model::{Model, ModelProvider};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateModelRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub provider: ModelProvider,
    #[validate(length(min = 1, max = 255))]
    pub model_identifier: String,
    pub version: Option<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateModelRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub version: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub id: Uuid,
    pub name: String,
    pub provider: ModelProvider,
    pub model_identifier: String,
    pub version: Option<String>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Model> for ModelResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            provider: model.provider,
            model_identifier: model.model_identifier,
            version: model.version,
            config: model.config,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub supported_models: Vec<String>,
}
