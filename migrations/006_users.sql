-- Peilbeheer HHVR: User Management
-- Authentication and authorization tables

-- Users table for authentication and authorization
CREATE TABLE IF NOT EXISTS users (
    id VARCHAR PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    email VARCHAR NOT NULL UNIQUE,
    full_name VARCHAR,

    -- Password hash (SHA-256 in production - use bcrypt/argon2)
    password_hash VARCHAR NOT NULL,

    -- Role-based access control
    role VARCHAR NOT NULL DEFAULT 'viewer', -- guest, viewer, operator, engineer, admin

    -- Additional custom permissions (JSON array for flexibility)
    custom_permissions JSON, -- '["scenarios:execute", "assets:sync"]'

    -- Metadata
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR REFERENCES users(id),
    updated_at TIMESTAMP,
    last_login TIMESTAMP,

    -- Account status
    is_active BOOLEAN DEFAULT TRUE
);

-- Indexen voor user lookups
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_created_by ON users(created_by);

-- User sessions table (optional - for session tracking)
CREATE TABLE IF NOT EXISTS user_sessions (
    id VARCHAR PRIMARY KEY,
    user_id VARCHAR NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- JWT token hash (for revocation)
    token_hash VARCHAR NOT NULL,

    -- Session metadata
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    last_accessed TIMESTAMP,

    -- Client info
    user_agent VARCHAR,
    ip_address VARCHAR,

    -- Session status
    is_revoked BOOLEAN DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_expires_at ON user_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_user_sessions_token_hash ON user_sessions(token_hash);

-- User activity log (audit trail)
CREATE TABLE IF NOT EXISTS user_activity_log (
    id VARCHAR PRIMARY KEY,
    user_id VARCHAR REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR NOT NULL, -- login, logout, create_scenario, etc.
    resource_type VARCHAR, -- scenario, asset, user, etc.
    resource_id VARCHAR,

    -- Action details (JSON)
    details JSON,

    -- Request metadata
    ip_address VARCHAR,
    user_agent VARCHAR,

    -- Timestamp
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_activity_log_user_id ON user_activity_log(user_id);
CREATE INDEX IF NOT EXISTS idx_user_activity_log_action ON user_activity_log(action);
CREATE INDEX IF NOT EXISTS idx_user_activity_log_created_at ON user_activity_log(created_at);
CREATE INDEX IF NOT EXISTS idx_user_activity_log_resource ON user_activity_log(resource_type, resource_id);
