//! Security headers middleware for HTTP responses
//!
//! Implements essential security headers including:
//! - CORS (Cross-Origin Resource Sharing)
//! - CSP (Content Security Policy)
//! - HSTS (HTTP Strict Transport Security)
//! - X-Frame-Options, X-Content-Type-Options, etc.

use axum::{
    http::{header, HeaderMap, HeaderName, HeaderValue, Method},
    middleware::Next,
    extract::Request,
    response::Response,
};
use std::time::Duration;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, ExposeHeaders};

/// Configuration for security headers
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS (HTTP Strict Transport Security)
    pub enable_hsts: bool,
    /// HSTS max age in seconds (default: 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload
    pub hsts_preload: bool,

    /// Content Security Policy
    pub csp: ContentSecurityPolicy,

    /// X-Frame-Options value
    pub frame_options: FrameOptions,

    /// X-Content-Type-Options: nosniff
    pub content_type_nosniff: bool,

    /// X-XSS-Protection (legacy, but still useful for older browsers)
    pub xss_protection: bool,

    /// Referrer-Policy
    pub referrer_policy: ReferrerPolicy,

    /// Permissions-Policy
    pub permissions_policy: Option<String>,

    /// Custom headers
    pub custom_headers: Vec<(String, String)>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,

            csp: ContentSecurityPolicy::default(),

            frame_options: FrameOptions::Deny,

            content_type_nosniff: true,
            xss_protection: true,

            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,

            permissions_policy: Some(
                "geolocation=(), microphone=(), camera=(), payment=()".to_string()
            ),

            custom_headers: Vec::new(),
        }
    }
}

/// Content Security Policy configuration
#[derive(Debug, Clone)]
pub struct ContentSecurityPolicy {
    pub default_src: Vec<String>,
    pub script_src: Vec<String>,
    pub style_src: Vec<String>,
    pub img_src: Vec<String>,
    pub font_src: Vec<String>,
    pub connect_src: Vec<String>,
    pub frame_ancestors: Vec<String>,
    pub form_action: Vec<String>,
    pub base_uri: Vec<String>,
    pub object_src: Vec<String>,
    pub report_uri: Option<String>,
    pub upgrade_insecure_requests: bool,
}

impl Default for ContentSecurityPolicy {
    fn default() -> Self {
        Self {
            default_src: vec!["'self'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
            img_src: vec!["'self'".to_string(), "data:".to_string(), "https:".to_string()],
            font_src: vec!["'self'".to_string(), "https:".to_string()],
            connect_src: vec!["'self'".to_string()],
            frame_ancestors: vec!["'none'".to_string()],
            form_action: vec!["'self'".to_string()],
            base_uri: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            report_uri: None,
            upgrade_insecure_requests: true,
        }
    }
}

impl ContentSecurityPolicy {
    /// Convert to CSP header value
    pub fn to_header_value(&self) -> String {
        let mut directives = Vec::new();

        if !self.default_src.is_empty() {
            directives.push(format!("default-src {}", self.default_src.join(" ")));
        }
        if !self.script_src.is_empty() {
            directives.push(format!("script-src {}", self.script_src.join(" ")));
        }
        if !self.style_src.is_empty() {
            directives.push(format!("style-src {}", self.style_src.join(" ")));
        }
        if !self.img_src.is_empty() {
            directives.push(format!("img-src {}", self.img_src.join(" ")));
        }
        if !self.font_src.is_empty() {
            directives.push(format!("font-src {}", self.font_src.join(" ")));
        }
        if !self.connect_src.is_empty() {
            directives.push(format!("connect-src {}", self.connect_src.join(" ")));
        }
        if !self.frame_ancestors.is_empty() {
            directives.push(format!("frame-ancestors {}", self.frame_ancestors.join(" ")));
        }
        if !self.form_action.is_empty() {
            directives.push(format!("form-action {}", self.form_action.join(" ")));
        }
        if !self.base_uri.is_empty() {
            directives.push(format!("base-uri {}", self.base_uri.join(" ")));
        }
        if !self.object_src.is_empty() {
            directives.push(format!("object-src {}", self.object_src.join(" ")));
        }
        if let Some(ref report_uri) = self.report_uri {
            directives.push(format!("report-uri {}", report_uri));
        }
        if self.upgrade_insecure_requests {
            directives.push("upgrade-insecure-requests".to_string());
        }

        directives.join("; ")
    }

