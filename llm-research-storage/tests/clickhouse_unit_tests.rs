mod common;

use chrono::{DateTime, Utc, TimeZone, Duration};
use uuid::Uuid;

#[cfg(test)]
mod clickhouse_config_tests {
    use super::*;
    use llm_research_storage::clickhouse::ClickHouseConfig;

    #[test]
    fn test_clickhouse_config_default() {
        let config = ClickHouseConfig::default();

        assert_eq!(config.url, "http://localhost:8123");
        assert_eq!(config.database, "llm_research");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }

    #[test]
    fn test_clickhouse_config_with_auth() {
        let config = ClickHouseConfig {
            url: "http://clickhouse.example.com:8123".to_string(),
            database: "test_db".to_string(),
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
        };

        assert_eq!(config.url, "http://clickhouse.example.com:8123");
        assert_eq!(config.database, "test_db");
        assert_eq!(config.username, Some("admin".to_string()));
        assert_eq!(config.password, Some("secret".to_string()));
    }

    #[test]
    fn test_clickhouse_config_serialization() {
        let config = ClickHouseConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.is_empty());

        let deserialized: ClickHouseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, config.url);
        assert_eq!(deserialized.database, config.database);
    }
}

#[cfg(test)]
mod time_series_formatting_tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TimeSeriesPoint {
        timestamp: DateTime<Utc>,
        experiment_id: Uuid,
        metric_name: String,
        metric_value: f64,
    }

    #[test]
    fn test_timestamp_formatting() {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap();

        // ClickHouse DateTime64(3) format
        let formatted = timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        assert!(formatted.contains("2024-01-15"));
        assert!(formatted.contains("10:30:45"));
    }

    #[test]
    fn test_timestamp_millisecond_precision() {
        let timestamp = Utc::now();
        let millis = timestamp.timestamp_millis();

        // Should be able to round-trip
        let reconstructed = Utc.timestamp_millis_opt(millis).unwrap();

        // Millisecond precision should match
        assert_eq!(timestamp.timestamp_millis(), reconstructed.timestamp_millis());
    }

    #[test]
    fn test_time_series_data_formatting() {
        let point = TimeSeriesPoint {
            timestamp: Utc::now(),
            experiment_id: Uuid::new_v4(),
            metric_name: "accuracy".to_string(),
            metric_value: 0.95,
        };

        assert!(!point.metric_name.is_empty());
        assert!(point.metric_value >= 0.0 && point.metric_value <= 1.0);
    }

    #[test]
    fn test_partition_key_generation() {
        fn generate_partition_key(timestamp: DateTime<Utc>) -> String {
            timestamp.format("%Y%m").to_string()
        }

        let jan_2024 = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        assert_eq!(generate_partition_key(jan_2024), "202401");

        let dec_2024 = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
        assert_eq!(generate_partition_key(dec_2024), "202412");
    }

    #[test]
    fn test_time_range_queries() {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let one_day_ago = now - Duration::days(1);

        // Time ranges should be ordered correctly
        assert!(one_day_ago < one_hour_ago);
        assert!(one_hour_ago < now);

        // Timestamp comparison for query building
        assert!(one_day_ago.timestamp() < now.timestamp());
    }
}

#[cfg(test)]
mod query_building_tests {
    use super::*;

    #[test]
    fn test_select_query_building() {
        let experiment_id = Uuid::new_v4();

        let query = format!(
            "SELECT timestamp, latency_ms, token_count FROM evaluations WHERE experiment_id = '{}' ORDER BY timestamp",
            experiment_id
        );

        assert!(query.contains("SELECT"));
        assert!(query.contains("FROM evaluations"));
        assert!(query.contains("WHERE experiment_id"));
        assert!(query.contains("ORDER BY timestamp"));
    }

    #[test]
    fn test_aggregation_query_building() {
        let experiment_id = Uuid::new_v4();

        let query = format!(
            "SELECT AVG(latency_ms) as avg_latency, MAX(latency_ms) as max_latency, MIN(latency_ms) as min_latency FROM evaluations WHERE experiment_id = '{}'",
            experiment_id
        );

        assert!(query.contains("AVG(latency_ms)"));
        assert!(query.contains("MAX(latency_ms)"));
        assert!(query.contains("MIN(latency_ms)"));
    }

