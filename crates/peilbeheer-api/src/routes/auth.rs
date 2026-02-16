//! Authentication and authorization routes.
//!
//! Endpoints for user login, logout, user management, and JWT token handling.

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use peilbeheer_core::{
    ChangePasswordRequest, Claims, CreateUserRequest, LoginRequest, LoginResponse,
    Permission, Role, UpdateUserRequest, User, UserInfo,
};

use crate::auth_service::{AuthError, AuthService};

/// Response wrapper for API errors.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
    detail: Option<String>,
}

/// Login endpoint - public access.
pub async fn login(
    Extension(auth): Extension<Arc<AuthService>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ErrorResponse> {
    auth.login(&req)
        .map(Json)
        .map_err(|e| match e {
            AuthError::InvalidCredentials => ErrorResponse {
                error: "Invalid credentials".to_string(),
                detail: Some("Username or password is incorrect".to_string()),
            },
            AuthError::UserInactive => ErrorResponse {
                error: "User inactive".to_string(),
                detail: Some("This user account has been disabled".to_string()),
            },
            _ => ErrorResponse {
                error: "Login failed".to_string(),
                detail: Some(e.to_string()),
            },
        })
}

/// Logout endpoint.
pub async fn logout() -> Result<StatusCode, ErrorResponse> {
    // In a stateless JWT system, logout is handled client-side by discarding the token
    Ok(StatusCode::OK)
}

/// Get current user info - simplified version that doesn't require JWT.
pub async fn get_current_user(
    Extension(auth): Extension<Arc<AuthService>>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // For now, return a placeholder
    // TODO: Implement proper JWT extraction
    Ok(Json(serde_json::json!({
        "message": "Authentication not yet implemented",
        "note": "Use the login endpoint to get a token, then include it in Authorization header"
    })))
}

/// List all users.
pub async fn list_users(
    Extension(auth): Extension<Arc<AuthService>>,
) -> Result<Json<Vec<User>>, ErrorResponse> {
    auth.list_users()
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to list users".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Get a specific user by ID.
pub async fn get_user(
    Extension(auth): Extension<Arc<AuthService>>,
    Path(id): Path<String>,
) -> Result<Json<User>, ErrorResponse> {
    auth.get_user_by_id(&id)
        .map_err(|e| ErrorResponse {
            error: "Failed to get user".to_string(),
            detail: Some(e.to_string()),
        })?
        .ok_or_else(|| ErrorResponse {
            error: "User not found".to_string(),
            detail: Some(format!("No user found with ID: {}", id)),
        })
        .map(Json)
}

/// Create a new user.
pub async fn create_user(
    Extension(auth): Extension<Arc<AuthService>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<User>, ErrorResponse> {
    auth.create_user(&req, None)
        .map(|user| {
            tracing::info!("User created: {}", user.username);
            Json(user)
        })
        .map_err(|e| match e {
            AuthError::UserAlreadyExists(username) => ErrorResponse {
                error: "User already exists".to_string(),
                detail: Some(format!("Username or email already in use: {}", username)),
            },
            _ => ErrorResponse {
                error: "Failed to create user".to_string(),
                detail: Some(e.to_string()),
            },
        })
}

/// Update a user (using POST instead of PUT for simplicity).
pub async fn update_user(
    Extension(auth): Extension<Arc<AuthService>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<User>, ErrorResponse> {
    auth.update_user(&id, &req)
        .map(|user| {
            tracing::info!("User updated: {}", user.username);
            Json(user)
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to update user".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Delete a user (using POST /users/:id/delete for simplicity).
pub async fn delete_user(
    Extension(auth): Extension<Arc<AuthService>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    auth.delete_user(&id)
        .map(|_| {
            tracing::info!("User deleted: {}", id);
            StatusCode::NO_CONTENT
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to delete user".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Change user password.
pub async fn change_password(
    Extension(auth): Extension<Arc<AuthService>>,
    Path(id): Path<String>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ErrorResponse> {
    auth.change_password(&id, &req.old_password, &req.new_password)
        .map(|_| {
            tracing::info!("Password changed for user: {}", id);
            StatusCode::NO_CONTENT
        })
        .map_err(|e| match e {
            AuthError::InvalidCredentials => ErrorResponse {
                error: "Invalid password".to_string(),
                detail: Some("The old password is incorrect".to_string()),
            },
            _ => ErrorResponse {
                error: "Failed to change password".to_string(),
                detail: Some(e.to_string()),
            },
        })
}

/// Get user permissions.
pub async fn get_user_permissions(
    Extension(auth): Extension<Arc<AuthService>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    auth.get_user_by_id(&id)
        .map_err(|e| ErrorResponse {
            error: "Failed to get user".to_string(),
            detail: Some(e.to_string()),
        })?
        .ok_or_else(|| ErrorResponse {
            error: "User not found".to_string(),
            detail: Some(format!("No user found with ID: {}", id)),
        })
        .map(|user| {
            let permissions = user.get_permissions()
                .into_iter()
                .map(|p| p.as_str().to_string())
                .collect::<Vec<_>>();

            Json(serde_json::json!({
                "user_id": user.id,
                "username": user.username,
                "role": user.role,
                "permissions": permissions,
            }))
        })
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.error.as_str() {
            "Invalid credentials" | "Invalid password" => StatusCode::UNAUTHORIZED,
            "User not found" => StatusCode::NOT_FOUND,
            "User already exists" | "Invalid role" => StatusCode::BAD_REQUEST,
            "Insufficient permissions" => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}
