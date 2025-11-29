//! Database indexing strategy and recommendations.
//!
//! This module provides utilities for defining, analyzing, and generating
//! database indexes to optimize query performance. It includes common index
//! patterns for experiments, models, and datasets, as well as tools for
//! generating SQL migration files.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Database index definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// Index name (following naming conventions).
    pub name: String,

    /// Table the index is on.
    pub table: String,

    /// Columns included in the index.
    pub columns: Vec<String>,

    /// Index strategy/type.
    pub strategy: IndexStrategy,

    /// Whether this is a unique index.
    pub unique: bool,

    /// Optional WHERE clause for partial indexes.
    pub where_clause: Option<String>,

    /// Optional index comment/description.
    pub comment: Option<String>,
}

impl IndexDefinition {
    /// Creates a new index definition.
    pub fn new(
        name: impl Into<String>,
        table: impl Into<String>,
        columns: Vec<String>,
        strategy: IndexStrategy,
    ) -> Self {
        Self {
            name: name.into(),
            table: table.into(),
            columns,
            strategy,
            unique: false,
            where_clause: None,
            comment: None,
        }
    }

    /// Creates a standard B-Tree index.
    pub fn btree(
        table: impl Into<String>,
        columns: Vec<String>,
    ) -> Self {
        let table = table.into();
        let name = generate_index_name(&table, &columns, IndexStrategy::BTree);

        Self::new(name, table, columns, IndexStrategy::BTree)
    }

    /// Creates a Hash index.
    pub fn hash(
        table: impl Into<String>,
        columns: Vec<String>,
    ) -> Self {
        let table = table.into();
        let name = generate_index_name(&table, &columns, IndexStrategy::Hash);

        Self::new(name, table, columns, IndexStrategy::Hash)
    }

    /// Creates a GIN (Generalized Inverted Index) for full-text search.
    pub fn gin(
        table: impl Into<String>,
        columns: Vec<String>,
    ) -> Self {
        let table = table.into();
        let name = generate_index_name(&table, &columns, IndexStrategy::GIN);

        Self::new(name, table, columns, IndexStrategy::GIN)
    }

    /// Creates a GiST (Generalized Search Tree) index.
    pub fn gist(
        table: impl Into<String>,
        columns: Vec<String>,
    ) -> Self {
        let table = table.into();
        let name = generate_index_name(&table, &columns, IndexStrategy::GiST);

        Self::new(name, table, columns, IndexStrategy::GiST)
    }

    /// Marks the index as unique.
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Adds a WHERE clause for partial indexing.
    pub fn where_clause(mut self, clause: impl Into<String>) -> Self {
        self.where_clause = Some(clause.into());
        self
    }

    /// Adds a comment to the index.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Generates CREATE INDEX SQL statement.
    pub fn to_sql(&self) -> String {
        let unique_str = if self.unique { "UNIQUE " } else { "" };
        let using_str = match self.strategy {
            IndexStrategy::BTree => "USING btree",
            IndexStrategy::Hash => "USING hash",
            IndexStrategy::GiST => "USING gist",
            IndexStrategy::GIN => "USING gin",
        };

        let columns_str = self.columns.join(", ");
        let mut sql = format!(
            "CREATE {}INDEX {} ON {} {} ({})",
            unique_str, self.name, self.table, using_str, columns_str
        );

        if let Some(where_clause) = &self.where_clause {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        sql.push(';');

        if let Some(comment) = &self.comment {
            sql.push_str(&format!(
                "\nCOMMENT ON INDEX {} IS '{}';",
                self.name, comment
            ));
        }

        sql
    }

    /// Generates DROP INDEX SQL statement.
    pub fn drop_sql(&self) -> String {
        format!("DROP INDEX IF EXISTS {};", self.name)
    }

    /// Returns the estimated size impact (low, medium, high).
    pub fn size_impact(&self) -> SizeImpact {
        match (self.columns.len(), &self.strategy) {
            (1, IndexStrategy::BTree) => SizeImpact::Low,
            (1, IndexStrategy::Hash) => SizeImpact::Low,
            (2..=3, IndexStrategy::BTree) => SizeImpact::Medium,
            (_, IndexStrategy::GIN) => SizeImpact::High,
            (_, IndexStrategy::GiST) => SizeImpact::High,
            _ => SizeImpact::Medium,
        }
    }
}

/// Index strategy/type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexStrategy {
    /// B-Tree index (default, good for most use cases).
    BTree,

