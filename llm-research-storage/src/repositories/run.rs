use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use llm_research_core::domain::{
    ExperimentRun, RunStatus,
    ids::{RunId, ExperimentId, UserId},
    config::ParameterValue,
    run::{EnvironmentSnapshot, RunMetrics, ArtifactRef, LogSummary, RunError},
};
use llm_research_core::Result;
use std::collections::HashMap;

pub struct RunRepository {
    pool: PgPool,
}

impl RunRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new experiment run
    pub async fn create(&self, run: &ExperimentRun) -> Result<ExperimentRun> {
        let status_str = run_status_to_str(&run.status);
        let parameters_json = serde_json::to_value(&run.parameters)?;
        let environment_json = serde_json::to_value(&run.environment)?;
        let metrics_json = serde_json::to_value(&run.metrics)?;
        let artifacts_json = serde_json::to_value(&run.artifacts)?;
        let logs_json = serde_json::to_value(&run.logs)?;
        let error_json = serde_json::to_value(&run.error)?;
        let metadata_json = serde_json::to_value(&run.metadata)?;

        let row = sqlx::query(
            r#"
            INSERT INTO experiment_runs (
                id, experiment_id, run_number, name, status, parameters,
                environment, metrics, artifacts, logs, parent_run_id, tags,
                started_at, ended_at, created_at, created_by, error, metadata
            )
            VALUES ($1, $2, $3, $4, $5::run_status, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id, experiment_id, run_number, name, status, parameters,
                      environment, metrics, artifacts, logs, parent_run_id, tags,
                      started_at, ended_at, created_at, created_by, error, metadata
            "#,
        )
        .bind(run.id.0)
        .bind(run.experiment_id.0)
        .bind(run.run_number as i32)
        .bind(&run.name)
        .bind(status_str)
        .bind(parameters_json)
        .bind(environment_json)
        .bind(metrics_json)
        .bind(artifacts_json)
        .bind(logs_json)
        .bind(run.parent_run_id.map(|id| id.0))
        .bind(&run.tags)
        .bind(run.started_at)
        .bind(run.ended_at)
        .bind(run.created_at)
        .bind(run.created_by.0)
        .bind(error_json)
        .bind(metadata_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_run(row))
    }

    /// Get run by ID
    pub async fn get_by_id(&self, id: &RunId) -> Result<Option<ExperimentRun>> {
        let row = sqlx::query(
            r#"
            SELECT id, experiment_id, run_number, name, status, parameters,
                   environment, metrics, artifacts, logs, parent_run_id, tags,
                   started_at, ended_at, created_at, created_by, error, metadata
            FROM experiment_runs
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_run))
    }

    /// Update an experiment run
    pub async fn update(&self, run: &ExperimentRun) -> Result<ExperimentRun> {
        let status_str = run_status_to_str(&run.status);
        let parameters_json = serde_json::to_value(&run.parameters)?;
        let environment_json = serde_json::to_value(&run.environment)?;
        let metrics_json = serde_json::to_value(&run.metrics)?;
        let artifacts_json = serde_json::to_value(&run.artifacts)?;
        let logs_json = serde_json::to_value(&run.logs)?;
        let error_json = serde_json::to_value(&run.error)?;
        let metadata_json = serde_json::to_value(&run.metadata)?;

        let row = sqlx::query(
            r#"
            UPDATE experiment_runs
            SET name = $2, status = $3::run_status, parameters = $4,
                environment = $5, metrics = $6, artifacts = $7, logs = $8,
                parent_run_id = $9, tags = $10, started_at = $11, ended_at = $12,
                error = $13, metadata = $14
            WHERE id = $1
            RETURNING id, experiment_id, run_number, name, status, parameters,
                      environment, metrics, artifacts, logs, parent_run_id, tags,
                      started_at, ended_at, created_at, created_by, error, metadata
            "#,
        )
        .bind(run.id.0)
        .bind(&run.name)
        .bind(status_str)
        .bind(parameters_json)
        .bind(environment_json)
        .bind(metrics_json)
        .bind(artifacts_json)
        .bind(logs_json)
        .bind(run.parent_run_id.map(|id| id.0))
        .bind(&run.tags)
        .bind(run.started_at)
        .bind(run.ended_at)
        .bind(error_json)
        .bind(metadata_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_run(row))
    }

    /// Delete a run
    pub async fn delete(&self, id: &RunId) -> Result<()> {
        sqlx::query("DELETE FROM experiment_runs WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get all runs for an experiment
    pub async fn get_runs_for_experiment(
        &self,
        experiment_id: &ExperimentId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ExperimentRun>> {
        let rows = sqlx::query(
            r#"
            SELECT id, experiment_id, run_number, name, status, parameters,
                   environment, metrics, artifacts, logs, parent_run_id, tags,
                   started_at, ended_at, created_at, created_by, error, metadata
            FROM experiment_runs
            WHERE experiment_id = $1
            ORDER BY run_number DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(experiment_id.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_run).collect())
    }

    /// Get the next run number for an experiment
    pub async fn get_next_run_number(&self, experiment_id: &ExperimentId) -> Result<u32> {
        let next_num: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(MAX(run_number), 0) + 1 FROM experiment_runs WHERE experiment_id = $1"
        )
        .bind(experiment_id.0)
        .fetch_one(&self.pool)
        .await?;

        Ok(next_num.unwrap_or(1) as u32)
    }

    /// Get latest run for an experiment
    pub async fn get_latest_run(&self, experiment_id: &ExperimentId) -> Result<Option<ExperimentRun>> {
        let row = sqlx::query(
            r#"
            SELECT id, experiment_id, run_number, name, status, parameters,
                   environment, metrics, artifacts, logs, parent_run_id, tags,
                   started_at, ended_at, created_at, created_by, error, metadata
            FROM experiment_runs
            WHERE experiment_id = $1
            ORDER BY run_number DESC
            LIMIT 1
            "#,
        )
        .bind(experiment_id.0)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_run))
    }

    /// Count runs for an experiment
    pub async fn count_for_experiment(&self, experiment_id: &ExperimentId) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM experiment_runs WHERE experiment_id = $1"
        )
        .bind(experiment_id.0)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }
}

