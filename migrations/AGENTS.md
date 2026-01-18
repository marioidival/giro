# Database Migrations - Intent Layer

## Purpose

This directory contains all SQL migrations that define and evolve the database schema for the Ralph Loop Manager. Migrations are automatically executed via SQLx each time the application starts.

**What this area does:**
- Defines the initial database schema (tables, columns, types)
- Manages schema evolution over time
- Establishes foreign key constraints and cascade rules
- Creates indexes to optimize common queries
- Ensures idempotence for safe multiple execution

**What this area does NOT do:**
- Does not contain Rust code (only pure SQL)
- Does not validate complex business rules (only DB constraints)
- Does not run data queries (only DDL - Data Definition Language)
- Does not replace repository tests (applies schema, does not validate queries)

## Structure

```
migrations/
├── 001_users.sql              # User table and authentication
├── 002_loops.sql              # Ralph loops table and configuration
├── 003_tasks.sql              # Hierarchical tasks table
└── 004_iterations_files.sql   # Iterations and generated files tables
```

### Naming Convention

`{number}_{description}.sql`

- `number`: 3-digit zero-padded (001, 002, 003...)
- `description`: snake_case describing the purpose
- **Example**: `005_add_loop_container_metrics.sql`

### Execution Order

Migrations run in ascending numeric order each application startup:

```
001_users.sql
  → 002_loops.sql
      → 003_tasks.sql
          → 004_iterations_files.sql
```

## Database Schema

### Table: users

Stores authentication and user identification information.

```sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,                    -- UUID v4
    username TEXT UNIQUE NOT NULL,          -- Unique username
    email TEXT UNIQUE NOT NULL,             -- Unique email
    password_hash TEXT NOT NULL,            -- Hash bcrypt (cost 12)
    created_at TEXT NOT NULL                -- ISO 8601 timestamp
);

CREATE INDEX idx_users_username ON users(username);
```

**Constraints:**
- `id`: PRIMARY KEY (UUID v4)
- `username`: UNIQUE - prevents duplicate users
- `email`: UNIQUE - prevents duplicate emails

**Indexes:**
- `idx_users_username`: Speeds up login/username lookups

---

### Table: loops

Stores Ralph loops with complete execution configuration.

```sql
CREATE TABLE IF NOT EXISTS loops (
    id TEXT PRIMARY KEY,                              -- UUID v4
    name TEXT NOT NULL,                               -- Descriptive name
    description TEXT,                                 -- Optional
    prd TEXT NOT NULL,                                -- Product Requirements Document (Markdown)
    owner_id TEXT NOT NULL,                           -- FK for users
    provider TEXT NOT NULL,                            -- "claude", "openai", "sourcegraph"
    model TEXT NOT NULL,                              -- Ex: "claude-3-opus"
    docker_image TEXT NOT NULL DEFAULT 'ralph-loop-manager:latest',
    cpu_limit INTEGER DEFAULT 1,                      -- Number of CPUs
    memory_limit INTEGER DEFAULT 1024,                 -- Memory in MB
    max_iterations INTEGER DEFAULT 100,                -- Iteration limit
    iteration_timeout INTEGER DEFAULT 300,             -- Timeout in seconds
    iteration_delay INTEGER DEFAULT 0,                -- Delay between iterations (ms)
    git_repo_url TEXT,                                -- Git repository URL
    git_branch_pattern TEXT DEFAULT 'ralph/{loop_id}/{timestamp}',
    status TEXT NOT NULL,                             -- "created", "running", "paused", "completed", "error"
    current_iteration INTEGER DEFAULT 0,              -- Current iteration
    container_id TEXT,                                -- Docker container ID
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
);

CREATE INDEX idx_loops_owner_id ON loops(owner_id);
CREATE INDEX idx_loops_status ON loops(status);
```

**Constraints:**
- `owner_id`: FK → users(id) - prevents loops without owner

