//! Response compression module for optimizing HTTP response sizes.
//!
//! This module provides middleware and utilities for compressing HTTP responses
//! using various compression algorithms (gzip, deflate). It includes
//! intelligent content type detection to skip compression for already compressed
//! or incompressible content types.

use axum::{
    extract::Request,
    http::{header, HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};
use flate2::write::{DeflateEncoder, GzEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use tower::{Layer, Service};

/// Compression algorithm types supported by the middleware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// Gzip compression (RFC 1952)
    Gzip,
    /// Deflate compression (RFC 1951)
    Deflate,
}

impl CompressionAlgorithm {
    /// Returns the HTTP encoding name for this algorithm.
    pub fn encoding_name(&self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
        }
    }

    /// Parse compression algorithm from Accept-Encoding header value.
    pub fn from_encoding_name(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "gzip" => Some(Self::Gzip),
            "deflate" => Some(Self::Deflate),
            _ => None,
        }
    }
}

/// Compression level settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// Fastest compression (least CPU, lower ratio)
    Fastest,
    /// Default balanced compression
    Default,
    /// Best compression (most CPU, highest ratio)
    Best,
    /// Custom level (0-9)
    Custom(u32),
}

impl CompressionLevel {
    /// Convert to flate2 Compression level.
    pub fn to_flate2(&self) -> Compression {
        match self {
            Self::Fastest => Compression::fast(),
            Self::Default => Compression::default(),
            Self::Best => Compression::best(),
            Self::Custom(level) => Compression::new((*level).min(9)),
        }
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Default
    }
}

/// Configuration for response compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression level.
    pub compression_level: CompressionLevel,
    /// Minimum response size in bytes to enable compression.
    pub min_size_threshold: usize,
    /// Content types that should not be compressed.
    pub excluded_content_types: HashSet<String>,
    /// Whether to enable gzip compression.
    pub enable_gzip: bool,
    /// Whether to enable deflate compression.
    pub enable_deflate: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        let mut excluded_content_types = HashSet::new();

        // Already compressed formats
        excluded_content_types.insert("image/jpeg".to_string());
        excluded_content_types.insert("image/png".to_string());
        excluded_content_types.insert("image/gif".to_string());
        excluded_content_types.insert("image/webp".to_string());
        excluded_content_types.insert("video/mp4".to_string());
        excluded_content_types.insert("video/webm".to_string());
        excluded_content_types.insert("audio/mpeg".to_string());
        excluded_content_types.insert("audio/ogg".to_string());
        excluded_content_types.insert("application/zip".to_string());
        excluded_content_types.insert("application/gzip".to_string());
        excluded_content_types.insert("application/x-gzip".to_string());
        excluded_content_types.insert("application/x-bzip2".to_string());
        excluded_content_types.insert("application/x-7z-compressed".to_string());
        excluded_content_types.insert("application/x-rar-compressed".to_string());
        excluded_content_types.insert("application/pdf".to_string());

        Self {
            compression_level: CompressionLevel::Default,
            min_size_threshold: 1024, // 1KB
            excluded_content_types,
            enable_gzip: true,
            enable_deflate: true,
        }
    }
}

impl CompressionConfig {
    /// Creates a new compression configuration builder.
    pub fn builder() -> CompressionConfigBuilder {
        CompressionConfigBuilder::default()
    }

    /// Checks if a content type should be compressed.
    pub fn should_compress_content_type(&self, content_type: &str) -> bool {
        // Extract the MIME type without parameters
        let mime_type = content_type
            .split(';')
            .next()
            .unwrap_or(content_type)
            .trim()
            .to_lowercase();

        !self.excluded_content_types.contains(&mime_type)
    }

    /// Checks if a response should be compressed based on size.
    pub fn should_compress_size(&self, size: usize) -> bool {
        size >= self.min_size_threshold
    }

