-- 004_iterations_files.sql
-- Create iterations and files tables with foreign keys and indexes

-- Iterations table: stores each execution iteration of a loop
CREATE TABLE IF NOT EXISTS iterations (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    iteration_number INTEGER NOT NULL,
    output TEXT,
    error TEXT,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    tokens_used INTEGER,
    FOREIGN KEY (loop_id) REFERENCES loops(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- Files table: stores files created/modified during iterations
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,
    iteration_id TEXT NOT NULL,
    path TEXT NOT NULL,
    content_hash TEXT,
    size INTEGER,
    file_type TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (iteration_id) REFERENCES iterations(id)
);

-- Index on loop_id and iteration_number for finding iterations by loop and order
CREATE INDEX IF NOT EXISTS idx_iterations_loop_number ON iterations(loop_id, iteration_number);

-- Index on iteration_id for finding files by iteration
CREATE INDEX IF NOT EXISTS idx_files_iteration ON files(iteration_id);