**Defaults:**
- `docker_image`: "ralph-loop-manager:latest"
- `cpu_limit`: 1
- `memory_limit`: 1024 MB
- `max_iterations`: 100
- `iteration_timeout`: 300s (5 minutes)
- `iteration_delay`: 0ms
- `git_branch_pattern`: "ralph/{loop_id}/{timestamp}"

**Indexes:**
- `idx_loops_owner_id`: Queries like "all loops of user X"
- `idx_loops_status`: Filtering by status ("running loops")

---

### Table: tasks

Stores hierarchical tasks within loops, with prioritization and status.

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,                          -- UUID v4
    loop_id TEXT NOT NULL,                        -- FK for loops
    title TEXT NOT NULL,                          -- Short title
    description TEXT NOT NULL,                    -- Detailed description
    status TEXT NOT NULL,                         -- "pending", "in_progress", "completed", "failed", "cancelled"
    priority INTEGER DEFAULT 0,                   -- Priority (higher = more important)
    parent_task_id TEXT,                          -- FK for tasks (subtasks)
    created_by TEXT NOT NULL,                     -- "user" or "llm"
    iteration_id TEXT,                            -- FK for iterations (when created during iteration)
    started_at TEXT,                              -- Start timestamp
    completed_at TEXT,                            -- Completion timestamp
    error_message TEXT,                           -- Error message if failed
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE INDEX idx_tasks_loop_status ON tasks(loop_id, status);
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
```

**Constraints:**
- `loop_id`: FK → loops(id) ON DELETE CASCADE - deletes tasks when loop is deleted
- `parent_task_id`: FK → tasks(id) ON DELETE SET NULL - sets NULL when parent is deleted

**Indexes:**
- `idx_tasks_loop_status`: Queries like "pending tasks of loop X"
- `idx_tasks_status_priority`: Queries like "next task to execute" (sorts by priority DESC)
- `idx_tasks_parent`: Queries like "subtasks of task X"

---

### Table: iterations

Stores each loop iteration execution, with output and metrics.

```sql
CREATE TABLE IF NOT EXISTS iterations (
    id TEXT PRIMARY KEY,                      -- UUID v4
    loop_id TEXT NOT NULL,                    -- FK for loops
    task_id TEXT NOT NULL,                    -- FK for tasks
    iteration_number INTEGER NOT NULL,        -- Sequential (1, 2, 3...)
    output TEXT,                              -- LLM output
    error TEXT,                               -- Error message if failed
    status TEXT NOT NULL,                     -- "running", "completed", "error"
    started_at TEXT NOT NULL,
    completed_at TEXT,
    tokens_used INTEGER,                      -- Tokens consumed by API
    FOREIGN KEY (loop_id) REFERENCES loops(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX idx_iterations_loop_number ON iterations(loop_id, iteration_number);
```

**Constraints:**
- `loop_id`: FK → loops(id) - tracking which loop
- `task_id`: FK → tasks(id) - tracking which task was being executed

**Indexes:**
- `idx_iterations_loop_number`: Queries like "all iterations of loop X in order"

---

### Table: files

Stores files created/modified during iterations.

```sql
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,                      -- UUID v4
    iteration_id TEXT NOT NULL,               -- FK for iterations
    path TEXT NOT NULL,                       -- File path
    content_hash TEXT,                        -- Content hash (for deduplication)
    size INTEGER,                             -- Size in bytes
    file_type TEXT,                           -- File type (ex: "rust", "sql", "md")
    created_at TEXT NOT NULL,
    FOREIGN KEY (iteration_id) REFERENCES iterations(id)
);

CREATE INDEX idx_files_iteration ON files(iteration_id);
```

**Constraints:**
- `iteration_id`: FK → iterations(id) - tracking which iteration created the file

**Indexes:**
- `idx_files_iteration`: Queries like "files of iteration X"

## Critical Invariants

### Idempotency

**ALL** migrations must be idempotent - run multiple times without error:

```sql
-- ✅ CORRECT - Idempotent
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    ...
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- ❌ WRONG - Fails if already exists
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    ...
);

