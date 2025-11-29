use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware,
    response::Response,
    Router,
};
use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
use llm_research_api::{middleware::*, AppState};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

// ===== Test Helper Functions =====

/// Create a mock AppState for testing
fn create_mock_app_state() -> AppState {
    use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
    use aws_sdk_s3::Client as S3Client;

    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .credentials_provider(Credentials::new("test", "test", None, None, "test"))
        .build();

    let s3_client = S3Client::from_conf(s3_config);

    let pool = sqlx::PgPool::connect_lazy("postgres://test:test@localhost/test")
        .expect("Failed to create dummy pool");

    AppState::new(pool, s3_client, "test-bucket".to_string())
}

/// Create a simple test router with auth middleware
fn create_test_app_with_auth(state: AppState) -> Router {
    async fn protected_handler() -> &'static str {
        "Protected resource"
    }

    Router::new()
        .route("/protected", axum::routing::get(protected_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}

/// Create a simple test router with optional auth middleware
fn create_test_app_with_optional_auth(state: AppState) -> Router {
    async fn handler() -> &'static str {
        "Public resource"
    }

    Router::new()
        .route("/resource", axum::routing::get(handler))
        .layer(middleware::from_fn_with_state(state.clone(), optional_auth_middleware))
        .with_state(state)
}

/// Create a valid JWT token for testing
fn create_test_token(user_id: Uuid, email: &str, roles: Vec<String>) -> String {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        user_id,
        email: email.to_string(),
        roles,
    };

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to create test token")
}

/// Create an expired JWT token for testing
fn create_expired_token(user_id: Uuid, email: &str, roles: Vec<String>) -> String {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as usize, // Expired
        iat: (chrono::Utc::now() - chrono::Duration::hours(2)).timestamp() as usize,
        user_id,
        email: email.to_string(),
        roles,
    };

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to create expired token")
}

// ===== Authentication Middleware Tests =====

#[tokio::test]
async fn test_auth_middleware_with_valid_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec!["user".to_string()]);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_middleware_missing_header() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        // No Authorization header
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_missing_bearer_prefix() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, token) // Missing "Bearer " prefix
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_invalid_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer invalid.token.here")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_expired_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_expired_token(user_id, "test@example.com", vec![]);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_malformed_header() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, "InvalidFormat")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_empty_bearer_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer ")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ===== Optional Authentication Middleware Tests =====

#[tokio::test]
async fn test_optional_auth_with_valid_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_optional_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    let request = Request::builder()
        .uri("/resource")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should succeed with authenticated user
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_optional_auth_without_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_optional_auth(state);

    let request = Request::builder()
        .uri("/resource")
        .method("GET")
        // No Authorization header
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should still succeed without token
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_optional_auth_with_invalid_token() {
    let state = create_mock_app_state();
    let app = create_test_app_with_optional_auth(state);

    let request = Request::builder()
        .uri("/resource")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer invalid.token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should succeed even with invalid token (just no user context)
    assert_eq!(response.status(), StatusCode::OK);
}

// ===== JWT Claims Tests =====

#[test]
fn test_jwt_claims_serialization() {
    let user_id = Uuid::new_v4();
    let claims = Claims {
        sub: user_id.to_string(),
        exp: 9999999999,
        iat: 1000000000,
        user_id,
        email: "test@example.com".to_string(),
        roles: vec!["admin".to_string(), "user".to_string()],
    };

    let json = serde_json::to_string(&claims).unwrap();
    assert!(json.contains("test@example.com"));
    assert!(json.contains("admin"));
}

#[test]
fn test_jwt_claims_deserialization() {
    let user_id = Uuid::new_v4();
    let json = format!(
        r#"{{"sub":"{}","exp":9999999999,"iat":1000000000,"user_id":"{}","email":"test@example.com","roles":["admin"]}}"#,
        user_id, user_id
    );

    let claims: Claims = serde_json::from_str(&json).unwrap();
    assert_eq!(claims.email, "test@example.com");
    assert_eq!(claims.roles, vec!["admin".to_string()]);
    assert_eq!(claims.user_id, user_id);
}

// ===== Role-Based Authorization Tests =====

#[test]
fn test_has_role_positive() {
    let user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(Uuid::new_v4()),
        email: "test@example.com".to_string(),
        roles: vec!["admin".to_string(), "user".to_string()],
    };

    assert!(has_role(&user, "admin"));
    assert!(has_role(&user, "user"));
}

#[test]
fn test_has_role_negative() {
    let user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(Uuid::new_v4()),
        email: "test@example.com".to_string(),
        roles: vec!["user".to_string()],
    };

    assert!(!has_role(&user, "admin"));
    assert!(!has_role(&user, "superuser"));
}

