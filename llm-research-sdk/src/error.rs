//! SDK error types and handling
//!
//! This module provides comprehensive error handling for the SDK,
//! including API errors, network errors, and validation errors.

use std::fmt;
use thiserror::Error;

/// The main error type for the SDK
#[derive(Error, Debug)]
pub enum SdkError {
    /// API returned an error response
    #[error("API error: {status} - {message}")]
    ApiError {
        status: u16,
        message: String,
        error_code: Option<String>,
        request_id: Option<String>,
    },

    /// Network or connection error
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// Request timed out
    #[error("Request timed out after {0} seconds")]
    Timeout(u64),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after} seconds")]
    RateLimited {
        retry_after: u64,
        limit: u64,
        remaining: u64,
    },

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Authorization failed
    #[error("Access denied: {0}")]
    AuthorizationError(String),

    /// Resource not found
    #[error("Resource not found: {resource_type} with ID {resource_id}")]
    NotFound {
        resource_type: String,
        resource_id: String,
    },

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// URL parsing error
    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Conflict error (e.g., duplicate resource)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Server error
    #[error("Server error: {0}")]
    ServerError(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for SDK operations
pub type SdkResult<T> = Result<T, SdkError>;

/// API error response structure
#[derive(Debug, serde::Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub request_id: Option<String>,
}

impl SdkError {
    /// Create an API error from a response
    pub fn from_response(status: u16, body: &str, request_id: Option<String>) -> Self {
        // Try to parse as JSON error response
        if let Ok(error_response) = serde_json::from_str::<ApiErrorResponse>(body) {
            return match status {
                401 => SdkError::AuthenticationError(error_response.message),
                403 => SdkError::AuthorizationError(error_response.message),
                404 => SdkError::NotFound {
                    resource_type: "unknown".to_string(),
                    resource_id: "unknown".to_string(),
                },
                409 => SdkError::Conflict(error_response.message),
                422 => SdkError::ValidationError(error_response.message),
                429 => SdkError::RateLimited {
                    retry_after: 60,
                    limit: 0,
                    remaining: 0,
                },
                500..=599 => SdkError::ServerError(error_response.message),
                _ => SdkError::ApiError {
                    status,
                    message: error_response.message,
                    error_code: Some(error_response.error),
                    request_id,
                },
            };
        }

        // Fall back to generic error
        SdkError::ApiError {
            status,
            message: body.to_string(),
            error_code: None,
            request_id,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SdkError::NetworkError(_)
                | SdkError::Timeout(_)
                | SdkError::RateLimited { .. }
                | SdkError::ServerError(_)
        )
    }

    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            SdkError::ApiError { status, .. } => Some(*status),
            SdkError::RateLimited { .. } => Some(429),
            SdkError::AuthenticationError(_) => Some(401),
            SdkError::AuthorizationError(_) => Some(403),
            SdkError::NotFound { .. } => Some(404),
            SdkError::Conflict(_) => Some(409),
            SdkError::ValidationError(_) => Some(422),
            SdkError::ServerError(_) => Some(500),
            _ => None,
        }
    }

    /// Get the request ID if available
    pub fn request_id(&self) -> Option<&str> {
        match self {
            SdkError::ApiError { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }
}

/// Validation error details
#[derive(Debug, Clone)]
pub struct ValidationErrors {
    pub errors: Vec<FieldError>,
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<String> = self
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect();
        write!(f, "{}", messages.join(", "))
    }
}

/// A single field validation error
#[derive(Debug, Clone)]
pub struct FieldError {
    pub field: String,
    pub message: String,
    pub code: Option<String>,
}

impl From<ValidationErrors> for SdkError {
    fn from(errors: ValidationErrors) -> Self {
        SdkError::ValidationError(errors.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_json_response() {
        let body = r#"{"error": "not_found", "message": "Experiment not found"}"#;
        let error = SdkError::from_response(404, body, Some("req-123".to_string()));

        assert!(matches!(error, SdkError::NotFound { .. }));
    }

    #[test]
    fn test_error_is_retryable() {
        let rate_limited = SdkError::RateLimited {
            retry_after: 60,
            limit: 100,
            remaining: 0,
        };
        assert!(rate_limited.is_retryable());

        let not_found = SdkError::NotFound {
            resource_type: "experiment".to_string(),
            resource_id: "123".to_string(),
        };
        assert!(!not_found.is_retryable());
    }

    #[test]
    fn test_error_status_code() {
        let api_error = SdkError::ApiError {
            status: 400,
            message: "Bad request".to_string(),
            error_code: None,
            request_id: None,
        };
        assert_eq!(api_error.status_code(), Some(400));

        let auth_error = SdkError::AuthenticationError("Invalid token".to_string());
        assert_eq!(auth_error.status_code(), Some(401));
    }
}
