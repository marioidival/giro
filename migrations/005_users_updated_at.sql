-- 005_users_updated_at.sql
-- Add updated_at column to users table for consistency with other tables

ALTER TABLE users ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
