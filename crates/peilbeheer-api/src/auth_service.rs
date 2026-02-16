//! Authentication and authorization service.
//!
//! This module provides JWT token generation, user management,
//! and password hashing for the Peilbeheer API.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;

use peilbeheer_core::{
    Claims, CreateUserRequest, LoginRequest, LoginResponse, Permission, Role,
    UpdateUserRequest, User, UserInfo,
};

use crate::db::Database;

/// JWT secret key (loaded from environment)
const JWT_SECRET_ENV: &str = "JWT_SECRET";
const DEFAULT_JWT_SECRET: &str = "change-this-secret-in-production";
/// Token expiration time (24 hours)
const TOKEN_EXPIRATION_HOURS: i64 = 24;

/// Authentication service configuration.
#[derive(Debug, Clone)]
pub struct AuthServiceConfig {
    /// JWT secret key for signing tokens
    pub jwt_secret: String,
    /// Token expiration time in hours
    pub token_expiration_hours: i64,
}

impl Default for AuthServiceConfig {
    fn default() -> Self {
        Self {
            jwt_secret: std::env::var(JWT_SECRET_ENV)
                .unwrap_or_else(|_| DEFAULT_JWT_SECRET.to_string()),
            token_expiration_hours: TOKEN_EXPIRATION_HOURS,
        }
    }
}

/// Authentication errors.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid username or password")]
    InvalidCredentials,
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
    #[error("User is inactive")]
    UserInactive,
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Insufficient permissions: {0:?}")]
    InsufficientPermissions(Vec<Permission>),
    #[error("Database error: {0}")]
    DatabaseError(#[from] anyhow::Error),
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
}

