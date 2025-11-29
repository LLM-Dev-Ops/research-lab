//! SDK configuration
//!
//! This module provides configuration options for the SDK client.

use std::time::Duration;
use crate::error::{SdkError, SdkResult};

/// Configuration for the SDK client
#[derive(Debug, Clone)]
pub struct SdkConfig {
    /// Base URL for the API
    pub base_url: String,

    /// Authentication method
    pub auth: AuthConfig,

    /// Request timeout
    pub timeout: Duration,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Initial backoff duration for retries
    pub retry_initial_backoff: Duration,

    /// Maximum backoff duration for retries
    pub retry_max_backoff: Duration,

    /// User agent string
    pub user_agent: String,

    /// Enable request/response logging
    pub enable_logging: bool,

    /// Custom headers to add to all requests
    pub custom_headers: Vec<(String, String)>,
}

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.llm-research-lab.io".to_string(),
            auth: AuthConfig::None,
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_initial_backoff: Duration::from_millis(100),
            retry_max_backoff: Duration::from_secs(30),
            user_agent: format!("llm-research-sdk/{}", env!("CARGO_PKG_VERSION")),
            enable_logging: false,
            custom_headers: Vec::new(),
        }
    }
}

impl SdkConfig {
    /// Create a new configuration with the given base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Create a new builder with the given base URL
    pub fn builder(base_url: impl Into<String>) -> SdkConfigBuilder {
        SdkConfigBuilder {
            config: Self::new(base_url),
        }
    }

    /// Set the authentication method
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = auth;
        self
    }

    /// Set the API key for authentication
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.auth = AuthConfig::ApiKey(api_key.into());
        self
    }

    /// Set the bearer token for authentication
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = AuthConfig::BearerToken(token.into());
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the retry backoff configuration
    pub fn with_retry_backoff(
        mut self,
        initial: Duration,
        max: Duration,
    ) -> Self {
        self.retry_initial_backoff = initial;
        self.retry_max_backoff = max;
        self
    }

    /// Set the user agent string
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Enable request/response logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Add a custom header to all requests
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.push((name.into(), value.into()));
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> SdkResult<()> {
        if self.base_url.is_empty() {
            return Err(SdkError::ConfigurationError(
                "Base URL cannot be empty".to_string(),
            ));
        }

        // Validate URL format
        url::Url::parse(&self.base_url)?;

        if self.timeout.is_zero() {
            return Err(SdkError::ConfigurationError(
                "Timeout cannot be zero".to_string(),
            ));
        }

        Ok(())
    }
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// No authentication
    None,

    /// API key authentication
    ApiKey(String),

    /// Bearer token (JWT) authentication
    BearerToken(String),

    /// Username and password for basic auth
    Basic { username: String, password: String },
}

impl AuthConfig {
    /// Get the authorization header value
    pub fn to_header_value(&self) -> Option<String> {
        match self {
            AuthConfig::None => None,
            AuthConfig::ApiKey(key) => Some(key.clone()),
            AuthConfig::BearerToken(token) => Some(format!("Bearer {}", token)),
            AuthConfig::Basic { username, password } => {
                let credentials = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    format!("{}:{}", username, password),
                );
                Some(format!("Basic {}", credentials))
            }
        }
    }

    /// Check if authentication is configured
    pub fn is_configured(&self) -> bool {
        !matches!(self, AuthConfig::None)
    }
}

/// Builder for SDK configuration
#[derive(Debug, Default)]
pub struct SdkConfigBuilder {
    config: SdkConfig,
}

impl SdkConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.auth = AuthConfig::ApiKey(key.into());
        self
    }

    /// Set the bearer token
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.config.auth = AuthConfig::BearerToken(token.into());
        self
    }

    /// Set the timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set max retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Enable logging
    pub fn logging(mut self, enable: bool) -> Self {
        self.config.enable_logging = enable;
        self
    }

    /// Add a custom header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.custom_headers.push((name.into(), value.into()));
        self
    }

    /// Set the authentication method
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.config.auth = auth;
        self
    }

    /// Set the timeout (alias for convenience)
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the connect timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set max retries (alias for convenience)
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Enable/disable logging (alias for convenience)
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.config.enable_logging = enable;
        self
    }

    /// Add a custom header (alias for convenience)
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.custom_headers.push((name.into(), value.into()));
        self
    }

    /// Build the configuration
    pub fn build(self) -> SdkConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SdkConfig::default();
        assert_eq!(config.base_url, "https://api.llm-research-lab.io");
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_builder() {
        let config = SdkConfigBuilder::new()
            .base_url("https://api.example.com")
            .api_key("test-key")
            .timeout(Duration::from_secs(60))
            .build();

        assert_eq!(config.base_url, "https://api.example.com");
        assert!(matches!(config.auth, AuthConfig::ApiKey(_)));
    }

    #[test]
    fn test_auth_header() {
        let api_key = AuthConfig::ApiKey("my-key".to_string());
        assert_eq!(api_key.to_header_value(), Some("my-key".to_string()));

        let bearer = AuthConfig::BearerToken("my-token".to_string());
        assert_eq!(
            bearer.to_header_value(),
            Some("Bearer my-token".to_string())
        );

        let none = AuthConfig::None;
        assert_eq!(none.to_header_value(), None);
    }

    #[test]
    fn test_invalid_config() {
        let config = SdkConfig::new("");
        assert!(config.validate().is_err());
    }
}