    /// Hash index (equality comparisons only).
    Hash,

    /// GiST (Generalized Search Tree) for geometric data.
    GiST,

    /// GIN (Generalized Inverted Index) for full-text search and arrays.
    GIN,
}

impl std::fmt::Display for IndexStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexStrategy::BTree => write!(f, "btree"),
            IndexStrategy::Hash => write!(f, "hash"),
            IndexStrategy::GiST => write!(f, "gist"),
            IndexStrategy::GIN => write!(f, "gin"),
        }
    }
}

/// Index size impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeImpact {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for SizeImpact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeImpact::Low => write!(f, "low"),
            SizeImpact::Medium => write!(f, "medium"),
            SizeImpact::High => write!(f, "high"),
        }
    }
}

/// Index recommendation based on query patterns.
#[derive(Debug, Clone)]
pub struct IndexRecommendation {
    /// Recommended index definition.
    pub index: IndexDefinition,

    /// Reason for the recommendation.
    pub reason: String,

    /// Priority level (1-5, where 5 is highest).
    pub priority: u8,

    /// Query patterns that would benefit from this index.
    pub query_patterns: Vec<String>,
}

impl IndexRecommendation {
    /// Creates a new index recommendation.
    pub fn new(
        index: IndexDefinition,
        reason: impl Into<String>,
        priority: u8,
    ) -> Self {
        Self {
            index,
            reason: reason.into(),
            priority: priority.min(5),
            query_patterns: Vec::new(),
        }
    }

    /// Adds a query pattern that would benefit from this index.
    pub fn add_query_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.query_patterns.push(pattern.into());
        self
    }

    /// Adds multiple query patterns.
    pub fn add_query_patterns(mut self, patterns: Vec<String>) -> Self {
        self.query_patterns.extend(patterns);
        self
    }
}

/// Analyzes query patterns and recommends indexes.
pub struct IndexAnalyzer {
    /// Table schemas for analysis.
    schemas: HashMap<String, TableSchema>,
}

impl IndexAnalyzer {
    /// Creates a new index analyzer.
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Adds a table schema for analysis.
    pub fn add_table(&mut self, schema: TableSchema) {
        self.schemas.insert(schema.name.clone(), schema);
    }

    /// Analyzes a query and recommends indexes.
    pub fn analyze_query(&self, query: &str) -> Vec<IndexRecommendation> {
        let mut recommendations = Vec::new();

        // Simple heuristic-based analysis
        let query_upper = query.to_uppercase();

        // Check for WHERE clauses
        if query_upper.contains("WHERE") {
            // Extract potential column names (simplified)
            if let Some(table) = self.extract_table_name(&query_upper) {
                if let Some(schema) = self.schemas.get(&table) {
                    for column in &schema.columns {
                        if query_upper.contains(&column.to_uppercase()) {
                            let index = IndexDefinition::btree(
                                table.clone(),
                                vec![column.clone()],
                            );

                            recommendations.push(IndexRecommendation::new(
                                index,
                                format!("Column '{}' used in WHERE clause", column),
                                3,
                            ));
                        }
                    }
                }
            }
        }

        recommendations
    }

    /// Extracts table name from query (simplified).
    fn extract_table_name(&self, query: &str) -> Option<String> {
        if let Some(pos) = query.find("FROM ") {
            let after_from = &query[pos + 5..];
            after_from
                .split_whitespace()
                .next()
                .map(|s| s.trim_end_matches(',').to_lowercase())
        } else {
            None
        }
    }
}

