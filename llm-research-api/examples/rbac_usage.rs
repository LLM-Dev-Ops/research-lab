/// Example usage of the RBAC system
///
/// This example demonstrates how to use the Role-Based Access Control system
/// in the llm-research-api crate.

use llm_research_api::security::{
    Role, Permission, RolePermissions, PermissionGuard,
    helpers,
};
use llm_research_api::middleware::auth::AuthUser;
use llm_research_core::domain::ids::UserId;
use uuid::Uuid;
use axum::{
    Router,
    routing::{get, post},
    extract::Extension,
    http::StatusCode,
    Json,
};
use serde_json::json;

// Example: Checking permissions in a handler
async fn create_experiment_handler(
    Extension(user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Method 1: Using helper functions
    if !helpers::can_create_experiment(&user) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 2: Using direct permission check
    if !user.has_permission(Permission::CreateExperiment) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 3: Checking multiple permissions
    let required_permissions = vec![
        Permission::CreateExperiment,
        Permission::ReadDataset,
    ];
    if !user.has_all_permissions(&required_permissions) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(json!({ "message": "Experiment created successfully" })))
}

// Example: Checking roles
async fn admin_only_handler(
    Extension(user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Method 1: Using helper
    if !helpers::is_admin(&user) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Method 2: Direct role check
    if !user.has_role(Role::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(json!({ "message": "Admin action completed" })))
}

// Example: Setting up routes with permission guards
fn protected_routes() -> Router {
    Router::new()
        // Require specific permission
        .route(
            "/experiments",
            post(create_experiment_handler)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_permission(Permission::CreateExperiment)
                ))
        )
        // Require admin role
        .route(
            "/admin/users",
            get(admin_only_handler)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_role(Role::Admin)
                ))
        )
        // Require any of multiple permissions
        .route(
            "/data/export",
            get(export_data_handler)
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
            post(create_dataset_handler)
                .layer(axum::middleware::from_fn(
                    PermissionGuard::require_any_role(vec![
                        Role::Admin,
                        Role::DataEngineer,
                    ])
                ))
        )
}

async fn export_data_handler(
    Extension(user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({ "message": "Data exported" })))
}

async fn create_dataset_handler(
    Extension(user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({ "message": "Dataset created" })))
}

// Example: Resource ownership
use llm_research_api::security::ResourceOwnership;

struct Experiment {
    id: Uuid,
    owner_id: UserId,
    name: String,
}

impl ResourceOwnership for Experiment {
    fn owner_id(&self) -> UserId {
        self.owner_id
    }

    // Use default implementation for can_access
    // Or override for custom logic:
    fn can_access(&self, user: &AuthUser, permission: Permission) -> bool {
        // Owners can do anything
        if self.owner_id() == user.user_id {
            return true;
        }

        // Check role-based permissions
        user.has_permission(permission)
    }
}

async fn update_experiment_handler(
    Extension(user): Extension<AuthUser>,
    // In a real handler, you'd extract the experiment ID and load from DB
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Load experiment from database
    let experiment = Experiment {
        id: Uuid::new_v4(),
        owner_id: UserId::from(Uuid::new_v4()),
        name: "Test Experiment".to_string(),
    };

    // Check if user can update this specific experiment
    if !experiment.can_access(&user, Permission::UpdateExperiment) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(json!({ "message": "Experiment updated" })))
}

// Example: Checking permissions programmatically
fn demonstrate_permission_checks() {
    // Check what permissions a role has
    let researcher_perms = RolePermissions::get_permissions(&Role::Researcher);
    println!("Researcher has {} permissions", researcher_perms.len());

    // Check if a specific role has a permission
    let can_create = RolePermissions::has_permission(&Role::Researcher, &Permission::CreateExperiment);
    println!("Researcher can create experiments: {}", can_create);

    // Check if any role in a list has a permission
    let roles = vec![Role::Viewer, Role::Analyst];
    let can_delete = RolePermissions::has_any_permission(&roles, &Permission::DeleteExperiment);
    println!("Viewer or Analyst can delete experiments: {}", can_delete);
}

// Example: Creating users with different roles
fn create_users() {
    let admin = AuthUser {
        user_id: UserId::from(Uuid::new_v4()),
        email: "admin@example.com".to_string(),
        roles: vec!["admin".to_string()],
    };

    let researcher = AuthUser {
        user_id: UserId::from(Uuid::new_v4()),
        email: "researcher@example.com".to_string(),
        roles: vec!["researcher".to_string()],
    };

    let multi_role_user = AuthUser {
        user_id: UserId::from(Uuid::new_v4()),
        email: "power-user@example.com".to_string(),
        roles: vec!["researcher".to_string(), "dataengineer".to_string()],
    };

    println!("Admin is admin: {}", admin.is_admin());
    println!("Researcher can create experiments: {}", researcher.has_permission(Permission::CreateExperiment));
    println!("Multi-role user can manage datasets: {}", multi_role_user.has_permission(Permission::CreateDataset));
}

fn main() {
    println!("=== RBAC System Examples ===\n");

    println!("1. Permission checks:");
    demonstrate_permission_checks();

    println!("\n2. User creation:");
    create_users();

    println!("\n3. See the handler functions above for usage in routes");
}