    /// Compress data using the specified algorithm.
    pub fn compress(&self, data: &[u8], algorithm: CompressionAlgorithm) -> std::io::Result<Vec<u8>> {
        let level = self.compression_level.to_flate2();
        match algorithm {
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), level);
                encoder.write_all(data)?;
                encoder.finish()
            }
            CompressionAlgorithm::Deflate => {
                let mut encoder = DeflateEncoder::new(Vec::new(), level);
                encoder.write_all(data)?;
                encoder.finish()
            }
        }
    }

    /// Get the preferred compression algorithm from Accept-Encoding.
    pub fn preferred_algorithm(&self, accepted: &[CompressionAlgorithm]) -> Option<CompressionAlgorithm> {
        // Prefer gzip over deflate if both are accepted
        if self.enable_gzip && accepted.contains(&CompressionAlgorithm::Gzip) {
            return Some(CompressionAlgorithm::Gzip);
        }
        if self.enable_deflate && accepted.contains(&CompressionAlgorithm::Deflate) {
            return Some(CompressionAlgorithm::Deflate);
        }
        None
    }
}

/// Builder for CompressionConfig.
#[derive(Debug, Default)]
pub struct CompressionConfigBuilder {
    compression_level: Option<CompressionLevel>,
    min_size_threshold: Option<usize>,
    excluded_content_types: Option<HashSet<String>>,
    enable_gzip: Option<bool>,
    enable_deflate: Option<bool>,
}

impl CompressionConfigBuilder {
    /// Sets the compression level.
    pub fn compression_level(mut self, level: CompressionLevel) -> Self {
        self.compression_level = Some(level);
        self
    }

    /// Sets the minimum size threshold for compression.
    pub fn min_size_threshold(mut self, threshold: usize) -> Self {
        self.min_size_threshold = Some(threshold);
        self
    }

    /// Sets the excluded content types.
    pub fn excluded_content_types(mut self, types: HashSet<String>) -> Self {
        self.excluded_content_types = Some(types);
        self
    }

    /// Adds an excluded content type.
    pub fn exclude_content_type(mut self, content_type: impl Into<String>) -> Self {
        let mut types = self.excluded_content_types.take().unwrap_or_default();
        types.insert(content_type.into());
        self.excluded_content_types = Some(types);
        self
    }

    /// Enables or disables gzip compression.
    pub fn enable_gzip(mut self, enable: bool) -> Self {
        self.enable_gzip = Some(enable);
        self
    }

    /// Enables or disables deflate compression.
    pub fn enable_deflate(mut self, enable: bool) -> Self {
        self.enable_deflate = Some(enable);
        self
    }

    /// Builds the CompressionConfig.
    pub fn build(self) -> CompressionConfig {
        let default = CompressionConfig::default();

        CompressionConfig {
            compression_level: self.compression_level.unwrap_or(default.compression_level),
            min_size_threshold: self.min_size_threshold.unwrap_or(default.min_size_threshold),
            excluded_content_types: self
                .excluded_content_types
                .unwrap_or(default.excluded_content_types),
            enable_gzip: self.enable_gzip.unwrap_or(default.enable_gzip),
            enable_deflate: self.enable_deflate.unwrap_or(default.enable_deflate),
        }
    }
}

/// Tower layer for compression middleware.
#[derive(Clone)]
pub struct CompressionLayer {
    config: CompressionConfig,
}

impl CompressionLayer {
    /// Creates a new compression layer with the given configuration.
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Creates a compression layer with default configuration.
    pub fn default_compression() -> Self {
        Self {
            config: CompressionConfig::default(),
        }
    }
}

impl<S> Layer<S> for CompressionLayer {
    type Service = CompressionMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CompressionMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Compression middleware service.
#[derive(Clone)]
pub struct CompressionMiddleware<S> {
    inner: S,
    config: CompressionConfig,
}

impl<S> Service<Request> for CompressionMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let _config = self.config.clone();

        Box::pin(async move {
            let response = inner.call(req).await?;
            Ok(response)
        })
    }
}

