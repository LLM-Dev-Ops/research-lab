use anyhow::Result;
use chrono::{DateTime, Utc};
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single metric data point
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct MetricPoint {
    pub run_id: Uuid,
    pub metric_name: String,
    pub value: f64,
    pub step: u64,
    pub timestamp: DateTime<Utc>,
}

/// Time range for querying metrics
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Aggregated metric statistics
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct MetricAggregation {
    pub metric_name: String,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p95: f64,
    pub p99: f64,
    pub count: u64,
}

/// Repository for managing metric time-series data
pub struct MetricTimeSeriesRepository {
    client: Client,
}

impl MetricTimeSeriesRepository {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Initialize the metrics table
    pub async fn init(&self) -> Result<()> {
        self.client
            .query(
                r#"
                CREATE TABLE IF NOT EXISTS experiment_metrics (
                    run_id UUID,
                    metric_name String,
                    value Float64,
                    step UInt64,
                    timestamp DateTime64(3)
                )
                ENGINE = MergeTree()
                PARTITION BY toYYYYMM(timestamp)
                ORDER BY (run_id, metric_name, timestamp)
                "#,
            )
            .execute()
            .await?;

        tracing::info!("Initialized experiment_metrics table");
        Ok(())
    }

    /// Insert a single metric point
    pub async fn insert_metric_point(&self, point: &MetricPoint) -> Result<()> {
        let mut insert = self.client.insert("experiment_metrics")?;
        insert.write(point).await?;
        insert.end().await?;

        tracing::debug!(
            "Inserted metric point: run={}, metric={}, value={}, step={}",
            point.run_id,
            point.metric_name,
            point.value,
            point.step
        );
        Ok(())
    }

    /// Insert multiple metric points in a batch
    pub async fn insert_metric_points(&self, points: &[MetricPoint]) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }

        let mut insert = self.client.insert("experiment_metrics")?;
        for point in points {
            insert.write(point).await?;
        }
        insert.end().await?;

        tracing::info!("Inserted {} metric points", points.len());
        Ok(())
    }

    /// Query metrics by run_id and optional metric name
    pub async fn query_metrics(
        &self,
        run_id: Uuid,
        metric_name: Option<&str>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<MetricPoint>> {
        let mut query = format!(
            "SELECT run_id, metric_name, value, step, timestamp
             FROM experiment_metrics
             WHERE run_id = '{}'",
            run_id
        );

        if let Some(name) = metric_name {
            query.push_str(&format!(" AND metric_name = '{}'", name));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" ORDER BY timestamp ASC");

        let metrics = self.client.query(&query).fetch_all::<MetricPoint>().await?;

        tracing::debug!(
            "Queried {} metrics for run_id={}",
            metrics.len(),
            run_id
        );
        Ok(metrics)
    }

    /// Query metrics for multiple runs
    pub async fn query_metrics_for_runs(
        &self,
        run_ids: &[Uuid],
        metric_name: Option<&str>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<MetricPoint>> {
        if run_ids.is_empty() {
            return Ok(Vec::new());
        }

        let run_ids_str = run_ids
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(", ");

        let mut query = format!(
            "SELECT run_id, metric_name, value, step, timestamp
             FROM experiment_metrics
             WHERE run_id IN ({})",
            run_ids_str
        );

        if let Some(name) = metric_name {
            query.push_str(&format!(" AND metric_name = '{}'", name));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" ORDER BY run_id, timestamp ASC");

        let metrics = self.client.query(&query).fetch_all::<MetricPoint>().await?;

        tracing::debug!("Queried {} metrics for {} runs", metrics.len(), run_ids.len());
        Ok(metrics)
    }

    /// Compute aggregations for metrics
    pub async fn aggregate_metrics(
        &self,
        run_id: Uuid,
        metric_name: Option<&str>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<MetricAggregation>> {
        let mut query = format!(
            "SELECT
                metric_name,
                min(value) as min,
                max(value) as max,
                avg(value) as avg,
                quantile(0.95)(value) as p95,
                quantile(0.99)(value) as p99,
                count() as count
             FROM experiment_metrics
             WHERE run_id = '{}'",
            run_id
        );

        if let Some(name) = metric_name {
            query.push_str(&format!(" AND metric_name = '{}'", name));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" GROUP BY metric_name ORDER BY metric_name");

        let aggregations = self
            .client
            .query(&query)
            .fetch_all::<MetricAggregation>()
            .await?;

        tracing::debug!(
            "Computed {} metric aggregations for run_id={}",
            aggregations.len(),
            run_id
        );
        Ok(aggregations)
    }

    /// Aggregate metrics across multiple runs
    pub async fn aggregate_metrics_for_runs(
        &self,
        run_ids: &[Uuid],
        metric_name: Option<&str>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<MetricAggregation>> {
        if run_ids.is_empty() {
            return Ok(Vec::new());
        }

        let run_ids_str = run_ids
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(", ");

        let mut query = format!(
            "SELECT
                metric_name,
                min(value) as min,
                max(value) as max,
                avg(value) as avg,
                quantile(0.95)(value) as p95,
                quantile(0.99)(value) as p99,
                count() as count
             FROM experiment_metrics
             WHERE run_id IN ({})",
            run_ids_str
        );

        if let Some(name) = metric_name {
            query.push_str(&format!(" AND metric_name = '{}'", name));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" GROUP BY metric_name ORDER BY metric_name");

        let aggregations = self
            .client
            .query(&query)
            .fetch_all::<MetricAggregation>()
            .await?;

        tracing::debug!(
            "Computed {} metric aggregations across {} runs",
            aggregations.len(),
            run_ids.len()
        );
        Ok(aggregations)
    }

    /// Delete metrics for a specific run
    pub async fn delete_metrics(&self, run_id: Uuid) -> Result<()> {
        self.client
            .query(&format!(
                "ALTER TABLE experiment_metrics DELETE WHERE run_id = '{}'",
                run_id
            ))
            .execute()
            .await?;

        tracing::info!("Deleted metrics for run_id={}", run_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metric_point_creation() {
        let point = MetricPoint {
            run_id: Uuid::new_v4(),
            metric_name: "accuracy".to_string(),
            value: 0.95,
            step: 100,
            timestamp: Utc::now(),
        };

        assert_eq!(point.metric_name, "accuracy");
        assert_eq!(point.value, 0.95);
        assert_eq!(point.step, 100);
    }
}
