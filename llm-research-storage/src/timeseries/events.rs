use anyhow::Result;
use chrono::{DateTime, Utc};
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Event types for tracking experiment lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    ExperimentCreated,
    ExperimentStarted,
    ExperimentCompleted,
    ExperimentFailed,
    ExperimentCancelled,
    RunCreated,
    RunStarted,
    RunCompleted,
    RunFailed,
    RunCancelled,
    CheckpointCreated,
    ArtifactUploaded,
    Custom(String),
}

impl EventType {
    pub fn as_str(&self) -> &str {
        match self {
            EventType::ExperimentCreated => "experiment_created",
            EventType::ExperimentStarted => "experiment_started",
            EventType::ExperimentCompleted => "experiment_completed",
            EventType::ExperimentFailed => "experiment_failed",
            EventType::ExperimentCancelled => "experiment_cancelled",
            EventType::RunCreated => "run_created",
            EventType::RunStarted => "run_started",
            EventType::RunCompleted => "run_completed",
            EventType::RunFailed => "run_failed",
            EventType::RunCancelled => "run_cancelled",
            EventType::CheckpointCreated => "checkpoint_created",
            EventType::ArtifactUploaded => "artifact_uploaded",
            EventType::Custom(s) => s,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "experiment_created" => EventType::ExperimentCreated,
            "experiment_started" => EventType::ExperimentStarted,
            "experiment_completed" => EventType::ExperimentCompleted,
            "experiment_failed" => EventType::ExperimentFailed,
            "experiment_cancelled" => EventType::ExperimentCancelled,
            "run_created" => EventType::RunCreated,
            "run_started" => EventType::RunStarted,
            "run_completed" => EventType::RunCompleted,
            "run_failed" => EventType::RunFailed,
            "run_cancelled" => EventType::RunCancelled,
            "checkpoint_created" => EventType::CheckpointCreated,
            "artifact_uploaded" => EventType::ArtifactUploaded,
            other => EventType::Custom(other.to_string()),
        }
    }
}

/// An experiment event
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct ExperimentEvent {
    pub event_id: Uuid,
    pub experiment_id: Uuid,
    pub run_id: Option<Uuid>,
    pub event_type: String,
    pub payload: String,
    pub timestamp: DateTime<Utc>,
}

impl ExperimentEvent {
    pub fn new(
        experiment_id: Uuid,
        run_id: Option<Uuid>,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            experiment_id,
            run_id,
            event_type: event_type.as_str().to_string(),
            payload: payload.to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn get_event_type(&self) -> EventType {
        EventType::from_str(&self.event_type)
    }

    pub fn get_payload(&self) -> Result<serde_json::Value> {
        Ok(serde_json::from_str(&self.payload)?)
    }
}

/// Time range for querying events
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Repository for managing experiment events
pub struct EventRepository {
    client: Client,
}

impl EventRepository {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Initialize the events table
    pub async fn init(&self) -> Result<()> {
        self.client
            .query(
                r#"
                CREATE TABLE IF NOT EXISTS experiment_events (
                    event_id UUID,
                    experiment_id UUID,
                    run_id Nullable(UUID),
                    event_type String,
                    payload String,
                    timestamp DateTime64(3)
                )
                ENGINE = MergeTree()
                PARTITION BY toYYYYMM(timestamp)
                ORDER BY (experiment_id, timestamp)
                "#,
            )
            .execute()
            .await?;

        tracing::info!("Initialized experiment_events table");
        Ok(())
    }

    /// Insert a single event
    pub async fn insert_event(&self, event: &ExperimentEvent) -> Result<()> {
        let mut insert = self.client.insert("experiment_events")?;
        insert.write(event).await?;
        insert.end().await?;

        tracing::debug!(
            "Inserted event: id={}, experiment={}, type={}",
            event.event_id,
            event.experiment_id,
            event.event_type
        );
        Ok(())
    }

    /// Insert multiple events in a batch
    pub async fn insert_events(&self, events: &[ExperimentEvent]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let mut insert = self.client.insert("experiment_events")?;
        for event in events {
            insert.write(event).await?;
        }
        insert.end().await?;

        tracing::info!("Inserted {} events", events.len());
        Ok(())
    }

