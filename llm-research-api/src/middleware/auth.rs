use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use llm_research_core::domain::ids::UserId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ApiError, AppState};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,       // Subject (user ID)
    pub exp: usize,        // Expiration time
    pub iat: usize,        // Issued at
    pub user_id: Uuid,     // User UUID
    pub email: String,     // User email
    pub roles: Vec<String>, // User roles
}

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: UserId,
    pub email: String,
    pub roles: Vec<String>,
}

/// JWT authentication middleware
pub async fn auth_middleware(
    State(_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    // Check for Bearer token
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized);
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // Decode and validate JWT
    let claims = validate_token(token)?;

    // Insert user info into request extensions
    let auth_user = AuthUser {
        user_id: UserId::from(claims.user_id),
        email: claims.email,
        roles: claims.roles,
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

/// Validate JWT token and extract claims
fn validate_token(token: &str) -> Result<Claims, ApiError> {
    // TODO: Load secret from environment or config
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());

    // Decode header to check algorithm
    let _header = decode_header(token)?;

    // Validation rules
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    // Decode and validate token
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Optional auth middleware - doesn't fail if no token is provided
pub async fn optional_auth_middleware(
    State(_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract Authorization header
    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];

            // Try to validate token
            if let Ok(claims) = validate_token(token) {
                let auth_user = AuthUser {
                    user_id: UserId::from(claims.user_id),
                    email: claims.email,
                    roles: claims.roles,
                };
                request.extensions_mut().insert(auth_user);
            }
        }
    }

    next.run(request).await
}

/// Helper to check if user has required role
pub fn has_role(user: &AuthUser, required_role: &str) -> bool {
    user.roles.iter().any(|r| r == required_role)
}

/// Helper to check if user has any of the required roles
pub fn has_any_role(user: &AuthUser, required_roles: &[&str]) -> bool {
    user.roles.iter().any(|r| required_roles.contains(&r.as_str()))
}
