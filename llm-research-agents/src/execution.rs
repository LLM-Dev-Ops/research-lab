//! Agentics Execution Span System
//!
//! This module implements the Foundational Execution Unit contract for the
//! Agentics execution system. Every externally-invoked operation MUST emit
//! execution spans that integrate into the hierarchical ExecutionGraph
//! produced by a Core orchestrator.
//!
//! # Invariants (NON-NEGOTIABLE)
//!
//! The following hierarchy MUST always hold:
//!
//! ```text
//! Core
//!   └─ Repo (this repo)
//!       └─ Agent (one or more)
//! ```
//!
//! - This repo MUST NOT execute silently (without spans).
//! - Every agent MUST have its own span.
//! - Artifacts MUST be attached at agent or repo level only.
//! - `parent_span_id` MUST be provided by the caller (Core).
//! - If no agent span exists, execution is INVALID.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The canonical name of this repository in the Agentics execution system.
pub const REPO_NAME: &str = "llm-research-lab";

// ---------------------------------------------------------------------------
// Execution Context
// ---------------------------------------------------------------------------

/// Execution context provided by the Core orchestrator.
///
/// Every externally-invoked operation MUST receive this context.
/// Execution MUST be rejected if `parent_span_id` is missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique identifier for this execution run.
    pub execution_id: Uuid,
    /// Span ID of the Core-level span that triggered this repo execution.
    /// This becomes the parent of the repo-level span.
    pub parent_span_id: Uuid,
}

// ---------------------------------------------------------------------------
// Span Enums
// ---------------------------------------------------------------------------

/// Type of execution span.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    /// Repo-level span (one per repo invocation).
    Repo,
    /// Agent-level span (one per agent execution).
    Agent,
}

/// Status of an execution span.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpanStatus {
    /// Span is currently executing.
    Running,
    /// Span completed successfully.
    Completed,
    /// Span failed.
    Failed,
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

/// An artifact produced during agent execution.
///
/// Artifacts MUST be attached to the agent-level span that produced them.
/// They MUST include a stable reference (ID, URI, hash, or filename).
/// Evidence MUST be machine-verifiable and MUST NOT be inferred or synthesized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    /// Stable reference ID for this artifact.
    pub id: String,
    /// Optional URI where the artifact can be retrieved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Content hash for integrity verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Filename if the artifact is file-based.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Type classification of the artifact.
    pub artifact_type: String,
    /// Machine-verifiable evidence data.
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Execution Span
// ---------------------------------------------------------------------------

/// An execution span within the Agentics ExecutionGraph.
///
/// Spans form an append-only, causally ordered tree via `parent_span_id`.
/// The structure MUST be JSON-serializable without loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpan {
    /// Unique identifier for this span.
    pub span_id: Uuid,
    /// ID of the parent span (Core span for repo, repo span for agent).
    pub parent_span_id: Uuid,
    /// Whether this is a repo-level or agent-level span.
    #[serde(rename = "type")]
    pub span_type: SpanType,
    /// Status of this span.
    pub status: SpanStatus,
    /// Repository name (present on all spans).
    pub repo_name: String,
    /// Agent name (present on agent-level spans only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// When this span started.
    pub start_time: DateTime<Utc>,
    /// When this span ended (None if still running).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    /// Failure reason(s), if status is FAILED.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// Artifacts produced by this span.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ExecutionArtifact>,
    /// Nested child spans (agent spans nested under repo span).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ExecutionSpan>,
}

