# API Key Authentication - Quick Reference

## Generate a Key

```rust
use llm_research_api::{ApiKeyService, ApiScope, RateLimitTier};
use llm_research_core::domain::ids::UserId;

let service = ApiKeyService::new();
let (key, secret) = service.generate_key(
    "Key Name",
    UserId::new(),
    vec!["admin".to_string()],
    vec![ApiScope::All],
    RateLimitTier::Pro,
    None,
)?;
// Save `secret` - it's shown only once!
```

## Validate a Key

```rust
let validated = service.validate_key(&secret)?;
```

## Revoke a Key

```rust
service.revoke_key(key.id)?;
```

## Rotate a Key

```rust
let (new_key, new_secret) = service.rotate_key(old_key.id)?;
```

## List User Keys

```rust
let keys = service.list_keys(user_id)?;
```

## Add Middleware

```rust
use axum::Router;

let app = Router::new()
    .route("/api", get(handler))
    .layer(axum::middleware::from_fn_with_state(
        service,
        api_key_auth_middleware,
    ));
```

## Use in Handler

```rust
use axum::extract::Request;
use llm_research_api::{get_api_key_user, require_scope_permission};

async fn handler(request: Request) -> Result<String, ApiError> {
    let user = get_api_key_user(&request)?;
    require_scope_permission(&user, "experiments", "write")?;
    Ok("Success".to_string())
}
```

## Scopes

```rust
// Full access
ApiScope::All

// Specific permissions
ApiScope::Experiments(vec![
    ExperimentPermission::Read,
    ExperimentPermission::Write,
])

// Multiple scopes
vec![
    ApiScope::Experiments(vec![ExperimentPermission::Read]),
    ApiScope::Models(vec![ModelPermission::Read]),
]
```

## Rate Limit Tiers

| Tier | Requests/Hour |
|------|---------------|
| `RateLimitTier::Free` | 100 |
| `RateLimitTier::Basic` | 1,000 |
| `RateLimitTier::Pro` | 10,000 |
| `RateLimitTier::Enterprise` | 100,000 |
| `RateLimitTier::Unlimited` | ∞ |

## Authentication Headers

```bash
# Method 1
curl -H "X-API-Key: llm_sk_..." http://api.example.com

# Method 2
curl -H "Authorization: ApiKey llm_sk_..." http://api.example.com
```

## Key Format

```
llm_sk_<base64_random_bytes>
```

Example: `llm_sk_xJ4kL9mN2pQ5rS8tU1vW3xY6zA0bC4dE7fG9hI2jK5`

## Check Permissions

```rust
// On ApiKey
key.has_scope_permission("experiments", "read")

// On ApiKeyUser
user.has_scope_permission("experiments", "read")
user.has_role("admin")
user.has_any_role(&["admin", "user"])
```

## Security Best Practices

✅ Store keys in environment variables
✅ Use HTTPS for all API calls
✅ Set expiration dates
✅ Use minimum required scopes
✅ Rotate keys regularly (90 days)
✅ Revoke unused keys

❌ Never commit keys to git
❌ Never log full keys
❌ Never use expired keys
❌ Don't share keys between services

## Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `Unauthorized` | Invalid/expired/revoked key | Check key validity |
| `Forbidden` | Missing permission | Add required scope |
| Rate limit exceeded | Too many requests | Upgrade tier or throttle |

## Testing

```bash
# Run tests
cargo test -p llm-research-api api_key

# Run example
cargo run --example api_key_usage
```

## Files

- Implementation: `src/security/api_key.rs`
- Tests: `tests/api_key_tests.rs`
- Example: `examples/api_key_usage.rs`
- Docs: `API_KEY_AUTHENTICATION.md`
