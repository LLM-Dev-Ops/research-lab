use async_trait::async_trait;
use llm_research_core::domain::{
    Experiment, ExperimentStatus, ExperimentConfig,
    ids::{ExperimentId, UserId},
};
use llm_research_core::{Repository, Result};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct ExperimentRepository {
    pool: PgPool,
}

impl ExperimentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new experiment
    pub async fn create(&self, experiment: &Experiment) -> Result<Experiment> {
        let status_str = status_to_str(&experiment.status);
        let config_json = serde_json::to_value(&experiment.config)?;
        let metadata_json = serde_json::to_value(&experiment.metadata)?;
        let collaborators: Vec<Uuid> = experiment.collaborators.iter().map(|id| id.0).collect();

        let row = sqlx::query(
            r#"
            INSERT INTO experiments (
                id, name, description, hypothesis, owner_id, collaborators, tags,
                status, config, created_at, updated_at, archived_at, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::experiment_status, $9, $10, $11, $12, $13)
            RETURNING id, name, description, hypothesis, owner_id, collaborators, tags,
                      status, config, created_at, updated_at, archived_at, metadata
            "#,
        )
        .bind(experiment.id.0)
        .bind(&experiment.name)
        .bind(&experiment.description)
        .bind(&experiment.hypothesis)
        .bind(experiment.owner_id.0)
        .bind(&collaborators)
        .bind(&experiment.tags)
        .bind(status_str)
        .bind(config_json)
        .bind(experiment.created_at)
        .bind(experiment.updated_at)
        .bind(experiment.archived_at)
        .bind(metadata_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_experiment(row))
    }

    /// Get experiment by ID
    pub async fn get_by_id(&self, id: &ExperimentId) -> Result<Option<Experiment>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                   status, config, created_at, updated_at, archived_at, metadata
            FROM experiments
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_experiment))
    }

    /// Update an experiment
    pub async fn update(&self, experiment: &Experiment) -> Result<Experiment> {
        let status_str = status_to_str(&experiment.status);
        let config_json = serde_json::to_value(&experiment.config)?;
        let metadata_json = serde_json::to_value(&experiment.metadata)?;
        let collaborators: Vec<Uuid> = experiment.collaborators.iter().map(|id| id.0).collect();

        let row = sqlx::query(
            r#"
            UPDATE experiments
            SET name = $2, description = $3, hypothesis = $4, owner_id = $5,
                collaborators = $6, tags = $7, status = $8::experiment_status,
                config = $9, updated_at = $10, archived_at = $11, metadata = $12
            WHERE id = $1
            RETURNING id, name, description, hypothesis, owner_id, collaborators, tags,
                      status, config, created_at, updated_at, archived_at, metadata
            "#,
        )
        .bind(experiment.id.0)
        .bind(&experiment.name)
        .bind(&experiment.description)
        .bind(&experiment.hypothesis)
        .bind(experiment.owner_id.0)
        .bind(&collaborators)
        .bind(&experiment.tags)
        .bind(status_str)
        .bind(config_json)
        .bind(experiment.updated_at)
        .bind(experiment.archived_at)
        .bind(metadata_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_experiment(row))
    }

    /// Delete an experiment
    pub async fn delete(&self, id: &ExperimentId) -> Result<()> {
        sqlx::query("DELETE FROM experiments WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List experiments with pagination
    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Experiment>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                   status, config, created_at, updated_at, archived_at, metadata
            FROM experiments
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_experiment).collect())
    }

    /// List experiments with cursor-based pagination
    pub async fn list_cursor(
        &self,
        limit: i64,
        cursor: Option<ExperimentId>,
    ) -> Result<Vec<Experiment>> {
        let rows = if let Some(cursor_id) = cursor {
            // Get the created_at timestamp of the cursor
            let cursor_time: Option<DateTime<Utc>> = sqlx::query_scalar(
                "SELECT created_at FROM experiments WHERE id = $1"
            )
            .bind(cursor_id.0)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(time) = cursor_time {
                sqlx::query(
                    r#"
                    SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                           status, config, created_at, updated_at, archived_at, metadata
                    FROM experiments
                    WHERE created_at < $1 OR (created_at = $1 AND id < $2)
                    ORDER BY created_at DESC, id DESC
                    LIMIT $3
                    "#,
                )
                .bind(time)
                .bind(cursor_id.0)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            } else {
                Vec::new()
            }
        } else {
            sqlx::query(
                r#"
                SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                       status, config, created_at, updated_at, archived_at, metadata
                FROM experiments
                ORDER BY created_at DESC, id DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Self::row_to_experiment).collect())
    }

    /// Search experiments by name
    pub async fn search_by_name(&self, name_query: &str, limit: i64) -> Result<Vec<Experiment>> {
        let search_pattern = format!("%{}%", name_query);
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                   status, config, created_at, updated_at, archived_at, metadata
            FROM experiments
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_experiment).collect())
    }

    /// Filter experiments by status
    pub async fn filter_by_status(
        &self,
        status: ExperimentStatus,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Experiment>> {
        let status_str = status_to_str(&status);

        let rows = sqlx::query(
            r#"
            SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                   status, config, created_at, updated_at, archived_at, metadata
            FROM experiments
            WHERE status = $1::experiment_status
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_experiment).collect())
    }

    /// Filter experiments by owner
    pub async fn filter_by_owner(
        &self,
        owner_id: &UserId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Experiment>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                   status, config, created_at, updated_at, archived_at, metadata
            FROM experiments
            WHERE owner_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(owner_id.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_experiment).collect())
    }

    /// Filter experiments by tags
    pub async fn filter_by_tags(
        &self,
        tags: &[String],
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Experiment>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, hypothesis, owner_id, collaborators, tags,
                   status, config, created_at, updated_at, archived_at, metadata
            FROM experiments
            WHERE tags && $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tags)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_experiment).collect())
    }

    /// Count total experiments
    pub async fn count(&self) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM experiments")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.unwrap_or(0))
    }

    /// Count experiments by status
    pub async fn count_by_status(&self, status: ExperimentStatus) -> Result<i64> {
        let status_str = status_to_str(&status);

        let count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM experiments WHERE status = $1::experiment_status"
        )
        .bind(status_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    /// Helper function to convert database row to Experiment
    fn row_to_experiment(row: sqlx::postgres::PgRow) -> Experiment {
        let id: Uuid = row.get("id");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let hypothesis: Option<String> = row.get("hypothesis");
        let owner_id: Uuid = row.get("owner_id");
        let collaborators_uuids: Vec<Uuid> = row.get("collaborators");
        let tags: Vec<String> = row.get("tags");
        let status_str: String = row.get("status");
        let config_json: serde_json::Value = row.get("config");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");
        let archived_at: Option<DateTime<Utc>> = row.get("archived_at");
        let metadata_json: serde_json::Value = row.get("metadata");

        let status = str_to_status(&status_str);
        let collaborators: Vec<UserId> = collaborators_uuids.into_iter().map(UserId).collect();
        let config: ExperimentConfig = serde_json::from_value(config_json)
            .unwrap_or_else(|_| ExperimentConfig::default());
        let metadata: HashMap<String, serde_json::Value> = serde_json::from_value(metadata_json)
            .unwrap_or_else(|_| HashMap::new());

        Experiment {
            id: ExperimentId(id),
            name,
            description,
            hypothesis,
            owner_id: UserId(owner_id),
            collaborators,
            tags,
            status,
            config,
            created_at,
            updated_at,
            archived_at,
            metadata,
        }
    }
}