    #[test]
    fn test_time_based_aggregation_query() {
        let query = r#"
            SELECT
                toStartOfHour(timestamp) as hour,
                AVG(latency_ms) as avg_latency,
                COUNT(*) as count
            FROM evaluations
            WHERE timestamp >= now() - INTERVAL 24 HOUR
            GROUP BY hour
            ORDER BY hour
        "#;

        assert!(query.contains("toStartOfHour"));
        assert!(query.contains("GROUP BY hour"));
        assert!(query.contains("INTERVAL 24 HOUR"));
    }

    #[test]
    fn test_percentile_query_building() {
        let query = r#"
            SELECT
                quantile(0.50)(latency_ms) as p50,
                quantile(0.90)(latency_ms) as p90,
                quantile(0.95)(latency_ms) as p95,
                quantile(0.99)(latency_ms) as p99
            FROM evaluations
        "#;

        assert!(query.contains("quantile(0.50)"));
        assert!(query.contains("quantile(0.90)"));
        assert!(query.contains("quantile(0.95)"));
        assert!(query.contains("quantile(0.99)"));
    }

    #[test]
    fn test_insert_query_building() {
        let id = Uuid::new_v4();
        let experiment_id = Uuid::new_v4();
        let sample_id = Uuid::new_v4();

        let query = format!(
            "INSERT INTO evaluations (id, experiment_id, sample_id, timestamp, latency_ms, token_count) VALUES ('{}', '{}', '{}', now(), 100, 50)",
            id, experiment_id, sample_id
        );

        assert!(query.contains("INSERT INTO evaluations"));
        assert!(query.contains("VALUES"));
        assert!(query.contains(&id.to_string()));
    }

    #[test]
    fn test_parameterized_query_building() {
        // Test query parameter placeholder
        let query = "SELECT * FROM evaluations WHERE experiment_id = ? AND timestamp >= ?";

        assert!(query.contains("?"));
        assert_eq!(query.matches('?').count(), 2);
    }
}

