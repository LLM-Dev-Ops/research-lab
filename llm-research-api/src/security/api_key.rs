use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Duration, Utc};
use llm_research_core::domain::ids::UserId;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::error::ApiError;

// ============================================================================
// Constants
// ============================================================================

const API_KEY_PREFIX: &str = "llm_sk_";
const API_KEY_LENGTH: usize = 32; // bytes before base64 encoding

// ============================================================================
// Permission Enums
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentPermission {
    Read,
    Write,
    Delete,
    Execute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelPermission {
    Read,
    Write,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetPermission {
    Read,
    Write,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricPermission {
    Read,
    Write,
    Delete,
}

// ============================================================================
// API Scopes
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type", content = "permissions")]
pub enum ApiScope {
    All,
    Experiments(Vec<ExperimentPermission>),
    Models(Vec<ModelPermission>),
    Datasets(Vec<DatasetPermission>),
    Metrics(Vec<MetricPermission>),
}

impl ApiScope {
    /// Check if this scope grants a specific permission
    pub fn has_permission(&self, permission_type: &str, permission: &str) -> bool {
        match self {
            ApiScope::All => true,
            ApiScope::Experiments(perms) => {
                permission_type == "experiments"
                    && perms
                        .iter()
                        .any(|p| format!("{:?}", p).to_lowercase() == permission.to_lowercase())
            }
            ApiScope::Models(perms) => {
                permission_type == "models"
                    && perms
                        .iter()
                        .any(|p| format!("{:?}", p).to_lowercase() == permission.to_lowercase())
            }
            ApiScope::Datasets(perms) => {
                permission_type == "datasets"
                    && perms
                        .iter()
                        .any(|p| format!("{:?}", p).to_lowercase() == permission.to_lowercase())
            }
            ApiScope::Metrics(perms) => {
                permission_type == "metrics"
                    && perms
                        .iter()
                        .any(|p| format!("{:?}", p).to_lowercase() == permission.to_lowercase())
            }
        }
    }
}

// ============================================================================
// Rate Limit Tier
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitTier {
    Free,       // 100 requests/hour
    Basic,      // 1,000 requests/hour
    Pro,        // 10,000 requests/hour
    Enterprise, // 100,000 requests/hour
    Unlimited,  // No limit
}

impl RateLimitTier {
    /// Get the maximum requests per hour for this tier
    pub fn max_requests_per_hour(&self) -> Option<u32> {
        match self {
            RateLimitTier::Free => Some(100),
            RateLimitTier::Basic => Some(1_000),
            RateLimitTier::Pro => Some(10_000),
            RateLimitTier::Enterprise => Some(100_000),
            RateLimitTier::Unlimited => None,
        }
    }
}

// ============================================================================
// API Key
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub owner_id: UserId,
    pub roles: Vec<String>,
    pub scopes: Vec<ApiScope>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub rate_limit_tier: RateLimitTier,
}

impl ApiKey {
    /// Check if the API key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Check if the API key is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired()
    }

    /// Check if the API key has a specific scope permission
    pub fn has_scope_permission(&self, permission_type: &str, permission: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.has_permission(permission_type, permission))
    }

    /// Update last used timestamp
    pub fn update_last_used(&mut self) {
        self.last_used_at = Some(Utc::now());
    }
}

// ============================================================================
// API Key User (for request context)
// ============================================================================

#[derive(Debug, Clone)]
pub struct ApiKeyUser {
    pub key_id: Uuid,
    pub owner_id: UserId,
    pub roles: Vec<String>,
    pub scopes: Vec<ApiScope>,
    pub rate_limit_tier: RateLimitTier,
}

impl ApiKeyUser {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the required roles
    pub fn has_any_role(&self, required_roles: &[&str]) -> bool {
        self.roles
            .iter()
            .any(|r| required_roles.contains(&r.as_str()))
    }

    /// Check if user has a specific scope permission
    pub fn has_scope_permission(&self, permission_type: &str, permission: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.has_permission(permission_type, permission))
    }
}

// ============================================================================
// API Key Service
// ============================================================================

pub struct ApiKeyService {
    // In-memory storage for demo purposes
    // In production, this would be backed by a database
    keys: Arc<RwLock<HashMap<Uuid, ApiKey>>>,
    key_hashes: Arc<RwLock<HashMap<String, Uuid>>>, // hash -> key_id mapping
}

