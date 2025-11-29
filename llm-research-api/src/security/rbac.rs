use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use llm_research_core::domain::ids::UserId;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

/// Roles in the system with hierarchical access levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full system access - all permissions
    Admin,
    /// Can create and run experiments, read models and datasets
    Researcher,
    /// Read-only access to experiments, results, and metrics
    Analyst,
    /// Can manage datasets and read other resources
    DataEngineer,
    /// Can manage models and read other resources
    ModelEngineer,
    /// Basic read-only access to all resources
    Viewer,
}

impl Role {
    /// Parse role from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "researcher" => Some(Role::Researcher),
            "analyst" => Some(Role::Analyst),
            "dataengineer" | "data_engineer" => Some(Role::DataEngineer),
            "modelengineer" | "model_engineer" => Some(Role::ModelEngineer),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }

    /// Convert role to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Researcher => "researcher",
            Role::Analyst => "analyst",
            Role::DataEngineer => "dataengineer",
            Role::ModelEngineer => "modelengineer",
            Role::Viewer => "viewer",
        }
    }
}

/// Fine-grained permissions for resource operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Experiment permissions
    CreateExperiment,
    ReadExperiment,
    UpdateExperiment,
    DeleteExperiment,
    RunExperiment,

    // Dataset permissions
    CreateDataset,
    ReadDataset,
    UpdateDataset,
    DeleteDataset,

    // Model permissions
    CreateModel,
    ReadModel,
    UpdateModel,
    DeleteModel,

    // Prompt permissions
    CreatePrompt,
    ReadPrompt,
    UpdatePrompt,
    DeletePrompt,

    // Analytics and metrics
    ReadMetrics,
    ExportData,

    // Administrative permissions
    ManageUsers,
    ManageApiKeys,
    ViewAuditLogs,
}

/// Maps roles to their granted permissions
pub struct RolePermissions;

impl RolePermissions {
    /// Get all permissions granted to a specific role
    pub fn get_permissions(role: &Role) -> HashSet<Permission> {
        match role {
            Role::Admin => Self::admin_permissions(),
            Role::Researcher => Self::researcher_permissions(),
            Role::Analyst => Self::analyst_permissions(),
            Role::DataEngineer => Self::data_engineer_permissions(),
            Role::ModelEngineer => Self::model_engineer_permissions(),
            Role::Viewer => Self::viewer_permissions(),
        }
    }

    /// Check if a role has a specific permission
    pub fn has_permission(role: &Role, permission: &Permission) -> bool {
        Self::get_permissions(role).contains(permission)
    }

    /// Check if any of the given roles has a specific permission
    pub fn has_any_permission(roles: &[Role], permission: &Permission) -> bool {
        roles.iter().any(|role| Self::has_permission(role, permission))
    }

    /// Admin has all permissions
    fn admin_permissions() -> HashSet<Permission> {
        vec![
            // Experiments
            Permission::CreateExperiment,
            Permission::ReadExperiment,
            Permission::UpdateExperiment,
            Permission::DeleteExperiment,
            Permission::RunExperiment,
            // Datasets
            Permission::CreateDataset,
            Permission::ReadDataset,
            Permission::UpdateDataset,
            Permission::DeleteDataset,
            // Models
            Permission::CreateModel,
            Permission::ReadModel,
            Permission::UpdateModel,
            Permission::DeleteModel,
            // Prompts
            Permission::CreatePrompt,
            Permission::ReadPrompt,
            Permission::UpdatePrompt,
            Permission::DeletePrompt,
            // Analytics
            Permission::ReadMetrics,
            Permission::ExportData,
            // Administration
            Permission::ManageUsers,
            Permission::ManageApiKeys,
            Permission::ViewAuditLogs,
        ]
        .into_iter()
        .collect()
    }

