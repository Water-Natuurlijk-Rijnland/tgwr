//! Authentication and authorization types.
//!
//! This module provides domain models for user authentication,
//! JWT tokens, role-based access control (RBAC), and permissions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// User roles for RBAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Guest - read-only access to public data
    Guest,
    /// Viewer - read access to most data
    Viewer,
    /// Operator - can execute simulations and manage assets
    Operator,
    /// Engineer - can create and modify scenarios
    Engineer,
    /// Admin - full system access
    Admin,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Guest => "guest",
            Self::Viewer => "viewer",
            Self::Operator => "operator",
            Self::Engineer => "engineer",
            Self::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "guest" => Some(Self::Guest),
            "viewer" => Some(Self::Viewer),
            "operator" => Some(Self::Operator),
            "engineer" => Some(Self::Engineer),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    /// Get the numeric level for hierarchy comparison (higher = more permissions)
    pub fn level(&self) -> u8 {
        match self {
            Self::Guest => 0,
            Self::Viewer => 1,
            Self::Operator => 2,
            Self::Engineer => 3,
            Self::Admin => 4,
        }
    }

    /// Check if this role has at least the required level
    pub fn has_level(&self, required: u8) -> bool {
        self.level() >= required
    }

    /// Check if this role has at least the permissions of another role
    pub fn has_role(&self, other: Role) -> bool {
        self.level() >= other.level()
    }
}

/// Specific permissions for fine-grained access control.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Scenario permissions
    ScenariosRead,
    ScenariosCreate,
    ScenariosUpdate,
    ScenariosDelete,
    ScenariosExecute,

    // Result permissions
    ResultsRead,
    ResultsDelete,

    // Asset permissions
    AssetsRead,
    AssetsUpdate,
    AssetsSync,

    // User management permissions
    UsersRead,
    UsersCreate,
    UsersUpdate,
    UsersDelete,

    // System permissions
    SystemStatus,
    SystemConfigure,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ScenariosRead => "scenarios:read",
            Self::ScenariosCreate => "scenarios:create",
            Self::ScenariosUpdate => "scenarios:update",
            Self::ScenariosDelete => "scenarios:delete",
            Self::ScenariosExecute => "scenarios:execute",
            Self::ResultsRead => "results:read",
            Self::ResultsDelete => "results:delete",
            Self::AssetsRead => "assets:read",
            Self::AssetsUpdate => "assets:update",
            Self::AssetsSync => "assets:sync",
            Self::UsersRead => "users:read",
            Self::UsersCreate => "users:create",
            Self::UsersUpdate => "users:update",
            Self::UsersDelete => "users:delete",
            Self::SystemStatus => "system:status",
            Self::SystemConfigure => "system:configure",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "scenarios:read" => Some(Self::ScenariosRead),
            "scenarios:create" => Some(Self::ScenariosCreate),
            "scenarios:update" => Some(Self::ScenariosUpdate),
            "scenarios:delete" => Some(Self::ScenariosDelete),
            "scenarios:execute" => Some(Self::ScenariosExecute),
            "results:read" => Some(Self::ResultsRead),
            "results:delete" => Some(Self::ResultsDelete),
            "assets:read" => Some(Self::AssetsRead),
            "assets:update" => Some(Self::AssetsUpdate),
            "assets:sync" => Some(Self::AssetsSync),
            "users:read" => Some(Self::UsersRead),
            "users:create" => Some(Self::UsersCreate),
            "users:update" => Some(Self::UsersUpdate),
            "users:delete" => Some(Self::UsersDelete),
            "system:status" => Some(Self::SystemStatus),
            "system:configure" => Some(Self::SystemConfigure),
            _ => None,
        }
    }

    /// Get all permissions for a given role
    pub fn for_role(role: Role) -> HashSet<Permission> {
        match role {
            Role::Guest => [
                Permission::ScenariosRead,
                Permission::ResultsRead,
                Permission::AssetsRead,
                Permission::SystemStatus,
            ]
            .into_iter()
            .collect(),
            Role::Viewer => [
                Permission::ScenariosRead,
                Permission::ResultsRead,
                Permission::AssetsRead,
                Permission::UsersRead,
                Permission::SystemStatus,
            ]
            .into_iter()
            .collect(),
            Role::Operator => [
                Permission::ScenariosRead,
                Permission::ScenariosExecute,
                Permission::ResultsRead,
                Permission::AssetsRead,
                Permission::AssetsUpdate,
                Permission::AssetsSync,
                Permission::SystemStatus,
            ]
            .into_iter()
            .collect(),
            Role::Engineer => [
                Permission::ScenariosRead,
                Permission::ScenariosCreate,
                Permission::ScenariosUpdate,
                Permission::ScenariosExecute,
                Permission::ResultsRead,
                Permission::ResultsDelete,
                Permission::AssetsRead,
                Permission::AssetsUpdate,
                Permission::SystemStatus,
            ]
            .into_iter()
            .collect(),
            Role::Admin => [
                Permission::ScenariosRead,
                Permission::ScenariosCreate,
                Permission::ScenariosUpdate,
                Permission::ScenariosDelete,
                Permission::ScenariosExecute,
                Permission::ResultsRead,
                Permission::ResultsDelete,
                Permission::AssetsRead,
                Permission::AssetsUpdate,
                Permission::AssetsSync,
                Permission::UsersRead,
                Permission::UsersCreate,
                Permission::UsersUpdate,
                Permission::UsersDelete,
                Permission::SystemStatus,
                Permission::SystemConfigure,
            ]
            .into_iter()
            .collect(),
        }
    }
}