impl ApiKeyService {
    /// Create a new API Key Service
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            key_hashes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new API key
    ///
    /// Returns a tuple of (ApiKey metadata, actual key string)
    /// The actual key string is only returned once and should be stored securely by the user
    pub fn generate_key(
        &self,
        name: &str,
        owner_id: UserId,
        roles: Vec<String>,
        scopes: Vec<ApiScope>,
        rate_limit_tier: RateLimitTier,
        expires_in: Option<Duration>,
    ) -> Result<(ApiKey, String), ApiError> {
        // Generate secure random bytes
        let mut rng = rand::thread_rng();
        let mut key_bytes = vec![0u8; API_KEY_LENGTH];
        rng.fill(&mut key_bytes[..]);

        // Encode to base64
        let key_secret = base64::encode(&key_bytes);

        // Create the full API key with prefix
        let full_key = format!("{}{}", API_KEY_PREFIX, key_secret);

        // Hash the key for storage
        let key_hash = Self::hash_key(&full_key);

        // Extract prefix for identification (first 8 chars after the llm_sk_ prefix)
        let key_prefix = if key_secret.len() >= 8 {
            format!("{}{}", API_KEY_PREFIX, &key_secret[..8])
        } else {
            full_key.clone()
        };

        // Calculate expiration
        let expires_at = expires_in.map(|duration| Utc::now() + duration);

        // Create API key metadata
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            name: name.to_string(),
            key_hash: key_hash.clone(),
            key_prefix,
            owner_id,
            roles,
            scopes,
            created_at: Utc::now(),
            expires_at,
            last_used_at: None,
            is_active: true,
            rate_limit_tier,
        };

        // Store the key
        {
            let mut keys = self.keys.write().map_err(|_| {
                ApiError::Internal("Failed to acquire write lock on keys".to_string())
            })?;
            keys.insert(api_key.id, api_key.clone());
        }

        // Store the hash mapping
        {
            let mut key_hashes = self.key_hashes.write().map_err(|_| {
                ApiError::Internal("Failed to acquire write lock on key_hashes".to_string())
            })?;
            key_hashes.insert(key_hash, api_key.id);
        }

        Ok((api_key, full_key))
    }

    /// Validate an API key and return the associated metadata
    pub fn validate_key(&self, key: &str) -> Result<ApiKey, ApiError> {
        // Hash the provided key
        let key_hash = Self::hash_key(key);

        // Look up the key ID from the hash
        let key_id = {
            let key_hashes = self.key_hashes.read().map_err(|_| {
                ApiError::Internal("Failed to acquire read lock on key_hashes".to_string())
            })?;

            key_hashes
                .get(&key_hash)
                .copied()
                .ok_or(ApiError::Unauthorized)?
        };

        // Get the key metadata
        let mut api_key = {
            let keys = self.keys.read().map_err(|_| {
                ApiError::Internal("Failed to acquire read lock on keys".to_string())
            })?;

            keys.get(&key_id)
                .cloned()
                .ok_or(ApiError::Unauthorized)?
        };

        // Validate the key
        if !api_key.is_valid() {
            return Err(ApiError::Unauthorized);
        }

        // Update last used timestamp
        api_key.update_last_used();

        // Store the updated key
        {
            let mut keys = self.keys.write().map_err(|_| {
                ApiError::Internal("Failed to acquire write lock on keys".to_string())
            })?;
            keys.insert(api_key.id, api_key.clone());
        }

        Ok(api_key)
    }

    /// Hash an API key using SHA-256
    pub fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Revoke an API key
    pub fn revoke_key(&self, key_id: Uuid) -> Result<(), ApiError> {
        let mut keys = self.keys.write().map_err(|_| {
            ApiError::Internal("Failed to acquire write lock on keys".to_string())
        })?;

        if let Some(api_key) = keys.get_mut(&key_id) {
            api_key.is_active = false;
            Ok(())
        } else {
            Err(ApiError::NotFound(format!("API key {} not found", key_id)))
        }
    }

    /// List all API keys for a specific owner
    pub fn list_keys(&self, owner_id: UserId) -> Result<Vec<ApiKey>, ApiError> {
        let keys = self.keys.read().map_err(|_| {
            ApiError::Internal("Failed to acquire read lock on keys".to_string())
        })?;

        let user_keys: Vec<ApiKey> = keys
            .values()
            .filter(|key| key.owner_id == owner_id)
            .cloned()
            .collect();

        Ok(user_keys)
    }

    /// Rotate an API key (revoke old key and generate new one)
    pub fn rotate_key(&self, key_id: Uuid) -> Result<(ApiKey, String), ApiError> {
        // Get the old key
        let old_key = {
            let keys = self.keys.read().map_err(|_| {
                ApiError::Internal("Failed to acquire read lock on keys".to_string())
            })?;

            keys.get(&key_id)
                .cloned()
                .ok_or(ApiError::NotFound(format!("API key {} not found", key_id)))?
        };

        // Revoke the old key
        self.revoke_key(key_id)?;

        // Generate a new key with the same settings
        let new_name = format!("{} (rotated)", old_key.name);
        let expires_in = old_key
            .expires_at
            .map(|exp| exp.signed_duration_since(Utc::now()));

        self.generate_key(
            &new_name,
            old_key.owner_id,
            old_key.roles,
            old_key.scopes,
            old_key.rate_limit_tier,
            expires_in,
        )
    }

    /// Get a specific API key by ID
    pub fn get_key(&self, key_id: Uuid) -> Result<ApiKey, ApiError> {
        let keys = self.keys.read().map_err(|_| {
            ApiError::Internal("Failed to acquire read lock on keys".to_string())
        })?;

        keys.get(&key_id)
            .cloned()
            .ok_or(ApiError::NotFound(format!("API key {} not found", key_id)))
    }
}