#[async_trait]
impl Repository<Experiment, ExperimentId> for ExperimentRepository {
    async fn find_by_id(&self, id: &ExperimentId) -> Result<Option<Experiment>> {
        self.get_by_id(id).await
    }

    async fn save(&self, entity: &Experiment) -> Result<Experiment> {
        // Try to find existing experiment
        if self.get_by_id(&entity.id).await?.is_some() {
            self.update(entity).await
        } else {
            self.create(entity).await
        }
    }

    async fn delete(&self, id: &ExperimentId) -> Result<()> {
        self.delete(id).await
    }
}

// Helper functions for status conversion
fn status_to_str(status: &ExperimentStatus) -> &'static str {
    match status {
        ExperimentStatus::Draft => "draft",
        ExperimentStatus::Active => "active",
        ExperimentStatus::Paused => "paused",
        ExperimentStatus::Completed => "completed",
        ExperimentStatus::Archived => "archived",
        ExperimentStatus::Failed => "failed",
    }
}

fn str_to_status(s: &str) -> ExperimentStatus {
    match s {
        "draft" => ExperimentStatus::Draft,
        "active" => ExperimentStatus::Active,
        "paused" => ExperimentStatus::Paused,
        "completed" => ExperimentStatus::Completed,
        "archived" => ExperimentStatus::Archived,
        "failed" => ExperimentStatus::Failed,
        _ => ExperimentStatus::Draft,
    }
}