CREATE INDEX idx_users_username ON users(username);
```

### Foreign Keys

**ALL** relationships must have FK constraints defined:

```sql
-- ✅ CORRECT - FK with appropriate ON DELETE
FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE

-- ❌ WRONG - No FK constraint (risk of orphaned records)
loop_id TEXT NOT NULL
```

### Cascade Rules

**ALWAYS** define appropriate cascade behavior:

- **CASCADE DELETE**: When parent is deleted, children are deleted
  - `tasks.loop_id` → `loops.id` (if deleting loop, delete tasks)
  - `iterations.loop_id` → `loops.id` (if deleting loop, delete iterations)
  - `files.iteration_id` → `iterations.id` (if deleting iteration, delete files)

- **SET NULL**: When parent is deleted, children have FK set to NULL
  - `tasks.parent_task_id` → `tasks.id` (if deleting task parent, subtasks continue)

- **NO ACTION**: Default - does not delete children if parent is deleted (constraint fails)
  - `loops.owner_id` → `users.id` (do not delete loop if user deleted)

### Text Timestamps

**ALWAYS** use TEXT for timestamps (ISO 8601):

```sql
-- ✅ CORRECT - Text timestamp
created_at TEXT NOT NULL  -- "2026-01-18T14:30:00Z"

-- ❌ WRONG - SQLite has no native DATETIME type
created_at DATETIME NOT NULL
```

### Performance-Critical Indexes

**ALWAYS** create indexes for frequent queries:

```sql
-- Queries that filter by status
SELECT * FROM loops WHERE status = 'running';
-- ↓ Need index
CREATE INDEX idx_loops_status ON loops(status);

-- Queries that order by priority
SELECT * FROM tasks WHERE status = 'pending' ORDER BY priority DESC LIMIT 1;
-- ↓ Need composite index
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
```

## Usage Patterns

### Creating New Migration

1. **Identify next version** (ex: 005 if last is 004)
2. **Create file** `005_{description}.sql` in the `migrations/` directory
3. **Write idempotent SQL** with `CREATE TABLE IF NOT EXISTS`
4. **Add indexes** for queries that will be impacted
5. **Define FK constraints** appropriate
6. **Test** by running the application (migrations run automatically)

**Example: Add container metrics table**

```sql
-- 005_container_metrics.sql
-- Add table to track CPU/memory metrics of containers

CREATE TABLE IF NOT EXISTS container_metrics (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    iteration_id TEXT,
    cpu_usage_percent REAL,
    memory_usage_mb INTEGER,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id),
    FOREIGN KEY (iteration_id) REFERENCES iterations(id)
);

CREATE INDEX IF NOT EXISTS idx_container_metrics_loop ON container_metrics(loop_id);
CREATE INDEX IF NOT EXISTS idx_container_metrics_recorded ON container_metrics(recorded_at);
```

### Adding Column

```sql
-- 006_add_loop_auto_commit_flag.sql
-- Add flag to enable auto-commit of PRs

-- SQLite does not support ALTER TABLE ... IF NOT EXISTS
-- Use verification or create new migration that fails if already exists
ALTER TABLE loops ADD COLUMN auto_commit INTEGER DEFAULT 0;
```

### Adding Index

```sql
-- 007_add_loop_created_at_index.sql
-- Add index to sort loops by creation date

CREATE INDEX IF NOT EXISTS idx_loops_created_at ON loops(created_at DESC);
```

### Running Migrations Manually

```bash
# Migrations run automatically on startup
cargo run --bin ralph-server

# Output in log:
# INFO ralph_repositories::database: Applying migrations
# INFO ralph_repositories::database: Database pragmas verified successfully
# INFO ralph_server: Database initialized and migrations applied
```

### Checking Current Schema

```bash
# Connect to SQLite database
sqlite3 ralph.db

