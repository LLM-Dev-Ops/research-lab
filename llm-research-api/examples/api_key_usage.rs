//! Example demonstrating the API Key authentication system
//!
//! This example shows how to:
//! - Generate API keys with different permissions and rate limits
//! - Validate API keys
//! - Use API key middleware in routes
//! - Manage API key lifecycle (rotation, revocation)

use chrono::Duration;
use llm_research_api::{
    ApiKeyService, ApiScope, ExperimentPermission, ModelPermission,
    DatasetPermission, RateLimitTier,
    api_key_auth_middleware, optional_api_key_auth_middleware,
};
use llm_research_core::domain::ids::UserId;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    println!("=== API Key Authentication System Demo ===\n");

    // Create the API key service
    let service = ApiKeyService::new();

    // Example 1: Generate a simple API key with all permissions
    println!("1. Generating an admin API key with full access...");
    let owner_id = UserId::new();
    let (admin_key, admin_key_secret) = service
        .generate_key(
            "Admin Key",
            owner_id,
            vec!["admin".to_string()],
            vec![ApiScope::All],
            RateLimitTier::Unlimited,
            None, // No expiration
        )
        .unwrap();

    println!("   ✓ Created key: {} (ID: {})", admin_key.key_prefix, admin_key.id);
    println!("   ✓ Full key (save this!): {}", admin_key_secret);
    println!("   ✓ Rate limit: {:?}", admin_key.rate_limit_tier);
    println!();

    // Example 2: Generate a scoped API key for experiments only
    println!("2. Generating a scoped API key for experiments (read/write only)...");
    let (experiment_key, experiment_key_secret) = service
        .generate_key(
            "Experiment Key",
            owner_id,
            vec!["researcher".to_string()],
            vec![ApiScope::Experiments(vec![
                ExperimentPermission::Read,
                ExperimentPermission::Write,
            ])],
            RateLimitTier::Pro,
            Some(Duration::days(30)), // Expires in 30 days
        )
        .unwrap();

    println!("   ✓ Created key: {} (ID: {})", experiment_key.key_prefix, experiment_key.id);
    println!("   ✓ Permissions: Experiments (Read, Write)");
    println!("   ✓ Expires: {:?}", experiment_key.expires_at);
    println!();

    // Example 3: Generate a multi-scope API key
    println!("3. Generating a multi-scope API key...");
    let (multi_scope_key, multi_scope_secret) = service
        .generate_key(
            "Multi-Scope Key",
            owner_id,
            vec!["data_scientist".to_string()],
            vec![
                ApiScope::Experiments(vec![
                    ExperimentPermission::Read,
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
        )
        .unwrap();

    println!("   ✓ Created key: {} (ID: {})", multi_scope_key.key_prefix, multi_scope_key.id);
    println!("   ✓ Scopes: Experiments, Models, Datasets");
    println!("   ✓ Rate limit: {} req/hour", multi_scope_key.rate_limit_tier.max_requests_per_hour().unwrap());
    println!();

    // Example 4: Validate an API key
    println!("4. Validating API keys...");
    match service.validate_key(&admin_key_secret) {
        Ok(validated_key) => {
            println!("   ✓ Key validated successfully!");
            println!("   ✓ Owner: {}", validated_key.owner_id);
            println!("   ✓ Active: {}", validated_key.is_active);
            println!("   ✓ Last used: {:?}", validated_key.last_used_at);
        }
        Err(e) => println!("   ✗ Validation failed: {}", e),
    }
    println!();

    // Example 5: List all keys for a user
    println!("5. Listing all API keys for user...");
    let user_keys = service.list_keys(owner_id).unwrap();
    println!("   ✓ Found {} keys:", user_keys.len());
    for key in &user_keys {
        println!("      - {} ({}) - {}", key.name, key.key_prefix, 
                 if key.is_active { "Active" } else { "Revoked" });
    }
    println!();

    // Example 6: Revoke an API key
    println!("6. Revoking an API key...");
    service.revoke_key(experiment_key.id).unwrap();
    println!("   ✓ Key {} revoked", experiment_key.key_prefix);
    
    // Try to validate the revoked key
    match service.validate_key(&experiment_key_secret) {
        Ok(_) => println!("   ✗ Revoked key should not validate!"),
        Err(_) => println!("   ✓ Revoked key correctly rejected"),
    }
    println!();

    // Example 7: Rotate an API key
    println!("7. Rotating an API key...");
    let (rotated_key, rotated_secret) = service.rotate_key(admin_key.id).unwrap();
    println!("   ✓ New key created: {}", rotated_key.key_prefix);
    println!("   ✓ Old key revoked");
    
    // Old key should no longer work
    match service.validate_key(&admin_key_secret) {
        Ok(_) => println!("   ✗ Old key should not validate!"),
        Err(_) => println!("   ✓ Old key correctly rejected"),
    }
    
    // New key should work
    match service.validate_key(&rotated_secret) {
        Ok(_) => println!("   ✓ New key validates successfully"),
        Err(_) => println!("   ✗ New key should validate!"),
    }
    println!();

    // Example 8: Check scope permissions
    println!("8. Checking scope permissions...");
    println!("   Multi-scope key permissions:");
    println!("   - Can read experiments? {}", 
             multi_scope_key.has_scope_permission("experiments", "read"));
    println!("   - Can delete experiments? {}", 
             multi_scope_key.has_scope_permission("experiments", "delete"));
    println!("   - Can read models? {}", 
             multi_scope_key.has_scope_permission("models", "read"));
    println!("   - Can write models? {}", 
             multi_scope_key.has_scope_permission("models", "write"));
    println!();

    // Example 9: Rate limit tiers
    println!("9. Rate limit tier information:");
    let tiers = vec![
        RateLimitTier::Free,
        RateLimitTier::Basic,
        RateLimitTier::Pro,
        RateLimitTier::Enterprise,
        RateLimitTier::Unlimited,
    ];
    
    for tier in tiers {
        match tier.max_requests_per_hour() {
            Some(limit) => println!("   {:?}: {} requests/hour", tier, limit),
            None => println!("   {:?}: Unlimited requests", tier),
        }
    }
    println!();

    println!("=== Demo Complete ===");
}