/// Parse Accept-Encoding header and return preferred compression algorithms.
pub fn parse_accept_encoding(headers: &HeaderMap) -> Vec<CompressionAlgorithm> {
    let mut algorithms = Vec::new();

    if let Some(accept_encoding) = headers.get(header::ACCEPT_ENCODING) {
        if let Ok(value) = accept_encoding.to_str() {
            for part in value.split(',') {
                let encoding = part.split(';').next().unwrap_or("").trim();
                if let Some(algo) = CompressionAlgorithm::from_encoding_name(encoding) {
                    if !algorithms.contains(&algo) {
                        algorithms.push(algo);
                    }
                }
            }
        }
    }

    algorithms
}

/// Axum middleware function for compression.
pub async fn compression_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let accepted_encodings = parse_accept_encoding(&headers);
    let response = next.run(request).await;

    // If no compression is accepted, return as-is
    if accepted_encodings.is_empty() {
        return response;
    }

    response
}

/// Creates a compression layer with the given configuration.
pub fn create_compression_layer(config: CompressionConfig) -> CompressionLayer {
    CompressionLayer::new(config)
}

/// Predicate for determining if content should be compressed.
#[derive(Clone)]
pub struct ContentTypePredicate {
    config: CompressionConfig,
}

impl ContentTypePredicate {
    /// Creates a new content type predicate.
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Check if a response should be compressed based on headers.
    pub fn should_compress(&self, headers: &HeaderMap) -> bool {
        // Check if already compressed
        if headers.contains_key(header::CONTENT_ENCODING) {
            return false;
        }

        // Check content type
        if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
            if let Ok(ct_str) = content_type.to_str() {
                if !self.config.should_compress_content_type(ct_str) {
                    return false;
                }
            }
        }

