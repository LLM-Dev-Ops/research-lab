use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("JWT encoding error: {0}")]
    JwtEncode(#[from] JwtError),

    #[error("JWT decoding error: {0}")]
    JwtDecode(String),

    #[error("Invalid token type: expected {expected}, got {actual}")]
    InvalidTokenType { expected: String, actual: String },

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token claims")]
    InvalidClaims,

    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    #[error("Invalid refresh token")]
    InvalidRefreshToken,
}

pub type AuthResult<T> = Result<T, AuthError>;

// ============================================================================
// Token Types
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenType {
    pub fn as_str(&self) -> &str {
        match self {
            TokenType::Access => "access",
            TokenType::Refresh => "refresh",
        }
    }
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens (loaded from JWT_SECRET env var)
    pub secret_key: String,
    /// Access token expiry in seconds (default: 15 minutes = 900 seconds)
    pub access_token_expiry: i64,
    /// Refresh token expiry in seconds (default: 7 days = 604800 seconds)
    pub refresh_token_expiry: i64,
    /// Token issuer
    pub issuer: String,
    /// Token audience
    pub audience: String,
}

impl JwtConfig {
    /// Create a new JWT configuration with defaults
    pub fn new() -> AuthResult<Self> {
        let secret_key = env::var("JWT_SECRET").unwrap_or_else(|_| {
            // Development fallback - in production, this should always be set
            tracing::warn!(
                "JWT_SECRET not set in environment, using default (DEVELOPMENT ONLY)"
            );
            "development-secret-key-change-in-production".to_string()
        });

        Ok(Self {
            secret_key,
            access_token_expiry: 900,  // 15 minutes
            refresh_token_expiry: 604800, // 7 days
            issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "llm-research-api".to_string()),
            audience: env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| "llm-research-users".to_string()),
        })
    }

    /// Create a custom configuration
    pub fn with_settings(
        secret_key: String,
        access_token_expiry: i64,
        refresh_token_expiry: i64,
        issuer: String,
        audience: String,
    ) -> Self {
        Self {
            secret_key,
            access_token_expiry,
            refresh_token_expiry,
            issuer,
            audience,
        }
    }

    /// Load from environment with custom defaults
    pub fn from_env_with_defaults(
        default_access_expiry: i64,
        default_refresh_expiry: i64,
    ) -> AuthResult<Self> {
        let secret_key = env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET not set in environment, using default (DEVELOPMENT ONLY)"
            );
            "development-secret-key-change-in-production".to_string()
        });

        let access_token_expiry = env::var("JWT_ACCESS_EXPIRY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default_access_expiry);

        let refresh_token_expiry = env::var("JWT_REFRESH_EXPIRY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default_refresh_expiry);

        Ok(Self {
            secret_key,
            access_token_expiry,
            refresh_token_expiry,
            issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "llm-research-api".to_string()),
            audience: env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| "llm-research-users".to_string()),
        })
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            secret_key: "development-secret-key-change-in-production".to_string(),
            access_token_expiry: 900,
            refresh_token_expiry: 604800,
            issuer: "llm-research-api".to_string(),
            audience: "llm-research-users".to_string(),
        })
    }
}

// ============================================================================
// Token Pair
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// The access token
    pub access_token: String,
    /// The refresh token
    pub refresh_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Access token expiration time in seconds
    pub expires_in: i64,
}

impl TokenPair {
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }
}

// ============================================================================
// Claims
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID as string)
    pub sub: String,
    /// Expiration time (as Unix timestamp)
    pub exp: i64,
    /// Issued at (as Unix timestamp)
    pub iat: i64,
    /// Not before (as Unix timestamp)
    pub nbf: i64,
    /// JWT ID (for blacklisting)
    pub jti: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// User UUID
    pub user_id: Uuid,
    /// User email
    pub email: String,
    /// User roles
    pub roles: Vec<String>,
    /// Token type (access or refresh)
    pub token_type: TokenType,
}

impl Claims {
    /// Create new access token claims
    pub fn new_access(
        user_id: Uuid,
        email: String,
        roles: Vec<String>,
        config: &JwtConfig,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::seconds(config.access_token_expiry);
        let jti = Uuid::new_v4();

        Self {
            sub: user_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: jti.to_string(),
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            user_id,
            email,
            roles,
            token_type: TokenType::Access,
        }
    }

    /// Create new refresh token claims
    pub fn new_refresh(
        user_id: Uuid,
        email: String,
        roles: Vec<String>,
        config: &JwtConfig,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::seconds(config.refresh_token_expiry);
        let jti = Uuid::new_v4();

        Self {
            sub: user_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: jti.to_string(),
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            user_id,
            email,
            roles,
            token_type: TokenType::Refresh,
        }
    }

