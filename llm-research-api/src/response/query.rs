//! Query optimization utilities for building efficient database queries.
//!
//! This module provides tools for constructing optimized SQL queries with
//! dynamic filtering, sorting, field selection, and performance tracking.
//! It includes a slow query logger middleware for identifying performance
//! bottlenecks.

use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::request::Parts,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Query builder for constructing optimized SQL queries.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: String,
    fields: Vec<String>,
    filters: Vec<FilterSpec>,
    sorts: Vec<SortSpec>,
    limit: Option<usize>,
    offset: Option<usize>,
    joins: Vec<JoinClause>,
}

impl QueryBuilder {
    /// Creates a new query builder for the specified table.
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            fields: vec!["*".to_string()],
            filters: Vec::new(),
            sorts: Vec::new(),
            limit: None,
            offset: None,
            joins: Vec::new(),
        }
    }

    /// Selects specific fields.
    pub fn select(mut self, fields: Vec<String>) -> Self {
        if !fields.is_empty() {
            self.fields = fields;
        }
        self
    }

    /// Adds a filter condition.
    pub fn filter(mut self, filter: FilterSpec) -> Self {
        self.filters.push(filter);
        self
    }

    /// Adds multiple filter conditions.
    pub fn filters(mut self, filters: Vec<FilterSpec>) -> Self {
        self.filters.extend(filters);
        self
    }

    /// Adds a sort specification.
    pub fn sort(mut self, sort: SortSpec) -> Self {
        self.sorts.push(sort);
        self
    }

    /// Adds multiple sort specifications.
    pub fn sorts(mut self, sorts: Vec<SortSpec>) -> Self {
        self.sorts.extend(sorts);
        self
    }

    /// Sets the limit for results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the offset for results.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Adds a JOIN clause.
    pub fn join(mut self, join: JoinClause) -> Self {
        self.joins.push(join);
        self
    }

    /// Builds the SQL query string.
    pub fn build(&self) -> String {
        let mut query = format!("SELECT {} FROM {}", self.fields.join(", "), self.table);

        // Add JOINs
        for join in &self.joins {
            query.push_str(&format!(
                " {} JOIN {} ON {}",
                join.join_type, join.table, join.condition
            ));
        }

        // Add WHERE clause
        if !self.filters.is_empty() {
            let conditions: Vec<String> = self.filters.iter().map(|f| f.to_sql()).collect();
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        // Add ORDER BY clause
        if !self.sorts.is_empty() {
            let order_clauses: Vec<String> = self.sorts.iter().map(|s| s.to_sql()).collect();
            query.push_str(&format!(" ORDER BY {}", order_clauses.join(", ")));
        }

        // Add LIMIT and OFFSET
        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        query
    }

    /// Returns the table name.
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Returns the selected fields.
    pub fn selected_fields(&self) -> &[String] {
        &self.fields
    }
}

/// JOIN clause specification.
#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub condition: String,
}

impl JoinClause {
    /// Creates a new JOIN clause.
    pub fn new(join_type: JoinType, table: impl Into<String>, condition: impl Into<String>) -> Self {
        Self {
            join_type,
            table: table.into(),
            condition: condition.into(),
        }
    }

    /// Creates an INNER JOIN.
    pub fn inner(table: impl Into<String>, condition: impl Into<String>) -> Self {
        Self::new(JoinType::Inner, table, condition)
    }

    /// Creates a LEFT JOIN.
    pub fn left(table: impl Into<String>, condition: impl Into<String>) -> Self {
        Self::new(JoinType::Left, table, condition)
    }

    /// Creates a RIGHT JOIN.
    pub fn right(table: impl Into<String>, condition: impl Into<String>) -> Self {
        Self::new(JoinType::Right, table, condition)
    }
}

/// JOIN types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl std::fmt::Display for JoinType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER"),
            JoinType::Left => write!(f, "LEFT"),
            JoinType::Right => write!(f, "RIGHT"),
            JoinType::Full => write!(f, "FULL"),
        }
    }
}

/// Filter specification for dynamic query filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSpec {
    /// Field to filter on.
    pub field: String,

    /// Filter operator.
    pub operator: FilterOperator,

    /// Filter value (stored as string, converted as needed).
    pub value: String,
}