impl Default for ApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ApiKeyService {
    fn clone(&self) -> Self {
        Self {
            keys: Arc::clone(&self.keys),
            key_hashes: Arc::clone(&self.key_hashes),
        }
    }
}

// ============================================================================
// API Key Authentication Middleware
// ============================================================================

/// Middleware to authenticate requests using API keys
///
/// Supports two formats:
/// - Header: `X-API-Key: llm_sk_...`
/// - Header: `Authorization: ApiKey llm_sk_...`
pub async fn api_key_auth_middleware(
    State(service): State<ApiKeyService>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Try to extract API key from headers
    let api_key = extract_api_key_from_request(&request)?;

    // Validate the API key
    let api_key_data = service.validate_key(&api_key)?;

    // Create API key user for request context
    let api_key_user = ApiKeyUser {
        key_id: api_key_data.id,
        owner_id: api_key_data.owner_id,
        roles: api_key_data.roles,
        scopes: api_key_data.scopes,
        rate_limit_tier: api_key_data.rate_limit_tier,
    };

    // Insert user info into request extensions
    request.extensions_mut().insert(api_key_user);

    Ok(next.run(request).await)
}

/// Optional API key authentication middleware
///
/// Unlike the strict middleware, this doesn't fail if no API key is provided
pub async fn optional_api_key_auth_middleware(
    State(service): State<ApiKeyService>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate API key
    if let Ok(api_key) = extract_api_key_from_request(&request) {
        if let Ok(api_key_data) = service.validate_key(&api_key) {
            let api_key_user = ApiKeyUser {
                key_id: api_key_data.id,
                owner_id: api_key_data.owner_id,
                roles: api_key_data.roles,
                scopes: api_key_data.scopes,
                rate_limit_tier: api_key_data.rate_limit_tier,
            };
            request.extensions_mut().insert(api_key_user);
        }
    }

    next.run(request).await
}

/// Extract API key from request headers
fn extract_api_key_from_request(request: &Request) -> Result<String, ApiError> {
    // Try X-API-Key header first
    if let Some(api_key) = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
    {
        return Ok(api_key.to_string());
    }

    // Try Authorization header with "ApiKey" scheme
    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if auth_header.starts_with("ApiKey ") {
            return Ok(auth_header[7..].to_string());
        }
    }

    Err(ApiError::Unauthorized)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract API key user from request extensions
pub fn get_api_key_user(request: &Request) -> Result<&ApiKeyUser, ApiError> {
    request
        .extensions()
        .get::<ApiKeyUser>()
        .ok_or(ApiError::Unauthorized)
}

