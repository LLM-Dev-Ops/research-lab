# Role-Based Access Control (RBAC) System

## Overview

The RBAC system provides fine-grained access control for the LLM Research API. It includes role definitions, permissions, and middleware for protecting routes.

## Roles

The system defines six roles with hierarchical access levels:

### Admin
- **Description**: Full system access with all permissions
- **Use Case**: System administrators and super users
- **Permissions**: All permissions (23 total)

### Researcher
- **Description**: Can create and run experiments, read models and datasets
- **Use Case**: Scientists and researchers conducting experiments
- **Permissions**:
  - Full access to experiments (create, read, update, delete, run)
  - Full access to prompts
  - Read-only access to datasets and models
  - Can read metrics and export data

### Analyst
- **Description**: Read-only access to experiments, results, and metrics
- **Use Case**: Data analysts reviewing experiment results
- **Permissions**:
  - Read-only access to experiments, datasets, models, prompts
  - Full access to metrics and data export

### DataEngineer
- **Description**: Can manage datasets and read other resources
- **Use Case**: Engineers managing data pipelines and datasets
- **Permissions**:
  - Full access to datasets (create, read, update, delete)
  - Read-only access to experiments, models, prompts
  - Can read metrics and export data

### ModelEngineer
- **Description**: Can manage models and read other resources
- **Use Case**: Engineers managing model configurations
- **Permissions**:
  - Full access to models (create, read, update, delete)
  - Read-only access to experiments, datasets, prompts
  - Can read metrics

### Viewer
- **Description**: Basic read-only access to all resources
- **Use Case**: Stakeholders who need visibility but not editing rights
- **Permissions**:
  - Read-only access to experiments, datasets, models, prompts, metrics
  - Cannot export data or modify anything

## Permissions

The system defines 23 fine-grained permissions organized by resource type:

### Experiment Permissions
- `CreateExperiment` - Create new experiments
- `ReadExperiment` - View experiment details
- `UpdateExperiment` - Modify existing experiments
- `DeleteExperiment` - Remove experiments
- `RunExperiment` - Execute experiment runs

### Dataset Permissions
- `CreateDataset` - Create new datasets
- `ReadDataset` - View dataset details
- `UpdateDataset` - Modify existing datasets
- `DeleteDataset` - Remove datasets

### Model Permissions
- `CreateModel` - Create new model configurations
- `ReadModel` - View model details
- `UpdateModel` - Modify existing models
- `DeleteModel` - Remove models

### Prompt Permissions
- `CreatePrompt` - Create new prompt templates
- `ReadPrompt` - View prompt templates
- `UpdatePrompt` - Modify existing prompts
- `DeletePrompt` - Remove prompts

### Analytics Permissions
- `ReadMetrics` - View metrics and analytics
- `ExportData` - Export data from the system

### Administrative Permissions
- `ManageUsers` - Create, update, delete users
- `ManageApiKeys` - Manage API keys
- `ViewAuditLogs` - View audit logs

## Usage

### 1. Checking Permissions in Handlers

```rust
use llm_research_api::security::{Permission, helpers};
use llm_research_api::middleware::auth::AuthUser;

async fn create_experiment(
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Method 1: Using helper functions
    if !helpers::can_create_experiment(&user) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 2: Direct permission check
    if !user.has_permission(Permission::CreateExperiment) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 3: Check multiple permissions
    if !user.has_all_permissions(&[
        Permission::CreateExperiment,
        Permission::ReadDataset,
    ]) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Proceed with creation
    Ok(Json(json!({ "message": "Created" })))
}
```

### 2. Protecting Routes with Middleware

```rust
use llm_research_api::security::{Permission, Role, PermissionGuard};

fn routes() -> Router {
    Router::new()
        // Require specific permission
        .route(
            "/experiments",
            post(create_experiment)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_permission(Permission::CreateExperiment)
                ))
        )
        // Require specific role
        .route(
            "/admin/users",
            get(list_users)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_role(Role::Admin)
                ))
        )
        // Require any of multiple permissions
        .route(
            "/data/export",
            get(export_data)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_any_permission(vec![
                        Permission::ExportData,
                        Permission::ManageUsers,
                    ])
                ))
        )
        // Require any of multiple roles
        .route(
            "/datasets",
            post(create_dataset)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_any_role(vec![
                        Role::Admin,
                        Role::DataEngineer,
                    ])
                ))
        )
}
```