impl FilterSpec {
    /// Creates a new filter specification.
    pub fn new(field: impl Into<String>, operator: FilterOperator, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            operator,
            value: value.into(),
        }
    }

    /// Creates an equality filter.
    pub fn eq(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::Eq, value)
    }

    /// Creates a not-equal filter.
    pub fn ne(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::Ne, value)
    }

    /// Creates a greater-than filter.
    pub fn gt(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::Gt, value)
    }

    /// Creates a less-than filter.
    pub fn lt(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::Lt, value)
    }

    /// Creates a LIKE filter.
    pub fn like(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::Like, pattern)
    }

    /// Creates an IN filter.
    pub fn in_list(field: impl Into<String>, values: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::In, values)
    }

    /// Converts the filter to a SQL WHERE clause fragment.
    pub fn to_sql(&self) -> String {
        let sanitized_field = sanitize_identifier(&self.field);

        match self.operator {
            FilterOperator::Eq => format!("{} = '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::Ne => format!("{} != '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::Gt => format!("{} > '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::Gte => format!("{} >= '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::Lt => format!("{} < '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::Lte => format!("{} <= '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::Like => format!("{} LIKE '{}'", sanitized_field, sanitize_value(&self.value)),
            FilterOperator::In => format!("{} IN ({})", sanitized_field, self.value),
            FilterOperator::NotIn => format!("{} NOT IN ({})", sanitized_field, self.value),
            FilterOperator::IsNull => format!("{} IS NULL", sanitized_field),
            FilterOperator::IsNotNull => format!("{} IS NOT NULL", sanitized_field),
        }
    }
}

/// Filter operators for query conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    In,
    NotIn,
    IsNull,
    IsNotNull,
}

/// Sort specification for query ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    /// Field to sort by.
    pub field: String,

    /// Sort direction.
    pub direction: SortDirection,
}

impl SortSpec {
    /// Creates a new sort specification.
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            field: field.into(),
            direction,
        }
    }

    /// Creates an ascending sort.
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Asc)
    }

    /// Creates a descending sort.
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Desc)
    }

    /// Converts the sort to a SQL ORDER BY clause fragment.
    pub fn to_sql(&self) -> String {
        format!("{} {}", sanitize_identifier(&self.field), self.direction)
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl std::fmt::Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortDirection::Asc => write!(f, "ASC"),
            SortDirection::Desc => write!(f, "DESC"),
        }
    }
}

/// Field selection for partial responses (field projection).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldSelection {
    /// Fields to include in the response.
    pub fields: HashSet<String>,
}

impl FieldSelection {
    /// Creates a new field selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field to the selection.
    pub fn add(mut self, field: impl Into<String>) -> Self {
        self.fields.insert(field.into());
        self
    }

    /// Adds multiple fields to the selection.
    pub fn add_many(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for field in fields {
            self.fields.insert(field.into());
        }
        self
    }

    /// Checks if a field is selected.
    pub fn contains(&self, field: &str) -> bool {
        self.fields.is_empty() || self.fields.contains(field)
    }

