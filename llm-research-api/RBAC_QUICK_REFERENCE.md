# RBAC Quick Reference

## Roles

```rust
Role::Admin          // Full access
Role::Researcher     // Experiments + read access
Role::Analyst        // Read-only + analytics
Role::DataEngineer   // Dataset management
Role::ModelEngineer  // Model management
Role::Viewer         // Basic read-only
```

## Common Permissions

```rust
// Experiments
Permission::CreateExperiment
Permission::ReadExperiment
Permission::UpdateExperiment
Permission::DeleteExperiment
Permission::RunExperiment

// Datasets
Permission::CreateDataset
Permission::ReadDataset
Permission::UpdateDataset
Permission::DeleteDataset

// Models
Permission::CreateModel
Permission::ReadModel
Permission::UpdateModel
Permission::DeleteModel

// Admin
Permission::ManageUsers
Permission::ViewAuditLogs
```

## Quick Checks

```rust
// In handler
use llm_research_api::security::helpers;

helpers::can_create_experiment(&user)
helpers::can_manage_datasets(&user)
helpers::can_manage_models(&user)
helpers::is_admin(&user)

// Direct checks
user.has_permission(Permission::CreateExperiment)
user.has_role(Role::Admin)
user.is_admin()
```

## Protect Routes

```rust
use llm_research_api::security::{Permission, Role, PermissionGuard};

// By permission
.route("/experiments", post(handler)
    .layer(axum::middleware::from_fn(
        PermissionGuard::require_permission(Permission::CreateExperiment)
    )))

// By role
.route("/admin", get(handler)
    .layer(axum::middleware::from_fn(
        PermissionGuard::require_role(Role::Admin)
    )))

// Any permission
.route("/export", get(handler)
    .layer(axum::middleware::from_fn(
        PermissionGuard::require_any_permission(vec![
            Permission::ExportData,
            Permission::ManageUsers,
        ])
    )))

// Any role
.route("/datasets", post(handler)
    .layer(axum::middleware::from_fn(
        PermissionGuard::require_any_role(vec![
            Role::Admin,
            Role::DataEngineer,
        ])
    )))
```

## Resource Ownership

```rust
use llm_research_api::security::ResourceOwnership;

impl ResourceOwnership for MyResource {
    fn owner_id(&self) -> UserId {
        self.owner_id
    }
}

// Usage
if !resource.can_access(&user, Permission::UpdateExperiment) {
    return Err(StatusCode::FORBIDDEN);
}
```

## Who Can Do What?

| Action | Who Can Do It |
|--------|---------------|
| Create Experiment | Admin, Researcher |
| Run Experiment | Admin, Researcher |
| Create Dataset | Admin, DataEngineer |
| Create Model | Admin, ModelEngineer |
| Export Data | Admin, Researcher, Analyst, DataEngineer |
| View Metrics | Everyone |
| Manage Users | Admin only |

## Multi-Role Users

```rust
// Users can have multiple roles
let user = AuthUser {
    user_id: user_id,
    email: "user@example.com".to_string(),
    roles: vec!["researcher".to_string(), "dataengineer".to_string()],
};

// They get permissions from ALL their roles
user.has_permission(Permission::CreateExperiment) // true (from researcher)
user.has_permission(Permission::CreateDataset)    // true (from dataengineer)
```

## Error Handling

```rust
// Middleware returns ApiError::Forbidden
// Handler should return StatusCode::FORBIDDEN

async fn handler(Extension(user): Extension<AuthUser>)
    -> Result<Json<Value>, StatusCode>
{
    if !user.has_permission(Permission::CreateExperiment) {
        return Err(StatusCode::FORBIDDEN);
    }
    // ...
}
```