/// User account stored in the database.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub role: String,
    #[serde(default)]
    pub custom_permissions: Vec<String>, // Additional permissions beyond role
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_active: bool,
}

impl User {
    /// Get the user's role as a Role enum
    pub fn get_role(&self) -> Option<Role> {
        Role::from_str(&self.role)
    }

    /// Get all permissions for this user (role + custom)
    pub fn get_permissions(&self) -> HashSet<Permission> {
        let mut perms = self.get_role()
            .map(Permission::for_role)
            .unwrap_or_default();

        for p_str in &self.custom_permissions {
            if let Some(p) = Permission::from_str(p_str) {
                perms.insert(p);
            }
        }

        perms
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.get_permissions().contains(permission)
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        let user_perms = self.get_permissions();
        permissions.iter().any(|p| user_perms.contains(p))
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        let user_perms = self.get_permissions();
        permissions.iter().all(|p| user_perms.contains(p))
    }
}

/// JWT token claims.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub username: String,
    pub email: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub exp: i64, // Expiration time (Unix timestamp)
    pub iat: i64, // Issued at (Unix timestamp)
}

impl Claims {
    /// Create claims from a user with specified expiration
    pub fn from_user(user: &User, exp: i64) -> Self {
        let permissions = user.get_permissions()
            .into_iter()
            .map(|p| p.as_str().to_string())
            .collect();

        Self {
            sub: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            permissions,
            exp,
            iat: Utc::now().timestamp(),
        }
    }

    /// Check if claims have a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(&permission.as_str().to_string())
    }

    /// Check if claims have any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| self.permissions.contains(&p.as_str().to_string()))
    }
}

/// Login request.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String, // In production, this should be hashed
}

/// Login response with JWT token.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64, // seconds
    pub user: UserInfo,
}

/// User info returned in login response (no sensitive data).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub role: String,
    pub permissions: Vec<String>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        let permissions = user.get_permissions()
            .into_iter()
            .map(|p| p.as_str().to_string())
            .collect();

        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            role: user.role,
            permissions,
        }
    }
}

/// Request to create a new user.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub role: String,
    #[serde(default)]
    pub custom_permissions: Vec<String>,
}

/// Request to update a user.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub custom_permissions: Option<Vec<String>>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Request to change password.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_hierarchy() {
        assert!(Role::Admin.has_role(Role::Engineer));
        assert!(Role::Admin.has_role(Role::Guest));
        assert!(Role::Engineer.has_role(Role::Viewer));
        assert!(!Role::Viewer.has_role(Role::Engineer));
        assert!(!Role::Guest.has_role(Role::Viewer));
    }

    #[test]
    fn test_role_permissions() {
        let admin_perms = Permission::for_role(Role::Admin);
        assert!(admin_perms.contains(&Permission::UsersDelete));
        assert!(admin_perms.contains(&Permission::SystemConfigure));

        let viewer_perms = Permission::for_role(Role::Viewer);
        assert!(viewer_perms.contains(&Permission::ScenariosRead));
        assert!(!viewer_perms.contains(&Permission::ScenariosCreate));
        assert!(!viewer_perms.contains(&Permission::UsersDelete));
    }

    #[test]
    fn test_user_permissions() {
        let mut user = User {
            id: "1".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: None,
            role: "viewer".to_string(),
            custom_permissions: vec!["scenarios:create".to_string()],
            created_at: Utc::now(),
            created_by: None,
            updated_at: None,
            last_login: None,
            is_active: true,
        };

        // Viewer base role doesn't have ScenariosCreate
        assert!(!user.has_permission(&Permission::ScenariosCreate));

        // But custom_permissions does
        assert!(user.has_permission(&Permission::ScenariosCreate));

        // Has base viewer permissions
        assert!(user.has_permission(&Permission::ScenariosRead));
    }

    #[test]
    fn test_permission_serialization() {
        let perm = Permission::ScenariosExecute;
        assert_eq!(perm.as_str(), "scenarios:execute");
        assert_eq!(Permission::from_str("scenarios:execute"), Some(perm));
    }
}