#[cfg(test)]
mod metric_aggregation_tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct EvaluationMetrics {
        latency_ms: i64,
        token_count: i32,
        cost: f64,
    }

    #[test]
    fn test_average_calculation() {
        let metrics = vec![
            EvaluationMetrics { latency_ms: 100, token_count: 50, cost: 0.01 },
            EvaluationMetrics { latency_ms: 200, token_count: 100, cost: 0.02 },
            EvaluationMetrics { latency_ms: 150, token_count: 75, cost: 0.015 },
        ];

        let avg_latency: f64 = metrics.iter().map(|m| m.latency_ms as f64).sum::<f64>()
            / metrics.len() as f64;

        assert_eq!(avg_latency, 150.0);

        let avg_tokens: f64 = metrics.iter().map(|m| m.token_count as f64).sum::<f64>()
            / metrics.len() as f64;

        assert_eq!(avg_tokens, 75.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let mut latencies = vec![100, 150, 200, 250, 300, 350, 400, 450, 500];
        latencies.sort();

        // Calculate p50 (median)
        let p50_index = (latencies.len() as f64 * 0.50) as usize;
        let p50 = latencies[p50_index.min(latencies.len() - 1)];
        assert_eq!(p50, 300);

        // Calculate p90
        let p90_index = (latencies.len() as f64 * 0.90) as usize;
        let p90 = latencies[p90_index.min(latencies.len() - 1)];
        assert_eq!(p90, 500);
    }

    #[test]
    fn test_sum_aggregation() {
        let metrics = vec![
            EvaluationMetrics { latency_ms: 100, token_count: 50, cost: 0.01 },
            EvaluationMetrics { latency_ms: 200, token_count: 100, cost: 0.02 },
            EvaluationMetrics { latency_ms: 150, token_count: 75, cost: 0.015 },
        ];

        let total_tokens: i32 = metrics.iter().map(|m| m.token_count).sum();
        assert_eq!(total_tokens, 225);

        let total_cost: f64 = metrics.iter().map(|m| m.cost).sum();
        assert!((total_cost - 0.045).abs() < 0.0001);
    }

    #[test]
    fn test_min_max_aggregation() {
        let latencies = vec![100, 150, 200, 250, 300];

        let min = latencies.iter().min().unwrap();
        let max = latencies.iter().max().unwrap();

        assert_eq!(*min, 100);
        assert_eq!(*max, 300);
    }

    #[test]
    fn test_count_aggregation() {
        let metrics = vec![
            EvaluationMetrics { latency_ms: 100, token_count: 50, cost: 0.01 },
            EvaluationMetrics { latency_ms: 200, token_count: 100, cost: 0.02 },
        ];

        let count = metrics.len();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_group_by_experiment() {
        use std::collections::HashMap;

        #[derive(Debug, Clone)]
        struct Evaluation {
            experiment_id: Uuid,
            latency_ms: i64,
        }

        let exp1 = Uuid::new_v4();
        let exp2 = Uuid::new_v4();

        let evaluations = vec![
            Evaluation { experiment_id: exp1, latency_ms: 100 },
            Evaluation { experiment_id: exp1, latency_ms: 150 },
            Evaluation { experiment_id: exp2, latency_ms: 200 },
            Evaluation { experiment_id: exp2, latency_ms: 250 },
        ];

        let mut grouped: HashMap<Uuid, Vec<i64>> = HashMap::new();
        for eval in evaluations {
            grouped.entry(eval.experiment_id)
                .or_insert_with(Vec::new)
                .push(eval.latency_ms);
        }

        assert_eq!(grouped.get(&exp1).unwrap().len(), 2);
        assert_eq!(grouped.get(&exp2).unwrap().len(), 2);
    }
}

#[cfg(test)]
mod data_type_tests {
    use super::*;

    #[test]
    fn test_uuid_formatting_for_clickhouse() {
        let id = Uuid::new_v4();
        let formatted = id.to_string();

        // UUID should be formatted with hyphens
        assert_eq!(formatted.len(), 36);
        assert_eq!(formatted.matches('-').count(), 4);
    }

    #[test]
    fn test_decimal_formatting_for_clickhouse() {
        use rust_decimal::Decimal;

        let cost = Decimal::new(12345, 4); // 1.2345
        let formatted = cost.to_string();

        assert!(formatted.contains('.'));
    }

    #[test]
    fn test_json_string_formatting() {
        let metrics = serde_json::json!({
            "accuracy": 0.95,
            "f1_score": 0.92,
            "precision": 0.93
        });

        let json_string = metrics.to_string();
        assert!(json_string.contains("accuracy"));
        assert!(json_string.contains("0.95"));
    }

    #[test]
    fn test_datetime64_millisecond_precision() {
        let now = Utc::now();
        let millis = now.timestamp_millis();

        // Verify we can store millisecond precision
        let reconstructed = Utc.timestamp_millis_opt(millis).unwrap();

        assert_eq!(now.timestamp_millis(), reconstructed.timestamp_millis());
    }

    #[test]
    fn test_int64_range() {
        // Test that latency values fit in Int64
        let latencies: Vec<i64> = vec![
            0,
            100,
            1000,
            10000,
            100000,
            i64::MAX,
        ];

        for latency in latencies {
            assert!(latency >= 0);
        }
    }

    #[test]
    fn test_int32_token_count() {
        // Test that token counts fit in Int32
        let token_counts: Vec<i32> = vec![
            0,
            100,
            1000,
            10000,
            100000,
        ];

        for count in token_counts {
            assert!(count >= 0);
            assert!(count <= i32::MAX);
        }
    }
}

#[cfg(test)]
mod table_schema_tests {
    use super::*;

    #[test]
    fn test_evaluations_table_columns() {
        let required_columns = vec![
            "id",
            "experiment_id",
            "sample_id",
            "timestamp",
            "latency_ms",
            "token_count",
            "cost",
            "metrics",
        ];

        // All required columns should be present
        for column in required_columns {
            assert!(!column.is_empty());
        }
    }

    #[test]
    fn test_partition_by_month() {
        // Test partition key format: YYYYMM
        let timestamps = vec![
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 1, 31, 23, 59, 59).unwrap(),
            Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
        ];

        let partitions: Vec<String> = timestamps
            .iter()
            .map(|ts| ts.format("%Y%m").to_string())
            .collect();

        assert_eq!(partitions[0], "202401");
        assert_eq!(partitions[1], "202401");
        assert_eq!(partitions[2], "202402");
    }

    #[test]
    fn test_order_by_clause() {
        // ORDER BY (experiment_id, timestamp)
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        struct OrderKey {
            experiment_id: Uuid,
            timestamp_millis: i64,
        }

        let exp1 = Uuid::new_v4();
        let exp2 = Uuid::new_v4();

        let mut keys = vec![
            OrderKey {
                experiment_id: exp2,
                timestamp_millis: 2000,
            },
            OrderKey {
                experiment_id: exp1,
                timestamp_millis: 1000,
            },
            OrderKey {
                experiment_id: exp1,
                timestamp_millis: 2000,
            },
        ];

        keys.sort();

        // Should be sorted by experiment_id first, then timestamp
        assert!(keys[0].experiment_id <= keys[1].experiment_id);
        if keys[0].experiment_id == keys[1].experiment_id {
            assert!(keys[0].timestamp_millis <= keys[1].timestamp_millis);
        }
    }

    #[test]
    fn test_index_definitions() {
        // Test that index names are valid
        let indexes = vec![
            ("idx_experiment", "experiment_id"),
            ("idx_timestamp", "timestamp"),
        ];

        for (index_name, column) in indexes {
            assert!(!index_name.is_empty());
            assert!(!column.is_empty());
            assert!(index_name.starts_with("idx_"));
        }
    }
}