impl Default for IndexAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Table schema information.
#[derive(Debug, Clone)]
pub struct TableSchema {
    /// Table name.
    pub name: String,

    /// Column names.
    pub columns: Vec<String>,

    /// Existing indexes.
    pub existing_indexes: Vec<String>,
}

impl TableSchema {
    /// Creates a new table schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            existing_indexes: Vec::new(),
        }
    }

    /// Adds a column to the schema.
    pub fn add_column(mut self, column: impl Into<String>) -> Self {
        self.columns.push(column.into());
        self
    }

    /// Adds multiple columns.
    pub fn add_columns(mut self, columns: Vec<String>) -> Self {
        self.columns.extend(columns);
        self
    }

    /// Adds an existing index.
    pub fn add_index(mut self, index: impl Into<String>) -> Self {
        self.existing_indexes.push(index.into());
        self
    }
}

/// Common index patterns for the LLM research platform.
pub struct CommonIndexPatterns;

impl CommonIndexPatterns {
    /// Returns standard indexes for the experiments table.
    pub fn experiments_indexes() -> Vec<IndexDefinition> {
        vec![
            // Primary lookup indexes
            IndexDefinition::btree("experiments", vec!["id".to_string()]).unique(),
            IndexDefinition::btree("experiments", vec!["name".to_string()]),

            // Status filtering
            IndexDefinition::btree("experiments", vec!["status".to_string()])
                .where_clause("status != 'completed'")
                .with_comment("Index for filtering active experiments"),

            // Date-based queries
            IndexDefinition::btree("experiments", vec!["created_at".to_string()]),
            IndexDefinition::btree("experiments", vec!["updated_at".to_string()]),

            // Composite indexes for common query patterns
            IndexDefinition::btree(
                "experiments",
                vec!["user_id".to_string(), "created_at".to_string()],
            )
            .with_comment("User's experiments ordered by creation date"),

            IndexDefinition::btree(
                "experiments",
                vec!["status".to_string(), "created_at".to_string()],
            )
            .with_comment("Experiments by status and date"),
        ]
    }

    /// Returns standard indexes for the models table.
    pub fn models_indexes() -> Vec<IndexDefinition> {
        vec![
            // Primary lookup
            IndexDefinition::btree("models", vec!["id".to_string()]).unique(),
            IndexDefinition::btree("models", vec!["name".to_string()]),

            // Provider filtering
            IndexDefinition::btree("models", vec!["provider".to_string()]),

            // Model family/version queries
            IndexDefinition::btree("models", vec!["model_family".to_string()]),
            IndexDefinition::btree(
                "models",
                vec!["model_family".to_string(), "version".to_string()],
            ),

            // Full-text search on description
            IndexDefinition::gin("models", vec!["description".to_string()])
                .with_comment("Full-text search on model descriptions"),
        ]
    }

    /// Returns standard indexes for the datasets table.
    pub fn datasets_indexes() -> Vec<IndexDefinition> {
        vec![
            // Primary lookup
            IndexDefinition::btree("datasets", vec!["id".to_string()]).unique(),
            IndexDefinition::btree("datasets", vec!["name".to_string()]),

            // Format filtering
            IndexDefinition::btree("datasets", vec!["format".to_string()]),

            // Size-based queries
            IndexDefinition::btree("datasets", vec!["size_bytes".to_string()]),

            // Version queries
            IndexDefinition::btree(
                "datasets",
                vec!["name".to_string(), "version".to_string()],
            )
            .with_comment("Dataset versions"),

            // Tags search (assuming JSONB column)
            IndexDefinition::gin("datasets", vec!["tags".to_string()])
                .with_comment("GIN index for tag searches"),
        ]
    }

