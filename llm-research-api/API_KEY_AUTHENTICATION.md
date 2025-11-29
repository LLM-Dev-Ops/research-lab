# API Key Authentication System

## Overview

The API Key authentication system provides a secure, flexible way to authenticate service accounts and automated systems accessing the LLM Research API. It supports fine-grained permissions, rate limiting, and key lifecycle management.

## Key Features

- **Secure Key Generation**: Uses cryptographically secure random bytes (32 bytes, base64 encoded)
- **SHA-256 Hashing**: Keys are hashed before storage for security
- **Prefixed Keys**: All keys start with `llm_sk_` for easy identification
- **Fine-Grained Permissions**: Scope-based access control for different resources
- **Rate Limiting**: Multiple tiers from Free to Unlimited
- **Key Expiration**: Optional expiration dates for temporary access
- **Key Rotation**: Seamless key rotation without service interruption
- **Audit Trail**: Last used timestamp tracking

## Architecture

### Core Components

1. **ApiKey**: Metadata structure containing key information
2. **ApiKeyService**: Service for managing API key lifecycle
3. **ApiKeyUser**: Request context representing the authenticated user
4. **ApiScope**: Fine-grained permission system
5. **Middleware**: Authentication middleware for Axum

## Usage

### 1. Generating API Keys

```rust
use llm_research_api::{ApiKeyService, ApiScope, RateLimitTier};
use llm_research_core::domain::ids::UserId;
use chrono::Duration;

let service = ApiKeyService::new();
let owner_id = UserId::new();

// Generate a key with full access
let (api_key, secret) = service.generate_key(
    "Production API Key",
    owner_id,
    vec!["admin".to_string()],
    vec![ApiScope::All],
    RateLimitTier::Pro,
    Some(Duration::days(90)), // Expires in 90 days
)?;

// The secret is only shown once - store it securely!
println!("API Key: {}", secret);
```

### 2. Scoped Permissions

```rust
use llm_research_api::{
    ApiScope, ExperimentPermission, ModelPermission, DatasetPermission
};

// Create a key with specific scopes
let (scoped_key, secret) = service.generate_key(
    "Research Team Key",
    owner_id,
    vec!["researcher".to_string()],
    vec![
        ApiScope::Experiments(vec![
            ExperimentPermission::Read,
            ExperimentPermission::Write,
            ExperimentPermission::Execute,
        ]),
        ApiScope::Models(vec![ModelPermission::Read]),
        ApiScope::Datasets(vec![
            DatasetPermission::Read,
            DatasetPermission::Write,
        ]),
    ],
    RateLimitTier::Basic,
    None,
)?;
```

### 3. Using Middleware

```rust
use axum::{Router, routing::get};
use llm_research_api::{api_key_auth_middleware, ApiKeyService};

let service = ApiKeyService::new();

let app = Router::new()
    .route("/protected", get(protected_handler))
    .layer(axum::middleware::from_fn_with_state(
        service.clone(),
        api_key_auth_middleware,
    ));
```

### 4. Accessing User Info in Handlers

```rust
use axum::{extract::Request, http::StatusCode};
use llm_research_api::{get_api_key_user, require_scope_permission, ApiError};

async fn protected_handler(request: Request) -> Result<String, ApiError> {
    // Get the authenticated user
    let user = get_api_key_user(&request)?;
    
    // Check if user has required permission
    require_scope_permission(&user, "experiments", "write")?;
    
    Ok(format!("Hello, user {}", user.owner_id))
}
```

### 5. Key Management

```rust
// List all keys for a user
let keys = service.list_keys(owner_id)?;

// Revoke a key
service.revoke_key(key_id)?;

// Rotate a key (creates new key with same permissions, revokes old key)
let (new_key, new_secret) = service.rotate_key(key_id)?;

// Get specific key
let key = service.get_key(key_id)?;
```

## API Key Format

API keys follow this format:

```
llm_sk_<base64_encoded_random_bytes>
```

Example:
```
llm_sk_xJ4kL9mN2pQ5rS8tU1vW3xY6zA0bC4dE7fG9hI2jK5
```

### Key Components

- **Prefix**: `llm_sk_` - Identifies this as an LLM Research secret key
- **Random Data**: 32 cryptographically secure random bytes, base64 encoded
- **Total Length**: ~50 characters

### Key Storage

- Only the **SHA-256 hash** of the full key is stored
- The **key prefix** (first 8 chars after prefix) is stored for identification
- The actual key is **never stored** and shown only once during creation

## Permission System

### Available Scopes

#### 1. All Access
```rust
ApiScope::All
```
Grants access to all resources and operations.

#### 2. Experiments
```rust
ApiScope::Experiments(vec![
    ExperimentPermission::Read,    // View experiments
    ExperimentPermission::Write,   // Create/update experiments
    ExperimentPermission::Delete,  // Delete experiments
    ExperimentPermission::Execute, // Run experiments
])
```

#### 3. Models
```rust
ApiScope::Models(vec![
    ModelPermission::Read,   // View models
    ModelPermission::Write,  // Create/update models
    ModelPermission::Delete, // Delete models
])
```

#### 4. Datasets
```rust
ApiScope::Datasets(vec![
    DatasetPermission::Read,   // View datasets
    DatasetPermission::Write,  // Create/update datasets
    DatasetPermission::Delete, // Delete datasets
])
```

#### 5. Metrics
```rust
ApiScope::Metrics(vec![
    MetricPermission::Read,   // View metrics
    MetricPermission::Write,  // Record metrics
    MetricPermission::Delete, // Delete metrics
])
```