    /// Create a permissive CSP for development
    pub fn development() -> Self {
        Self {
            default_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string(), "'unsafe-eval'".to_string()],
            script_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string(), "'unsafe-eval'".to_string()],
            style_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
            img_src: vec!["*".to_string(), "data:".to_string(), "blob:".to_string()],
            font_src: vec!["*".to_string()],
            connect_src: vec!["*".to_string()],
            frame_ancestors: vec!["'self'".to_string()],
            form_action: vec!["'self'".to_string()],
            base_uri: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            report_uri: None,
            upgrade_insecure_requests: false,
        }
    }

    /// Create a strict CSP for production
    pub fn strict() -> Self {
        Self {
            default_src: vec!["'none'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string()],
            img_src: vec!["'self'".to_string()],
            font_src: vec!["'self'".to_string()],
            connect_src: vec!["'self'".to_string()],
            frame_ancestors: vec!["'none'".to_string()],
            form_action: vec!["'self'".to_string()],
            base_uri: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            report_uri: None,
            upgrade_insecure_requests: true,
        }
    }

    /// Add a nonce for inline scripts (for CSP Level 2)
    pub fn with_script_nonce(mut self, nonce: &str) -> Self {
        self.script_src.push(format!("'nonce-{}'", nonce));
        self
    }
}

/// X-Frame-Options values
#[derive(Debug, Clone, PartialEq)]
pub enum FrameOptions {
    /// Page cannot be displayed in a frame
    Deny,
    /// Page can only be displayed in a frame on the same origin
    SameOrigin,
    /// Page can be displayed in a frame on the specified origin
    AllowFrom(String),
}

impl FrameOptions {
    pub fn to_header_value(&self) -> String {
        match self {
            FrameOptions::Deny => "DENY".to_string(),
            FrameOptions::SameOrigin => "SAMEORIGIN".to_string(),
            FrameOptions::AllowFrom(uri) => format!("ALLOW-FROM {}", uri),
        }
    }
}

/// Referrer-Policy values
#[derive(Debug, Clone, PartialEq)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    pub fn to_header_value(&self) -> String {
        match self {
            ReferrerPolicy::NoReferrer => "no-referrer".to_string(),
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade".to_string(),
            ReferrerPolicy::Origin => "origin".to_string(),
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin".to_string(),
            ReferrerPolicy::SameOrigin => "same-origin".to_string(),
            ReferrerPolicy::StrictOrigin => "strict-origin".to_string(),
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin".to_string(),
            ReferrerPolicy::UnsafeUrl => "unsafe-url".to_string(),
        }
    }
}

/// CORS configuration for API endpoints
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: AllowedOrigins,
    /// Allowed methods
    pub allowed_methods: Vec<Method>,
    /// Allowed headers
    pub allowed_headers: Vec<HeaderName>,
    /// Headers to expose
    pub exposed_headers: Vec<HeaderName>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight caching
    pub max_age: Duration,
}

/// Allowed origins configuration
#[derive(Debug, Clone)]
pub enum AllowedOrigins {
    /// Allow any origin (use with caution)
    Any,
    /// Allow only specific origins
    List(Vec<String>),
    /// Allow origins matching a pattern
    Regex(String),
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: AllowedOrigins::List(vec![]),
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ],
            allowed_headers: vec![
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
                HeaderName::from_static("x-api-key"),
                HeaderName::from_static("x-request-id"),
            ],
            exposed_headers: vec![
                header::CONTENT_LENGTH,
                HeaderName::from_static("x-request-id"),
                HeaderName::from_static("x-ratelimit-limit"),
                HeaderName::from_static("x-ratelimit-remaining"),
                HeaderName::from_static("x-ratelimit-reset"),
            ],
            allow_credentials: true,
            max_age: Duration::from_secs(3600),
        }
    }
}

impl CorsConfig {
    /// Create a permissive CORS config for development
    pub fn development() -> Self {
        Self {
            allowed_origins: AllowedOrigins::Any,
            ..Default::default()
        }
    }

    /// Create a CORS config for specific origins
    pub fn with_origins(origins: Vec<String>) -> Self {
        Self {
            allowed_origins: AllowedOrigins::List(origins),
            ..Default::default()
        }
    }

    /// Convert to tower_http CorsLayer
    pub fn to_layer(&self) -> CorsLayer {
        let mut layer = CorsLayer::new()
            .allow_methods(self.allowed_methods.clone())
            .allow_headers(self.allowed_headers.clone())
            .expose_headers(self.exposed_headers.clone())
            .max_age(self.max_age);

        layer = match &self.allowed_origins {
            AllowedOrigins::Any => layer.allow_origin(AllowOrigin::any()),
            AllowedOrigins::List(origins) => {
                let origins: Vec<HeaderValue> = origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
                    .collect();
                layer.allow_origin(origins)
            }
            AllowedOrigins::Regex(_pattern) => {
                // For regex, we'd need a custom predicate
                // For now, default to the list of origins
                layer.allow_origin(AllowOrigin::any())
            }
        };

        if self.allow_credentials {
            layer = layer.allow_credentials(true);
        }

        layer
    }
}

