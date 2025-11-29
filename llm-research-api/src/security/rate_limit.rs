//! Rate limiting implementation using token bucket algorithm
//!
//! This module provides a comprehensive rate limiting system with:
//! - Token bucket algorithm for smooth rate limiting
//! - Multiple rate limit scopes (global, per-IP, per-user, per-API key, per-endpoint)
//! - Tower middleware integration
//! - Automatic cleanup of expired entries
//! - Thread-safe concurrent access using DashMap

use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{ConnectInfo, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::{Layer, Service};
use uuid::Uuid;

/// Configuration for rate limiting
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum requests per hour
    pub requests_per_hour: u32,
    /// Maximum burst size (tokens that can accumulate)
    pub burst_size: u32,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(
        requests_per_second: u32,
        requests_per_minute: u32,
        requests_per_hour: u32,
        burst_size: u32,
    ) -> Self {
        Self {
            requests_per_second,
            requests_per_minute,
            requests_per_hour,
            burst_size,
        }
    }

    /// Default configuration for anonymous users (stricter limits)
    pub fn anonymous_limit() -> Self {
        Self {
            requests_per_second: 2,
            requests_per_minute: 60,
            requests_per_hour: 1000,
            burst_size: 5,
        }
    }

    /// Default configuration for authenticated users (more permissive)
    pub fn authenticated_limit() -> Self {
        Self {
            requests_per_second: 10,
            requests_per_minute: 300,
            requests_per_hour: 10000,
            burst_size: 20,
        }
    }

    /// Default configuration for API key users (even more permissive)
    pub fn api_key_limit() -> Self {
        Self {
            requests_per_second: 50,
            requests_per_minute: 1500,
            requests_per_hour: 50000,
            burst_size: 100,
        }
    }
}

/// Information about the current rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Number of requests remaining in the current window
    pub remaining: u32,
    /// Total limit for the current window
    pub limit: u32,
    /// When the rate limit will reset
    pub reset_at: DateTime<Utc>,
}

impl RateLimitInfo {
    pub fn new(remaining: u32, limit: u32, reset_at: DateTime<Utc>) -> Self {
        Self {
            remaining,
            limit,
            reset_at,
        }
    }
}

/// Rate limit error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    #[error("Too many requests. Retry after {retry_after:?}")]
    TooManyRequests { retry_after: Duration },
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        match self {
            RateLimitError::TooManyRequests { retry_after } => {
                let mut headers = HeaderMap::new();
                if let Ok(retry_seconds) = retry_after.as_secs().to_string().parse() {
                    headers.insert("Retry-After", retry_seconds);
                }

                let body = Json(json!({
                    "error": "Too Many Requests",
                    "message": format!("Rate limit exceeded. Retry after {} seconds", retry_after.as_secs()),
                    "retry_after_seconds": retry_after.as_secs(),
                }));

                (StatusCode::TOO_MANY_REQUESTS, headers, body).into_response()
            }
        }
    }
}

/// Type alias for user ID
pub type UserId = Uuid;

/// Different scopes for rate limiting
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RateLimitKey {
    /// Global rate limit (applies to all requests)
    Global,
    /// Rate limit by IP address
    ByIp(IpAddr),
    /// Rate limit by authenticated user ID
    ByUser(UserId),
    /// Rate limit by API key
    ByApiKey(String),
    /// Rate limit by endpoint/route
    ByEndpoint(String),
    /// Combined rate limit (user + endpoint)
    Combined {
        user: Option<UserId>,
        endpoint: String,
    },
}

impl fmt::Display for RateLimitKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateLimitKey::Global => write!(f, "global"),
            RateLimitKey::ByIp(ip) => write!(f, "ip:{}", ip),
            RateLimitKey::ByUser(id) => write!(f, "user:{}", id),
            RateLimitKey::ByApiKey(key) => write!(f, "apikey:{}", key),
            RateLimitKey::ByEndpoint(endpoint) => write!(f, "endpoint:{}", endpoint),
            RateLimitKey::Combined { user, endpoint } => {
                if let Some(user_id) = user {
                    write!(f, "combined:user:{}:endpoint:{}", user_id, endpoint)
                } else {
                    write!(f, "combined:anonymous:endpoint:{}", endpoint)
                }
            }
        }
    }
}