/// Authentication service.
pub struct AuthService {
    db: Arc<Database>,
    config: AuthServiceConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    /// Create a new authentication service.
    pub fn new(db: Arc<Database>, config: AuthServiceConfig) -> anyhow::Result<Self> {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_ref());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_ref());

        Ok(Self {
            db,
            config,
            encoding_key,
            decoding_key,
        })
    }

    /// Create with default config.
    pub fn with_default_config(db: Arc<Database>) -> anyhow::Result<Self> {
        Self::new(db, AuthServiceConfig::default())
    }

    /// Generate a unique user ID.
    fn generate_user_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        format!("usr_{:x}", timestamp)
    }

    /// Hash a password (simple SHA-256 for now - use bcrypt/argon2 in production).
    fn hash_password(password: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"peilbeheer-salt"); // Add salt
        format!("{:x}", hasher.finalize())
    }

    /// Verify a password against a hash.
    fn verify_password(password: &str, hash: &str) -> bool {
        Self::hash_password(password) == hash
    }

    /// Login a user and return a JWT token.
    pub fn login(&self, req: &LoginRequest) -> Result<LoginResponse, AuthError> {
        // Get user from database
        let user = self.get_user_by_username(&req.username)?
            .ok_or(AuthError::InvalidCredentials)?;

        // Check if user is active
        if !user.is_active {
            return Err(AuthError::UserInactive);
        }

        // Verify password (in production, compare with hashed password from DB)
        // For now, we'll store the password hash in a separate column or validate externally
        // This is a simplified version - in production, use proper password hashing
        let stored_hash = self.get_password_hash(&user.id)?;
        if !Self::verify_password(&req.password, &stored_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        // Update last login
        let _ = self.update_last_login(&user.id);

        // Generate JWT token
        let exp = Utc::now()
            .checked_add_signed(Duration::hours(self.config.token_expiration_hours))
            .unwrap()
            .timestamp();

        let claims = Claims::from_user(&user, exp);

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.token_expiration_hours * 3600,
            user: UserInfo::from(user),
        })
    }

    /// Verify a JWT token and return the claims.
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )?;

        Ok(token_data.claims)
    }

    /// Create a new user.
    pub fn create_user(
        &self,
        req: &CreateUserRequest,
        creator: Option<&str>,
    ) -> Result<User, AuthError> {
        // Check if username already exists
        if let Some(_) = self.get_user_by_username(&req.username)? {
            return Err(AuthError::UserAlreadyExists(req.username.clone()));
        }

        // Check if email already exists
        if let Some(_) = self.get_user_by_email(&req.email)? {
            return Err(AuthError::UserAlreadyExists(req.email.clone()));
        }

        // Validate role
        let role = Role::from_str(&req.role)
            .ok_or_else(|| AuthError::InvalidToken("Invalid role".to_string()))?;

        // Check if creator has permission to create users with this role
        if let Some(creator_id) = creator {
            let creator = self.get_user_by_id(creator_id)?
                .ok_or_else(|| AuthError::UserNotFound(creator_id.to_string()))?;

            // Can only create users with lower or equal role
            let creator_role = creator.get_role().unwrap_or(Role::Guest);
            if role.level() > creator_role.level() {
                return Err(AuthError::InsufficientPermissions(vec![
                    Permission::UsersCreate,
                ]));
            }
        }

        let id = Self::generate_user_id();
        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let password_hash = Self::hash_password(&req.password);
        let perms_json = serde_json::to_string(&req.custom_permissions).unwrap();

        // Insert user
        self.db.execute(
            r#"
            INSERT INTO users (
                id, username, email, full_name, password_hash,
                role, custom_permissions, created_at, created_by, is_active
            ) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', {})
            "#,
            &[
                &id.as_bytes(),
                &req.username.as_bytes(),
                &req.email.as_bytes(),
                &req.full_name.as_ref().map(|s| s.as_bytes()).unwrap_or(&[]),
                &password_hash.as_bytes(),
                &req.role.as_bytes(),
                &perms_json.as_bytes(),
                &now_str.as_bytes(),
                &creator.map(|s| s.as_bytes()).unwrap_or(&[]),
                &(1_i32),
            ],
        )?;

        Ok(User {
            id,
            username: req.username.clone(),
            email: req.email.clone(),
            full_name: req.full_name.clone(),
            role: req.role.clone(),
            custom_permissions: req.custom_permissions.clone(),
            created_at: now,
            created_by: creator.map(|s| s.to_string()),
            updated_at: None,
            last_login: None,
            is_active: true,
        })
    }

    /// Get a user by ID.
    pub fn get_user_by_id(&self, id: &str) -> Result<Option<User>, AuthError> {
        let result = self.db.query_row(
            "SELECT id, username, email, full_name, role, custom_permissions, created_at, created_by, updated_at, last_login, is_active FROM users WHERE id = ?",
            &[&id.as_bytes()],
            |row| {
                Ok(User {
                    id: row.get::<_, String>(0)?,
                    username: row.get::<_, String>(1)?,
                    email: row.get::<_, String>(2)?,
                    full_name: row.get::<_, Option<String>>(3)?,
                    role: row.get::<_, String>(4)?,
                    custom_permissions: parse_json_array(row.get::<_, Option<String>>(5)?),
                    created_at: parse_timestamp(row.get::<_, String>(6)?.as_str()),
                    created_by: row.get::<_, Option<String>>(7)?,
                    updated_at: row.get::<_, Option<String>>(8)?.map(|s| parse_timestamp(&s)),
                    last_login: row.get::<_, Option<String>>(9)?.map(|s| parse_timestamp(&s)),
                    is_active: row.get::<_, i32>(10)? == 1,
                })
            },
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(e) if e.to_string().contains("QueryReturnedNoRows") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a user by username.
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>, AuthError> {
        let result = self.db.query_row(
            "SELECT id, username, email, full_name, role, custom_permissions, created_at, created_by, updated_at, last_login, is_active FROM users WHERE username = ?",
            &[&username.as_bytes()],
            |row| {
                Ok(User {
                    id: row.get::<_, String>(0)?,
                    username: row.get::<_, String>(1)?,
                    email: row.get::<_, String>(2)?,
                    full_name: row.get::<_, Option<String>>(3)?,
                    role: row.get::<_, String>(4)?,
                    custom_permissions: parse_json_array(row.get::<_, Option<String>>(5)?),
                    created_at: parse_timestamp(row.get::<_, String>(6)?.as_str()),
                    created_by: row.get::<_, Option<String>>(7)?,
                    updated_at: row.get::<_, Option<String>>(8)?.map(|s| parse_timestamp(&s)),
                    last_login: row.get::<_, Option<String>>(9)?.map(|s| parse_timestamp(&s)),
                    is_active: row.get::<_, i32>(10)? == 1,
                })
            },
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(e) if e.to_string().contains("QueryReturnedNoRows") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a user by email.
    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let result = self.db.query_row(
            "SELECT id, username, email, full_name, role, custom_permissions, created_at, created_by, updated_at, last_login, is_active FROM users WHERE email = ?",
            &[&email.as_bytes()],
            |row| {
                Ok(User {
                    id: row.get::<_, String>(0)?,
                    username: row.get::<_, String>(1)?,
                    email: row.get::<_, String>(2)?,
                    full_name: row.get::<_, Option<String>>(3)?,
                    role: row.get::<_, String>(4)?,
                    custom_permissions: parse_json_array(row.get::<_, Option<String>>(5)?),
                    created_at: parse_timestamp(row.get::<_, String>(6)?.as_str()),
                    created_by: row.get::<_, Option<String>>(7)?,
                    updated_at: row.get::<_, Option<String>>(8)?.map(|s| parse_timestamp(&s)),
                    last_login: row.get::<_, Option<String>>(9)?.map(|s| parse_timestamp(&s)),
                    is_active: row.get::<_, i32>(10)? == 1,
                })
            },
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(e) if e.to_string().contains("QueryReturnedNoRows") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all users.
    pub fn list_users(&self) -> Result<Vec<User>, AuthError> {
        let users = self.db.query(
            "SELECT id, username, email, full_name, role, custom_permissions, created_at, created_by, updated_at, last_login, is_active FROM users ORDER BY username",
            &[],
            |row| {
                Ok(User {
                    id: row.get::<_, String>(0)?,
                    username: row.get::<_, String>(1)?,
                    email: row.get::<_, String>(2)?,
                    full_name: row.get::<_, Option<String>>(3)?,
                    role: row.get::<_, String>(4)?,
                    custom_permissions: parse_json_array(row.get::<_, Option<String>>(5)?),
                    created_at: parse_timestamp(row.get::<_, String>(6)?.as_str()),
                    created_by: row.get::<_, Option<String>>(7)?,
                    updated_at: row.get::<_, Option<String>>(8)?.map(|s| parse_timestamp(&s)),
                    last_login: row.get::<_, Option<String>>(9)?.map(|s| parse_timestamp(&s)),
                    is_active: row.get::<_, i32>(10)? == 1,
                })
            },
        )?;

        Ok(users)
    }

    /// Update a user.
    pub fn update_user(&self, id: &str, req: &UpdateUserRequest) -> Result<User, AuthError> {
        let user = self.get_user_by_id(id)?
            .ok_or_else(|| AuthError::UserNotFound(id.to_string()))?;

        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();

        // Build update query dynamically based on provided fields
        let mut updates = Vec::new();
        if req.email.is_some() {
            updates.push(format!("email = '{}'", req.email.as_ref().unwrap()));
        }
        if req.full_name.is_some() {
            updates.push(format!("full_name = '{}'", req.full_name.as_ref().unwrap()));
        }
        if req.role.is_some() {
            updates.push(format!("role = '{}'", req.role.as_ref().unwrap()));
        }
        if req.custom_permissions.is_some() {
            let perms_json = serde_json::to_string(req.custom_permissions.as_ref().unwrap())
                .unwrap_or_default();
            updates.push(format!("custom_permissions = '{}'", perms_json));
        }
        if req.is_active.is_some() {
            updates.push(format!("is_active = {}", if req.is_active.unwrap() { 1 } else { 0 }));
        }

        updates.push(format!("updated_at = '{}'", now_str));

        self.db.execute(
            &format!("UPDATE users SET {} WHERE id = '{}'", updates.join(", "), id),
            &[],
        )?;

        // Return updated user
        self.get_user_by_id(id)?
            .ok_or_else(|| AuthError::UserNotFound(id.to_string()))
    }

    /// Delete a user.
    pub fn delete_user(&self, id: &str) -> Result<(), AuthError> {
        self.db.execute(
            "DELETE FROM users WHERE id = ?",
            &[&id.as_bytes()],
        )?;
        Ok(())
    }

    /// Change user password.
    pub fn change_password(
        &self,
        id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        let stored_hash = self.get_password_hash(id)?;

        if !Self::verify_password(old_password, &stored_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        let new_hash = Self::hash_password(new_password);

        self.db.execute(
            "UPDATE users SET password_hash = '{}' WHERE id = '{}'",
            &[&new_hash.as_bytes(), &id.as_bytes()],
        )?;

        Ok(())
    }

    /// Get password hash for a user.
    fn get_password_hash(&self, id: &str) -> Result<String, AuthError> {
        self.db
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?",
                &[&id.as_bytes()],
                |row| row.get::<_, String>(0),
            )
            .map_err(|e| {
                if e.to_string().contains("QueryReturnedNoRows") {
                    AuthError::UserNotFound(id.to_string())
                } else {
                    AuthError::DatabaseError(e.into())
                }
            })
    }

    /// Update last login timestamp.
    fn update_last_login(&self, id: &str) -> Result<(), AuthError> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        self.db.execute(
            "UPDATE users SET last_login = '{}' WHERE id = '{}'",
            &[&now.as_bytes(), &id.as_bytes()],
        )?;
        Ok(())
    }

    /// Check if a user has specific permissions.
    pub fn check_permissions(
        &self,
        user_id: &str,
        required_permissions: &[Permission],
    ) -> Result<bool, AuthError> {
        let user = self.get_user_by_id(user_id)?
            .ok_or_else(|| AuthError::UserNotFound(user_id.to_string()))?;

        Ok(user.has_all_permissions(required_permissions))
    }

    /// Create default admin user if no users exist.
    pub fn ensure_default_admin(&self) -> Result<bool, AuthError> {
        let users = self.list_users()?;
        if users.is_empty() {
            let admin_req = CreateUserRequest {
                username: "admin".to_string(),
                email: "admin@peilbeheer.local".to_string(),
                password: "admin123".to_string(), // Default password - should be changed
                full_name: Some("System Administrator".to_string()),
                role: "admin".to_string(),
                custom_permissions: vec![],
            };

            self.create_user(&admin_req, None)?;
            tracing::warn!("Created default admin user - please change the password!");
            return Ok(true);
        }
        Ok(false)
    }
}

/// Helper to parse timestamp strings.
fn parse_timestamp(s: &str) -> chrono::DateTime<chrono::Utc> {
    use chrono::NaiveDateTime;
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.6f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.6f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// Helper to parse JSON arrays from strings.
fn parse_json_array(s: Option<String>) -> Vec<String> {
    s.and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test123";
        let hash = AuthService::hash_password(password);
        assert!(hash.len() == 64); // SHA-256 produces 64 hex chars
        assert!(AuthService::verify_password(password, &hash));
        assert!(!AuthService::verify_password("wrong", &hash));
    }

    #[test]
    fn test_role_permissions() {
        let admin_perms = Permission::for_role(Role::Admin);
        assert!(admin_perms.contains(&Permission::UsersDelete));

        let viewer_perms = Permission::for_role(Role::Viewer);
        assert!(!viewer_perms.contains(&Permission::UsersDelete));
        assert!(viewer_perms.contains(&Permission::ScenariosRead));
    }
}
