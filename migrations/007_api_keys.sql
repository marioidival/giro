-- 007_api_keys.sql
-- Create api_keys table for storing encrypted LLM API keys

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    encrypted_key TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Unique index on user_id + provider to prevent duplicate keys per provider
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_user_provider ON api_keys(user_id, provider);

-- Index on is_active for filtering active keys
CREATE INDEX IF NOT EXISTS idx_api_keys_is_active ON api_keys(is_active);