/// Token bucket state for a specific key
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Number of available tokens
    tokens: f64,
    /// Maximum number of tokens (burst size)
    capacity: f64,
    /// Rate at which tokens are refilled (tokens per second)
    refill_rate: f64,
    /// Last time the bucket was updated
    last_update: Instant,
    /// When this bucket will reset (for reporting to clients)
    reset_at: DateTime<Utc>,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        let now = Instant::now();
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_update: now,
            reset_at: Utc::now() + chrono::Duration::seconds(60),
        }
    }

    /// Try to consume a token, return true if successful
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_update = now;

        // Update reset time if we're at or near capacity
        if self.tokens >= self.capacity * 0.99 {
            self.reset_at = Utc::now() + chrono::Duration::seconds(60);
        }
    }

    /// Get the number of remaining tokens
    fn remaining(&self) -> u32 {
        self.tokens.floor() as u32
    }

    /// Get when the next token will be available
    fn next_token_available(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds_needed = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds_needed)
        }
    }
}

/// Rate limiter implementation using token bucket algorithm
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Arc<DashMap<String, TokenBucket>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        let limiter = Self {
            config,
            buckets: Arc::new(DashMap::new()),
        };

        // Start background cleanup task
        limiter.start_cleanup_task();

        limiter
    }

    /// Check if a request is allowed for the given key
    pub fn check(&self, key: &RateLimitKey) -> Result<RateLimitInfo, RateLimitError> {
        let key_str = key.to_string();

        // Get or create bucket for this key
        let mut bucket_ref = self.buckets
            .entry(key_str)
            .or_insert_with(|| {
                let refill_rate = self.config.requests_per_second as f64;
                let capacity = self.config.burst_size as f64;
                TokenBucket::new(capacity, refill_rate)
            });

        let bucket = bucket_ref.value_mut();

        if bucket.try_consume() {
            Ok(RateLimitInfo {
                remaining: bucket.remaining(),
                limit: self.config.burst_size,
                reset_at: bucket.reset_at,
            })
        } else {
            let retry_after = bucket.next_token_available();
            Err(RateLimitError::TooManyRequests { retry_after })
        }
    }

    /// Start a background task to clean up expired buckets
    fn start_cleanup_task(&self) {
        let buckets = Arc::clone(&self.buckets);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;

                // Remove buckets that haven't been used in the last hour
                let now = Instant::now();
                buckets.retain(|_, bucket| {
                    now.duration_since(bucket.last_update) < Duration::from_secs(3600)
                });

                tracing::debug!("Rate limiter cleanup: {} active buckets", buckets.len());
            }
        });
    }

    /// Get the current configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Get the number of active buckets (for monitoring)
    pub fn active_buckets(&self) -> usize {
        self.buckets.len()
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config,
            buckets: Arc::clone(&self.buckets),
        }
    }
}

/// Tower layer for rate limiting
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<RateLimiter>,
}

impl RateLimitLayer {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(config)),
        }
    }

    pub fn with_limiter(limiter: RateLimiter) -> Self {
        Self {
            limiter: Arc::new(limiter),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: Arc::clone(&self.limiter),
        }
    }
}

/// Tower service for rate limiting
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: Arc<RateLimiter>,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let limiter = Arc::clone(&self.limiter);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Extract client identifier from request
            let key = extract_rate_limit_key(&req);

            // Check rate limit
            match limiter.check(&key) {
                Ok(info) => {
                    // Allow request, add rate limit headers
                    let mut response = inner.call(req).await?;
                    add_rate_limit_headers(response.headers_mut(), &info);
                    Ok(response)
                }
                Err(err) => {
                    // Rate limit exceeded
                    Ok(err.into_response())
                }
            }
        })
    }
}

/// Extract rate limit key from request
fn extract_rate_limit_key(req: &Request) -> RateLimitKey {
    // Try to get IP address from ConnectInfo
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return RateLimitKey::ByIp(connect_info.0.ip());
    }

    // Try to get from X-Forwarded-For header
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip_str) = forwarded_str.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                    return RateLimitKey::ByIp(ip);
                }
            }
        }
    }

    // Try to get from X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return RateLimitKey::ByIp(ip);
            }
        }
    }

    // Check for API key in Authorization header
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                return RateLimitKey::ByApiKey(token.to_string());
            }
        }
    }

    // Check for API key in X-API-Key header
    if let Some(api_key) = req.headers().get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            return RateLimitKey::ByApiKey(key_str.to_string());
        }
    }

    // Default to endpoint-based rate limiting
    let endpoint = req.uri().path().to_string();
    RateLimitKey::ByEndpoint(endpoint)
}

