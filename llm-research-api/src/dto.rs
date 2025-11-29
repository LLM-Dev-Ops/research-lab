pub mod experiment;
pub mod model;
pub mod dataset;
pub mod prompt;
pub mod evaluation;

pub use experiment::*;
pub use model::*;
pub use dataset::*;
pub use prompt::*;
pub use evaluation::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// Pagination structures
#[derive(Debug, Deserialize, Validate)]
pub struct PaginationQuery {
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<i64>,
    pub cursor: Option<Uuid>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            limit: Some(20),
            cursor: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<Uuid>,
    pub has_more: bool,
    pub total: Option<i64>,
}

// Error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}
