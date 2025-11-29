# JWT Authentication Service

A comprehensive JWT (JSON Web Token) authentication service for the llm-research-api crate, providing secure token generation, validation, and refresh functionality.

## Features

- **Dual Token System**: Access tokens (short-lived) and refresh tokens (long-lived)
- **Secure Defaults**: HS256 algorithm with configurable expiration times
- **Token Type Validation**: Ensures access tokens and refresh tokens are used correctly
- **Blacklisting Support**: JWT IDs (jti) can be extracted for token revocation
- **Environment Configuration**: Loads settings from environment variables with sensible fallbacks
- **Comprehensive Claims**: Includes user ID, email, roles, and standard JWT claims
- **Extensive Testing**: Full test coverage for all functionality

## Architecture

### Core Components

1. **JwtConfig**: Configuration for JWT generation and validation
2. **JwtService**: Main service for token operations
3. **Claims**: Full JWT claims with user information
4. **RefreshClaims**: Minimal claims for refresh tokens
5. **TokenPair**: Access + refresh token bundle
6. **TokenType**: Enum distinguishing access vs refresh tokens
7. **AuthError**: Comprehensive error types

## Configuration

### Environment Variables

```bash
# Required (falls back to development default)
JWT_SECRET=your-secret-key-min-32-chars

# Optional (with defaults)
JWT_ISSUER=llm-research-api           # Token issuer
JWT_AUDIENCE=llm-research-users       # Token audience
JWT_ACCESS_EXPIRY=900                 # Access token expiry (seconds, default: 15 min)
JWT_REFRESH_EXPIRY=604800             # Refresh token expiry (seconds, default: 7 days)
```

### Default Configuration

```rust
use llm_research_api::security::{JwtConfig, JwtService};

// Create with defaults from environment
let service = JwtService::default()?;

// Or create with custom configuration
let config = JwtConfig::with_settings(
    "your-secret-key".to_string(),
    900,      // 15 minutes access token
    604800,   // 7 days refresh token
    "llm-research-api".to_string(),
    "llm-research-users".to_string(),
);
let service = JwtService::new(config);
```

## Usage

### 1. Generate Token Pair

```rust
use llm_research_api::security::JwtService;
use uuid::Uuid;

let service = JwtService::default()?;

let user_id = Uuid::new_v4();
let email = "user@example.com";
let roles = vec!["user".to_string(), "researcher".to_string()];

let token_pair = service.generate_token_pair(user_id, email, roles)?;

// Returns:
// {
//   "access_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
//   "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
//   "token_type": "Bearer",
//   "expires_in": 900
// }
```

### 2. Validate Access Token

```rust
// Validate and extract claims from access token
let claims = service.validate_access_token(&token_pair.access_token)?;

println!("User ID: {}", claims.user_id);
println!("Email: {}", claims.email);
println!("Roles: {:?}", claims.roles);
println!("Expires at: {}", claims.exp);
```

### 3. Validate Refresh Token

```rust
// Validate and extract claims from refresh token
let refresh_claims = service.validate_refresh_token(&token_pair.refresh_token)?;

println!("User ID: {}", refresh_claims.user_id);
println!("Email: {}", refresh_claims.email);
```

### 4. Refresh Tokens

```rust
// Generate new token pair using a valid refresh token
let new_token_pair = service.refresh_tokens(&token_pair.refresh_token)?;

// Both access and refresh tokens are regenerated
assert_ne!(token_pair.access_token, new_token_pair.access_token);
assert_ne!(token_pair.refresh_token, new_token_pair.refresh_token);
```

### 5. Extract JWT ID for Blacklisting

```rust
// Extract the JWT ID (jti) for blacklisting
let jti = service.extract_jti(&token_pair.access_token)?;

// Store jti in a blacklist database/cache
blacklist.add(jti).await?;
```

## Integration with Axum

### Adding to AppState

```rust
use llm_research_api::security::JwtService;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub jwt_service: JwtService,
    // ... other fields
}

impl AppState {
    pub fn new(db_pool: PgPool) -> Result<Self, Box<dyn std::error::Error>> {
        let jwt_service = JwtService::default()?;
        Ok(Self {
            db_pool,
            jwt_service,
        })
    }
}
```

### Login Handler Example

```rust
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use llm_research_api::security::TokenPair;

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    #[serde(flatten)]
    tokens: TokenPair,
    user: UserInfo,
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    // Verify credentials
    let user = verify_credentials(&state.db_pool, &req.email, &req.password).await?;

    // Generate tokens
    let tokens = state.jwt_service.generate_token_pair(
        user.id,
        &user.email,
        user.roles.clone(),
    )?;

    Ok(Json(LoginResponse {
        tokens,
        user: user.into(),
    }))
}
```

### Token Refresh Handler Example

```rust
#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

async fn refresh_tokens(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<TokenPair>, ApiError> {
    // Generate new token pair
    let tokens = state.jwt_service.refresh_tokens(&req.refresh_token)?;
    Ok(Json(tokens))
}
```

### Using with Auth Middleware

```rust
use axum::middleware;
use llm_research_api::middleware::auth::auth_middleware;

let protected_routes = Router::new()
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));
```

