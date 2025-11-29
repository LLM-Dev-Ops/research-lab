//! Integration tests for API Key authentication system

use chrono::Duration;
use llm_research_api::{
    ApiKeyService, ApiScope, RateLimitTier,
    ExperimentPermission, ModelPermission, DatasetPermission,
};
use llm_research_core::domain::ids::UserId;

#[test]
fn test_api_key_generation() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    let (api_key, secret) = service
        .generate_key(
            "Test Key",
            owner_id,
            vec!["admin".to_string()],
            vec![ApiScope::All],
            RateLimitTier::Pro,
            None,
        )
        .unwrap();

    assert!(secret.starts_with("llm_sk_"));
    assert_eq!(api_key.name, "Test Key");
    assert_eq!(api_key.owner_id, owner_id);
    assert!(api_key.is_active);
    assert!(api_key.is_valid());
}

#[test]
fn test_api_key_validation() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    let (_api_key, secret) = service
        .generate_key(
            "Test Key",
            owner_id,
            vec![],
            vec![ApiScope::All],
            RateLimitTier::Free,
            None,
        )
        .unwrap();

    // Valid key should authenticate
    let validated = service.validate_key(&secret).unwrap();
    assert_eq!(validated.name, "Test Key");
    assert!(validated.last_used_at.is_some());
}

#[test]
fn test_api_key_expiration() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    // Create an already-expired key
    let (_api_key, secret) = service
        .generate_key(
            "Expired Key",
            owner_id,
            vec![],
            vec![ApiScope::All],
            RateLimitTier::Free,
            Some(Duration::seconds(-1)),
        )
        .unwrap();

    // Validation should fail
    assert!(service.validate_key(&secret).is_err());
}

#[test]
fn test_api_key_revocation() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    let (api_key, secret) = service
        .generate_key(
            "Test Key",
            owner_id,
            vec![],
            vec![ApiScope::All],
            RateLimitTier::Basic,
            None,
        )
        .unwrap();

    // Key should work initially
    assert!(service.validate_key(&secret).is_ok());

    // Revoke the key
    service.revoke_key(api_key.id).unwrap();

    // Key should no longer work
    assert!(service.validate_key(&secret).is_err());
}

#[test]
fn test_api_key_rotation() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    let (old_key, old_secret) = service
        .generate_key(
            "Original Key",
            owner_id,
            vec!["admin".to_string()],
            vec![ApiScope::All],
            RateLimitTier::Pro,
            None,
        )
        .unwrap();

    // Rotate the key
    let (new_key, new_secret) = service.rotate_key(old_key.id).unwrap();

    // Old key should be invalid
    assert!(service.validate_key(&old_secret).is_err());

    // New key should be valid
    let validated = service.validate_key(&new_secret).unwrap();
    assert_eq!(validated.id, new_key.id);
    assert_eq!(validated.owner_id, old_key.owner_id);
    assert_eq!(validated.roles, old_key.roles);
}

#[test]
fn test_list_keys() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    // Create multiple keys
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

    // List keys
    let keys = service.list_keys(owner_id).unwrap();
    assert_eq!(keys.len(), 2);
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
fn test_multi_scope_permissions() {
    let service = ApiKeyService::new();
    let owner_id = UserId::new();

    let (api_key, _) = service
        .generate_key(
            "Multi-Scope Key",
            owner_id,
            vec![],
            vec![
                ApiScope::Experiments(vec![ExperimentPermission::Read]),
                ApiScope::Models(vec![ModelPermission::Read, ModelPermission::Write]),
                ApiScope::Datasets(vec![DatasetPermission::Read]),
            ],
            RateLimitTier::Basic,
            None,
        )
        .unwrap();

    // Check experiment permissions
    assert!(api_key.has_scope_permission("experiments", "read"));
    assert!(!api_key.has_scope_permission("experiments", "write"));

    // Check model permissions
    assert!(api_key.has_scope_permission("models", "read"));
    assert!(api_key.has_scope_permission("models", "write"));
    assert!(!api_key.has_scope_permission("models", "delete"));

    // Check dataset permissions
    assert!(api_key.has_scope_permission("datasets", "read"));
    assert!(!api_key.has_scope_permission("datasets", "write"));
}

#[test]
fn test_rate_limit_tiers() {
    assert_eq!(RateLimitTier::Free.max_requests_per_hour(), Some(100));
    assert_eq!(RateLimitTier::Basic.max_requests_per_hour(), Some(1_000));
    assert_eq!(RateLimitTier::Pro.max_requests_per_hour(), Some(10_000));
    assert_eq!(RateLimitTier::Enterprise.max_requests_per_hour(), Some(100_000));
    assert_eq!(RateLimitTier::Unlimited.max_requests_per_hour(), None);
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

#[test]
fn test_invalid_key_validation() {
    let service = ApiKeyService::new();

    // Try to validate a key that doesn't exist
    let result = service.validate_key("llm_sk_invalid_key_123");
    assert!(result.is_err());
}