### 3. Checking Roles

```rust
use llm_research_api::security::{Role, helpers};

async fn admin_only_handler(
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Method 1: Using helper
    if !helpers::is_admin(&user) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 2: Direct role check
    if !user.has_role(Role::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 3: Check multiple roles
    if !user.has_any_role(&[Role::Admin, Role::DataEngineer]) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(json!({ "message": "Admin action completed" })))
}
```

### 4. Resource Ownership

Implement the `ResourceOwnership` trait for resources that should support ownership-based access control:

```rust
use llm_research_api::security::ResourceOwnership;
use llm_research_core::domain::ids::UserId;

struct Experiment {
    id: Uuid,
    owner_id: UserId,
    name: String,
}

impl ResourceOwnership for Experiment {
    fn owner_id(&self) -> UserId {
        self.owner_id
    }

    // Optional: Override default can_access for custom logic
    fn can_access(&self, user: &AuthUser, permission: Permission) -> bool {
        // Owners can do anything
        if self.owner_id() == user.user_id {
            return true;
        }

        // Check role-based permissions
        user.has_permission(permission)
    }
}

// Usage in handler
async fn update_experiment(
    Extension(user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let experiment = load_experiment(id).await?;

    if !experiment.can_access(&user, Permission::UpdateExperiment) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Proceed with update
    Ok(Json(json!({ "message": "Updated" })))
}
```

### 5. Helper Functions

The `helpers` module provides convenient functions for common checks:

```rust
use llm_research_api::security::helpers;

// Experiment helpers
helpers::can_create_experiment(&user);
helpers::can_read_experiment(&user);
helpers::can_update_experiment(&user);
helpers::can_delete_experiment(&user);
helpers::can_run_experiment(&user);

// Resource management helpers
helpers::can_manage_datasets(&user);  // All dataset permissions
helpers::can_manage_models(&user);     // All model permissions

// Other helpers
helpers::can_export_data(&user);
helpers::can_view_audit_logs(&user);
helpers::can_manage_users(&user);
helpers::is_admin(&user);
```

## Role Permission Matrix

| Permission | Admin | Researcher | Analyst | DataEngineer | ModelEngineer | Viewer |
|-----------|-------|------------|---------|--------------|---------------|--------|
| CreateExperiment | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| ReadExperiment | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| UpdateExperiment | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| DeleteExperiment | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| RunExperiment | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| CreateDataset | ✓ | ✗ | ✗ | ✓ | ✗ | ✗ |
| ReadDataset | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| UpdateDataset | ✓ | ✗ | ✗ | ✓ | ✗ | ✗ |
| DeleteDataset | ✓ | ✗ | ✗ | ✓ | ✗ | ✗ |
| CreateModel | ✓ | ✗ | ✗ | ✗ | ✓ | ✗ |
| ReadModel | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| UpdateModel | ✓ | ✗ | ✗ | ✗ | ✓ | ✗ |
| DeleteModel | ✓ | ✗ | ✗ | ✗ | ✓ | ✗ |
| CreatePrompt | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| ReadPrompt | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| UpdatePrompt | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| DeletePrompt | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| ReadMetrics | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| ExportData | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| ManageUsers | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| ManageApiKeys | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| ViewAuditLogs | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |

## Best Practices

1. **Use middleware for route-level protection**: Apply `PermissionGuard` middleware to protect entire routes or route groups.

2. **Use handler-level checks for fine-grained control**: Check permissions within handlers when you need to make decisions based on resource ownership or complex logic.

3. **Combine roles for power users**: Users can have multiple roles. The system checks if ANY of their roles grants the required permission.

4. **Use helper functions for readability**: The `helpers` module provides semantic function names that make code more readable.

5. **Implement ResourceOwnership for shared resources**: Allow resource owners to manage their own resources regardless of their role.

6. **Log permission denials**: Consider adding audit logging when permission checks fail for security monitoring.

## Testing

The RBAC module includes comprehensive tests. Run them with:

```bash
cargo test -p llm-research-api security::rbac::tests
```

## Future Enhancements

Potential future improvements:
- Dynamic role creation via API
- Custom permission sets
- Time-based permissions
- Resource-specific permissions (e.g., access to specific experiments)
- Permission delegation
- Role hierarchies (roles inherit from other roles)
