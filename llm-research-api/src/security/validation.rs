//! Request validation middleware and extractors
//!
//! Provides comprehensive validation for API requests including:
//! - JSON payload validation using validator crate
//! - Input sanitization
//! - Custom validation rules
//! - Field-level error reporting

use axum::{
    async_trait,
    body::Body,
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;
use validator::{Validate, ValidationError, ValidationErrors};

/// Validation error with detailed field information
#[derive(Debug, Error)]
pub enum ValidationErrorKind {
    #[error("JSON parsing error: {0}")]
    JsonParse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Field validation failed")]
    FieldErrors(HashMap<String, Vec<FieldError>>),
}

/// Individual field error
#[derive(Debug, Clone)]
pub struct FieldError {
    pub code: String,
    pub message: String,
    pub params: HashMap<String, serde_json::Value>,
}

impl From<&ValidationError> for FieldError {
    fn from(err: &ValidationError) -> Self {
        Self {
            code: err.code.to_string(),
            message: err.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| {
                format!("Validation failed: {}", err.code)
            }),
            params: err
                .params
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }
}

/// Validated JSON extractor that automatically validates the request body
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = ValidationRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Extract JSON
        let Json(value): Json<T> = Json::from_request(req, state)
            .await
            .map_err(|e| ValidationRejection::JsonParse(e))?;

        // Validate
        value.validate().map_err(|e| ValidationRejection::Validation(e))?;

        Ok(ValidatedJson(value))
    }
}

/// Rejection type for validation failures
#[derive(Debug)]
pub enum ValidationRejection {
    JsonParse(JsonRejection),
    Validation(ValidationErrors),
}

impl IntoResponse for ValidationRejection {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            ValidationRejection::JsonParse(e) => {
                let message = match e {
                    JsonRejection::JsonDataError(e) => {
                        format!("Invalid JSON data: {}", e.body_text())
                    }
                    JsonRejection::JsonSyntaxError(e) => {
                        format!("Invalid JSON syntax: {}", e.body_text())
                    }
                    JsonRejection::MissingJsonContentType(e) => {
                        format!("Missing Content-Type: application/json header")
                    }
                    JsonRejection::BytesRejection(e) => {
                        format!("Failed to read request body: {}", e)
                    }
                    _ => format!("JSON parsing error"),
                };
                (
                    StatusCode::BAD_REQUEST,
                    json!({
                        "error": "invalid_request",
                        "message": message,
                        "details": null
                    }),
                )
            }
            ValidationRejection::Validation(errors) => {
                let field_errors = format_validation_errors(&errors);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    json!({
                        "error": "validation_error",
                        "message": "One or more fields failed validation",
                        "details": field_errors
                    }),
                )
            }
        };

        (status, Json(body)).into_response()
    }
}

/// Format validation errors into a structured response
fn format_validation_errors(errors: &ValidationErrors) -> HashMap<String, Vec<HashMap<String, serde_json::Value>>> {
    let mut result = HashMap::new();

    for (field, field_errors) in errors.field_errors() {
        let formatted: Vec<HashMap<String, serde_json::Value>> = field_errors
            .iter()
            .map(|e| {
                let mut error_map = HashMap::new();
                error_map.insert("code".to_string(), json!(e.code.to_string()));
                error_map.insert(
                    "message".to_string(),
                    json!(e.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| {
                        get_error_message(&e.code, &e.params)
                    })),
                );
                if !e.params.is_empty() {
                    error_map.insert("params".to_string(), json!(e.params));
                }
                error_map
            })
            .collect();
        result.insert(field.to_string(), formatted);
    }

    // Handle nested errors
    for (field, nested) in errors.0.iter() {
        if let validator::ValidationErrorsKind::Struct(nested_errors) = nested {
            let nested_formatted = format_validation_errors(nested_errors);
            for (nested_field, nested_errs) in nested_formatted {
                result.insert(format!("{}.{}", field, nested_field), nested_errs);
            }
        }
    }

    result
}