## Rate Limit Tiers

| Tier | Requests/Hour | Use Case |
|------|---------------|----------|
| Free | 100 | Testing, development |
| Basic | 1,000 | Small applications |
| Pro | 10,000 | Production applications |
| Enterprise | 100,000 | High-volume production |
| Unlimited | ∞ | Internal services |

```rust
use llm_research_api::RateLimitTier;

let tier = RateLimitTier::Pro;
if let Some(limit) = tier.max_requests_per_hour() {
    println!("Rate limit: {} req/hour", limit);
}
```

## Authentication Methods

The middleware supports two authentication header formats:

### Method 1: X-API-Key Header
```bash
curl -H "X-API-Key: llm_sk_..." https://api.example.com/experiments
```

### Method 2: Authorization Header
```bash
curl -H "Authorization: ApiKey llm_sk_..." https://api.example.com/experiments
```

## Security Best Practices

### 1. Key Generation
- ✅ Use the provided `ApiKeyService::generate_key()` method
- ✅ Keys are generated using cryptographically secure random bytes
- ❌ Never generate keys manually or use predictable values

### 2. Key Storage
- ✅ Store only the hash (done automatically)
- ✅ Show the actual key only once during creation
- ❌ Never log or display full keys after initial creation

### 3. Key Distribution
- ✅ Transmit keys over secure channels (HTTPS, encrypted email)
- ✅ Use environment variables or secret management systems
- ❌ Never commit keys to version control

### 4. Key Rotation
- ✅ Rotate keys regularly (every 90 days recommended)
- ✅ Use the `rotate_key()` method for seamless rotation
- ✅ Provide advance notice before revoking old keys

### 5. Scope Management
- ✅ Grant minimum required permissions (principle of least privilege)
- ✅ Use scoped keys instead of `ApiScope::All` when possible
- ✅ Review and audit key permissions regularly

### 6. Expiration
- ✅ Set expiration dates for temporary access
- ✅ Use short-lived keys for testing
- ❌ Don't use expired keys in production

## Example: Complete Integration

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{Request, State},
    Json,
};
use llm_research_api::{
    ApiKeyService, ApiScope, RateLimitTier,
    api_key_auth_middleware, get_api_key_user,
    require_scope_permission, ApiError,
};
use llm_research_core::domain::ids::UserId;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ExperimentResponse {
    id: String,
    name: String,
}

// Protected handler
async fn list_experiments(request: Request) -> Result<Json<Vec<ExperimentResponse>>, ApiError> {
    // Authenticate and get user
    let user = get_api_key_user(&request)?;
    
    // Check permissions
    require_scope_permission(&user, "experiments", "read")?;
    
    // Your business logic here...
    Ok(Json(vec![]))
}

#[tokio::main]
async fn main() {
    // Initialize the API key service
    let api_key_service = ApiKeyService::new();
    
    // Create an admin key
    let owner_id = UserId::new();
    let (_key, secret) = api_key_service.generate_key(
        "Admin Key",
        owner_id,
        vec!["admin".to_string()],
        vec![ApiScope::All],
        RateLimitTier::Unlimited,
        None,
    ).unwrap();
    
    println!("Admin API Key: {}", secret);
    
    // Build the application with middleware
    let app = Router::new()
        .route("/experiments", get(list_experiments))
        .layer(axum::middleware::from_fn_with_state(
            api_key_service,
            api_key_auth_middleware,
        ));
    
    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

## Testing

Run the example:
```bash
cargo run --example api_key_usage
```

Run the tests:
```bash
cargo test -p llm-research-api api_key
```

## Migration Guide

### From JWT to API Keys

If you're currently using JWT authentication and want to add API key support:

1. **Keep JWT for user authentication**
   - Continue using JWT for web/mobile users
   - Use API keys for service accounts and automation

2. **Add API key middleware to specific routes**
   ```rust
   let app = Router::new()
       // JWT-protected routes
       .route("/user/*", get(user_handler))
       .layer(jwt_middleware)
       
       // API key-protected routes
       .route("/api/v1/*", get(api_handler))
       .layer(api_key_middleware);
   ```

3. **Combine both authentication methods**
   ```rust
   // Use optional middleware to accept both
   .layer(optional_jwt_middleware)
   .layer(optional_api_key_auth_middleware)
   ```

## Troubleshooting

### Key Validation Fails

**Problem**: API key validation returns Unauthorized error

**Solutions**:
- Verify the key is not expired: `key.is_expired()`
- Check if the key is active: `key.is_active`
- Ensure the key hash matches what's stored
- Confirm the header format is correct

### Permission Denied

**Problem**: Authenticated but getting Forbidden error

**Solutions**:
- Check the user's scopes: `user.scopes`
- Verify the required permission exists
- Use `has_scope_permission()` to debug
- Review the scope configuration

### Rate Limit Issues

**Problem**: Too many requests error

**Solutions**:
- Check the tier: `tier.max_requests_per_hour()`
- Upgrade to a higher tier
- Implement request throttling on client side
- Use request batching where possible

## Future Enhancements

- [ ] Database persistence for API keys
- [ ] Key usage analytics and monitoring
- [ ] IP address whitelisting
- [ ] Multiple key rotation (blue/green)
- [ ] Webhook notifications for key events
- [ ] GraphQL API support
- [ ] Key templates for common use cases

## Support

For questions or issues:
- Check the examples in `examples/api_key_usage.rs`
- Review the test cases in `src/security/api_key.rs`
- Open an issue on GitHub

## License

This API key system is part of the LLM Research Lab project and follows the same license.
