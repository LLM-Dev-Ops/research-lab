use llm_research_core::domain::*;
use uuid::Uuid;

// ===== ID Tests =====

#[test]
fn test_experiment_id_conversions() {
    let uuid = Uuid::new_v4();
    let id = ExperimentId::from_uuid(uuid);

    assert_eq!(id.as_uuid(), &uuid);
    assert_eq!(*id.as_uuid(), uuid);

    // Test From trait
    let id2: ExperimentId = uuid.into();
    assert_eq!(id, id2);

    // Test Into trait
    let uuid2: Uuid = id.into();
    assert_eq!(uuid, uuid2);
}

#[test]
fn test_run_id_conversions() {
    let uuid = Uuid::new_v4();
    let id = RunId::from_uuid(uuid);

    assert_eq!(id.as_uuid(), &uuid);

    let id2: RunId = uuid.into();
    assert_eq!(id, id2);

    let uuid2: Uuid = id.into();
    assert_eq!(uuid, uuid2);
}

#[test]
fn test_user_id_conversions() {
    let uuid = Uuid::new_v4();
    let id = UserId::from_uuid(uuid);

    assert_eq!(id.as_uuid(), &uuid);

    let id2: UserId = uuid.into();
    assert_eq!(id, id2);

    let uuid2: Uuid = id.into();
    assert_eq!(uuid, uuid2);
}

#[test]
fn test_experiment_id_display() {
    let id = ExperimentId::new();
    let display_str = format!("{}", id);
    assert_eq!(display_str, id.as_uuid().to_string());
}

#[test]
fn test_experiment_id_default() {
    let id1 = ExperimentId::default();
    let id2 = ExperimentId::default();
    assert_ne!(id1, id2); // Should generate different UUIDs
}

// ===== SemanticVersion Tests =====

#[test]
fn test_semantic_version_creation() {
    let v = SemanticVersion::new(1, 2, 3);
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.pre_release, None);
    assert_eq!(v.build_metadata, None);
}

#[test]
fn test_semantic_version_with_pre_release() {
    let v = SemanticVersion::new(1, 0, 0)
        .with_pre_release("alpha.1".to_string());
    assert_eq!(v.pre_release, Some("alpha.1".to_string()));
}

#[test]
fn test_semantic_version_with_build_metadata() {
    let v = SemanticVersion::new(1, 0, 0)
        .with_build_metadata("build.123".to_string());
    assert_eq!(v.build_metadata, Some("build.123".to_string()));
}

#[test]
fn test_semantic_version_parsing() {
    let v = SemanticVersion::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);

    let v = SemanticVersion::parse("2.0.0-beta").unwrap();
    assert_eq!(v.major, 2);
    assert_eq!(v.pre_release, Some("beta".to_string()));

    let v = SemanticVersion::parse("1.0.0+20240101").unwrap();
    assert_eq!(v.build_metadata, Some("20240101".to_string()));

    let v = SemanticVersion::parse("1.0.0-rc.1+build.456").unwrap();
    assert_eq!(v.pre_release, Some("rc.1".to_string()));
    assert_eq!(v.build_metadata, Some("build.456".to_string()));
}

#[test]
fn test_semantic_version_parsing_errors() {
    assert!(SemanticVersion::parse("1.2").is_err());
    assert!(SemanticVersion::parse("1.2.x").is_err());
    assert!(SemanticVersion::parse("invalid").is_err());
    assert!(SemanticVersion::parse("").is_err());
}

#[test]
fn test_semantic_version_comparison() {
    let v1 = SemanticVersion::new(1, 0, 0);
    let v2 = SemanticVersion::new(1, 0, 1);
    let v3 = SemanticVersion::new(1, 1, 0);
    let v4 = SemanticVersion::new(2, 0, 0);

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v3 < v4);

    // Pre-release versions have lower precedence
    let v_release = SemanticVersion::new(1, 0, 0);
    let v_pre = SemanticVersion::new(1, 0, 0)
        .with_pre_release("alpha".to_string());
    assert!(v_pre < v_release);
}