    /// Validate the claims
    pub fn validate(&self, expected_token_type: TokenType) -> AuthResult<()> {
        // Check token type
        if self.token_type != expected_token_type {
            return Err(AuthError::InvalidTokenType {
                expected: expected_token_type.as_str().to_string(),
                actual: self.token_type.as_str().to_string(),
            });
        }

        // Check expiration
        let now = Utc::now().timestamp();
        if self.exp < now {
            return Err(AuthError::TokenExpired);
        }

        // Check not before
        if self.nbf > now {
            return Err(AuthError::InvalidClaims);
        }

        Ok(())
    }
}

// ============================================================================
// Refresh Claims (minimal for refresh tokens)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    /// Subject (user ID as string)
    pub sub: String,
    /// Expiration time (as Unix timestamp)
    pub exp: i64,
    /// Issued at (as Unix timestamp)
    pub iat: i64,
    /// JWT ID
    pub jti: String,
    /// User UUID
    pub user_id: Uuid,
    /// User email
    pub email: String,
    /// User roles
    pub roles: Vec<String>,
    /// Token type
    pub token_type: TokenType,
}

impl From<Claims> for RefreshClaims {
    fn from(claims: Claims) -> Self {
        Self {
            sub: claims.sub,
            exp: claims.exp,
            iat: claims.iat,
            jti: claims.jti,
            user_id: claims.user_id,
            email: claims.email,
            roles: claims.roles,
            token_type: claims.token_type,
        }
    }
}

// ============================================================================
// JWT Service
// ============================================================================

