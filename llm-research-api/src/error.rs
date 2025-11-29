use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("JWT error: {0}")]
    Jwt(String),
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        ApiError::Validation(format!("Validation failed: {:?}", errors))
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        ApiError::Jwt(err.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message, details) = match &self {
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, "Validation error", Some(msg.clone())),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "Resource not found", Some(msg.clone())),
            ApiError::Internal(err) => {
                tracing::error!("Internal error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error", Some(err.clone()))
            },
            ApiError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error", None)
            },
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", None),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", None),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad request", Some(msg.clone())),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", Some(msg.clone())),
            ApiError::Storage(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "Storage error", Some(msg.clone())),
            ApiError::Jwt(msg) => (StatusCode::UNAUTHORIZED, "JWT error", Some(msg.clone())),
        };

        let mut response_json = json!({
            "error": message,
        });

        if let Some(details_msg) = details {
            response_json["details"] = json!(details_msg);
        }

        (status, Json(response_json)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
