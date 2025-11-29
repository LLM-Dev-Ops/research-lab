/// Example demonstrating the JWT authentication service
///
/// This example shows how to:
/// 1. Initialize the JWT service
/// 2. Generate token pairs (access + refresh tokens)
/// 3. Validate access tokens
/// 4. Validate refresh tokens
/// 5. Refresh tokens using a refresh token
/// 6. Extract JWT IDs for blacklisting
///
/// To run this example:
/// ```bash
/// JWT_SECRET="your-secret-key" cargo run --example jwt_auth_example
/// ```

use llm_research_api::security::{JwtConfig, JwtService};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== JWT Authentication Service Example ===\n");

    // 1. Initialize the JWT service with default configuration
    println!("1. Initializing JWT service with default configuration...");
    let service = JwtService::default()?;
    println!("   ✓ JWT service initialized");
    println!("   - Access token expiry: {} seconds (15 minutes)", service.config().access_token_expiry);
    println!("   - Refresh token expiry: {} seconds (7 days)", service.config().refresh_token_expiry);
    println!("   - Issuer: {}", service.config().issuer);
    println!("   - Audience: {}\n", service.config().audience);

    // 2. Generate token pair for a user
    println!("2. Generating token pair for a user...");
    let user_id = Uuid::new_v4();
    let email = "user@example.com";
    let roles = vec!["user".to_string(), "researcher".to_string()];

    let token_pair = service.generate_token_pair(user_id, email, roles.clone())?;
    println!("   ✓ Token pair generated");
    println!("   - Token type: {}", token_pair.token_type);
    println!("   - Expires in: {} seconds", token_pair.expires_in);
    println!("   - Access token: {}...", &token_pair.access_token[..50]);
    println!("   - Refresh token: {}...\n", &token_pair.refresh_token[..50]);

    // 3. Validate access token
    println!("3. Validating access token...");
    let claims = service.validate_access_token(&token_pair.access_token)?;
    println!("   ✓ Access token is valid");
    println!("   - User ID: {}", claims.user_id);
    println!("   - Email: {}", claims.email);
    println!("   - Roles: {:?}", claims.roles);
    println!("   - Token type: {:?}", claims.token_type);
    println!("   - JWT ID (jti): {}", claims.jti);
    println!("   - Issued at: {}", claims.iat);
    println!("   - Expires at: {}\n", claims.exp);

    // 4. Validate refresh token
    println!("4. Validating refresh token...");
    let refresh_claims = service.validate_refresh_token(&token_pair.refresh_token)?;
    println!("   ✓ Refresh token is valid");
    println!("   - User ID: {}", refresh_claims.user_id);
    println!("   - Email: {}", refresh_claims.email);
    println!("   - Roles: {:?}", refresh_claims.roles);
    println!("   - Token type: {:?}\n", refresh_claims.token_type);

    // 5. Refresh tokens using the refresh token
    println!("5. Refreshing tokens...");
    let new_token_pair = service.refresh_tokens(&token_pair.refresh_token)?;
    println!("   ✓ Tokens refreshed");
    println!("   - New access token: {}...", &new_token_pair.access_token[..50]);
    println!("   - New refresh token: {}...", &new_token_pair.refresh_token[..50]);
    println!("   - Tokens are different: {}\n",
             token_pair.access_token != new_token_pair.access_token);

    // 6. Extract JWT ID for blacklisting
    println!("6. Extracting JWT ID for blacklisting...");
    let jti = service.extract_jti(&token_pair.access_token)?;
    println!("   ✓ JWT ID extracted: {}", jti);
    println!("   - This can be stored in a blacklist to revoke the token\n");

    // 7. Demonstrate token validation errors
    println!("7. Demonstrating validation errors...");

    // Try to use refresh token as access token
    println!("   - Attempting to validate refresh token as access token...");
    match service.validate_access_token(&token_pair.refresh_token) {
        Err(e) => println!("   ✓ Correctly rejected: {}", e),
        Ok(_) => println!("   ✗ Should have been rejected!"),
    }

    // Try to use access token as refresh token
    println!("   - Attempting to validate access token as refresh token...");
    match service.validate_refresh_token(&token_pair.access_token) {
        Err(e) => println!("   ✓ Correctly rejected: {}", e),
        Ok(_) => println!("   ✗ Should have been rejected!"),
    }

    // Try to validate an invalid token
    println!("   - Attempting to validate invalid token...");
    match service.validate_access_token("invalid.token.here") {
        Err(e) => println!("   ✓ Correctly rejected: {}", e),
        Ok(_) => println!("   ✗ Should have been rejected!"),
    }

    // 8. Custom configuration example
    println!("\n8. Creating service with custom configuration...");
    let custom_config = JwtConfig::with_settings(
        "custom-secret-key".to_string(),
        3600,   // 1 hour for access token
        2592000, // 30 days for refresh token
        "custom-issuer".to_string(),
        "custom-audience".to_string(),
    );
    let custom_service = JwtService::new(custom_config);
    println!("   ✓ Custom JWT service created");
    println!("   - Access token expiry: {} seconds (1 hour)", custom_service.config().access_token_expiry);
    println!("   - Refresh token expiry: {} seconds (30 days)", custom_service.config().refresh_token_expiry);

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