    /// Returns standard indexes for the experiment_runs table.
    pub fn experiment_runs_indexes() -> Vec<IndexDefinition> {
        vec![
            // Primary lookup
            IndexDefinition::btree("experiment_runs", vec!["id".to_string()]).unique(),

            // Foreign key indexes
            IndexDefinition::btree("experiment_runs", vec!["experiment_id".to_string()])
                .with_comment("Lookup runs by experiment"),

            IndexDefinition::btree("experiment_runs", vec!["model_id".to_string()])
                .with_comment("Lookup runs by model"),

            // Status filtering
            IndexDefinition::btree("experiment_runs", vec!["status".to_string()]),

            // Time-based queries
            IndexDefinition::btree("experiment_runs", vec!["started_at".to_string()]),
            IndexDefinition::btree("experiment_runs", vec!["completed_at".to_string()]),

            // Composite for common queries
            IndexDefinition::btree(
                "experiment_runs",
                vec![
                    "experiment_id".to_string(),
                    "status".to_string(),
                    "started_at".to_string(),
                ],
            )
            .with_comment("Experiment runs by status and date"),
        ]
    }

    /// Returns all common indexes.
    pub fn all_indexes() -> Vec<IndexDefinition> {
        let mut indexes = Vec::new();
        indexes.extend(Self::experiments_indexes());
        indexes.extend(Self::models_indexes());
        indexes.extend(Self::datasets_indexes());
        indexes.extend(Self::experiment_runs_indexes());
        indexes
    }
}

/// Generates a SQL migration file for creating indexes.
pub fn generate_migration(indexes: &[IndexDefinition]) -> String {
    let mut migration = String::new();

    migration.push_str("-- Migration: Create indexes for performance optimization\n");
    migration.push_str("-- Generated at: ");
    migration.push_str(&chrono::Utc::now().to_rfc3339());
    migration.push_str("\n\n");

    migration.push_str("-- ==== UP Migration ====\n\n");

    for index in indexes {
        if let Some(comment) = &index.comment {
            migration.push_str(&format!("-- {}\n", comment));
        }
        migration.push_str(&index.to_sql());
        migration.push_str("\n\n");
    }

    migration.push_str("-- ==== DOWN Migration ====\n\n");

    for index in indexes {
        migration.push_str(&index.drop_sql());
        migration.push('\n');
    }

    migration
}