#[test]
fn test_semantic_version_display() {
    let v1 = SemanticVersion::new(1, 2, 3);
    assert_eq!(v1.to_string(), "1.2.3");

    let v2 = SemanticVersion::new(1, 0, 0)
        .with_pre_release("beta.1".to_string());
    assert_eq!(v2.to_string(), "1.0.0-beta.1");

    let v3 = SemanticVersion::new(2, 1, 0)
        .with_pre_release("rc.2".to_string())
        .with_build_metadata("build.999".to_string());
    assert_eq!(v3.to_string(), "2.1.0-rc.2+build.999");
}

// ===== ContentHash Tests =====

#[test]
fn test_content_hash_computation() {
    let data = "hello world";
    let hash1 = ContentHash::from_str(data);
    let hash2 = ContentHash::from_str(data);

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.as_str(), hash2.as_str());
}

#[test]
fn test_content_hash_different_data() {
    let hash1 = ContentHash::from_str("hello");
    let hash2 = ContentHash::from_str("world");

    assert_ne!(hash1, hash2);
}

#[test]
fn test_content_hash_from_bytes() {
    let data = b"test data";
    let hash1 = ContentHash::from_bytes(data);
    let hash2 = ContentHash::from_str("test data");

    assert_eq!(hash1, hash2);
}

#[test]
fn test_content_hash_display() {
    let hash = ContentHash::from_str("test");
    let display = format!("{}", hash);
    assert_eq!(display, hash.as_str());
    assert!(!display.is_empty());
    // SHA256 produces 64 hex characters
    assert_eq!(display.len(), 64);
}

// ===== ExperimentStatus Tests =====

#[test]
fn test_experiment_status_transitions() {
    use ExperimentStatus::*;

    // Draft transitions
    assert!(Draft.can_transition_to(&Active));
    assert!(Draft.can_transition_to(&Archived));
    assert!(!Draft.can_transition_to(&Paused));
    assert!(!Draft.can_transition_to(&Completed));
    assert!(!Draft.can_transition_to(&Failed));

    // Active transitions
    assert!(Active.can_transition_to(&Paused));
    assert!(Active.can_transition_to(&Completed));
    assert!(Active.can_transition_to(&Failed));
    assert!(Active.can_transition_to(&Archived));
    assert!(!Active.can_transition_to(&Draft));

    // Paused transitions
    assert!(Paused.can_transition_to(&Active));
    assert!(Paused.can_transition_to(&Completed));
    assert!(Paused.can_transition_to(&Failed));
    assert!(Paused.can_transition_to(&Archived));

    // Completed transitions
    assert!(Completed.can_transition_to(&Active));
    assert!(Completed.can_transition_to(&Archived));
    assert!(!Completed.can_transition_to(&Draft));

    // Failed transitions
    assert!(Failed.can_transition_to(&Active));
    assert!(Failed.can_transition_to(&Archived));

    // Archived transitions
    assert!(Archived.can_transition_to(&Active));
    assert!(!Archived.can_transition_to(&Draft));
}

#[test]
fn test_experiment_status_is_terminal() {
    assert!(ExperimentStatus::Archived.is_terminal());
    assert!(!ExperimentStatus::Active.is_terminal());
    assert!(!ExperimentStatus::Draft.is_terminal());
    assert!(!ExperimentStatus::Completed.is_terminal());
}

#[test]
fn test_experiment_status_is_active() {
    assert!(ExperimentStatus::Active.is_active());
    assert!(!ExperimentStatus::Draft.is_active());
    assert!(!ExperimentStatus::Paused.is_active());
}

#[test]
fn test_experiment_status_can_execute_runs() {
    assert!(ExperimentStatus::Active.can_execute_runs());
    assert!(!ExperimentStatus::Draft.can_execute_runs());
    assert!(!ExperimentStatus::Paused.can_execute_runs());
    assert!(!ExperimentStatus::Completed.can_execute_runs());
}

// ===== RunStatus Tests =====