impl ExecutionSpan {
    /// Create a new repo-level span.
    ///
    /// `parent_span_id` is the Core-level span that triggered this execution.
    pub fn new_repo(parent_span_id: Uuid) -> Self {
        Self {
            span_id: Uuid::new_v4(),
            parent_span_id,
            span_type: SpanType::Repo,
            status: SpanStatus::Running,
            repo_name: REPO_NAME.to_string(),
            agent_name: None,
            start_time: Utc::now(),
            end_time: None,
            failure_reason: None,
            artifacts: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Create a new agent-level span nested under a repo span.
    ///
    /// `parent_span_id` is the repo-level span_id.
    pub fn new_agent(parent_span_id: Uuid, agent_name: &str) -> Self {
        Self {
            span_id: Uuid::new_v4(),
            parent_span_id,
            span_type: SpanType::Agent,
            status: SpanStatus::Running,
            repo_name: REPO_NAME.to_string(),
            agent_name: Some(agent_name.to_string()),
            start_time: Utc::now(),
            end_time: None,
            failure_reason: None,
            artifacts: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Create a failed repo span for context-validation errors.
    ///
    /// Used when execution is rejected before any work starts (e.g., missing
    /// parent_span_id). Uses `Uuid::nil()` as the parent since no valid
    /// context was provided.
    pub fn rejected(reason: String) -> Self {
        let now = Utc::now();
        Self {
            span_id: Uuid::new_v4(),
            parent_span_id: Uuid::nil(),
            span_type: SpanType::Repo,
            status: SpanStatus::Failed,
            repo_name: REPO_NAME.to_string(),
            agent_name: None,
            start_time: now,
            end_time: Some(now),
            failure_reason: Some(reason),
            artifacts: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Mark this span as completed.
    pub fn complete(&mut self) {
        self.status = SpanStatus::Completed;
        self.end_time = Some(Utc::now());
    }

    /// Mark this span as failed with a reason.
    pub fn fail(&mut self, reason: String) {
        self.status = SpanStatus::Failed;
        self.end_time = Some(Utc::now());
        self.failure_reason = Some(reason);
    }

    /// Attach an artifact to this span.
    pub fn add_artifact(&mut self, artifact: ExecutionArtifact) {
        self.artifacts.push(artifact);
    }

    /// Add a child span (agent span under repo span).
    pub fn add_child(&mut self, child: ExecutionSpan) {
        self.children.push(child);
    }

    /// Validate that this repo span satisfies the agent-span invariant.
    ///
    /// A repo span MUST have at least one agent child span.
    /// Every agent span MUST have a name.
    pub fn validate_agent_spans(&self) -> Result<(), ExecutionError> {
        if self.span_type == SpanType::Repo && self.children.is_empty() {
            return Err(ExecutionError::NoAgentSpans);
        }
        for child in &self.children {
            if child.span_type != SpanType::Agent {
                continue;
            }
            if child.agent_name.is_none() {
                return Err(ExecutionError::AgentWithoutSpan {
                    agent_name: "<unnamed>".to_string(),
                });
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Execution Result (Output Contract)
// ---------------------------------------------------------------------------

/// The complete execution output from this repository.
///
/// This is the top-level response envelope that wraps the business result
/// with the mandatory execution span hierarchy.
///
/// # Output Contract
///
/// - MUST include repo-level span
/// - MUST include nested agent-level spans
/// - MUST include artifacts and evidence at correct levels
/// - MUST be JSON-serializable without loss
/// - Span structure MUST be append-only and causally ordered via parent_span_id
#[derive(Debug, Serialize)]
pub struct ExecutionResult<T: Serialize> {
    /// The execution ID from the incoming context.
    pub execution_id: Uuid,
    /// The repo-level span with nested agent spans.
    pub repo_span: ExecutionSpan,
    /// The business result (None if execution failed before producing output).
    pub result: Option<T>,
}

impl<T: Serialize> ExecutionResult<T> {
    /// Create an error result for when execution is rejected before any
    /// business logic runs (e.g., missing execution context).
    pub fn rejected(reason: String) -> Self {
        Self {
            execution_id: Uuid::nil(),
            repo_span: ExecutionSpan::rejected(reason),
            result: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors related to execution context and span enforcement.
#[derive(Debug, thiserror::Error, Serialize)]
pub enum ExecutionError {
    /// Execution context was not provided.
    #[error("Missing execution_context: every externally-invoked operation requires an execution context with execution_id and parent_span_id")]
    MissingExecutionContext,

    /// parent_span_id was not provided or is invalid.
    #[error("Missing or invalid parent_span_id in execution context")]
    MissingParentSpanId,

    /// No agent-level spans were emitted during execution.
    #[error("Execution INVALID: no agent-level spans were emitted")]
    NoAgentSpans,

    /// An agent executed without emitting a span.
    #[error("Agent '{agent_name}' executed without emitting a span")]
    AgentWithoutSpan {
        /// Name of the agent that violated the span requirement.
        agent_name: String,
    },
}

// ---------------------------------------------------------------------------
// Validation Helpers
// ---------------------------------------------------------------------------

/// Validate an execution context, rejecting if it is absent or if
/// `parent_span_id` is nil.
pub fn validate_execution_context(
    ctx: &Option<ExecutionContext>,
) -> Result<&ExecutionContext, ExecutionError> {
    let ctx = ctx.as_ref().ok_or(ExecutionError::MissingExecutionContext)?;
    if ctx.parent_span_id.is_nil() {
        return Err(ExecutionError::MissingParentSpanId);
    }
    Ok(ctx)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_repo_span_creation() {
        let parent = Uuid::new_v4();
        let span = ExecutionSpan::new_repo(parent);

        assert_eq!(span.parent_span_id, parent);
        assert_eq!(span.span_type, SpanType::Repo);
        assert_eq!(span.status, SpanStatus::Running);
        assert_eq!(span.repo_name, REPO_NAME);
        assert!(span.agent_name.is_none());
        assert!(span.end_time.is_none());
        assert!(span.children.is_empty());
    }

    #[test]
    fn test_agent_span_creation() {
        let parent = Uuid::new_v4();
        let span = ExecutionSpan::new_agent(parent, "hypothesis-agent");

        assert_eq!(span.parent_span_id, parent);
        assert_eq!(span.span_type, SpanType::Agent);
        assert_eq!(span.status, SpanStatus::Running);
        assert_eq!(span.agent_name.as_deref(), Some("hypothesis-agent"));
    }

    #[test]
    fn test_span_complete() {
        let mut span = ExecutionSpan::new_repo(Uuid::new_v4());
        assert!(span.end_time.is_none());

        span.complete();

        assert_eq!(span.status, SpanStatus::Completed);
        assert!(span.end_time.is_some());
    }

    #[test]
    fn test_span_fail() {
        let mut span = ExecutionSpan::new_repo(Uuid::new_v4());
        span.fail("something broke".to_string());

        assert_eq!(span.status, SpanStatus::Failed);
        assert!(span.end_time.is_some());
        assert_eq!(span.failure_reason.as_deref(), Some("something broke"));
    }

    #[test]
    fn test_validate_agent_spans_empty_children() {
        let span = ExecutionSpan::new_repo(Uuid::new_v4());
        let result = span.validate_agent_spans();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_agent_spans_with_agent() {
        let mut repo_span = ExecutionSpan::new_repo(Uuid::new_v4());
        let agent_span = ExecutionSpan::new_agent(repo_span.span_id, "test-agent");
        repo_span.add_child(agent_span);

        assert!(repo_span.validate_agent_spans().is_ok());
    }

    #[test]
    fn test_artifact_attachment() {
        let mut span = ExecutionSpan::new_agent(Uuid::new_v4(), "test-agent");
        span.add_artifact(ExecutionArtifact {
            id: "artifact-1".to_string(),
            uri: None,
            hash: Some("abc123".to_string()),
            filename: None,
            artifact_type: "decision_event".to_string(),
            data: serde_json::json!({"event_id": "123"}),
        });

        assert_eq!(span.artifacts.len(), 1);
        assert_eq!(span.artifacts[0].id, "artifact-1");
    }

    #[test]
    fn test_execution_context_validation_missing() {
        let ctx: Option<ExecutionContext> = None;
        assert!(validate_execution_context(&ctx).is_err());
    }

    #[test]
    fn test_execution_context_validation_nil_parent() {
        let ctx = Some(ExecutionContext {
            execution_id: Uuid::new_v4(),
            parent_span_id: Uuid::nil(),
        });
        assert!(validate_execution_context(&ctx).is_err());
    }

    #[test]
    fn test_execution_context_validation_valid() {
        let ctx = Some(ExecutionContext {
            execution_id: Uuid::new_v4(),
            parent_span_id: Uuid::new_v4(),
        });
        assert!(validate_execution_context(&ctx).is_ok());
    }

    #[test]
    fn test_span_json_serialization() {
        let mut repo_span = ExecutionSpan::new_repo(Uuid::new_v4());
        let mut agent_span = ExecutionSpan::new_agent(repo_span.span_id, "test-agent");
        agent_span.add_artifact(ExecutionArtifact {
            id: "artifact-1".to_string(),
            uri: None,
            hash: None,
            filename: None,
            artifact_type: "test".to_string(),
            data: serde_json::json!({"key": "value"}),
        });
        agent_span.complete();
        repo_span.add_child(agent_span);
        repo_span.complete();

        let json = serde_json::to_string_pretty(&repo_span)
            .expect("Span must be JSON-serializable without loss");

        assert!(json.contains("\"type\": \"repo\""));
        assert!(json.contains("\"type\": \"agent\""));
        assert!(json.contains("\"test-agent\""));
        assert!(json.contains("\"COMPLETED\""));
    }

    #[test]
    fn test_execution_result_serialization() {
        let parent = Uuid::new_v4();
        let mut repo_span = ExecutionSpan::new_repo(parent);
        let mut agent_span = ExecutionSpan::new_agent(repo_span.span_id, "test-agent");
        agent_span.complete();
        repo_span.add_child(agent_span);
        repo_span.complete();

        let result = ExecutionResult {
            execution_id: Uuid::new_v4(),
            repo_span,
            result: Some(serde_json::json!({"status": "ok"})),
        };

        let json = serde_json::to_string(&result)
            .expect("ExecutionResult must be JSON-serializable");
        assert!(json.contains("execution_id"));
        assert!(json.contains("repo_span"));
        assert!(json.contains("result"));
    }

    #[test]
    fn test_execution_result_rejected() {
        let result = ExecutionResult::<serde_json::Value>::rejected(
            "no context".to_string(),
        );

        assert!(result.execution_id.is_nil());
        assert_eq!(result.repo_span.status, SpanStatus::Failed);
        assert!(result.result.is_none());
    }

    #[test]
    fn test_rejected_span_includes_all_emitted_spans_on_failure() {
        let mut repo_span = ExecutionSpan::new_repo(Uuid::new_v4());
        let mut agent_span = ExecutionSpan::new_agent(repo_span.span_id, "failing-agent");
        agent_span.fail("computation error".to_string());
        repo_span.add_child(agent_span);
        repo_span.fail("agent failure".to_string());

        // On failure, still return all emitted spans
        assert_eq!(repo_span.children.len(), 1);
        assert_eq!(repo_span.status, SpanStatus::Failed);
        assert_eq!(repo_span.children[0].status, SpanStatus::Failed);
    }
}