    /// Returns true if all fields should be included.
    pub fn is_all(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the fields as a vector.
    pub fn to_vec(&self) -> Vec<String> {
        if self.is_all() {
            vec!["*".to_string()]
        } else {
            self.fields.iter().cloned().collect()
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for FieldSelection
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query_params: HashMap<String, String> = parts
            .uri
            .query()
            .map(|q| {
                url::form_urlencoded::parse(q.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_default();

        let mut selection = FieldSelection::new();

        if let Some(fields_str) = query_params.get("fields") {
            for field in fields_str.split(',') {
                let field = field.trim();
                if !field.is_empty() {
                    selection = selection.add(field);
                }
            }
        }

        Ok(selection)
    }
}

/// Query optimizer for analyzing and optimizing queries.
#[derive(Debug, Clone)]
pub struct QueryOptimizer {
    /// Threshold for slow query warnings (in milliseconds).
    slow_query_threshold: Duration,
}

impl QueryOptimizer {
    /// Creates a new query optimizer.
    pub fn new(slow_query_threshold: Duration) -> Self {
        Self {
            slow_query_threshold,
        }
    }

    /// Analyzes a query and provides optimization hints.
    pub fn analyze(&self, query: &str) -> Vec<OptimizationHint> {
        let mut hints = Vec::new();

        // Check for missing WHERE clause on large tables
        if !query.to_uppercase().contains("WHERE") && !query.to_uppercase().contains("LIMIT") {
            hints.push(OptimizationHint::MissingWhereClause);
        }

        // Check for SELECT *
        if query.contains("SELECT *") {
            hints.push(OptimizationHint::SelectStar);
        }

        // Check for missing LIMIT
        if !query.to_uppercase().contains("LIMIT") {
            hints.push(OptimizationHint::MissingLimit);
        }

        // Check for OR conditions (may not use indexes efficiently)
        if query.to_uppercase().contains(" OR ") {
            hints.push(OptimizationHint::OrCondition);
        }

        hints
    }

    /// Returns the slow query threshold.
    pub fn slow_query_threshold(&self) -> Duration {
        self.slow_query_threshold
    }
}

/// Optimization hints for query improvement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationHint {
    /// Query is missing a WHERE clause.
    MissingWhereClause,

    /// Query uses SELECT *.
    SelectStar,

    /// Query is missing a LIMIT clause.
    MissingLimit,

    /// Query contains OR conditions that may not use indexes.
    OrCondition,

    /// Suggest using an index on a specific field.
    SuggestIndex(String),

    /// Query may cause a full table scan.
    PotentialFullTableScan,
}

impl std::fmt::Display for OptimizationHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimizationHint::MissingWhereClause => {
                write!(f, "Consider adding a WHERE clause to filter results")
            }
            OptimizationHint::SelectStar => {
                write!(f, "Avoid SELECT * and specify only needed columns")
            }
            OptimizationHint::MissingLimit => {
                write!(f, "Consider adding a LIMIT clause to constrain results")
            }
            OptimizationHint::OrCondition => {
                write!(f, "OR conditions may not use indexes efficiently")
            }
            OptimizationHint::SuggestIndex(field) => {
                write!(f, "Consider adding an index on field: {}", field)
            }
            OptimizationHint::PotentialFullTableScan => {
                write!(f, "Query may cause a full table scan")
            }
        }
    }
}

/// Slow query logger for tracking query performance.
#[derive(Debug, Clone)]
pub struct SlowQueryLogger {
    threshold: Duration,
}

impl SlowQueryLogger {
    /// Creates a new slow query logger with the given threshold.
    pub fn new(threshold: Duration) -> Self {
        Self { threshold }
    }

    /// Logs a query execution if it exceeds the threshold.
    pub fn log_query(&self, query: &str, duration: Duration, result: &str) {
        if duration >= self.threshold {
            warn!(
                target: "slow_query",
                query = %query,
                duration_ms = duration.as_millis(),
                result = %result,
                "Slow query detected"
            );
        } else {
            info!(
                target: "query",
                query = %query,
                duration_ms = duration.as_millis(),
                result = %result,
                "Query executed"
            );
        }
    }

    /// Tracks query execution and logs if slow.
    pub async fn track<F, T, E>(&self, query: &str, f: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        let start = Instant::now();
        let result = f.await;
        let duration = start.elapsed();

        let result_str = if result.is_ok() { "success" } else { "error" };
        self.log_query(query, duration, result_str);

        result
    }

    /// Returns the slow query threshold.
    pub fn threshold(&self) -> Duration {
        self.threshold
    }
}

/// Configuration for slow query logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryConfig {
    /// Threshold in milliseconds for considering a query slow.
    pub threshold_ms: u64,

    /// Whether to enable slow query logging.
    pub enabled: bool,
}

impl Default for SlowQueryConfig {
    fn default() -> Self {
        Self {
            threshold_ms: 1000, // 1 second
            enabled: true,
        }
    }
}

impl SlowQueryConfig {
    /// Converts to a Duration.
    pub fn threshold(&self) -> Duration {
        Duration::from_millis(self.threshold_ms)
    }

    /// Creates a SlowQueryLogger from this config.
    pub fn logger(&self) -> Option<SlowQueryLogger> {
        if self.enabled {
            Some(SlowQueryLogger::new(self.threshold()))
        } else {
            None
        }
    }
}

