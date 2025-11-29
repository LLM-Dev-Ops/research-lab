use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_research_core::{Evaluation, Repository, Result};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct EvaluationRepository {
    pool: PgPool,
}

impl EvaluationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new evaluation
    pub async fn create(&self, evaluation: &Evaluation) -> Result<Evaluation> {
        let row = sqlx::query(
            r#"
            INSERT INTO evaluations (
                id, experiment_run_id, sample_id, input, output, expected_output,
                latency_ms, token_count, cost, metrics, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, experiment_run_id, sample_id, input, output, expected_output,
                      latency_ms, token_count, cost, metrics, created_at
            "#
        )
        .bind(&evaluation.id)
        .bind(&evaluation.experiment_id)
        .bind(&evaluation.sample_id)
        .bind(&evaluation.input)
        .bind(&evaluation.output)
        .bind(&evaluation.expected_output)
        .bind(&evaluation.latency_ms)
        .bind(&evaluation.token_count)
        .bind(&evaluation.cost)
        .bind(&evaluation.metrics)
        .bind(&evaluation.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_evaluation(row))
    }

    /// Get evaluation by ID
    pub async fn get_by_id(&self, id: &Uuid) -> Result<Option<Evaluation>> {
        let row = sqlx::query(
            r#"
            SELECT id, experiment_run_id, sample_id, input, output, expected_output,
                   latency_ms, token_count, cost, metrics, created_at
            FROM evaluations
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_evaluation))
    }

    /// Delete an evaluation
    pub async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM evaluations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List evaluations for a run
    pub async fn list_for_run(
        &self,
        run_id: &Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Evaluation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, experiment_run_id, sample_id, input, output, expected_output,
                   latency_ms, token_count, cost, metrics, created_at
            FROM evaluations
            WHERE experiment_run_id = $1
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(run_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_evaluation).collect())
    }

    /// Count evaluations for a run
    pub async fn count_for_run(&self, run_id: &Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM evaluations WHERE experiment_run_id = $1"
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    /// Get aggregated metrics for a run
    pub async fn get_aggregated_metrics(
        &self,
        run_id: &Uuid,
    ) -> Result<EvaluationAggregates> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as "count!",
                AVG(latency_ms) as avg_latency,
                MIN(latency_ms) as min_latency,
                MAX(latency_ms) as max_latency,
                SUM(token_count) as total_tokens,
                SUM(cost) as total_cost
            FROM evaluations
            WHERE experiment_run_id = $1
            "#
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(EvaluationAggregates {
            count: row.get("count!"),
            avg_latency_ms: row.get("avg_latency"),
            min_latency_ms: row.get("min_latency"),
            max_latency_ms: row.get("max_latency"),
            total_tokens: row.get::<Option<i64>, _>("total_tokens").unwrap_or(0),
            total_cost: row.get("total_cost"),
        })
    }
}

#[async_trait]
impl Repository<Evaluation, Uuid> for EvaluationRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Evaluation>> {
        self.get_by_id(id).await
    }

    async fn save(&self, entity: &Evaluation) -> Result<Evaluation> {
        // Evaluations are typically insert-only
        self.create(entity).await
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        self.delete(id).await
    }
}

#[derive(Debug)]
pub struct EvaluationAggregates {
    pub count: i64,
    pub avg_latency_ms: Option<f64>,
    pub min_latency_ms: Option<i64>,
    pub max_latency_ms: Option<i64>,
    pub total_tokens: i64,
    pub total_cost: Option<Decimal>,
}

fn row_to_evaluation<T>(row: T) -> Evaluation
where
    T: EvaluationRowLike,
{
    Evaluation {
        id: row.get_id(),
        experiment_id: row.get_experiment_run_id(),
        sample_id: row.get_sample_id(),
        input: row.get_input(),
        output: row.get_output(),
        expected_output: row.get_expected_output(),
        latency_ms: row.get_latency_ms(),
        token_count: row.get_token_count(),
        cost: row.get_cost(),
        metrics: row.get_metrics(),
        created_at: row.get_created_at(),
    }
}

trait EvaluationRowLike {
    fn get_id(&self) -> Uuid;
    fn get_experiment_run_id(&self) -> Uuid;
    fn get_sample_id(&self) -> Uuid;
    fn get_input(&self) -> String;
    fn get_output(&self) -> String;
    fn get_expected_output(&self) -> Option<String>;
    fn get_latency_ms(&self) -> i64;
    fn get_token_count(&self) -> i32;
    fn get_cost(&self) -> Option<Decimal>;
    fn get_metrics(&self) -> serde_json::Value;
    fn get_created_at(&self) -> DateTime<Utc>;
}

impl EvaluationRowLike for sqlx::postgres::PgRow {
    fn get_id(&self) -> Uuid {
        use sqlx::Row;
        self.get("id")
    }
    fn get_experiment_run_id(&self) -> Uuid {
        use sqlx::Row;
        self.get("experiment_run_id")
    }
    fn get_sample_id(&self) -> Uuid {
        use sqlx::Row;
        self.get("sample_id")
    }
    fn get_input(&self) -> String {
        use sqlx::Row;
        self.get("input")
    }
    fn get_output(&self) -> String {
        use sqlx::Row;
        self.get("output")
    }
    fn get_expected_output(&self) -> Option<String> {
        use sqlx::Row;
        self.get("expected_output")
    }
    fn get_latency_ms(&self) -> i64 {
        use sqlx::Row;
        self.get("latency_ms")
    }
    fn get_token_count(&self) -> i32 {
        use sqlx::Row;
        self.get("token_count")
    }
    fn get_cost(&self) -> Option<Decimal> {
        use sqlx::Row;
        self.get("cost")
    }
    fn get_metrics(&self) -> serde_json::Value {
        use sqlx::Row;
        self.get("metrics")
    }
    fn get_created_at(&self) -> DateTime<Utc> {
        use sqlx::Row;
        self.get("created_at")
    }
}
