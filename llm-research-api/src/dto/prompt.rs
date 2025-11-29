use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePromptTemplateRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(min = 1))]
    pub template: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdatePromptTemplateRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub description: Option<String>,
    #[validate(length(min = 1))]
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub template: String,
    pub variables: Vec<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