#[test]
fn test_run_status_is_terminal() {
    assert!(RunStatus::Completed.is_terminal());
    assert!(RunStatus::Failed.is_terminal());
    assert!(RunStatus::Cancelled.is_terminal());
    assert!(RunStatus::TimedOut.is_terminal());
    assert!(!RunStatus::Pending.is_terminal());
    assert!(!RunStatus::Queued.is_terminal());
    assert!(!RunStatus::Running.is_terminal());
}

#[test]
fn test_run_status_is_running() {
    assert!(RunStatus::Running.is_running());
    assert!(!RunStatus::Pending.is_running());
    assert!(!RunStatus::Completed.is_running());
}

#[test]
fn test_run_status_is_successful() {
    assert!(RunStatus::Completed.is_successful());
    assert!(!RunStatus::Failed.is_successful());
    assert!(!RunStatus::Cancelled.is_successful());
    assert!(!RunStatus::Running.is_successful());
}

// ===== Experiment Tests =====

#[test]
fn test_experiment_creation() {
    let owner_id = UserId::new();
    let config = ExperimentConfig::default();

    let experiment = Experiment::new(
        "Test Experiment".to_string(),
        Some("Description".to_string()),
        Some("Hypothesis".to_string()),
        owner_id,
        config,
    );

    assert_eq!(experiment.name, "Test Experiment");
    assert_eq!(experiment.description, Some("Description".to_string()));
    assert_eq!(experiment.hypothesis, Some("Hypothesis".to_string()));
    assert_eq!(experiment.status, ExperimentStatus::Draft);
    assert_eq!(experiment.owner_id, owner_id);
    assert!(experiment.collaborators.is_empty());
    assert!(experiment.tags.is_empty());
}

#[test]
fn test_experiment_state_transitions() {
    let owner_id = UserId::new();
    let config = ExperimentConfig::default();

    let mut experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    // Activate
    assert!(experiment.activate().is_ok());
    assert_eq!(experiment.status, ExperimentStatus::Active);

    // Pause
    assert!(experiment.pause().is_ok());
    assert_eq!(experiment.status, ExperimentStatus::Paused);

    // Resume
    assert!(experiment.activate().is_ok());
    assert_eq!(experiment.status, ExperimentStatus::Active);

    // Complete
    assert!(experiment.complete().is_ok());
    assert_eq!(experiment.status, ExperimentStatus::Completed);

    // Archive
    assert!(experiment.archive().is_ok());
    assert_eq!(experiment.status, ExperimentStatus::Archived);
    assert!(experiment.archived_at.is_some());
}

#[test]
fn test_experiment_invalid_transitions() {
    let owner_id = UserId::new();
    let config = ExperimentConfig::default();

    let mut experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    // Cannot pause from Draft
    assert!(experiment.pause().is_err());

    // Cannot complete from Draft
    assert!(experiment.complete().is_err());

    // Cannot fail from Draft
    assert!(experiment.fail().is_err());
}

#[test]
fn test_experiment_collaborators() {
    let owner_id = UserId::new();
    let collab1 = UserId::new();
    let collab2 = UserId::new();
    let config = ExperimentConfig::default();

    let mut experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    experiment.add_collaborator(collab1);
    experiment.add_collaborator(collab2);
    assert_eq!(experiment.collaborators.len(), 2);

    // Adding duplicate should not increase count
    experiment.add_collaborator(collab1);
    assert_eq!(experiment.collaborators.len(), 2);

    // Owner cannot be added as collaborator
    experiment.add_collaborator(owner_id);
    assert_eq!(experiment.collaborators.len(), 2);

    // Check access
    assert!(experiment.has_access(&owner_id));
    assert!(experiment.has_access(&collab1));

    // Remove collaborator
    experiment.remove_collaborator(&collab1);
    assert_eq!(experiment.collaborators.len(), 1);
    assert!(!experiment.has_access(&collab1));
}

#[test]
fn test_experiment_tags() {
    let owner_id = UserId::new();
    let config = ExperimentConfig::default();

    let mut experiment = Experiment::new(
        "Test".to_string(),
        None,
        None,
        owner_id,
        config,
    );

    experiment.add_tag("ml".to_string());
    experiment.add_tag("nlp".to_string());
    assert_eq!(experiment.tags.len(), 2);

    // Duplicate tag
    experiment.add_tag("ml".to_string());
    assert_eq!(experiment.tags.len(), 2);

    experiment.remove_tag("ml");
    assert_eq!(experiment.tags.len(), 1);
}

