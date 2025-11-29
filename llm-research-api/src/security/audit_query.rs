//! Query utilities for audit logs stored in PostgreSQL

use super::audit::{AuditEvent, AuditResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Filters for querying audit logs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLogFilter {
    /// Filter by event type
    pub event_type: Option<String>,

    /// Filter by actor type
    pub actor_type: Option<String>,

    /// Filter by actor ID (user or API key)
    pub actor_id: Option<Uuid>,

    /// Filter by resource type
    pub resource_type: Option<String>,

    /// Filter by resource ID
    pub resource_id: Option<Uuid>,

    /// Filter by action
    pub action: Option<String>,

    /// Filter by outcome status
    pub outcome_status: Option<String>,

    /// Filter by IP address
    pub ip_address: Option<String>,

    /// Filter by request ID
    pub request_id: Option<String>,

    /// Filter events after this timestamp
    pub after: Option<DateTime<Utc>>,

    /// Filter events before this timestamp
    pub before: Option<DateTime<Utc>>,

    /// Limit number of results
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Service for querying audit logs
pub struct AuditLogQuery {
    pool: PgPool,
}

impl AuditLogQuery {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Query audit logs with filters
    pub async fn query(&self, filter: &AuditLogFilter) -> AuditResult<Vec<AuditEvent>> {
        let mut query = String::from(
            r#"
            SELECT
                id, timestamp, event_type, actor, resource, action, outcome,
                details, ip_address, user_agent, request_id, duration_ms
            FROM audit_log
            WHERE 1=1
            "#,
        );

        let mut params: Vec<String> = Vec::new();
        let mut param_num = 1;

        // Build WHERE clause dynamically
        if let Some(ref event_type) = filter.event_type {
            query.push_str(&format!(
                " AND event_type->>'type' = ${}",
                param_num
            ));
            params.push(event_type.clone());
            param_num += 1;
        }

        if let Some(ref actor_type) = filter.actor_type {
            query.push_str(&format!(" AND actor->>'type' = ${}", param_num));
            params.push(actor_type.clone());
            param_num += 1;
        }

        if let Some(ref resource_type) = filter.resource_type {
            query.push_str(&format!(
                " AND resource->>'type' = ${}",
                param_num
            ));
            params.push(resource_type.clone());
            param_num += 1;
        }

        if let Some(ref ip) = filter.ip_address {
            query.push_str(&format!(" AND ip_address = ${}", param_num));
            params.push(ip.clone());
            param_num += 1;
        }

        if let Some(ref request_id) = filter.request_id {
            query.push_str(&format!(" AND request_id = ${}", param_num));
            params.push(request_id.clone());
            param_num += 1;
        }

        if let Some(after) = filter.after {
            query.push_str(&format!(" AND timestamp > ${}", param_num));
            params.push(after.to_rfc3339());
            param_num += 1;
        }

        if let Some(before) = filter.before {
            query.push_str(&format!(" AND timestamp < ${}", param_num));
            params.push(before.to_rfc3339());
            param_num += 1;
        }

        // Order by timestamp descending (most recent first)
        query.push_str(" ORDER BY timestamp DESC");

        // Add limit and offset
        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT ${}", param_num));
            params.push(limit.to_string());
            param_num += 1;
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET ${}", param_num));
            params.push(offset.to_string());
        }

        // Execute query - note: this is a simplified example
        // In production, you'd want to use sqlx's query builder properly
        self.execute_query(&query, &params).await
    }

    /// Get audit events for a specific resource
    pub async fn get_resource_history(
        &self,
        resource_type: &str,
        resource_id: Uuid,
        limit: i64,
    ) -> AuditResult<Vec<AuditEvent>> {
        self.query(&AuditLogFilter {
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            limit: Some(limit),
            ..Default::default()
        })
        .await
    }

    /// Get failed login attempts
    pub async fn get_failed_logins(
        &self,
        since: DateTime<Utc>,
        limit: i64,
    ) -> AuditResult<Vec<AuditEvent>> {
        self.query(&AuditLogFilter {
            event_type: Some("authentication".to_string()),
            action: Some("login_failed".to_string()),
            after: Some(since),
            limit: Some(limit),
            ..Default::default()
        })
        .await
    }

    /// Get all events for a specific user
    pub async fn get_user_activity(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> AuditResult<Vec<AuditEvent>> {
        self.query(&AuditLogFilter {
            actor_type: Some("user".to_string()),
            actor_id: Some(user_id),
            limit: Some(limit),
            ..Default::default()
        })
        .await
    }

    /// Get denied access attempts
    pub async fn get_denied_access(
        &self,
        since: DateTime<Utc>,
        limit: i64,
    ) -> AuditResult<Vec<AuditEvent>> {
        self.query(&AuditLogFilter {
            outcome_status: Some("denied".to_string()),
            after: Some(since),
            limit: Some(limit),
            ..Default::default()
        })
        .await
    }

    /// Get events by IP address (useful for tracking suspicious activity)
    pub async fn get_events_by_ip(
        &self,
        ip_address: &str,
        limit: i64,
    ) -> AuditResult<Vec<AuditEvent>> {
        self.query(&AuditLogFilter {
            ip_address: Some(ip_address.to_string()),
            limit: Some(limit),
            ..Default::default()
        })
        .await
    }

    /// Get all modification events for a resource
    pub async fn get_modification_history(
        &self,
        resource_type: &str,
        resource_id: Uuid,
    ) -> AuditResult<Vec<AuditEvent>> {
        self.query(&AuditLogFilter {
            event_type: Some("data_modification".to_string()),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            limit: Some(100),
            ..Default::default()
        })
        .await
    }

    /// Get statistics about audit events
    pub async fn get_statistics(
        &self,
        since: DateTime<Utc>,
    ) -> AuditResult<AuditStatistics> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE outcome->>'status' = 'success') as successful,
                COUNT(*) FILTER (WHERE outcome->>'status' = 'failure') as failed,
                COUNT(*) FILTER (WHERE outcome->>'status' = 'denied') as denied,
                COUNT(DISTINCT ip_address) as unique_ips,
                COUNT(DISTINCT request_id) as unique_requests
            FROM audit_log
            WHERE timestamp > $1
            "#,
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        use sqlx::Row;
        let stats = AuditStatistics {
            total: row.get("total"),
            successful: row.get("successful"),
            failed: row.get("failed"),
            denied: row.get("denied"),
            unique_ips: row.get("unique_ips"),
            unique_requests: row.get("unique_requests"),
        };

        Ok(stats)
    }

    /// Execute the query (simplified - in production use proper parameterized queries)
    async fn execute_query(
        &self,
        _query: &str,
        _params: &[String],
    ) -> AuditResult<Vec<AuditEvent>> {
        // This is a placeholder. In production, you would:
        // 1. Use sqlx's query builder or query_as!
        // 2. Properly bind parameters
        // 3. Map results to AuditEvent
        //
        // For now, return empty vec as this requires the full database setup
        Ok(Vec::new())
    }
}

/// Statistics about audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
    pub denied: i64,
    pub unique_ips: i64,
    pub unique_requests: i64,
}

impl AuditStatistics {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.successful as f64 / self.total as f64) * 100.0
        }
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.failed as f64 / self.total as f64) * 100.0
        }
    }

    pub fn denial_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.denied as f64 / self.total as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_statistics() {
        let stats = AuditStatistics {
            total: 100,
            successful: 80,
            failed: 15,
            denied: 5,
            unique_ips: 25,
            unique_requests: 90,
        };

        assert_eq!(stats.success_rate(), 80.0);
        assert_eq!(stats.failure_rate(), 15.0);
        assert_eq!(stats.denial_rate(), 5.0);
    }

    #[test]
    fn test_filter_default() {
        let filter = AuditLogFilter::default();
        assert!(filter.event_type.is_none());
        assert!(filter.limit.is_none());
    }

    #[test]
    fn test_filter_builder() {
        let filter = AuditLogFilter {
            event_type: Some("authentication".to_string()),
            limit: Some(50),
            ..Default::default()
        };

        assert_eq!(filter.event_type, Some("authentication".to_string()));
        assert_eq!(filter.limit, Some(50));
        assert!(filter.offset.is_none());
    }
}
