-- 006_git_credentials.sql
-- Create git_credentials table for storing encrypted Git tokens

CREATE TABLE IF NOT EXISTS git_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    encrypted_token TEXT NOT NULL,
    username TEXT,
    email TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Unique index on user_id + provider to prevent duplicate credentials per provider
CREATE UNIQUE INDEX IF NOT EXISTS idx_git_credentials_user_provider ON git_credentials(user_id, provider);