// ===== ExperimentRun Tests =====

#[test]
fn test_experiment_run_creation() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let run = ExperimentRun::new(
        experiment_id,
        1,
        "Run 1".to_string(),
        user_id,
    );

    assert_eq!(run.experiment_id, experiment_id);
    assert_eq!(run.run_number, 1);
    assert_eq!(run.name, "Run 1");
    assert_eq!(run.status, RunStatus::Pending);
    assert_eq!(run.created_by, user_id);
    assert!(run.parameters.is_empty());
    assert!(run.started_at.is_none());
    assert!(run.ended_at.is_none());
}

#[test]
fn test_experiment_run_lifecycle() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let mut run = ExperimentRun::new(
        experiment_id,
        1,
        "Test Run".to_string(),
        user_id,
    );

    // Queue
    run.queue();
    assert_eq!(run.status, RunStatus::Queued);

    // Start
    run.start();
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.started_at.is_some());
    assert!(run.is_running());

    // Complete
    run.complete();
    assert_eq!(run.status, RunStatus::Completed);
    assert!(run.ended_at.is_some());
    assert!(run.is_terminal());
    assert!(run.is_successful());
}

#[test]
fn test_experiment_run_failure() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let mut run = ExperimentRun::new(
        experiment_id,
        1,
        "Test Run".to_string(),
        user_id,
    );

    run.start();

    let error = RunError {
        error_type: "RuntimeError".to_string(),
        message: "Out of memory".to_string(),
        stacktrace: None,
        occurred_at: chrono::Utc::now(),
        is_retryable: true,
        metadata: Default::default(),
    };

    run.fail(error);
    assert_eq!(run.status, RunStatus::Failed);
    assert!(run.ended_at.is_some());
    assert!(run.error.is_some());
    assert!(!run.is_successful());
}

#[test]
fn test_experiment_run_cancel() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let mut run = ExperimentRun::new(
        experiment_id,
        1,
        "Test Run".to_string(),
        user_id,
    );

    run.start();
    run.cancel();

    assert_eq!(run.status, RunStatus::Cancelled);
    assert!(run.is_terminal());
}

#[test]
fn test_experiment_run_timeout() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let mut run = ExperimentRun::new(
        experiment_id,
        1,
        "Test Run".to_string(),
        user_id,
    );

    run.start();
    run.timeout();

    assert_eq!(run.status, RunStatus::TimedOut);
    assert!(run.is_terminal());
}

#[test]
fn test_experiment_run_duration() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let mut run = ExperimentRun::new(
        experiment_id,
        1,
        "Test Run".to_string(),
        user_id,
    );

    assert!(run.duration_seconds().is_none());

    run.start();
    std::thread::sleep(std::time::Duration::from_millis(10));
    run.complete();

    let duration = run.duration_seconds();
    assert!(duration.is_some());
    assert!(duration.unwrap() >= 0);
}

#[test]
fn test_experiment_run_with_parameters() {
    use std::collections::HashMap;

    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();

    let mut params = HashMap::new();
    params.insert("learning_rate".to_string(), ParameterValue::from(0.001f64));
    params.insert("batch_size".to_string(), ParameterValue::from(32i64));
    params.insert("enabled".to_string(), ParameterValue::from(true));

    let run = ExperimentRun::new(
        experiment_id,
        1,
        "Test Run".to_string(),
        user_id,
    ).with_parameters(params);

    assert_eq!(run.parameters.len(), 3);
}

#[test]
fn test_experiment_run_with_parent() {
    let experiment_id = ExperimentId::new();
    let user_id = UserId::new();
    let parent_id = RunId::new();

    let run = ExperimentRun::new(
        experiment_id,
        2,
        "Child Run".to_string(),
        user_id,
    ).with_parent(parent_id);

    assert_eq!(run.parent_run_id, Some(parent_id));
}
