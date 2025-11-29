use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use super::ids::{ExperimentId, UserId};
use super::config::ExperimentConfig;

// ===== Experiment Status =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Archived,
    Failed,
}

impl ExperimentStatus {
    pub fn can_transition_to(&self, target: &ExperimentStatus) -> bool {
        use ExperimentStatus::*;
        match (self, target) {
            // From Draft
            (Draft, Active) | (Draft, Archived) => true,

            // From Active
            (Active, Paused) | (Active, Completed) | (Active, Failed) | (Active, Archived) => true,

            // From Paused
            (Paused, Active) | (Paused, Completed) | (Paused, Failed) | (Paused, Archived) => true,

            // From Completed
            (Completed, Archived) | (Completed, Active) => true,

            // From Failed
            (Failed, Active) | (Failed, Archived) => true,

            // From Archived - can be reactivated
            (Archived, Active) => true,

            // Same state is allowed (no-op)
            (a, b) if a == b => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, ExperimentStatus::Archived)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, ExperimentStatus::Active)
    }

    pub fn can_execute_runs(&self) -> bool {
        matches!(self, ExperimentStatus::Active)
    }
}

// ===== Experiment Domain Model =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct Experiment {
    pub id: ExperimentId,

    #[validate(length(min = 1, max = 255))]
    pub name: String,

    pub description: Option<String>,

    pub hypothesis: Option<String>,

    pub owner_id: UserId,

    pub collaborators: Vec<UserId>,

    pub tags: Vec<String>,

    pub status: ExperimentStatus,

    pub config: ExperimentConfig,

    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,

    pub archived_at: Option<DateTime<Utc>>,

    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Experiment {
    pub fn new(
        name: String,
        description: Option<String>,
        hypothesis: Option<String>,
        owner_id: UserId,
        config: ExperimentConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ExperimentId::new(),
            name,
            description,
            hypothesis,
            owner_id,
            collaborators: Vec::new(),
            tags: Vec::new(),
            status: ExperimentStatus::Draft,
            config,
            created_at: now,
            updated_at: now,
            archived_at: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_collaborators(mut self, collaborators: Vec<UserId>) -> Self {
        self.collaborators = collaborators;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    // State transition methods

    pub fn activate(&mut self) -> Result<(), String> {
        if !self.status.can_transition_to(&ExperimentStatus::Active) {
            return Err(format!(
                "Cannot transition from {:?} to Active",
                self.status
            ));
        }
        self.status = ExperimentStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if !self.status.can_transition_to(&ExperimentStatus::Paused) {
            return Err(format!(
                "Cannot transition from {:?} to Paused",
                self.status
            ));
        }
        self.status = ExperimentStatus::Paused;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), String> {
        if !self.status.can_transition_to(&ExperimentStatus::Completed) {
            return Err(format!(
                "Cannot transition from {:?} to Completed",
                self.status
            ));
        }
        self.status = ExperimentStatus::Completed;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn fail(&mut self) -> Result<(), String> {
        if !self.status.can_transition_to(&ExperimentStatus::Failed) {
            return Err(format!(
                "Cannot transition from {:?} to Failed",
                self.status
            ));
        }
        self.status = ExperimentStatus::Failed;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn archive(&mut self) -> Result<(), String> {
        if !self.status.can_transition_to(&ExperimentStatus::Archived) {
            return Err(format!(
                "Cannot transition from {:?} to Archived",
                self.status
            ));
        }
        self.status = ExperimentStatus::Archived;
        self.updated_at = Utc::now();
        self.archived_at = Some(Utc::now());
        Ok(())
    }

    // Helper methods

    pub fn add_collaborator(&mut self, user_id: UserId) {
        if !self.collaborators.contains(&user_id) && user_id != self.owner_id {
            self.collaborators.push(user_id);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_collaborator(&mut self, user_id: &UserId) {
        if let Some(pos) = self.collaborators.iter().position(|id| id == user_id) {
            self.collaborators.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    pub fn update_config(&mut self, config: ExperimentConfig) {
        self.config = config;
        self.updated_at = Utc::now();
    }

    pub fn update_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    pub fn is_owner(&self, user_id: &UserId) -> bool {
        &self.owner_id == user_id
    }

    pub fn is_collaborator(&self, user_id: &UserId) -> bool {
        self.collaborators.contains(user_id)
    }

    pub fn has_access(&self, user_id: &UserId) -> bool {
        self.is_owner(user_id) || self.is_collaborator(user_id)
    }

    pub fn can_execute_runs(&self) -> bool {
        self.status.can_execute_runs()
    }

    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

// ===== Experiment Summary (for listings) =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperimentSummary {
    pub id: ExperimentId,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: UserId,
    pub status: ExperimentStatus,
    pub tags: Vec<String>,
    pub run_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Experiment> for ExperimentSummary {
    fn from(experiment: &Experiment) -> Self {
        Self {
            id: experiment.id,
            name: experiment.name.clone(),
            description: experiment.description.clone(),
            owner_id: experiment.owner_id,
            status: experiment.status,
            tags: experiment.tags.clone(),
            run_count: 0, // This would be populated from a repository
            created_at: experiment.created_at,
            updated_at: experiment.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ExperimentConfig;

    #[test]
    fn test_experiment_status_transitions() {
        use ExperimentStatus::*;

        // Valid transitions from Draft
        assert!(Draft.can_transition_to(&Active));
        assert!(Draft.can_transition_to(&Archived));
        assert!(!Draft.can_transition_to(&Paused));
        assert!(!Draft.can_transition_to(&Completed));

        // Valid transitions from Active
        assert!(Active.can_transition_to(&Paused));
        assert!(Active.can_transition_to(&Completed));
        assert!(Active.can_transition_to(&Failed));
        assert!(Active.can_transition_to(&Archived));
        assert!(!Active.can_transition_to(&Draft));

        // Valid transitions from Paused
        assert!(Paused.can_transition_to(&Active));
        assert!(Paused.can_transition_to(&Completed));
        assert!(Paused.can_transition_to(&Failed));

        // Valid transitions from Completed
        assert!(Completed.can_transition_to(&Archived));
        assert!(Completed.can_transition_to(&Active)); // Can rerun

        // Valid transitions from Archived
        assert!(Archived.can_transition_to(&Active)); // Can reactivate
        assert!(!Archived.can_transition_to(&Draft));
    }

    #[test]
    fn test_experiment_creation() {
        let user_id = UserId::new();
        let config = ExperimentConfig::default();

        let experiment = Experiment::new(
            "Test Experiment".to_string(),
            Some("A test experiment".to_string()),
            Some("Testing hypothesis".to_string()),
            user_id,
            config,
        );

        assert_eq!(experiment.name, "Test Experiment");
        assert_eq!(experiment.status, ExperimentStatus::Draft);
        assert!(experiment.is_owner(&user_id));
        assert!(experiment.collaborators.is_empty());
    }

    #[test]
    fn test_experiment_lifecycle() {
        let user_id = UserId::new();
        let config = ExperimentConfig::default();

        let mut experiment = Experiment::new(
            "Test Experiment".to_string(),
            None,
            None,
            user_id,
            config,
        );

        // Activate experiment
        assert!(experiment.activate().is_ok());
        assert_eq!(experiment.status, ExperimentStatus::Active);
        assert!(experiment.can_execute_runs());

        // Pause experiment
        assert!(experiment.pause().is_ok());
        assert_eq!(experiment.status, ExperimentStatus::Paused);
        assert!(!experiment.can_execute_runs());

        // Resume experiment
        assert!(experiment.activate().is_ok());
        assert_eq!(experiment.status, ExperimentStatus::Active);

        // Complete experiment
        assert!(experiment.complete().is_ok());
        assert_eq!(experiment.status, ExperimentStatus::Completed);

        // Archive experiment
        assert!(experiment.archive().is_ok());
        assert_eq!(experiment.status, ExperimentStatus::Archived);
        assert!(experiment.is_terminal());
        assert!(experiment.archived_at.is_some());
    }

    #[test]
    fn test_experiment_collaborators() {
        let owner_id = UserId::new();
        let collaborator1 = UserId::new();
        let collaborator2 = UserId::new();
        let config = ExperimentConfig::default();

        let mut experiment = Experiment::new(
            "Test Experiment".to_string(),
            None,
            None,
            owner_id,
            config,
        );

        // Add collaborators
        experiment.add_collaborator(collaborator1);
        experiment.add_collaborator(collaborator2);
        assert_eq!(experiment.collaborators.len(), 2);

        // Check access
        assert!(experiment.has_access(&owner_id));
        assert!(experiment.has_access(&collaborator1));
        assert!(experiment.has_access(&collaborator2));

        // Remove collaborator
        experiment.remove_collaborator(&collaborator1);
        assert_eq!(experiment.collaborators.len(), 1);
        assert!(!experiment.has_access(&collaborator1));
    }

    #[test]
    fn test_experiment_tags() {
        let user_id = UserId::new();
        let config = ExperimentConfig::default();

        let mut experiment = Experiment::new(
            "Test Experiment".to_string(),
            None,
            None,
            user_id,
            config,
        )
        .with_tags(vec!["test".to_string(), "ml".to_string()]);

        assert_eq!(experiment.tags.len(), 2);

        // Add tag
        experiment.add_tag("nlp".to_string());
        assert_eq!(experiment.tags.len(), 3);

        // Adding duplicate tag should not increase count
        experiment.add_tag("nlp".to_string());
        assert_eq!(experiment.tags.len(), 3);

        // Remove tag
        experiment.remove_tag("test");
        assert_eq!(experiment.tags.len(), 2);
    }

    #[test]
    fn test_invalid_state_transitions() {
        let user_id = UserId::new();
        let config = ExperimentConfig::default();

        let mut experiment = Experiment::new(
            "Test Experiment".to_string(),
            None,
            None,
            user_id,
            config,
        );

        // Cannot pause from Draft
        assert!(experiment.pause().is_err());

        // Cannot complete from Draft
        assert!(experiment.complete().is_err());
    }
}