/// Generates an index name following PostgreSQL conventions.
///
/// Format: idx_{table}_{columns}_{strategy}
fn generate_index_name(table: &str, columns: &[String], strategy: IndexStrategy) -> String {
    let columns_str = columns.join("_");
    let strategy_suffix = match strategy {
        IndexStrategy::BTree => "",
        IndexStrategy::Hash => "_hash",
        IndexStrategy::GiST => "_gist",
        IndexStrategy::GIN => "_gin",
    };

    format!("idx_{}_{}{}", table, columns_str, strategy_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_definition_btree() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]);

        assert_eq!(index.table, "users");
        assert_eq!(index.columns, vec!["email".to_string()]);
        assert_eq!(index.strategy, IndexStrategy::BTree);
        assert!(!index.unique);
    }

    #[test]
    fn test_index_definition_unique() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]).unique();

        assert!(index.unique);
    }

    #[test]
    fn test_index_definition_with_where_clause() {
        let index = IndexDefinition::btree("users", vec!["status".to_string()])
            .where_clause("status = 'active'");

        assert_eq!(index.where_clause, Some("status = 'active'".to_string()));
    }

    #[test]
    fn test_index_definition_to_sql_basic() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]);
        let sql = index.to_sql();

        assert!(sql.contains("CREATE INDEX"));
        assert!(sql.contains("idx_users_email"));
        assert!(sql.contains("USING btree"));
        assert!(sql.contains("(email)"));
    }

    #[test]
    fn test_index_definition_to_sql_unique() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]).unique();
        let sql = index.to_sql();

        assert!(sql.contains("CREATE UNIQUE INDEX"));
    }

    #[test]
    fn test_index_definition_to_sql_with_where() {
        let index = IndexDefinition::btree("users", vec!["status".to_string()])
            .where_clause("status = 'active'");
        let sql = index.to_sql();

        assert!(sql.contains("WHERE status = 'active'"));
    }

    #[test]
    fn test_index_definition_drop_sql() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]);
        let sql = index.drop_sql();

        assert_eq!(sql, "DROP INDEX IF EXISTS idx_users_email;");
    }

    #[test]
    fn test_index_size_impact() {
        let btree_single = IndexDefinition::btree("users", vec!["email".to_string()]);
        assert_eq!(btree_single.size_impact(), SizeImpact::Low);

        let btree_composite = IndexDefinition::btree(
            "users",
            vec!["email".to_string(), "name".to_string()],
        );
        assert_eq!(btree_composite.size_impact(), SizeImpact::Medium);

        let gin = IndexDefinition::gin("posts", vec!["content".to_string()]);
        assert_eq!(gin.size_impact(), SizeImpact::High);
    }

    #[test]
    fn test_generate_index_name() {
        assert_eq!(
            generate_index_name("users", &vec!["email".to_string()], IndexStrategy::BTree),
            "idx_users_email"
        );

        assert_eq!(
            generate_index_name("users", &vec!["email".to_string()], IndexStrategy::Hash),
            "idx_users_email_hash"
        );

        assert_eq!(
            generate_index_name(
                "users",
                &vec!["first_name".to_string(), "last_name".to_string()],
                IndexStrategy::BTree
            ),
            "idx_users_first_name_last_name"
        );
    }

    #[test]
    fn test_common_indexes_experiments() {
        let indexes = CommonIndexPatterns::experiments_indexes();

        assert!(!indexes.is_empty());
        assert!(indexes.iter().any(|i| i.table == "experiments" && i.columns.contains(&"id".to_string())));
        assert!(indexes.iter().any(|i| i.table == "experiments" && i.columns.contains(&"status".to_string())));
    }

    #[test]
    fn test_common_indexes_models() {
        let indexes = CommonIndexPatterns::models_indexes();

        assert!(!indexes.is_empty());
        assert!(indexes.iter().any(|i| i.table == "models" && i.strategy == IndexStrategy::GIN));
    }

    #[test]
    fn test_generate_migration() {
        let indexes = vec![
            IndexDefinition::btree("users", vec!["email".to_string()]).unique(),
            IndexDefinition::btree("users", vec!["created_at".to_string()]),
        ];

        let migration = generate_migration(&indexes);

        assert!(migration.contains("-- Migration:"));
        assert!(migration.contains("-- ==== UP Migration ===="));
        assert!(migration.contains("-- ==== DOWN Migration ===="));
        assert!(migration.contains("CREATE UNIQUE INDEX"));
        assert!(migration.contains("DROP INDEX IF EXISTS"));
    }

    #[test]
    fn test_table_schema_builder() {
        let schema = TableSchema::new("users")
            .add_column("id")
            .add_column("email")
            .add_column("created_at")
            .add_index("idx_users_email");

        assert_eq!(schema.name, "users");
        assert_eq!(schema.columns.len(), 3);
        assert_eq!(schema.existing_indexes.len(), 1);
    }

    #[test]
    fn test_index_analyzer_creation() {
        let analyzer = IndexAnalyzer::new();
        assert!(analyzer.schemas.is_empty());
    }

    #[test]
    fn test_index_recommendation_creation() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]);
        let recommendation = IndexRecommendation::new(
            index,
            "Frequently used in WHERE clauses",
            4,
        );

        assert_eq!(recommendation.priority, 4);
        assert_eq!(recommendation.reason, "Frequently used in WHERE clauses");
    }

    #[test]
    fn test_index_recommendation_priority_clamping() {
        let index = IndexDefinition::btree("users", vec!["email".to_string()]);
        let recommendation = IndexRecommendation::new(index, "Test", 10);

        assert_eq!(recommendation.priority, 5); // Clamped to max
    }
}