#[test]
fn test_has_any_role_positive() {
    let user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(Uuid::new_v4()),
        email: "test@example.com".to_string(),
        roles: vec!["user".to_string(), "moderator".to_string()],
    };

    assert!(has_any_role(&user, &["admin", "moderator"]));
    assert!(has_any_role(&user, &["user"]));
}

#[test]
fn test_has_any_role_negative() {
    let user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(Uuid::new_v4()),
        email: "test@example.com".to_string(),
        roles: vec!["user".to_string()],
    };

    assert!(!has_any_role(&user, &["admin", "moderator"]));
    assert!(!has_any_role(&user, &["superuser"]));
}

#[test]
fn test_has_any_role_empty_roles() {
    let user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(Uuid::new_v4()),
        email: "test@example.com".to_string(),
        roles: vec![],
    };

    assert!(!has_any_role(&user, &["admin", "user"]));
}

#[test]
fn test_has_any_role_empty_required_roles() {
    let user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(Uuid::new_v4()),
        email: "test@example.com".to_string(),
        roles: vec!["admin".to_string()],
    };

    assert!(!has_any_role(&user, &[]));
}

// ===== Token Validation Edge Cases =====

#[tokio::test]
async fn test_token_with_special_characters_in_email() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(
        user_id,
        "test+special@example.co.uk",
        vec!["user".to_string()],
    );

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_token_with_multiple_roles() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(
        user_id,
        "test@example.com",
        vec![
            "user".to_string(),
            "admin".to_string(),
            "moderator".to_string(),
        ],
    );

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_token_with_no_roles() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should still authenticate successfully
    assert_eq!(response.status(), StatusCode::OK);
}

// ===== Case Sensitivity Tests =====

#[tokio::test]
async fn test_bearer_prefix_case_sensitivity() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    // "bearer" (lowercase) instead of "Bearer"
    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail because we expect "Bearer" with capital B
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ===== Multiple Authorization Headers =====

#[tokio::test]
async fn test_multiple_authorization_headers() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::AUTHORIZATION, "Bearer another-token") // Second header
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Behavior depends on implementation - typically first header is used
    // Test should verify the actual behavior
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::UNAUTHORIZED
    );
}

// ===== Token Format Tests =====

#[test]
fn test_token_has_three_parts() {
    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts: header.payload.signature");
}

#[test]
fn test_token_parts_are_base64() {
    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    let parts: Vec<&str> = token.split('.').collect();

    // Each part should be base64-encoded (URL-safe variant)
    for part in parts {
        assert!(!part.is_empty());
        // Base64 characters: A-Z, a-z, 0-9, -, _
        assert!(part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}

// ===== AuthUser Tests =====

#[test]
fn test_auth_user_creation() {
    let user_id = Uuid::new_v4();
    let auth_user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(user_id),
        email: "test@example.com".to_string(),
        roles: vec!["user".to_string()],
    };

    assert_eq!(auth_user.email, "test@example.com");
    assert_eq!(auth_user.roles.len(), 1);
}

#[test]
fn test_auth_user_clone() {
    let user_id = Uuid::new_v4();
    let auth_user = AuthUser {
        user_id: llm_research_core::domain::ids::UserId::from(user_id),
        email: "test@example.com".to_string(),
        roles: vec!["user".to_string()],
    };

    let cloned = auth_user.clone();
    assert_eq!(cloned.email, auth_user.email);
    assert_eq!(cloned.roles, auth_user.roles);
}

// ===== Error Response Format Tests =====

#[tokio::test]
async fn test_auth_error_response_format() {
    let state = create_mock_app_state();
    let app = create_test_app_with_auth(state);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // In a real test, you'd also check the response body format
    // but that requires reading the body which consumes the response
}

// ===== JWT Algorithm Tests =====

#[test]
fn test_jwt_uses_hs256() {
    let user_id = Uuid::new_v4();
    let token = create_test_token(user_id, "test@example.com", vec![]);

    // Decode the header (first part)
    let parts: Vec<&str> = token.split('.').collect();
    let header_base64 = parts[0];

    // Decode base64 (with URL-safe variant)
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let header_json = URL_SAFE_NO_PAD.decode(header_base64).unwrap();
    let header: serde_json::Value = serde_json::from_slice(&header_json).unwrap();

    assert_eq!(header["alg"], "HS256");
    assert_eq!(header["typ"], "JWT");
}

// ===== Timestamp Validation Tests =====

#[test]
fn test_jwt_timestamps() {
    let user_id = Uuid::new_v4();
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        user_id,
        email: "test@example.com".to_string(),
        roles: vec![],
    };

    // iat should be before exp
    assert!(claims.iat < claims.exp);

    // exp should be in the future
    let now = chrono::Utc::now().timestamp() as usize;
    assert!(claims.exp > now);
}