/// Get a human-readable error message for common validation codes
fn get_error_message(code: &str, params: &HashMap<std::borrow::Cow<'static, str>, serde_json::Value>) -> String {
    match code {
        "length" => {
            let min = params.get("min").and_then(|v| v.as_u64());
            let max = params.get("max").and_then(|v| v.as_u64());
            match (min, max) {
                (Some(min), Some(max)) => {
                    format!("Length must be between {} and {} characters", min, max)
                }
                (Some(min), None) => {
                    format!("Length must be at least {} characters", min)
                }
                (None, Some(max)) => {
                    format!("Length must be at most {} characters", max)
                }
                (None, None) => "Invalid length".to_string(),
            }
        }
        "range" => {
            let min = params.get("min").and_then(|v| v.as_f64());
            let max = params.get("max").and_then(|v| v.as_f64());
            match (min, max) {
                (Some(min), Some(max)) => {
                    format!("Value must be between {} and {}", min, max)
                }
                (Some(min), None) => {
                    format!("Value must be at least {}", min)
                }
                (None, Some(max)) => {
                    format!("Value must be at most {}", max)
                }
                (None, None) => "Invalid range".to_string(),
            }
        }
        "email" => "Invalid email address".to_string(),
        "url" => "Invalid URL".to_string(),
        "required" => "This field is required".to_string(),
        "regex" => "Invalid format".to_string(),
        "custom" => params
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Validation failed".to_string()),
        _ => format!("Validation failed: {}", code),
    }
}

// Custom validation functions

/// Validate that a string contains only alphanumeric characters, underscores, and hyphens
pub fn validate_identifier(value: &str) -> Result<(), ValidationError> {
    if value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        Ok(())
    } else {
        let mut err = ValidationError::new("identifier");
        err.message = Some("Must contain only alphanumeric characters, underscores, and hyphens".into());
        Err(err)
    }
}

/// Validate that a string is a valid slug (lowercase alphanumeric with hyphens)
pub fn validate_slug(value: &str) -> Result<(), ValidationError> {
    if value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        Ok(())
    } else {
        let mut err = ValidationError::new("slug");
        err.message = Some("Must be lowercase alphanumeric with hyphens only".into());
        Err(err)
    }
}

/// Validate JSON schema structure
pub fn validate_json_schema(value: &serde_json::Value) -> Result<(), ValidationError> {
    match value {
        serde_json::Value::Object(obj) => {
            // Basic schema validation - ensure it has a type or properties
            if obj.contains_key("type") || obj.contains_key("properties") || obj.contains_key("$ref") {
                Ok(())
            } else {
                let mut err = ValidationError::new("json_schema");
                err.message = Some("JSON schema must have 'type', 'properties', or '$ref' field".into());
                Err(err)
            }
        }
        _ => {
            let mut err = ValidationError::new("json_schema");
            err.message = Some("JSON schema must be an object".into());
            Err(err)
        }
    }
}

/// Validate S3 path format
pub fn validate_s3_path(value: &str) -> Result<(), ValidationError> {
    // S3 paths should be non-empty and not start with /
    if value.is_empty() {
        let mut err = ValidationError::new("s3_path");
        err.message = Some("S3 path cannot be empty".into());
        return Err(err);
    }

    if value.starts_with('/') {
        let mut err = ValidationError::new("s3_path");
        err.message = Some("S3 path should not start with /".into());
        return Err(err);
    }

    // Check for invalid characters
    if value.contains("..") {
        let mut err = ValidationError::new("s3_path");
        err.message = Some("S3 path cannot contain '..'".into());
        return Err(err);
    }

    Ok(())
}