// Helper functions
fn run_status_to_str(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Pending => "pending",
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Completed => "completed",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
        RunStatus::TimedOut => "timedout",
    }
}

fn str_to_run_status(s: &str) -> RunStatus {
    match s {
        "pending" => RunStatus::Pending,
        "queued" => RunStatus::Queued,
        "running" => RunStatus::Running,
        "completed" => RunStatus::Completed,
        "failed" => RunStatus::Failed,
        "cancelled" => RunStatus::Cancelled,
        "timedout" => RunStatus::TimedOut,
        _ => RunStatus::Pending,
    }
}

fn row_to_run(row: sqlx::postgres::PgRow) -> ExperimentRun {
    let id: Uuid = row.get("id");
    let experiment_id: Uuid = row.get("experiment_id");
    let run_number: i32 = row.get("run_number");
    let name: String = row.get("name");
    let status_str: String = row.get("status");
    let parameters_json: serde_json::Value = row.get("parameters");
    let environment_json: serde_json::Value = row.get("environment");
    let metrics_json: serde_json::Value = row.get("metrics");
    let artifacts_json: serde_json::Value = row.get("artifacts");
    let logs_json: serde_json::Value = row.get("logs");
    let parent_run_id: Option<Uuid> = row.get("parent_run_id");
    let tags: Vec<String> = row.get("tags");
    let started_at: Option<DateTime<Utc>> = row.get("started_at");
    let ended_at: Option<DateTime<Utc>> = row.get("ended_at");
    let created_at: DateTime<Utc> = row.get("created_at");
    let created_by: Uuid = row.get("created_by");
    let error_json: serde_json::Value = row.get("error");
    let metadata_json: serde_json::Value = row.get("metadata");

    let status = str_to_run_status(&status_str);
    let parameters: HashMap<String, ParameterValue> = serde_json::from_value(parameters_json)
        .unwrap_or_else(|_| HashMap::new());
    let environment: Option<EnvironmentSnapshot> = serde_json::from_value(environment_json)
        .ok();
    let metrics: RunMetrics = serde_json::from_value(metrics_json)
        .unwrap_or_else(|_| RunMetrics::default());
    let artifacts: Vec<ArtifactRef> = serde_json::from_value(artifacts_json)
        .unwrap_or_else(|_| Vec::new());
    let logs: LogSummary = serde_json::from_value(logs_json)
        .unwrap_or_else(|_| LogSummary::default());
    let error: Option<RunError> = serde_json::from_value(error_json)
        .ok();
    let metadata: HashMap<String, serde_json::Value> = serde_json::from_value(metadata_json)
        .unwrap_or_else(|_| HashMap::new());

    ExperimentRun {
        id: RunId(id),
        experiment_id: ExperimentId(experiment_id),
        run_number: run_number as u32,
        name,
        status,
        parameters,
        environment,
        metrics,
        artifacts,
        logs,
        parent_run_id: parent_run_id.map(RunId),
        tags,
        started_at,
        ended_at,
        created_at,
        created_by: UserId(created_by),
        error,
        metadata,
    }
}
