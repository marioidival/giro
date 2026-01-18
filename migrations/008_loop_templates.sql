-- 008_loop_templates.sql
-- Create loop_templates table for storing reusable loop configurations

CREATE TABLE IF NOT EXISTS loop_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    is_public INTEGER NOT NULL DEFAULT 0,
    owner_id TEXT,
    prd TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    docker_image TEXT,
    cpu_limit INTEGER,
    memory_limit INTEGER,
    max_iterations INTEGER,
    iteration_timeout INTEGER,
    iteration_delay INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index on owner_id for filtering templates by user
CREATE INDEX IF NOT EXISTS idx_loop_templates_owner_id ON loop_templates(owner_id);

-- Index on is_public for filtering public templates
CREATE INDEX IF NOT EXISTS idx_loop_templates_is_public ON loop_templates(is_public);