## Token Structure

### Access Token Claims

```json
{
  "sub": "user-id-string",
  "exp": 1234567890,
  "iat": 1234567000,
  "nbf": 1234567000,
  "jti": "unique-jwt-id",
  "iss": "llm-research-api",
  "aud": "llm-research-users",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "roles": ["user", "researcher"],
  "token_type": "access"
}
```

### Refresh Token Claims

Similar to access token but with:
- Longer expiration time
- `token_type: "refresh"`

## Security Considerations

### Best Practices

1. **Secret Key**: Use a strong, random secret key (minimum 32 characters)
   ```bash
   # Generate a secure secret
   openssl rand -base64 32
   ```

2. **HTTPS Only**: Always use HTTPS in production to prevent token interception

3. **Secure Storage**:
   - Store access tokens in memory (not localStorage)
   - Store refresh tokens in httpOnly cookies or secure storage

4. **Token Rotation**: Implement refresh token rotation to limit exposure

5. **Blacklisting**: Implement a token blacklist for logout and forced revocation
   ```rust
   // Example blacklist check in middleware
   let jti = jwt_service.extract_jti(token)?;
   if blacklist.contains(&jti).await? {
       return Err(ApiError::Unauthorized);
   }
   ```

6. **Expiration Times**:
   - Access tokens: Short-lived (15 minutes recommended)
   - Refresh tokens: Longer-lived (7 days recommended)

7. **Validation**: Always validate token type to prevent refresh token abuse

### Error Handling

```rust
use llm_research_api::security::AuthError;

match service.validate_access_token(token) {
    Ok(claims) => {
        // Token is valid
    }
    Err(AuthError::TokenExpired) => {
        // Prompt user to refresh
    }
    Err(AuthError::InvalidTokenType { .. }) => {
        // Wrong token type used
    }
    Err(AuthError::JwtDecode(msg)) => {
        // Invalid token format or signature
    }
    Err(e) => {
        // Other errors
    }
}
```

## Testing

The service includes comprehensive tests covering:

- Token generation
- Access token validation
- Refresh token validation
- Token type validation
- Token refresh
- JWT ID extraction
- Error cases

### Running Tests

```bash
# Run all auth module tests
cargo test -p llm-research-api security::auth::

# Run specific test
cargo test -p llm-research-api test_generate_token_pair

# Run with output
cargo test -p llm-research-api security::auth:: -- --nocapture
```

### Example Test

```rust
#[test]
fn test_token_refresh() {
    let service = JwtService::default().unwrap();

    let user_id = Uuid::new_v4();
    let token_pair = service.generate_token_pair(
        user_id,
        "test@example.com",
        vec!["user".to_string()],
    ).unwrap();

    let new_pair = service.refresh_tokens(&token_pair.refresh_token).unwrap();

    assert_ne!(token_pair.access_token, new_pair.access_token);
    assert_ne!(token_pair.refresh_token, new_pair.refresh_token);
}
```

## Example Application

See `examples/jwt_auth_example.rs` for a complete working example:

```bash
JWT_SECRET="your-secret-key" cargo run --example jwt_auth_example
```

## API Reference

### JwtService

#### Methods

- `new(config: JwtConfig) -> Self` - Create service with custom config
- `default() -> AuthResult<Self>` - Create service with default config
- `generate_token_pair(user_id, email, roles) -> AuthResult<TokenPair>` - Generate token pair
- `validate_access_token(token) -> AuthResult<Claims>` - Validate access token
- `validate_refresh_token(token) -> AuthResult<RefreshClaims>` - Validate refresh token
- `refresh_tokens(refresh_token) -> AuthResult<TokenPair>` - Generate new token pair
- `extract_jti(token) -> AuthResult<String>` - Extract JWT ID

### JwtConfig

#### Fields

- `secret_key: String` - Secret for signing tokens
- `access_token_expiry: i64` - Access token lifetime (seconds)
- `refresh_token_expiry: i64` - Refresh token lifetime (seconds)
- `issuer: String` - Token issuer
- `audience: String` - Token audience

### Claims

#### Fields

- `sub: String` - Subject (user ID)
- `exp: i64` - Expiration timestamp
- `iat: i64` - Issued at timestamp
- `nbf: i64` - Not before timestamp
- `jti: String` - JWT ID
- `iss: String` - Issuer
- `aud: String` - Audience
- `user_id: Uuid` - User UUID
- `email: String` - User email
- `roles: Vec<String>` - User roles
- `token_type: TokenType` - Token type (access/refresh)

## Troubleshooting

### Common Issues

1. **"JWT_SECRET not set in environment"**
   - Warning only in development
   - Set `JWT_SECRET` environment variable in production

2. **"Invalid token type"**
   - Ensure using `validate_access_token` for access tokens
   - Ensure using `validate_refresh_token` for refresh tokens

3. **"Token expired"**
   - Access token expired - use refresh token to get new one
   - Refresh token expired - user must re-authenticate

4. **"JWT decoding error"**
   - Invalid token format
   - Token signed with different secret
   - Corrupted token

## License

This component is part of the llm-research-lab project and follows the project's license.