# List all tables
.tables

# See schema of a table
.schema users

# See indexes
.indexes

# Quit
.quit
```

## Anti-patterns

### NEVER DO

**1. Non-idempotent DDL**
```sql
-- ❌ WRONG - Fails on second execution
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL
);

-- ✅ CORRECT - Idempotent
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL
);
```

**2. No Foreign Keys**
```sql
-- ❌ WRONG - Orphaned records possible
CREATE TABLE tasks (
    loop_id TEXT NOT NULL,  -- No FK!
    title TEXT NOT NULL
);

-- ✅ CORRECT - FK constraint
CREATE TABLE tasks (
    loop_id TEXT NOT NULL,
    title TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE
);
```

**3. Inappropriate CASCADE**
```sql
-- ❌ WRONG - Deleting loops when user deleted
-- This loses important data!
FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE

-- ✅ CORRECT - Prevent deletion if loop exists
FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE RESTRICT
-- Or use NO ACTION (default)
FOREIGN KEY (owner_id) REFERENCES users(id)
```

**4. Missing Indexes**
```sql
-- ❌ WRONG - Slow query: WHERE status = 'running'
CREATE TABLE loops (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL  -- No index!
);

-- ✅ CORRECT - Index for performance
CREATE TABLE loops (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL
);
CREATE INDEX idx_loops_status ON loops(status);
```

**5. Integer for Timestamps**
```sql
-- ❌ WRONG - Unix timestamp (unreadable, timezone issues)
created_at INTEGER NOT NULL

-- ✅ CORRECT - ISO 8601 string (readable, timezone included)
created_at TEXT NOT NULL  -- "2026-01-18T14:30:00Z"
```

**6. Modifying Existing Migration**

```sql
-- ❌ WRONG - Never modify migration that has already run in production
-- If 001_users.sql was already applied, do not edit the file!
-- This breaks migration integrity.

-- ✅ CORRECT - Create new migration for the change
-- 008_add_user_last_login.sql
ALTER TABLE users ADD COLUMN last_login TEXT;
```

## SQLx Migration System

### How It Works

```rust
// In ralph-repositories/src/database.rs
sqlx::migrate!("../migrations")
    .run(&pool)
    .await
    .context("Failed to run database migrations")?;
```

**What SQLx does:**
1. Reads all `.sql` files from the `migrations/` directory
2. Creates `_sqlx_migrations` table for tracking
3. Checks which migrations have already been applied
4. Executes pending migrations in numerical order
5. Records success in the `_sqlx_migrations` table

### Tracking Table

```sql
-- Created automatically by SQLx
CREATE TABLE _sqlx_migrations (
    version INTEGER PRIMARY KEY,
    description TEXT,
    installed_on TEXT NOT NULL,
    checksum TEXT
);
```

**Example content:**
| version | description | installed_on | checksum |
|---------|-------------|--------------|----------|
| 1 | create_users_table | 2026-01-18T14:00:00Z | abc123... |
| 2 | create_loops_table | 2026-01-18T14:00:01Z | def456... |
| 3 | create_tasks_table | 2026-01-18T14:00:02Z | ghi789... |
| 4 | create_iterations_files_tables | 2026-01-18T14:00:03Z | jkl012... |

### Rollbacks

**SQLx does not support automatic rollback.**

To revert schema, create revert migration:

```sql
-- 009_revert_container_metrics.sql
-- Revert migration 005 (remove table)

DROP TABLE IF EXISTS container_metrics;
DROP INDEX IF EXISTS idx_container_metrics_loop;
DROP INDEX IF EXISTS idx_container_metrics_recorded;
```

## Dependencies

### Dependencies (Cargo.toml)

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate", "chrono"] }
```

**Relevant features:**
- `migrate`: Enables `sqlx::migrate!()` macro
- `sqlite`: SQLite support
- `runtime-tokio`: Tokio async runtime

### Downstreams (who depends on the migrations)