    /// Query events by experiment_id
    pub async fn query_events(
        &self,
        experiment_id: Uuid,
        event_type: Option<EventType>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<ExperimentEvent>> {
        let mut query = format!(
            "SELECT event_id, experiment_id, run_id, event_type, payload, timestamp
             FROM experiment_events
             WHERE experiment_id = '{}'",
            experiment_id
        );

        if let Some(et) = event_type {
            query.push_str(&format!(" AND event_type = '{}'", et.as_str()));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" ORDER BY timestamp ASC");

        let events = self
            .client
            .query(&query)
            .fetch_all::<ExperimentEvent>()
            .await?;

        tracing::debug!(
            "Queried {} events for experiment_id={}",
            events.len(),
            experiment_id
        );
        Ok(events)
    }

    /// Query events by run_id
    pub async fn query_events_by_run(
        &self,
        run_id: Uuid,
        event_type: Option<EventType>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<ExperimentEvent>> {
        let mut query = format!(
            "SELECT event_id, experiment_id, run_id, event_type, payload, timestamp
             FROM experiment_events
             WHERE run_id = '{}'",
            run_id
        );

        if let Some(et) = event_type {
            query.push_str(&format!(" AND event_type = '{}'", et.as_str()));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" ORDER BY timestamp ASC");

        let events = self
            .client
            .query(&query)
            .fetch_all::<ExperimentEvent>()
            .await?;

        tracing::debug!("Queried {} events for run_id={}", events.len(), run_id);
        Ok(events)
    }

    /// Query events across multiple experiments
    pub async fn query_events_for_experiments(
        &self,
        experiment_ids: &[Uuid],
        event_type: Option<EventType>,
        time_range: Option<TimeRange>,
    ) -> Result<Vec<ExperimentEvent>> {
        if experiment_ids.is_empty() {
            return Ok(Vec::new());
        }

        let experiment_ids_str = experiment_ids
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(", ");

        let mut query = format!(
            "SELECT event_id, experiment_id, run_id, event_type, payload, timestamp
             FROM experiment_events
             WHERE experiment_id IN ({})",
            experiment_ids_str
        );

        if let Some(et) = event_type {
            query.push_str(&format!(" AND event_type = '{}'", et.as_str()));
        }

        if let Some(range) = time_range {
            query.push_str(&format!(
                " AND timestamp >= '{}' AND timestamp <= '{}'",
                range.start.format("%Y-%m-%d %H:%M:%S"),
                range.end.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        query.push_str(" ORDER BY experiment_id, timestamp ASC");

        let events = self
            .client
            .query(&query)
            .fetch_all::<ExperimentEvent>()
            .await?;

        tracing::debug!(
            "Queried {} events for {} experiments",
            events.len(),
            experiment_ids.len()
        );
        Ok(events)
    }

    /// Count events by type for an experiment
    pub async fn count_events_by_type(&self, experiment_id: Uuid) -> Result<Vec<(String, u64)>> {
        #[derive(Row, Deserialize)]
        struct EventCount {
            event_type: String,
            count: u64,
        }

        let query = format!(
            "SELECT event_type, count() as count
             FROM experiment_events
             WHERE experiment_id = '{}'
             GROUP BY event_type
             ORDER BY count DESC",
            experiment_id
        );

        let counts = self.client.query(&query).fetch_all::<EventCount>().await?;

        Ok(counts
            .into_iter()
            .map(|ec| (ec.event_type, ec.count))
            .collect())
    }

    /// Delete events for a specific experiment
    pub async fn delete_events(&self, experiment_id: Uuid) -> Result<()> {
        self.client
            .query(&format!(
                "ALTER TABLE experiment_events DELETE WHERE experiment_id = '{}'",
                experiment_id
            ))
            .execute()
            .await?;

        tracing::info!("Deleted events for experiment_id={}", experiment_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_conversion() {
        assert_eq!(
            EventType::ExperimentCreated.as_str(),
            "experiment_created"
        );
        assert_eq!(
            EventType::from_str("experiment_created"),
            EventType::ExperimentCreated
        );
        assert_eq!(
            EventType::from_str("custom_event"),
            EventType::Custom("custom_event".to_string())
        );
    }

    #[test]
    fn test_event_creation() {
        let experiment_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let payload = serde_json::json!({"status": "started"});

        let event = ExperimentEvent::new(
            experiment_id,
            Some(run_id),
            EventType::RunStarted,
            payload.clone(),
        );

        assert_eq!(event.experiment_id, experiment_id);
        assert_eq!(event.run_id, Some(run_id));
        assert_eq!(event.get_event_type(), EventType::RunStarted);
        assert_eq!(event.get_payload().unwrap(), payload);
    }
}