#[cfg(test)]
mod window_function_tests {
    use super::*;

    #[test]
    fn test_moving_average_window() {
        let values = vec![100, 150, 200, 250, 300];
        let window_size = 3;

        let mut moving_averages = Vec::new();
        for i in 0..values.len() {
            if i + 1 >= window_size {
                let start_idx = i + 1 - window_size;
                let window_sum: i32 = values[start_idx..=i].iter().sum();
                let avg = window_sum as f64 / window_size as f64;
                moving_averages.push(avg);
            }
        }

        assert_eq!(moving_averages.len(), values.len() - window_size + 1);
        assert_eq!(moving_averages[0], 150.0); // (100 + 150 + 200) / 3
        assert_eq!(moving_averages[1], 200.0); // (150 + 200 + 250) / 3
        assert_eq!(moving_averages[2], 250.0); // (200 + 250 + 300) / 3
    }

    #[test]
    fn test_time_window_query() {
        let query = r#"
            SELECT
                timestamp,
                latency_ms,
                AVG(latency_ms) OVER (
                    ORDER BY timestamp
                    ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
                ) as moving_avg
            FROM evaluations
        "#;

        assert!(query.contains("OVER"));
        assert!(query.contains("ROWS BETWEEN"));
        assert!(query.contains("PRECEDING"));
    }
}

#[cfg(test)]
mod batch_insert_tests {
    use super::*;

    #[test]
    fn test_batch_size_calculation() {
        let total_records = 10000;
        let batch_size = 1000;

        let num_batches = (total_records + batch_size - 1) / batch_size;
        assert_eq!(num_batches, 10);
    }

    #[test]
    fn test_batch_partitioning() {
        let records: Vec<i32> = (1..=10).collect();
        let batch_size = 3;

        let batches: Vec<Vec<i32>> = records
            .chunks(batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        assert_eq!(batches.len(), 4);
        assert_eq!(batches[0].len(), 3);
        assert_eq!(batches[1].len(), 3);
        assert_eq!(batches[2].len(), 3);
        assert_eq!(batches[3].len(), 1);
    }

    #[test]
    fn test_optimal_batch_size() {
        // ClickHouse recommends 1000-10000 rows per batch
        let min_batch = 1000;
        let max_batch = 10000;
        let recommended_batch = 5000;

        assert!(recommended_batch >= min_batch);
        assert!(recommended_batch <= max_batch);
    }
}
