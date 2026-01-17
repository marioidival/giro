-- 002_loops.sql
-- Create loops table with foreign key to users and indexes

CREATE TABLE IF NOT EXISTS loops (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    prd TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    docker_image TEXT NOT NULL DEFAULT 'ralph-loop-manager:latest',
    cpu_limit INTEGER DEFAULT 1,
    memory_limit INTEGER DEFAULT 1024,
    max_iterations INTEGER DEFAULT 100,
    iteration_timeout INTEGER DEFAULT 300,
    iteration_delay INTEGER DEFAULT 0,
    git_repo_url TEXT,
    git_branch_pattern TEXT DEFAULT 'ralph/{loop_id}/{timestamp}',
    status TEXT NOT NULL,
    current_iteration INTEGER DEFAULT 0,
    container_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
);

-- Index on owner_id for faster queries by owner
CREATE INDEX IF NOT EXISTS idx_loops_owner_id ON loops(owner_id);

-- Index on status for filtering by status
CREATE INDEX IF NOT EXISTS idx_loops_status ON loops(status);
