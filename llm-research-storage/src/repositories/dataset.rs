use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_research_core::{Dataset, Repository, Result};
use sqlx::PgPool;
use uuid::Uuid;

pub struct DatasetRepository {
    pool: PgPool,
}

impl DatasetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new dataset
    pub async fn create(&self, dataset: &Dataset) -> Result<Dataset> {
        let row = sqlx::query(
            r#"
            INSERT INTO datasets (
                id, name, description, s3_path, sample_count, schema,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, name, description, s3_path, sample_count, schema,
                      created_at, updated_at
            "#
        )
        .bind(&dataset.id)
        .bind(&dataset.name)
        .bind(&dataset.description)
        .bind(&dataset.s3_path)
        .bind(&dataset.sample_count)
        .bind(&dataset.schema)
        .bind(&dataset.created_at)
        .bind(&dataset.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_dataset(row))
    }

    /// Get dataset by ID
    pub async fn get_by_id(&self, id: &Uuid) -> Result<Option<Dataset>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, s3_path, sample_count, schema,
                   created_at, updated_at
            FROM datasets
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_dataset))
    }

    /// Update a dataset
    pub async fn update(&self, dataset: &Dataset) -> Result<Dataset> {
        let row = sqlx::query(
            r#"
            UPDATE datasets
            SET name = $2, description = $3, s3_path = $4,
                sample_count = $5, schema = $6, updated_at = $7
            WHERE id = $1
            RETURNING id, name, description, s3_path, sample_count, schema,
                      created_at, updated_at
            "#
        )
        .bind(&dataset.id)
        .bind(&dataset.name)
        .bind(&dataset.description)
        .bind(&dataset.s3_path)
        .bind(&dataset.sample_count)
        .bind(&dataset.schema)
        .bind(&dataset.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_dataset(row))
    }

    /// Delete a dataset
    pub async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM datasets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List datasets with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Dataset>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, s3_path, sample_count, schema,
                   created_at, updated_at
            FROM datasets
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_dataset).collect())
    }

    /// Search datasets by name
    pub async fn search_by_name(&self, name_query: &str, limit: i64) -> Result<Vec<Dataset>> {
        let search_pattern = format!("%{}%", name_query);

        let rows = sqlx::query(
            r#"
            SELECT id, name, description, s3_path, sample_count, schema,
                   created_at, updated_at
            FROM datasets
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_dataset).collect())
    }

    /// Count total datasets
    pub async fn count(&self) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM datasets")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.unwrap_or(0))
    }
}

#[async_trait]
impl Repository<Dataset, Uuid> for DatasetRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Dataset>> {
        self.get_by_id(id).await
    }

    async fn save(&self, entity: &Dataset) -> Result<Dataset> {
        if self.get_by_id(&entity.id).await?.is_some() {
            self.update(entity).await
        } else {
            self.create(entity).await
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        self.delete(id).await
    }
}

fn row_to_dataset<T>(row: T) -> Dataset
where
    T: DatasetRowLike,
{
    Dataset {
        id: row.get_id(),
        name: row.get_name(),
        description: row.get_description(),
        s3_path: row.get_s3_path(),
        sample_count: row.get_sample_count(),
        schema: row.get_schema(),
        created_at: row.get_created_at(),
        updated_at: row.get_updated_at(),
    }
}

trait DatasetRowLike {
    fn get_id(&self) -> Uuid;
    fn get_name(&self) -> String;
    fn get_description(&self) -> Option<String>;
    fn get_s3_path(&self) -> String;
    fn get_sample_count(&self) -> i64;
    fn get_schema(&self) -> serde_json::Value;
    fn get_created_at(&self) -> DateTime<Utc>;
    fn get_updated_at(&self) -> DateTime<Utc>;
}

impl DatasetRowLike for sqlx::postgres::PgRow {
    fn get_id(&self) -> Uuid {
        use sqlx::Row;
        self.get("id")
    }
    fn get_name(&self) -> String {
        use sqlx::Row;
        self.get("name")
    }
    fn get_description(&self) -> Option<String> {
        use sqlx::Row;
        self.get("description")
    }
    fn get_s3_path(&self) -> String {
        use sqlx::Row;
        self.get("s3_path")
    }
    fn get_sample_count(&self) -> i64 {
        use sqlx::Row;
        self.get("sample_count")
    }
    fn get_schema(&self) -> serde_json::Value {
        use sqlx::Row;
        self.get("schema")
    }
    fn get_created_at(&self) -> DateTime<Utc> {
        use sqlx::Row;
        self.get("created_at")
    }
    fn get_updated_at(&self) -> DateTime<Utc> {
        use sqlx::Row;
        self.get("updated_at")
    }
}