    /// Researcher can create/run experiments and read models/datasets
    fn researcher_permissions() -> HashSet<Permission> {
        vec![
            // Experiments - full access
            Permission::CreateExperiment,
            Permission::ReadExperiment,
            Permission::UpdateExperiment,
            Permission::DeleteExperiment,
            Permission::RunExperiment,
            // Datasets - read only
            Permission::ReadDataset,
            // Models - read only
            Permission::ReadModel,
            // Prompts - full access
            Permission::CreatePrompt,
            Permission::ReadPrompt,
            Permission::UpdatePrompt,
            Permission::DeletePrompt,
            // Analytics
            Permission::ReadMetrics,
            Permission::ExportData,
        ]
        .into_iter()
        .collect()
    }

    /// Analyst has read-only access to results and metrics
    fn analyst_permissions() -> HashSet<Permission> {
        vec![
            // Experiments - read only
            Permission::ReadExperiment,
            // Datasets - read only
            Permission::ReadDataset,
            // Models - read only
            Permission::ReadModel,
            // Prompts - read only
            Permission::ReadPrompt,
            // Analytics - full access
            Permission::ReadMetrics,
            Permission::ExportData,
        ]
        .into_iter()
        .collect()
    }

    /// DataEngineer can manage datasets and read other resources
    fn data_engineer_permissions() -> HashSet<Permission> {
        vec![
            // Experiments - read only
            Permission::ReadExperiment,
            // Datasets - full access
            Permission::CreateDataset,
            Permission::ReadDataset,
            Permission::UpdateDataset,
            Permission::DeleteDataset,
            // Models - read only
            Permission::ReadModel,
            // Prompts - read only
            Permission::ReadPrompt,
            // Analytics
            Permission::ReadMetrics,
            Permission::ExportData,
        ]
        .into_iter()
        .collect()
    }

    /// ModelEngineer can manage models and read other resources
    fn model_engineer_permissions() -> HashSet<Permission> {
        vec![
            // Experiments - read only
            Permission::ReadExperiment,
            // Datasets - read only
            Permission::ReadDataset,
            // Models - full access
            Permission::CreateModel,
            Permission::ReadModel,
            Permission::UpdateModel,
            Permission::DeleteModel,
            // Prompts - read only
            Permission::ReadPrompt,
            // Analytics
            Permission::ReadMetrics,
        ]
        .into_iter()
        .collect()
    }

    /// Viewer has read-only access to all resources
    fn viewer_permissions() -> HashSet<Permission> {
        vec![
            Permission::ReadExperiment,
            Permission::ReadDataset,
            Permission::ReadModel,
            Permission::ReadPrompt,
            Permission::ReadMetrics,
        ]
        .into_iter()
        .collect()
    }
}

/// Trait for resources that support ownership-based access control
pub trait ResourceOwnership {
    /// Get the user ID of the resource owner
    fn owner_id(&self) -> UserId;

    /// Check if a user can access this resource with the given permission
    ///
    /// Default implementation allows:
    /// - Resource owners to perform any action
    /// - Users with the required permission to perform the action
    fn can_access(&self, user: &AuthUser, permission: Permission) -> bool {
        // Parse user roles
        let user_roles: Vec<Role> = user
            .roles
            .iter()
            .filter_map(|r| Role::from_str(r))
            .collect();

        // Owner has full access
        if self.owner_id() == user.user_id {
            return true;
        }

        // Check if user has the required permission through their roles
        RolePermissions::has_any_permission(&user_roles, &permission)
    }
}