/// Check if the API key user has a required role
pub fn require_role(user: &ApiKeyUser, role: &str) -> Result<(), ApiError> {
    if user.has_role(role) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Check if the API key user has any of the required roles
pub fn require_any_role(user: &ApiKeyUser, roles: &[&str]) -> Result<(), ApiError> {
    if user.has_any_role(roles) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Check if the API key user has a specific scope permission
pub fn require_scope_permission(
    user: &ApiKeyUser,
    permission_type: &str,
    permission: &str,
) -> Result<(), ApiError> {
    if user.has_scope_permission(permission_type, permission) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

// ============================================================================
// Base64 encoding/decoding utilities
// ============================================================================

mod base64 {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
        STANDARD.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_key() {
        let service = ApiKeyService::new();
        let owner_id = UserId::new();
        let roles = vec!["admin".to_string()];
        let scopes = vec![ApiScope::All];

        // Generate a key
        let (api_key, full_key) = service
            .generate_key(
                "Test Key",
                owner_id,
                roles.clone(),
                scopes.clone(),
                RateLimitTier::Pro,
                None,
            )
            .unwrap();

        // Validate the generated key
        let validated_key = service.validate_key(&full_key).unwrap();

        assert_eq!(api_key.id, validated_key.id);
        assert_eq!(api_key.name, validated_key.name);
        assert_eq!(api_key.owner_id, validated_key.owner_id);
        assert!(validated_key.is_valid());
        assert!(full_key.starts_with(API_KEY_PREFIX));
    }

    #[test]
    fn test_key_expiration() {
        let service = ApiKeyService::new();
        let owner_id = UserId::new();

        // Generate an expired key
        let (api_key, full_key) = service
            .generate_key(
                "Expired Key",
                owner_id,
                vec![],
                vec![ApiScope::All],
                RateLimitTier::Free,
                Some(Duration::seconds(-1)), // Already expired
            )
            .unwrap();

        assert!(api_key.is_expired());
        assert!(!api_key.is_valid());

        // Validation should fail for expired key
        assert!(service.validate_key(&full_key).is_err());
    }

    #[test]
    fn test_revoke_key() {
        let service = ApiKeyService::new();
        let owner_id = UserId::new();

        let (api_key, full_key) = service
            .generate_key(
                "Test Key",
                owner_id,
                vec![],
                vec![ApiScope::All],
                RateLimitTier::Basic,
                None,
            )
            .unwrap();

        // Key should be valid initially
        assert!(service.validate_key(&full_key).is_ok());

        // Revoke the key
        service.revoke_key(api_key.id).unwrap();

        // Key should no longer be valid
        assert!(service.validate_key(&full_key).is_err());
    }

    #[test]
    fn test_list_keys() {
        let service = ApiKeyService::new();
        let owner_id = UserId::new();

        // Generate multiple keys
        service
            .generate_key(
                "Key 1",
                owner_id,
                vec![],
                vec![ApiScope::All],
                RateLimitTier::Free,
                None,
            )
            .unwrap();

        service
            .generate_key(
                "Key 2",
                owner_id,
                vec![],
                vec![ApiScope::All],
                RateLimitTier::Pro,
                None,
            )
            .unwrap();

        // List keys for owner
        let keys = service.list_keys(owner_id).unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_rotate_key() {
        let service = ApiKeyService::new();
        let owner_id = UserId::new();

        let (old_key, old_full_key) = service
            .generate_key(
                "Original Key",
                owner_id,
                vec!["admin".to_string()],
                vec![ApiScope::All],
                RateLimitTier::Enterprise,
                None,
            )
            .unwrap();

        // Rotate the key
        let (new_key, new_full_key) = service.rotate_key(old_key.id).unwrap();

        // Old key should be invalid
        assert!(service.validate_key(&old_full_key).is_err());

        // New key should be valid
        assert!(service.validate_key(&new_full_key).is_ok());

        // New key should have same settings
        assert_eq!(new_key.owner_id, old_key.owner_id);
        assert_eq!(new_key.roles, old_key.roles);
        assert_eq!(new_key.rate_limit_tier, old_key.rate_limit_tier);
    }

    #[test]
    fn test_scope_permissions() {
        let scope = ApiScope::Experiments(vec![
            ExperimentPermission::Read,
            ExperimentPermission::Write,
        ]);

        assert!(scope.has_permission("experiments", "read"));
        assert!(scope.has_permission("experiments", "write"));
        assert!(!scope.has_permission("experiments", "delete"));
        assert!(!scope.has_permission("models", "read"));
    }

    #[test]
    fn test_rate_limit_tiers() {
        assert_eq!(RateLimitTier::Free.max_requests_per_hour(), Some(100));
        assert_eq!(RateLimitTier::Basic.max_requests_per_hour(), Some(1_000));
        assert_eq!(RateLimitTier::Pro.max_requests_per_hour(), Some(10_000));
        assert_eq!(
            RateLimitTier::Enterprise.max_requests_per_hour(),
            Some(100_000)
        );
        assert_eq!(RateLimitTier::Unlimited.max_requests_per_hour(), None);
    }

    #[test]
    fn test_api_key_user_role_checks() {
        let user = ApiKeyUser {
            key_id: Uuid::new_v4(),
            owner_id: UserId::new(),
            roles: vec!["admin".to_string(), "user".to_string()],
            scopes: vec![ApiScope::All],
            rate_limit_tier: RateLimitTier::Pro,
        };

        assert!(user.has_role("admin"));
        assert!(user.has_role("user"));
        assert!(!user.has_role("superuser"));

        assert!(user.has_any_role(&["admin", "superuser"]));
        assert!(!user.has_any_role(&["superuser", "guest"]));
    }

    #[test]
    fn test_key_hash_consistency() {
        let key = "llm_sk_test_key_12345";
        let hash1 = ApiKeyService::hash_key(key);
        let hash2 = ApiKeyService::hash_key(key);

        assert_eq!(hash1, hash2);

        let different_key = "llm_sk_different_key";
        let hash3 = ApiKeyService::hash_key(different_key);

        assert_ne!(hash1, hash3);
    }
}
