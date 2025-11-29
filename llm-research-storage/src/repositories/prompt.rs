use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_research_core::{PromptTemplate, Repository, Result};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PromptTemplateRepository {
    pool: PgPool,
}

impl PromptTemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new prompt template
    pub async fn create(&self, prompt: &PromptTemplate) -> Result<PromptTemplate> {
        let row = sqlx::query(
            r#"
            INSERT INTO prompt_templates (
                id, name, description, template, variables, version,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, name, description, template, variables, version,
                      created_at, updated_at
            "#
        )
        .bind(&prompt.id)
        .bind(&prompt.name)
        .bind(&prompt.description)
        .bind(&prompt.template)
        .bind(&prompt.variables)
        .bind(&prompt.version)
        .bind(&prompt.created_at)
        .bind(&prompt.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_prompt(row))
    }

    /// Get prompt template by ID
    pub async fn get_by_id(&self, id: &Uuid) -> Result<Option<PromptTemplate>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, template, variables, version,
                   created_at, updated_at
            FROM prompt_templates
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_prompt))
    }

    /// Update a prompt template
    pub async fn update(&self, prompt: &PromptTemplate) -> Result<PromptTemplate> {
        let row = sqlx::query(
            r#"
            UPDATE prompt_templates
            SET name = $2, description = $3, template = $4,
                variables = $5, version = $6, updated_at = $7
            WHERE id = $1
            RETURNING id, name, description, template, variables, version,
                      created_at, updated_at
            "#
        )
        .bind(&prompt.id)
        .bind(&prompt.name)
        .bind(&prompt.description)
        .bind(&prompt.template)
        .bind(&prompt.variables)
        .bind(&prompt.version)
        .bind(&prompt.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_prompt(row))
    }

    /// Delete a prompt template
    pub async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM prompt_templates WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List prompt templates with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<PromptTemplate>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, template, variables, version,
                   created_at, updated_at
            FROM prompt_templates
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_prompt).collect())
    }

    /// Search prompt templates by name
    pub async fn search_by_name(&self, name_query: &str, limit: i64) -> Result<Vec<PromptTemplate>> {
        let search_pattern = format!("%{}%", name_query);

        let rows = sqlx::query(
            r#"
            SELECT id, name, description, template, variables, version,
                   created_at, updated_at
            FROM prompt_templates
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_prompt).collect())
    }

    /// Count total prompt templates
    pub async fn count(&self) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM prompt_templates")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.unwrap_or(0))
    }
}

#[async_trait]
impl Repository<PromptTemplate, Uuid> for PromptTemplateRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<PromptTemplate>> {
        self.get_by_id(id).await
    }

    async fn save(&self, entity: &PromptTemplate) -> Result<PromptTemplate> {
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

fn row_to_prompt<T>(row: T) -> PromptTemplate
where
    T: PromptRowLike,
{
    PromptTemplate {
        id: row.get_id(),
        name: row.get_name(),
        description: row.get_description(),
        template: row.get_template(),
        variables: row.get_variables(),
        version: row.get_version(),
        created_at: row.get_created_at(),
        updated_at: row.get_updated_at(),
    }
}

trait PromptRowLike {
    fn get_id(&self) -> Uuid;
    fn get_name(&self) -> String;
    fn get_description(&self) -> Option<String>;
    fn get_template(&self) -> String;
    fn get_variables(&self) -> Vec<String>;
    fn get_version(&self) -> i32;
    fn get_created_at(&self) -> DateTime<Utc>;
    fn get_updated_at(&self) -> DateTime<Utc>;
}

impl PromptRowLike for sqlx::postgres::PgRow {
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
    fn get_template(&self) -> String {
        use sqlx::Row;
        self.get("template")
    }
    fn get_variables(&self) -> Vec<String> {
        use sqlx::Row;
        self.get("variables")
    }
    fn get_version(&self) -> i32 {
        use sqlx::Row;
        self.get("version")
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