/// Sanitizes a SQL identifier (table/column name) to prevent SQL injection.
fn sanitize_identifier(identifier: &str) -> String {
    identifier
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Sanitizes a SQL value to prevent SQL injection.
fn sanitize_value(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_basic() {
        let query = QueryBuilder::new("users")
            .select(vec!["id".to_string(), "name".to_string()])
            .build();

        assert_eq!(query, "SELECT id, name FROM users");
    }

    #[test]
    fn test_query_builder_with_filter() {
        let query = QueryBuilder::new("users")
            .filter(FilterSpec::eq("status", "active"))
            .build();

        assert_eq!(query, "SELECT * FROM users WHERE status = 'active'");
    }

    #[test]
    fn test_query_builder_with_multiple_filters() {
        let query = QueryBuilder::new("users")
            .filter(FilterSpec::eq("status", "active"))
            .filter(FilterSpec::gt("age", "18"))
            .build();

        assert_eq!(query, "SELECT * FROM users WHERE status = 'active' AND age > '18'");
    }

    #[test]
    fn test_query_builder_with_sort() {
        let query = QueryBuilder::new("users")
            .sort(SortSpec::desc("created_at"))
            .build();

        assert_eq!(query, "SELECT * FROM users ORDER BY created_at DESC");
    }

    #[test]
    fn test_query_builder_with_limit_offset() {
        let query = QueryBuilder::new("users")
            .limit(10)
            .offset(20)
            .build();

        assert_eq!(query, "SELECT * FROM users LIMIT 10 OFFSET 20");
    }

    #[test]
    fn test_query_builder_with_join() {
        let query = QueryBuilder::new("users")
            .join(JoinClause::inner("posts", "posts.user_id = users.id"))
            .build();

        assert_eq!(query, "SELECT * FROM users INNER JOIN posts ON posts.user_id = users.id");
    }

    #[test]
    fn test_filter_spec_operators() {
        assert_eq!(FilterSpec::eq("id", "1").to_sql(), "id = '1'");
        assert_eq!(FilterSpec::ne("id", "1").to_sql(), "id != '1'");
        assert_eq!(FilterSpec::gt("age", "18").to_sql(), "age > '18'");
        assert_eq!(FilterSpec::lt("age", "65").to_sql(), "age < '65'");
        assert_eq!(FilterSpec::like("name", "%John%").to_sql(), "name LIKE '%John%'");
    }

    #[test]
    fn test_sort_spec_directions() {
        assert_eq!(SortSpec::asc("name").to_sql(), "name ASC");
        assert_eq!(SortSpec::desc("created_at").to_sql(), "created_at DESC");
    }

    #[test]
    fn test_field_selection_empty() {
        let selection = FieldSelection::new();
        assert!(selection.is_all());
        assert!(selection.contains("any_field"));
    }

    #[test]
    fn test_field_selection_specific_fields() {
        let selection = FieldSelection::new()
            .add("id")
            .add("name")
            .add("email");

        assert!(!selection.is_all());
        assert!(selection.contains("id"));
        assert!(selection.contains("name"));
        assert!(!selection.contains("password"));
    }

    #[test]
    fn test_field_selection_to_vec() {
        let selection = FieldSelection::new();
        assert_eq!(selection.to_vec(), vec!["*".to_string()]);

        let selection = FieldSelection::new().add("id").add("name");
        let vec = selection.to_vec();
        assert!(vec.contains(&"id".to_string()));
        assert!(vec.contains(&"name".to_string()));
    }

    #[test]
    fn test_query_optimizer_analyze_missing_where() {
        let optimizer = QueryOptimizer::new(Duration::from_secs(1));
        let hints = optimizer.analyze("SELECT * FROM users");

        assert!(hints.contains(&OptimizationHint::MissingWhereClause));
        assert!(hints.contains(&OptimizationHint::SelectStar));
        assert!(hints.contains(&OptimizationHint::MissingLimit));
    }

    #[test]
    fn test_query_optimizer_analyze_or_condition() {
        let optimizer = QueryOptimizer::new(Duration::from_secs(1));
        let hints = optimizer.analyze("SELECT * FROM users WHERE status = 'active' OR status = 'pending'");

        assert!(hints.contains(&OptimizationHint::OrCondition));
    }

    #[test]
    fn test_slow_query_logger_threshold() {
        let logger = SlowQueryLogger::new(Duration::from_millis(500));
        assert_eq!(logger.threshold(), Duration::from_millis(500));
    }

    #[test]
    fn test_slow_query_config_default() {
        let config = SlowQueryConfig::default();
        assert_eq!(config.threshold_ms, 1000);
        assert!(config.enabled);
    }

    #[test]
    fn test_slow_query_config_logger() {
        let config = SlowQueryConfig::default();
        let logger = config.logger();
        assert!(logger.is_some());

        let config = SlowQueryConfig {
            threshold_ms: 1000,
            enabled: false,
        };
        let logger = config.logger();
        assert!(logger.is_none());
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("users"), "users");
        assert_eq!(sanitize_identifier("user_id"), "user_id");
        assert_eq!(sanitize_identifier("users; DROP TABLE users;"), "usersDROPTABLEusers");
    }

    #[test]
    fn test_sanitize_value() {
        assert_eq!(sanitize_value("normal"), "normal");
        assert_eq!(sanitize_value("O'Brien"), "O''Brien");
        assert_eq!(sanitize_value("'; DROP TABLE users; --"), "''; DROP TABLE users; --");
    }
}
