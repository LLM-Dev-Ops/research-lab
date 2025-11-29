use chrono::{DateTime, Utc};
use llm_research_core::{
    domain::{
        config::ExperimentConfig,
        experiment::{Experiment, ExperimentStatus},
        ids::{ExperimentId, UserId},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateExperimentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub owner_id: Uuid,
    pub collaborators: Option<Vec<Uuid>>,
    pub tags: Option<Vec<String>>,
    pub config: ExperimentConfig,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateExperimentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub tags: Option<Vec<String>>,
    pub config: Option<ExperimentConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub owner_id: Uuid,
    pub collaborators: Vec<Uuid>,
    pub tags: Vec<String>,
    pub status: ExperimentStatus,
    pub config: ExperimentConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl From<Experiment> for ExperimentResponse {
    fn from(exp: Experiment) -> Self {
        Self {
            id: exp.id.0,
            name: exp.name,
            description: exp.description,
            hypothesis: exp.hypothesis,
            owner_id: exp.owner_id.0,
            collaborators: exp.collaborators.into_iter().map(|id| id.0).collect(),
            tags: exp.tags,
            status: exp.status,
            config: exp.config,
            created_at: exp.created_at,
            updated_at: exp.updated_at,
            archived_at: exp.archived_at,
            metadata: exp.metadata,
        }
    }
}

// Experiment Runs
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateRunRequest {
    pub config_overrides: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResponse {
    pub id: Uuid,
    pub experiment_id: Uuid,
    pub status: String,
    pub config: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct FailRunRequest {
    #[validate(length(min = 1))]
    pub error: String,
}
