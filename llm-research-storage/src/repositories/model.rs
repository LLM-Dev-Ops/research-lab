use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_research_core::{Model, ModelProvider, Repository, Result};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct ModelRepository {
    pool: PgPool,
}

impl ModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new model
    pub async fn create(&self, model: &Model) -> Result<Model> {
        let provider_str = provider_to_str(&model.provider);

        let row = sqlx::query(
            r#"
            INSERT INTO models (
                id, name, provider, model_identifier, version, config,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, name, provider, model_identifier, version, config,
                      created_at, updated_at
            "#,
        )
        .bind(model.id)
        .bind(&model.name)
        .bind(provider_str)
        .bind(&model.model_identifier)
        .bind(&model.version)
        .bind(&model.config)
        .bind(model.created_at)
        .bind(model.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_model(row))
    }

    /// Get model by ID
    pub async fn get_by_id(&self, id: &Uuid) -> Result<Option<Model>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, provider, model_identifier, version, config,
                   created_at, updated_at
            FROM models
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_model))
    }

    /// Update a model
    pub async fn update(&self, model: &Model) -> Result<Model> {
        let provider_str = provider_to_str(&model.provider);

        let row = sqlx::query(
            r#"
            UPDATE models
            SET name = $2, provider = $3, model_identifier = $4,
                version = $5, config = $6, updated_at = $7
            WHERE id = $1
            RETURNING id, name, provider, model_identifier, version, config,
                      created_at, updated_at
            "#,
        )
        .bind(model.id)
        .bind(&model.name)
        .bind(provider_str)
        .bind(&model.model_identifier)
        .bind(&model.version)
        .bind(&model.config)
        .bind(model.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_model(row))
    }

    /// Delete a model
    pub async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM models WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List models with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Model>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, provider, model_identifier, version, config,
                   created_at, updated_at
            FROM models
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_model).collect())
    }

    /// Filter by provider
    pub async fn filter_by_provider(
        &self,
        provider: ModelProvider,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Model>> {
        let provider_str = provider_to_str(&provider);

        let rows = sqlx::query(
            r#"
            SELECT id, name, provider, model_identifier, version, config,
                   created_at, updated_at
            FROM models
            WHERE provider = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(provider_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_model).collect())
    }

    /// Search models by name
    pub async fn search_by_name(&self, name_query: &str, limit: i64) -> Result<Vec<Model>> {
        let search_pattern = format!("%{}%", name_query);

        let rows = sqlx::query(
            r#"
            SELECT id, name, provider, model_identifier, version, config,
                   created_at, updated_at
            FROM models
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_model).collect())
    }

    /// Count total models
    pub async fn count(&self) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM models")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.unwrap_or(0))
    }
}

#[async_trait]
impl Repository<Model, Uuid> for ModelRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Model>> {
        self.get_by_id(id).await
    }

    async fn save(&self, entity: &Model) -> Result<Model> {
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

// Helper functions
fn provider_to_str(provider: &ModelProvider) -> &'static str {
    match provider {
        ModelProvider::OpenAI => "openai",
        ModelProvider::Anthropic => "anthropic",
        ModelProvider::Google => "google",
        ModelProvider::Cohere => "cohere",
        ModelProvider::HuggingFace => "huggingface",
        ModelProvider::Azure => "azure",
        ModelProvider::AWS => "aws",
        ModelProvider::Local => "local",
        ModelProvider::Custom => "custom",
    }
}

fn str_to_provider(s: &str) -> ModelProvider {
    match s {
        "openai" => ModelProvider::OpenAI,
        "anthropic" => ModelProvider::Anthropic,
        "google" => ModelProvider::Google,
        "cohere" => ModelProvider::Cohere,
        "huggingface" => ModelProvider::HuggingFace,
        "azure" => ModelProvider::Azure,
        "aws" => ModelProvider::AWS,
        "local" => ModelProvider::Local,
        "custom" => ModelProvider::Custom,
        _ => ModelProvider::Custom,
    }
}

fn row_to_model(row: sqlx::postgres::PgRow) -> Model {
    let id: Uuid = row.get("id");
    let name: String = row.get("name");
    let provider_str: String = row.get("provider");
    let model_identifier: String = row.get("model_identifier");
    let version: Option<String> = row.get("version");
    let config: serde_json::Value = row.get("config");
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");

    Model {
        id,
        name,
        provider: str_to_provider(&provider_str),
        model_identifier,
        version,
        config,
        created_at,
        updated_at,
    }
}