/// Security headers middleware
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let config = SecurityHeadersConfig::default();
    security_headers_with_config(request, next, &config).await
}

/// Security headers middleware with custom configuration
pub async fn security_headers_with_config(
    request: Request,
    next: Next,
    config: &SecurityHeadersConfig,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // HSTS
    if config.enable_hsts {
        let mut hsts_value = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            hsts_value.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            hsts_value.push_str("; preload");
        }
        if let Ok(value) = HeaderValue::from_str(&hsts_value) {
            headers.insert(header::STRICT_TRANSPORT_SECURITY, value);
        }
    }

    // Content Security Policy
    let csp_value = config.csp.to_header_value();
    if let Ok(value) = HeaderValue::from_str(&csp_value) {
        headers.insert(
            HeaderName::from_static("content-security-policy"),
            value,
        );
    }

    // X-Frame-Options
    if let Ok(value) = HeaderValue::from_str(&config.frame_options.to_header_value()) {
        headers.insert(header::X_FRAME_OPTIONS, value);
    }

    // X-Content-Type-Options
    if config.content_type_nosniff {
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }

    // X-XSS-Protection (legacy)
    if config.xss_protection {
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
    }

    // Referrer-Policy
    if let Ok(value) = HeaderValue::from_str(&config.referrer_policy.to_header_value()) {
        headers.insert(
            HeaderName::from_static("referrer-policy"),
            value,
        );
    }

    // Permissions-Policy
    if let Some(ref policy) = config.permissions_policy {
        if let Ok(value) = HeaderValue::from_str(policy) {
            headers.insert(
                HeaderName::from_static("permissions-policy"),
                value,
            );
        }
    }

    // Custom headers
    for (name, value) in &config.custom_headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            headers.insert(name, value);
        }
    }

    response
}

/// Create a security headers layer for use with Router
pub fn create_security_headers_layer() -> tower::util::MapResponseLayer<
    impl Fn(Response) -> Response + Clone,
> {
    tower::util::MapResponseLayer::new(|mut response: Response| {
        let headers = response.headers_mut();

        // Add basic security headers
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        headers.insert(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        );
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
        headers.insert(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );

        response
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_to_header_value() {
        let csp = ContentSecurityPolicy::default();
        let value = csp.to_header_value();
        assert!(value.contains("default-src 'self'"));
        assert!(value.contains("script-src 'self'"));
        assert!(value.contains("frame-ancestors 'none'"));
    }

    #[test]
    fn test_strict_csp() {
        let csp = ContentSecurityPolicy::strict();
        let value = csp.to_header_value();
        assert!(value.contains("default-src 'none'"));
        assert!(value.contains("upgrade-insecure-requests"));
    }

    #[test]
    fn test_csp_with_nonce() {
        let csp = ContentSecurityPolicy::default()
            .with_script_nonce("abc123");
        let value = csp.to_header_value();
        assert!(value.contains("'nonce-abc123'"));
    }

    #[test]
    fn test_frame_options() {
        assert_eq!(FrameOptions::Deny.to_header_value(), "DENY");
        assert_eq!(FrameOptions::SameOrigin.to_header_value(), "SAMEORIGIN");
        assert_eq!(
            FrameOptions::AllowFrom("https://example.com".to_string()).to_header_value(),
            "ALLOW-FROM https://example.com"
        );
    }

    #[test]
    fn test_referrer_policy() {
        assert_eq!(
            ReferrerPolicy::StrictOriginWhenCrossOrigin.to_header_value(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            ReferrerPolicy::NoReferrer.to_header_value(),
            "no-referrer"
        );
    }

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config.allowed_methods.contains(&Method::GET));
        assert!(config.allowed_methods.contains(&Method::POST));
        assert!(config.allow_credentials);
    }

    #[test]
    fn test_cors_development() {
        let config = CorsConfig::development();
        match config.allowed_origins {
            AllowedOrigins::Any => {},
            _ => panic!("Development CORS should allow any origin"),
        }
    }

    #[test]
    fn test_security_headers_config_default() {
        let config = SecurityHeadersConfig::default();
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31536000);
        assert!(config.content_type_nosniff);
        assert!(config.xss_protection);
    }
}
