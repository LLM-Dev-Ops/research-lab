use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ModelProvider {
    OpenAI,
    Anthropic,
    Google,
    Cohere,
    HuggingFace,
    Azure,
    AWS,
    Local,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Model {
    pub id: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub provider: ModelProvider,
    #[validate(length(min = 1, max = 255))]
    pub model_identifier: String,
    pub version: Option<String>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Model {
    pub fn new(
        name: String,
        provider: ModelProvider,
        model_identifier: String,
        version: Option<String>,
        config: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            provider,
            model_identifier,
            version,
            config,
            created_at: now,
            updated_at: now,
        }
    }
}