- `ralph-repositories` → Executes migrations via SQLx
- `ralph-server` → Initializes database (which runs migrations)
- `ralph-services` → Uses models that depend on the schema

## Pitfalls

### Common Confusions

**1. Migration Order Matters**

```sql
-- ❌ WRONG - Migration 002 tries to create FK for table that doesn't exist yet
-- 002_tasks.sql
FOREIGN KEY (loop_id) REFERENCES loops(id)  -- loops was created in 003!

-- ✅ CORRECT - loops exists before tasks
-- 002_loops.sql
CREATE TABLE loops (...);

-- 003_tasks.sql
CREATE TABLE tasks (
    loop_id TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id)  -- loops already exists
);
```

**2. SQLite ALTER TABLE Limitations**

```sql
-- SQLite does not support:
ALTER TABLE users DROP COLUMN password_hash;  -- ❌ NOT supported

-- Workaround: create new table and migrate data
-- 010_remove_password_hash.sql
CREATE TABLE users_new (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO users_new SELECT id, username, email, created_at FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
```

**3. Migration Naming and Execution**

```sql
-- ❌ WRONG - Migration 002 will run before 010
-- 002_add_important_feature.sql
-- 010_add_urgent_bugfix.sql

-- ✅ CORRECT - Use sequential numbers
-- 005_add_important_feature.sql
-- 006_add_urgent_bugfix.sql
```

**4. Type Mismatch between Schema and Rust**

```sql
-- Schema defines:
CREATE TABLE loops (
    status TEXT NOT NULL  -- "running", "paused", etc
);

-- But the Rust code expects LoopStatus enum:
// This will cause compile-time error with SQLx!
let status: LoopStatus = sqlx::query_as!(Loop, "SELECT status FROM loops WHERE id = ?", id)
    .fetch_one(&pool)
    .await?
    .status;  // ❌ ERROR: status is String, not LoopStatus

// ✅ CORRECT - Map String → Enum in repository
impl From<LoopRow> for Loop {
    fn from(row: LoopRow) -> Self {
        Self {
            status: LoopStatus::from_str(&row.status).unwrap_or(LoopStatus::Error),
            // ...
        }
    }
}
```

**5. NULL vs Empty String**

```sql
-- ❌ WRONG - Ambiguity
description TEXT  -- Can be NULL or "" ?

-- ✅ CORRECT - Be explicit
description TEXT NOT NULL DEFAULT ''  -- Never NULL, can be ""
-- OR
description TEXT  -- Can be NULL, but not ""
```

## SQLite Pragmas

The following pragmas are configured automatically in `ralph-repositories/src/database.rs`:

```rust
.pragma("foreign_keys", "1")           // FK constraints enabled
.pragma("journal_mode", "WAL")         // Write-Ahead Logging (better concurrency)
.pragma("synchronous", "NORMAL")       # Balance safety/performance
.pragma("cache_size", "-65536")        # 64MB cache
.pragma("auto_vacuum", "INCREMENTAL")  # Automatic VACUUM
```

**Why they matter:**
- `foreign_keys=ON`: Without this, FK constraints are ignored!
- `journal_mode=WAL`: Allows simultaneous reads and writes
- `synchronous=NORMAL`: Faster than FULL, still safe
- `cache_size=64MB`: Reduces disk I/O
- `auto_vacuum=INCREMENTAL`: Frees space automatically

## Downlinks (Additional Context)

**To understand better:**
- `/AGENTS.md` - General project architecture
- `/ralph-repositories/AGENTS.md` - How migrations are executed and used
- `/ralph-models/AGENTS.md` - Models that correspond to the schema tables
- `/ralph-services/AGENTS.md` - Business logic that depends on the schema

**Related files:**
- `ralph-repositories/src/database.rs` - Migration execution
- `ralph-server/src/main.rs` - Database initialization

---

**Last updated:** 2026-01-18
**Version:** 1.0
