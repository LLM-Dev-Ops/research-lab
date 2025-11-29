use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Dataset {
    pub id: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    pub s3_path: String,
    pub sample_count: i64,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Dataset {
    pub fn new(
        name: String,
        description: Option<String>,
        s3_path: String,
        sample_count: i64,
        schema: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            s3_path,
            sample_count,
            schema,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSample {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub index: i64,
    pub input: serde_json::Value,
    pub expected_output: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
}

impl DatasetSample {
    pub fn new(
        dataset_id: Uuid,
        index: i64,
        input: serde_json::Value,
        expected_output: Option<serde_json::Value>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            dataset_id,
            index,
            input,
            expected_output,
            metadata,
        }
    }
}
