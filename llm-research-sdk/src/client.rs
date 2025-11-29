//! HTTP client implementation
//!
//! This module provides the core HTTP client for the SDK with
//! retry logic, rate limiting handling, and request/response logging.

use crate::config::{AuthConfig, SdkConfig};
use crate::error::{SdkError, SdkResult};
use reqwest::{header, Client, Method, RequestBuilder, Response, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// The HTTP client for making API requests
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    config: Arc<SdkConfig>,
}

impl HttpClient {
    /// Create a new HTTP client with the given configuration
    pub fn new(config: SdkConfig) -> SdkResult<Self> {
        config.validate()?;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        // Add custom headers
        for (name, value) in &config.custom_headers {
            if let (Ok(name), Ok(value)) = (
                header::HeaderName::try_from(name.as_str()),
                header::HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(name, value);
            }
        }

        let client = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .user_agent(&config.user_agent)
            .default_headers(headers)
            .gzip(true)
            .brotli(true)
            .build()
            .map_err(SdkError::NetworkError)?;

        Ok(Self {
            client,
            config: Arc::new(config),
        })
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &SdkConfig {
        &self.config
    }

    /// Build the full URL for an endpoint
    pub fn url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> SdkResult<T> {
        self.request(Method::GET, path, Option::<()>::None).await
    }

    /// Make a GET request with query parameters
    pub async fn get_with_query<T: DeserializeOwned, Q: Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> SdkResult<T> {
        self.request_with_query(Method::GET, path, Option::<()>::None, Some(query))
            .await
    }

    /// Make a POST request
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: B,
    ) -> SdkResult<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    /// Make a PUT request
    pub async fn put<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: B,
    ) -> SdkResult<T> {
        self.request(Method::PUT, path, Some(body)).await
    }

    /// Make a PATCH request
    pub async fn patch<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: B,
    ) -> SdkResult<T> {
        self.request(Method::PATCH, path, Some(body)).await
    }

    /// Make a DELETE request
    pub async fn delete(&self, path: &str) -> SdkResult<()> {
        self.request_no_response(Method::DELETE, path, Option::<()>::None)
            .await
    }

    /// Make a request with optional body
    async fn request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
    ) -> SdkResult<T> {
        self.request_with_query::<T, B, ()>(method, path, body, None)
            .await
    }

    /// Make a request with optional body and query parameters
    async fn request_with_query<T: DeserializeOwned, B: Serialize, Q: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
        query: Option<&Q>,
    ) -> SdkResult<T> {
        let response = self
            .execute_with_retry(method, path, body, query)
            .await?;

        let status = response.status();
        let text = response.text().await.map_err(SdkError::NetworkError)?;

        if self.config.enable_logging {
            debug!("Response body: {}", text);
        }

        if status.is_success() {
            serde_json::from_str(&text).map_err(SdkError::SerializationError)
        } else {
            Err(self.handle_error_response(status, &text))
        }
    }

    /// Make a request that doesn't return a body
    async fn request_no_response<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
    ) -> SdkResult<()> {
        let response = self
            .execute_with_retry::<B, ()>(method, path, body, None)
            .await?;

        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            let text = response.text().await.map_err(SdkError::NetworkError)?;
            Err(self.handle_error_response(status, &text))
        }
    }

    /// Execute a request with retry logic
    async fn execute_with_retry<B: Serialize, Q: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
        query: Option<&Q>,
    ) -> SdkResult<Response> {
        let url = self.url(path);
        let body_json = body.as_ref().map(|b| serde_json::to_string(b).ok()).flatten();

        let mut attempts = 0;
        let mut last_error: Option<SdkError> = None;
        let mut backoff = self.config.retry_initial_backoff;

        while attempts <= self.config.max_retries {
            if attempts > 0 {
                info!(
                    "Retrying request (attempt {}/{}), waiting {:?}",
                    attempts, self.config.max_retries, backoff
                );
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, self.config.retry_max_backoff);
            }

            let mut request = self.client.request(method.clone(), &url);

            // Add authentication
            request = self.add_auth(request);

            // Add query parameters
            if let Some(q) = query {
                request = request.query(q);
            }

            // Add body
            if let Some(ref body_str) = body_json {
                request = request.body(body_str.clone());
            }

            if self.config.enable_logging {
                debug!("Request: {} {}", method, url);
                if let Some(ref body_str) = body_json {
                    debug!("Request body: {}", body_str);
                }
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();

                    // Check for rate limiting
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = response
                            .headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(60);

                        warn!("Rate limited, retry after {} seconds", retry_after);

                        if attempts < self.config.max_retries {
                            last_error = Some(SdkError::RateLimited {
                                retry_after,
                                limit: 0,
                                remaining: 0,
                            });
                            backoff = Duration::from_secs(retry_after);
                            attempts += 1;
                            continue;
                        }
                    }

                    // Check for retryable server errors
                    if status.is_server_error() && attempts < self.config.max_retries {
                        warn!("Server error {}, will retry", status);
                        last_error = Some(SdkError::ServerError(format!("Status: {}", status)));
                        attempts += 1;
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    error!("Request failed: {}", e);

                    if e.is_timeout() {
                        last_error = Some(SdkError::Timeout(self.config.timeout.as_secs()));
                    } else if e.is_connect() || e.is_request() {
                        last_error = Some(SdkError::NetworkError(e));
                    } else {
                        return Err(SdkError::NetworkError(e));
                    }

                    attempts += 1;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SdkError::Unknown("Request failed".to_string())))
    }

    /// Add authentication to a request
    fn add_auth(&self, request: RequestBuilder) -> RequestBuilder {
        match &self.config.auth {
            AuthConfig::None => request,
            AuthConfig::ApiKey(key) => request.header("X-API-Key", key.as_str()),
            AuthConfig::BearerToken(token) => {
                request.header(header::AUTHORIZATION, format!("Bearer {}", token))
            }
            AuthConfig::Basic { username, password } => request.basic_auth(username, Some(password)),
        }
    }

    /// Handle an error response
    fn handle_error_response(&self, status: StatusCode, body: &str) -> SdkError {
        let request_id = None; // TODO: Extract from response headers

        match status {
            StatusCode::BAD_REQUEST => {
                SdkError::from_response(status.as_u16(), body, request_id)
            }
            StatusCode::UNAUTHORIZED => {
                SdkError::AuthenticationError("Invalid or missing authentication".to_string())
            }
            StatusCode::FORBIDDEN => {
                SdkError::AuthorizationError("Access denied".to_string())
            }
            StatusCode::NOT_FOUND => SdkError::NotFound {
                resource_type: "unknown".to_string(),
                resource_id: "unknown".to_string(),
            },
            StatusCode::CONFLICT => SdkError::Conflict("Resource conflict".to_string()),
            StatusCode::UNPROCESSABLE_ENTITY => {
                SdkError::from_response(status.as_u16(), body, request_id)
            }
            StatusCode::TOO_MANY_REQUESTS => SdkError::RateLimited {
                retry_after: 60,
                limit: 0,
                remaining: 0,
            },
            _ if status.is_server_error() => {
                SdkError::ServerError(format!("Server error: {}", status))
            }
            _ => SdkError::from_response(status.as_u16(), body, request_id),
        }
    }
}

/// Pagination parameters for list requests
#[derive(Debug, Clone, Serialize, Default)]
pub struct PaginationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl PaginationParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

/// Pagination information in response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaginationInfo {
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_building() {
        let config = SdkConfig::new("https://api.example.com");
        let client = HttpClient::new(config).unwrap();

        assert_eq!(
            client.url("/experiments"),
            "https://api.example.com/experiments"
        );
        assert_eq!(
            client.url("experiments"),
            "https://api.example.com/experiments"
        );
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams::new()
            .with_limit(10)
            .with_offset(20);

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(20));
    }
}