#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Create a new JWT service with the given configuration
    pub fn new(config: JwtConfig) -> Self {
        let secret = config.secret_key.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            config,
        }
    }

    /// Create a new JWT service with default configuration
    pub fn default() -> AuthResult<Self> {
        let config = JwtConfig::new()?;
        Ok(Self::new(config))
    }

    /// Generate a token pair (access + refresh tokens)
    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        email: &str,
        roles: Vec<String>,
    ) -> AuthResult<TokenPair> {
        // Create access token claims
        let access_claims = Claims::new_access(
            user_id,
            email.to_string(),
            roles.clone(),
            &self.config,
        );

        // Create refresh token claims
        let refresh_claims = Claims::new_refresh(
            user_id,
            email.to_string(),
            roles,
            &self.config,
        );

        // Encode tokens
        let access_token = encode(&Header::new(Algorithm::HS256), &access_claims, &self.encoding_key)?;
        let refresh_token = encode(&Header::new(Algorithm::HS256), &refresh_claims, &self.encoding_key)?;

        Ok(TokenPair::new(
            access_token,
            refresh_token,
            self.config.access_token_expiry,
        ))
    }

    /// Validate an access token and return the claims
    pub fn validate_access_token(&self, token: &str) -> AuthResult<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| AuthError::JwtDecode(e.to_string()))?;

        // Validate token type
        token_data.claims.validate(TokenType::Access)?;

        Ok(token_data.claims)
    }

    /// Validate a refresh token and return the claims
    pub fn validate_refresh_token(&self, token: &str) -> AuthResult<RefreshClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| AuthError::JwtDecode(e.to_string()))?;

        // Validate token type
        token_data.claims.validate(TokenType::Refresh)?;

        Ok(token_data.claims.into())
    }

    /// Refresh tokens using a valid refresh token
    pub fn refresh_tokens(&self, refresh_token: &str) -> AuthResult<TokenPair> {
        // Validate the refresh token
        let refresh_claims = self.validate_refresh_token(refresh_token)?;

        // Generate new token pair with the same user info
        self.generate_token_pair(refresh_claims.user_id, &refresh_claims.email, refresh_claims.roles)
    }

    /// Extract JWT ID (jti) from a token for blacklisting purposes
    pub fn extract_jti(&self, token: &str) -> AuthResult<String> {
        // Decode without validation to extract jti
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| AuthError::JwtDecode(e.to_string()))?;

        Ok(token_data.claims.jti)
    }

    /// Get the configuration
    pub fn config(&self) -> &JwtConfig {
        &self.config
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> JwtConfig {
        JwtConfig::with_settings(
            "test-secret-key-for-testing-only".to_string(),
            900,     // 15 minutes
            604800,  // 7 days
            "test-issuer".to_string(),
            "test-audience".to_string(),
        )
    }

    #[test]
    fn test_jwt_config_creation() {
        let config = create_test_config();
        assert_eq!(config.access_token_expiry, 900);
        assert_eq!(config.refresh_token_expiry, 604800);
        assert_eq!(config.issuer, "test-issuer");
        assert_eq!(config.audience, "test-audience");
    }

    #[test]
    fn test_generate_token_pair() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let roles = vec!["user".to_string(), "admin".to_string()];

        let result = service.generate_token_pair(user_id, email, roles);
        assert!(result.is_ok());

        let token_pair = result.unwrap();
        assert!(!token_pair.access_token.is_empty());
        assert!(!token_pair.refresh_token.is_empty());
        assert_eq!(token_pair.token_type, "Bearer");
        assert_eq!(token_pair.expires_in, 900);
    }

    #[test]
    fn test_validate_access_token() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let roles = vec!["user".to_string()];

        let token_pair = service
            .generate_token_pair(user_id, email, roles.clone())
            .unwrap();

        let claims = service.validate_access_token(&token_pair.access_token);
        assert!(claims.is_ok());

        let claims = claims.unwrap();
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.roles, roles);
        assert_eq!(claims.token_type, TokenType::Access);
    }

    #[test]
    fn test_validate_refresh_token() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let roles = vec!["user".to_string()];

        let token_pair = service
            .generate_token_pair(user_id, email, roles.clone())
            .unwrap();

        let refresh_claims = service.validate_refresh_token(&token_pair.refresh_token);
        assert!(refresh_claims.is_ok());

        let refresh_claims = refresh_claims.unwrap();
        assert_eq!(refresh_claims.user_id, user_id);
        assert_eq!(refresh_claims.email, email);
        assert_eq!(refresh_claims.roles, roles);
        assert_eq!(refresh_claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_wrong_token_type_validation() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let roles = vec!["user".to_string()];

        let token_pair = service
            .generate_token_pair(user_id, email, roles)
            .unwrap();

        // Try to validate access token as refresh token
        let result = service.validate_refresh_token(&token_pair.access_token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidTokenType { .. }));

        // Try to validate refresh token as access token
        let result = service.validate_access_token(&token_pair.refresh_token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidTokenType { .. }));
    }

    #[test]
    fn test_refresh_tokens() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let roles = vec!["user".to_string()];

        let token_pair = service
            .generate_token_pair(user_id, email, roles.clone())
            .unwrap();

        // Refresh the tokens
        let new_token_pair = service.refresh_tokens(&token_pair.refresh_token);
        assert!(new_token_pair.is_ok());

        let new_token_pair = new_token_pair.unwrap();
        assert!(!new_token_pair.access_token.is_empty());
        assert!(!new_token_pair.refresh_token.is_empty());
        assert_ne!(token_pair.access_token, new_token_pair.access_token);
        assert_ne!(token_pair.refresh_token, new_token_pair.refresh_token);

        // Validate new access token
        let claims = service.validate_access_token(&new_token_pair.access_token);
        assert!(claims.is_ok());

        let claims = claims.unwrap();
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.roles, roles);
    }

    #[test]
    fn test_extract_jti() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let roles = vec!["user".to_string()];

        let token_pair = service
            .generate_token_pair(user_id, email, roles)
            .unwrap();

        let jti = service.extract_jti(&token_pair.access_token);
        assert!(jti.is_ok());
        assert!(!jti.unwrap().is_empty());
    }

    #[test]
    fn test_invalid_token() {
        let config = create_test_config();
        let service = JwtService::new(config);

        let result = service.validate_access_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_type_serialization() {
        let access = TokenType::Access;
        let refresh = TokenType::Refresh;

        assert_eq!(access.as_str(), "access");
        assert_eq!(refresh.as_str(), "refresh");

        let access_json = serde_json::to_string(&access).unwrap();
        let refresh_json = serde_json::to_string(&refresh).unwrap();

        assert_eq!(access_json, "\"access\"");
        assert_eq!(refresh_json, "\"refresh\"");
    }

    #[test]
    fn test_claims_validation() {
        let config = create_test_config();
        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();
        let roles = vec!["user".to_string()];

        let access_claims = Claims::new_access(user_id, email.clone(), roles.clone(), &config);
        assert!(access_claims.validate(TokenType::Access).is_ok());
        assert!(access_claims.validate(TokenType::Refresh).is_err());

        let refresh_claims = Claims::new_refresh(user_id, email, roles, &config);
        assert!(refresh_claims.validate(TokenType::Refresh).is_ok());
        assert!(refresh_claims.validate(TokenType::Access).is_err());
    }
}