/// Extension trait for AuthUser to work with RBAC
impl AuthUser {
    /// Get all roles for this user
    pub fn get_roles(&self) -> Vec<Role> {
        self.roles
            .iter()
            .filter_map(|r| Role::from_str(r))
            .collect()
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: Role) -> bool {
        self.get_roles().contains(&role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        let user_roles = self.get_roles();
        roles.iter().any(|r| user_roles.contains(r))
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        let user_roles = self.get_roles();
        RolePermissions::has_any_permission(&user_roles, &permission)
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        permissions
            .iter()
            .all(|p| self.has_permission(*p))
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions
            .iter()
            .any(|p| self.has_permission(*p))
    }

    /// Check if user is an admin
    pub fn is_admin(&self) -> bool {
        self.has_role(Role::Admin)
    }
}

/// Permission guard middleware factory
pub struct PermissionGuard;

impl PermissionGuard {
    /// Require a specific permission to access the route
    pub fn require_permission(
        permission: Permission,
    ) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
        move |mut request: Request, next: Next| {
            let permission = permission;
            Box::pin(async move {
                // Extract user from request extensions
                let user = request
                    .extensions()
                    .get::<AuthUser>()
                    .ok_or(ApiError::Unauthorized)?
                    .clone();

                // Check permission
                if !user.has_permission(permission) {
                    return Err(ApiError::Forbidden);
                }

                Ok(next.run(request).await)
            })
        }
    }

    /// Require any of the specified permissions to access the route
    pub fn require_any_permission(
        permissions: Vec<Permission>,
    ) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
        move |mut request: Request, next: Next| {
            let permissions = permissions.clone();
            Box::pin(async move {
                // Extract user from request extensions
                let user = request
                    .extensions()
                    .get::<AuthUser>()
                    .ok_or(ApiError::Unauthorized)?
                    .clone();

                // Check if user has any of the required permissions
                if !user.has_any_permission(&permissions) {
                    return Err(ApiError::Forbidden);
                }

                Ok(next.run(request).await)
            })
        }
    }

    /// Require a specific role to access the route
    pub fn require_role(
        role: Role,
    ) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
        move |mut request: Request, next: Next| {
            let role = role;
            Box::pin(async move {
                // Extract user from request extensions
                let user = request
                    .extensions()
                    .get::<AuthUser>()
                    .ok_or(ApiError::Unauthorized)?
                    .clone();

                // Check role
                if !user.has_role(role) {
                    return Err(ApiError::Forbidden);
                }

                Ok(next.run(request).await)
            })
        }
    }

    /// Require any of the specified roles to access the route
    pub fn require_any_role(
        roles: Vec<Role>,
    ) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
        move |mut request: Request, next: Next| {
            let roles = roles.clone();
            Box::pin(async move {
                // Extract user from request extensions
                let user = request
                    .extensions()
                    .get::<AuthUser>()
                    .ok_or(ApiError::Unauthorized)?
                    .clone();

                // Check if user has any of the required roles
                if !user.has_any_role(&roles) {
                    return Err(ApiError::Forbidden);
                }

                Ok(next.run(request).await)
            })
        }
    }
}

/// Helper functions for common permission checks
pub mod helpers {
    use super::*;

    /// Check if user can create experiments
    pub fn can_create_experiment(user: &AuthUser) -> bool {
        user.has_permission(Permission::CreateExperiment)
    }

    /// Check if user can read experiments
    pub fn can_read_experiment(user: &AuthUser) -> bool {
        user.has_permission(Permission::ReadExperiment)
    }

    /// Check if user can update experiments
    pub fn can_update_experiment(user: &AuthUser) -> bool {
        user.has_permission(Permission::UpdateExperiment)
    }

    /// Check if user can delete experiments
    pub fn can_delete_experiment(user: &AuthUser) -> bool {
        user.has_permission(Permission::DeleteExperiment)
    }

    /// Check if user can run experiments
    pub fn can_run_experiment(user: &AuthUser) -> bool {
        user.has_permission(Permission::RunExperiment)
    }

    /// Check if user can manage datasets
    pub fn can_manage_datasets(user: &AuthUser) -> bool {
        user.has_all_permissions(&[
            Permission::CreateDataset,
            Permission::UpdateDataset,
            Permission::DeleteDataset,
        ])
    }

