-- 003_tasks.sql
-- Create tasks table with foreign keys to loops and tasks, and indexes

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INTEGER DEFAULT 0,
    parent_task_id TEXT,
    created_by TEXT NOT NULL,
    iteration_id TEXT,
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

-- Composite index on loop_id and status for filtering tasks by loop and status
CREATE INDEX IF NOT EXISTS idx_tasks_loop_status ON tasks(loop_id, status);

-- Composite index on status and priority DESC for finding next pending task
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority DESC);

-- Index on parent_task_id for hierarchical task queries
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_task_id);