/// Add rate limit headers to response
fn add_rate_limit_headers(headers: &mut HeaderMap, info: &RateLimitInfo) {
    if let Ok(limit) = info.limit.to_string().parse() {
        headers.insert("X-RateLimit-Limit", limit);
    }
    if let Ok(remaining) = info.remaining.to_string().parse() {
        headers.insert("X-RateLimit-Remaining", remaining);
    }
    if let Ok(reset) = info.reset_at.timestamp().to_string().parse() {
        headers.insert("X-RateLimit-Reset", reset);
    }
}

/// Axum middleware function for rate limiting
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    req: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    let key = extract_rate_limit_key(&req);

    match limiter.check(&key) {
        Ok(info) => {
            let mut response = next.run(req).await;
            add_rate_limit_headers(response.headers_mut(), &info);
            Ok(response)
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_rate_limit_config_presets() {
        let anon = RateLimitConfig::anonymous_limit();
        assert_eq!(anon.requests_per_second, 2);
        assert_eq!(anon.burst_size, 5);

        let auth = RateLimitConfig::authenticated_limit();
        assert_eq!(auth.requests_per_second, 10);
        assert_eq!(auth.burst_size, 20);

        let api_key = RateLimitConfig::api_key_limit();
        assert_eq!(api_key.requests_per_second, 50);
        assert_eq!(api_key.burst_size, 100);
    }

    #[test]
    fn test_token_bucket_basic() {
        let mut bucket = TokenBucket::new(10.0, 1.0);

        // Should allow initial burst
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }

        // Should deny when empty
        assert!(!bucket.try_consume());
    }

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimitConfig::new(10, 100, 1000, 10);
        let limiter = RateLimiter::new(config);

        let key = RateLimitKey::ByIp(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        // Should allow burst
        for _ in 0..10 {
            assert!(limiter.check(&key).is_ok());
        }

        // Should deny when exhausted
        assert!(limiter.check(&key).is_err());
    }

    #[test]
    fn test_rate_limit_key_display() {
        let ip_key = RateLimitKey::ByIp(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(ip_key.to_string(), "ip:127.0.0.1");

        let user_id = Uuid::new_v4();
        let user_key = RateLimitKey::ByUser(user_id);
        assert_eq!(user_key.to_string(), format!("user:{}", user_id));

        let api_key = RateLimitKey::ByApiKey("test-key".to_string());
        assert_eq!(api_key.to_string(), "apikey:test-key");

        let endpoint_key = RateLimitKey::ByEndpoint("/api/test".to_string());
        assert_eq!(endpoint_key.to_string(), "endpoint:/api/test");

        let combined_key = RateLimitKey::Combined {
            user: Some(user_id),
            endpoint: "/api/test".to_string(),
        };
        assert_eq!(
            combined_key.to_string(),
            format!("combined:user:{}:endpoint:/api/test", user_id)
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let config = RateLimitConfig::new(10, 100, 1000, 5);
        let limiter = RateLimiter::new(config);

        let key = RateLimitKey::Global;

        // Exhaust initial tokens
        for _ in 0..5 {
            assert!(limiter.check(&key).is_ok());
        }
        assert!(limiter.check(&key).is_err());

        // Wait for tokens to refill (at 10/sec, should get 1 token in ~100ms)
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should allow at least one more request
        assert!(limiter.check(&key).is_ok());
    }

    #[test]
    fn test_rate_limit_info() {
        let now = Utc::now();
        let info = RateLimitInfo::new(42, 100, now);

        assert_eq!(info.remaining, 42);
        assert_eq!(info.limit, 100);
        assert_eq!(info.reset_at, now);
    }

    #[tokio::test]
    async fn test_different_keys_independent() {
        let config = RateLimitConfig::new(10, 100, 1000, 5);
        let limiter = RateLimiter::new(config);

        let key1 = RateLimitKey::ByIp(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let key2 = RateLimitKey::ByIp(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));

        // Exhaust key1
        for _ in 0..5 {
            assert!(limiter.check(&key1).is_ok());
        }
        assert!(limiter.check(&key1).is_err());

        // key2 should still work
        assert!(limiter.check(&key2).is_ok());
    }
}