    /// Check if user can manage models
    pub fn can_manage_models(user: &AuthUser) -> bool {
        user.has_all_permissions(&[
            Permission::CreateModel,
            Permission::UpdateModel,
            Permission::DeleteModel,
        ])
    }

    /// Check if user can export data
    pub fn can_export_data(user: &AuthUser) -> bool {
        user.has_permission(Permission::ExportData)
    }

    /// Check if user can view audit logs
    pub fn can_view_audit_logs(user: &AuthUser) -> bool {
        user.has_permission(Permission::ViewAuditLogs)
    }

    /// Check if user can manage other users
    pub fn can_manage_users(user: &AuthUser) -> bool {
        user.has_permission(Permission::ManageUsers)
    }

    /// Check if user has administrative privileges
    pub fn is_admin(user: &AuthUser) -> bool {
        user.is_admin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_user(roles: Vec<&str>) -> AuthUser {
        AuthUser {
            user_id: UserId::from(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            roles: roles.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_role_parsing() {
        assert_eq!(Role::from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("Admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::from_str("researcher"), Some(Role::Researcher));
        assert_eq!(Role::from_str("dataengineer"), Some(Role::DataEngineer));
        assert_eq!(Role::from_str("data_engineer"), Some(Role::DataEngineer));
        assert_eq!(Role::from_str("invalid"), None);
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let permissions = RolePermissions::get_permissions(&Role::Admin);

        assert!(permissions.contains(&Permission::CreateExperiment));
        assert!(permissions.contains(&Permission::DeleteExperiment));
        assert!(permissions.contains(&Permission::ManageUsers));
        assert!(permissions.contains(&Permission::ManageApiKeys));
        assert!(permissions.contains(&Permission::ViewAuditLogs));

        // Admin should have all permissions (22 total)
        assert_eq!(permissions.len(), 22);
    }

    #[test]
    fn test_researcher_permissions() {
        let permissions = RolePermissions::get_permissions(&Role::Researcher);

        // Can manage experiments
        assert!(permissions.contains(&Permission::CreateExperiment));
        assert!(permissions.contains(&Permission::RunExperiment));
        assert!(permissions.contains(&Permission::UpdateExperiment));

        // Can read datasets and models
        assert!(permissions.contains(&Permission::ReadDataset));
        assert!(permissions.contains(&Permission::ReadModel));

        // Cannot manage datasets or models
        assert!(!permissions.contains(&Permission::CreateDataset));
        assert!(!permissions.contains(&Permission::CreateModel));

        // Cannot manage users
        assert!(!permissions.contains(&Permission::ManageUsers));
    }

    #[test]
    fn test_analyst_permissions() {
        let permissions = RolePermissions::get_permissions(&Role::Analyst);

        // Read-only access
        assert!(permissions.contains(&Permission::ReadExperiment));
        assert!(permissions.contains(&Permission::ReadDataset));
        assert!(permissions.contains(&Permission::ReadModel));
        assert!(permissions.contains(&Permission::ReadMetrics));

        // Cannot create or modify
        assert!(!permissions.contains(&Permission::CreateExperiment));
        assert!(!permissions.contains(&Permission::UpdateExperiment));
        assert!(!permissions.contains(&Permission::DeleteExperiment));
    }

    #[test]
    fn test_data_engineer_permissions() {
        let permissions = RolePermissions::get_permissions(&Role::DataEngineer);

        // Can manage datasets
        assert!(permissions.contains(&Permission::CreateDataset));
        assert!(permissions.contains(&Permission::UpdateDataset));
        assert!(permissions.contains(&Permission::DeleteDataset));

        // Can read other resources
        assert!(permissions.contains(&Permission::ReadExperiment));
        assert!(permissions.contains(&Permission::ReadModel));

        // Cannot manage models
        assert!(!permissions.contains(&Permission::CreateModel));
        assert!(!permissions.contains(&Permission::UpdateModel));
    }

    #[test]
    fn test_model_engineer_permissions() {
        let permissions = RolePermissions::get_permissions(&Role::ModelEngineer);

        // Can manage models
        assert!(permissions.contains(&Permission::CreateModel));
        assert!(permissions.contains(&Permission::UpdateModel));
        assert!(permissions.contains(&Permission::DeleteModel));

        // Can read other resources
        assert!(permissions.contains(&Permission::ReadExperiment));
        assert!(permissions.contains(&Permission::ReadDataset));

        // Cannot manage datasets
        assert!(!permissions.contains(&Permission::CreateDataset));
        assert!(!permissions.contains(&Permission::UpdateDataset));
    }

    #[test]
    fn test_viewer_permissions() {
        let permissions = RolePermissions::get_permissions(&Role::Viewer);

        // Read-only access
        assert!(permissions.contains(&Permission::ReadExperiment));
        assert!(permissions.contains(&Permission::ReadDataset));
        assert!(permissions.contains(&Permission::ReadModel));

        // Cannot export data (unlike Analyst)
        assert!(!permissions.contains(&Permission::ExportData));

        // Cannot create or modify anything
        assert!(!permissions.contains(&Permission::CreateExperiment));
        assert!(!permissions.contains(&Permission::UpdateDataset));
    }

    #[test]
    fn test_auth_user_role_checks() {
        let user = create_test_user(vec!["researcher", "analyst"]);

        assert!(user.has_role(Role::Researcher));
        assert!(user.has_role(Role::Analyst));
        assert!(!user.has_role(Role::Admin));

        assert!(user.has_any_role(&[Role::Admin, Role::Researcher]));
        assert!(!user.has_any_role(&[Role::Admin, Role::Viewer]));
    }

    #[test]
    fn test_auth_user_permission_checks() {
        let user = create_test_user(vec!["researcher"]);

        assert!(user.has_permission(Permission::CreateExperiment));
        assert!(user.has_permission(Permission::RunExperiment));
        assert!(user.has_permission(Permission::ReadDataset));

        assert!(!user.has_permission(Permission::CreateDataset));
        assert!(!user.has_permission(Permission::ManageUsers));
    }

    #[test]
    fn test_auth_user_multiple_permissions() {
        let user = create_test_user(vec!["researcher"]);

        assert!(user.has_all_permissions(&[
            Permission::CreateExperiment,
            Permission::ReadExperiment,
            Permission::RunExperiment,
        ]));

        assert!(!user.has_all_permissions(&[
            Permission::CreateExperiment,
            Permission::CreateDataset, // Researcher doesn't have this
        ]));

        assert!(user.has_any_permission(&[
            Permission::CreateDataset, // Don't have
            Permission::CreateExperiment, // Have this one
        ]));
    }

    #[test]
    fn test_has_any_permission_with_multiple_roles() {
        let roles = vec![Role::Researcher, Role::Analyst];

        // Both roles have ReadExperiment
        assert!(RolePermissions::has_any_permission(&roles, &Permission::ReadExperiment));

        // Researcher has this, Analyst doesn't
        assert!(RolePermissions::has_any_permission(&roles, &Permission::CreateExperiment));

        // Neither role has this
        assert!(!RolePermissions::has_any_permission(&roles, &Permission::ManageUsers));
    }

    #[test]
    fn test_helper_functions() {
        let admin = create_test_user(vec!["admin"]);
        let researcher = create_test_user(vec!["researcher"]);
        let viewer = create_test_user(vec!["viewer"]);

        assert!(helpers::is_admin(&admin));
        assert!(!helpers::is_admin(&researcher));

        assert!(helpers::can_create_experiment(&researcher));
        assert!(!helpers::can_create_experiment(&viewer));

        assert!(helpers::can_manage_datasets(&admin));
        assert!(!helpers::can_manage_datasets(&researcher));

        assert!(helpers::can_export_data(&researcher));
        assert!(!helpers::can_export_data(&viewer));
    }
}