/// Validate that a string is a safe filename
pub fn validate_safe_filename(value: &str) -> Result<(), ValidationError> {
    // Check for path traversal attempts
    if value.contains("..") || value.contains('/') || value.contains('\\') {
        let mut err = ValidationError::new("filename");
        err.message = Some("Filename cannot contain path separators or '..'".into());
        return Err(err);
    }

    // Check for null bytes
    if value.contains('\0') {
        let mut err = ValidationError::new("filename");
        err.message = Some("Filename cannot contain null bytes".into());
        return Err(err);
    }

    // Check for reserved names on Windows
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4",
                   "LPT1", "LPT2", "LPT3", "LPT4"];
    let upper = value.to_uppercase();
    if reserved.iter().any(|r| upper == *r || upper.starts_with(&format!("{}.", r))) {
        let mut err = ValidationError::new("filename");
        err.message = Some("Filename uses a reserved name".into());
        return Err(err);
    }

    Ok(())
}

/// Validate UUID format (allows nil UUID)
pub fn validate_uuid_string(value: &str) -> Result<(), ValidationError> {
    if uuid::Uuid::parse_str(value).is_ok() {
        Ok(())
    } else {
        let mut err = ValidationError::new("uuid");
        err.message = Some("Invalid UUID format".into());
        Err(err)
    }
}

/// Validate that value is not a common XSS payload
pub fn validate_no_script_tags(value: &str) -> Result<(), ValidationError> {
    let lower = value.to_lowercase();
    if lower.contains("<script") || lower.contains("javascript:") || lower.contains("onerror=") {
        let mut err = ValidationError::new("xss");
        err.message = Some("Potentially unsafe content detected".into());
        Err(err)
    } else {
        Ok(())
    }
}

/// Input sanitization utilities
pub mod sanitize {
    /// Remove potentially dangerous HTML entities
    pub fn html_escape(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Trim and normalize whitespace
    pub fn normalize_whitespace(input: &str) -> String {
        input.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Remove control characters except newlines and tabs
    pub fn remove_control_chars(input: &str) -> String {
        input
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
            .collect()
    }

    /// Truncate string to max length with ellipsis
    pub fn truncate(input: &str, max_len: usize) -> String {
        if input.len() <= max_len {
            input.to_string()
        } else if max_len <= 3 {
            input.chars().take(max_len).collect()
        } else {
            format!("{}...", input.chars().take(max_len - 3).collect::<String>())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier() {
        assert!(validate_identifier("valid_id-123").is_ok());
        assert!(validate_identifier("invalid id").is_err());
        assert!(validate_identifier("invalid@id").is_err());
    }

    #[test]
    fn test_validate_slug() {
        assert!(validate_slug("valid-slug-123").is_ok());
        assert!(validate_slug("Invalid-Slug").is_err());
        assert!(validate_slug("invalid_slug").is_err());
    }

    #[test]
    fn test_validate_s3_path() {
        assert!(validate_s3_path("bucket/path/file.json").is_ok());
        assert!(validate_s3_path("/bucket/path").is_err());
        assert!(validate_s3_path("bucket/../escape").is_err());
        assert!(validate_s3_path("").is_err());
    }

    #[test]
    fn test_validate_safe_filename() {
        assert!(validate_safe_filename("file.txt").is_ok());
        assert!(validate_safe_filename("../escape.txt").is_err());
        assert!(validate_safe_filename("path/file.txt").is_err());
        assert!(validate_safe_filename("CON").is_err());
    }

    #[test]
    fn test_validate_no_script_tags() {
        assert!(validate_no_script_tags("normal text").is_ok());
        assert!(validate_no_script_tags("<script>alert('xss')</script>").is_err());
        assert!(validate_no_script_tags("javascript:void(0)").is_err());
    }

    #[test]
    fn test_sanitize_html_escape() {
        assert_eq!(
            sanitize::html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_sanitize_normalize_whitespace() {
        assert_eq!(
            sanitize::normalize_whitespace("  multiple   spaces  "),
            "multiple spaces"
        );
    }

    #[test]
    fn test_sanitize_truncate() {
        assert_eq!(sanitize::truncate("hello world", 5), "he...");
        assert_eq!(sanitize::truncate("hi", 5), "hi");
    }
}