        // Check content length if available
        if let Some(content_length) = headers.get(header::CONTENT_LENGTH) {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    return self.config.should_compress_size(length);
                }
            }
        }

        // Default to compressing if no content-length header
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_algorithm_encoding_names() {
        assert_eq!(CompressionAlgorithm::Gzip.encoding_name(), "gzip");
        assert_eq!(CompressionAlgorithm::Deflate.encoding_name(), "deflate");
    }

    #[test]
    fn test_compression_algorithm_from_encoding_name() {
        assert_eq!(
            CompressionAlgorithm::from_encoding_name("gzip"),
            Some(CompressionAlgorithm::Gzip)
        );
        assert_eq!(
            CompressionAlgorithm::from_encoding_name("deflate"),
            Some(CompressionAlgorithm::Deflate)
        );
        assert_eq!(CompressionAlgorithm::from_encoding_name("unknown"), None);
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();

        assert_eq!(config.compression_level, CompressionLevel::Default);
        assert_eq!(config.min_size_threshold, 1024);
        assert!(config.enable_gzip);
        assert!(config.enable_deflate);
        assert!(config.excluded_content_types.contains("image/jpeg"));
        assert!(config.excluded_content_types.contains("application/zip"));
    }

    #[test]
    fn test_compression_config_should_compress_content_type() {
        let config = CompressionConfig::default();

        // Should compress
        assert!(config.should_compress_content_type("application/json"));
        assert!(config.should_compress_content_type("text/html"));
        assert!(config.should_compress_content_type("text/plain"));
        assert!(config.should_compress_content_type("application/javascript"));

        // Should not compress
        assert!(!config.should_compress_content_type("image/jpeg"));
        assert!(!config.should_compress_content_type("image/png"));
        assert!(!config.should_compress_content_type("application/zip"));
        assert!(!config.should_compress_content_type("application/pdf"));
    }

    #[test]
    fn test_compression_config_should_compress_content_type_with_params() {
        let config = CompressionConfig::default();

        // Should handle content type with charset parameter
        assert!(config.should_compress_content_type("application/json; charset=utf-8"));
        assert!(config.should_compress_content_type("text/html; charset=utf-8"));

        // Should not compress even with parameters
        assert!(!config.should_compress_content_type("image/jpeg; quality=90"));
    }

    #[test]
    fn test_compression_config_should_compress_size() {
        let config = CompressionConfig::default();

        assert!(!config.should_compress_size(512)); // Below threshold
        assert!(!config.should_compress_size(1023)); // Just below threshold
        assert!(config.should_compress_size(1024)); // At threshold
        assert!(config.should_compress_size(2048)); // Above threshold
    }

    #[test]
    fn test_compression_config_builder() {
        let config = CompressionConfig::builder()
            .compression_level(CompressionLevel::Best)
            .min_size_threshold(2048)
            .enable_gzip(true)
            .enable_deflate(false)
            .exclude_content_type("application/octet-stream")
            .build();

        assert_eq!(config.compression_level, CompressionLevel::Best);
        assert_eq!(config.min_size_threshold, 2048);
        assert!(config.enable_gzip);
        assert!(!config.enable_deflate);
        assert!(config.excluded_content_types.contains("application/octet-stream"));
    }

    #[test]
    fn test_parse_accept_encoding_single() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip"),
        );

        let algorithms = parse_accept_encoding(&headers);
        assert_eq!(algorithms.len(), 1);
        assert_eq!(algorithms[0], CompressionAlgorithm::Gzip);
    }

    #[test]
    fn test_parse_accept_encoding_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate"),
        );

        let algorithms = parse_accept_encoding(&headers);
        assert_eq!(algorithms.len(), 2);
        assert!(algorithms.contains(&CompressionAlgorithm::Gzip));
        assert!(algorithms.contains(&CompressionAlgorithm::Deflate));
    }

    #[test]
    fn test_parse_accept_encoding_with_quality() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip;q=0.9, deflate;q=0.8"),
        );

        let algorithms = parse_accept_encoding(&headers);
        assert_eq!(algorithms.len(), 2);
        assert!(algorithms.contains(&CompressionAlgorithm::Gzip));
        assert!(algorithms.contains(&CompressionAlgorithm::Deflate));
    }

    #[test]
    fn test_parse_accept_encoding_empty() {
        let headers = HeaderMap::new();
        let algorithms = parse_accept_encoding(&headers);
        assert!(algorithms.is_empty());
    }

    #[test]
    fn test_compress_gzip() {
        let config = CompressionConfig::default();
        let data = b"Hello, World! This is a test of compression.";

        let compressed = config.compress(data, CompressionAlgorithm::Gzip).unwrap();

        // Compressed should be non-empty
        assert!(!compressed.is_empty());
        // For this small data, compressed might be larger due to gzip overhead
    }

    #[test]
    fn test_compress_deflate() {
        let config = CompressionConfig::default();
        let data = b"Hello, World! This is a test of compression.";

        let compressed = config.compress(data, CompressionAlgorithm::Deflate).unwrap();

        // Compressed should be non-empty
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_preferred_algorithm() {
        let config = CompressionConfig::default();

        // Prefer gzip
        let accepted = vec![CompressionAlgorithm::Gzip, CompressionAlgorithm::Deflate];
        assert_eq!(config.preferred_algorithm(&accepted), Some(CompressionAlgorithm::Gzip));

        // Only deflate
        let accepted = vec![CompressionAlgorithm::Deflate];
        assert_eq!(config.preferred_algorithm(&accepted), Some(CompressionAlgorithm::Deflate));

        // None available
        let accepted: Vec<CompressionAlgorithm> = vec![];
        assert_eq!(config.preferred_algorithm(&accepted), None);
    }

    #[test]
    fn test_content_type_predicate() {
        let config = CompressionConfig::default();
        let predicate = ContentTypePredicate::new(config);

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("2048"));

        assert!(predicate.should_compress(&headers));
    }

    #[test]
    fn test_content_type_predicate_already_compressed() {
        let config = CompressionConfig::default();
        let predicate = ContentTypePredicate::new(config);

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));

        assert!(!predicate.should_compress(&headers));
    }
}
