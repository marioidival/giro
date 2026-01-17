-- 001_users.sql
-- Create users table with username index for faster lookups

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Index on username for faster login/lookup operations
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
