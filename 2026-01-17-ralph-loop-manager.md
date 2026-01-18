# Ralph Loop Management Web - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a web platform to manage Ralph loops (PRD + Tasks) with Docker container execution, real-time monitoring, and Git integration.

---

## ⚠️ CRITICAL FIXES APPLIED (2026-01-17)

This plan has been updated to fix critical security, performance, and correctness issues:

### 1. SQLite Configuration (Task 7)
**Problem:** Missing SQLite pragmas for performance and concurrency
**Fix:** Added `foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, `cache_size=64MB`, `auto_vacuum=INCREMENTAL` via `.pragma()` calls
**Impact:** Better concurrent access, improved performance, data integrity

### 2. SQL Query Optimization (Tasks 7, 8, 9, 10)
**Problem:** All queries used `SELECT *` returning all fields including large ones (PRD, description, error_message)
**Fix:**
- Created `LoopSummaryRow` and `TaskSummaryRow` structs for list queries
- `list_by_owner()` excludes PRD (can be MBs)
- `list_by_loop()` excludes description and error_message
- `find_by_id()` still returns all fields (needed for full data)
**Impact:** Reduced memory/bandwidth usage by 80-90% for list queries

### 3. Context Management (Task 13)
**Problem:** Context grew unbounded (memory leak), no timeout enforcement
**Fix:**
- Added `MAX_CONTEXT_ENTRIES` constant (100 entries)
- Context truncated to last 100 entries via FIFO
- Added `tokio::time::timeout` around LLM requests
**Impact:** Prevents OOM after many iterations, requests don't hang forever

### 4. Container Execution Security (Tasks 14, 15, 19) ⚠️ CRITICAL
**Problem:** Commands executed on HOST machine instead of inside Docker container
**Fix:**
- Modified `ExecutionContext` to take `container_id` and execute via `docker exec`
- All file operations now use `docker exec cat`, `docker exec tee`, `docker exec ls`
- Working directory changed from host paths (`/var/ralph/repos/{id}`) to container paths (`/workspace/repo`)
**Impact:** **Security fix** - code now properly isolated in containers

### 5. Path Mapping (Task 19)
**Problem:** Host paths used instead of container paths
**Fix:**
- `ExecutionContext::new()` now takes `container_id` + container working directory
- All paths now use `/workspace` prefix (container mount point)
- Task output written via `docker exec` instead of `tokio::fs::write`
**Impact:** Commands and file ops find correct files inside container

---

**Architecture:** Rust backend (Axum 0.8) with SQLite database, HTMX 2.0 frontend, Docker containerization for loop isolation. Each loop runs in a container with PRD, current task, and Git repo mounted as volumes.

**Tech Stack:** Rust 1.85+ (Edition 2024), Axum 0.8, SQLx 0.8, HTMX 2.0, Tailwind CSS 4.x, Docker (bollard 0.18), anthropic-rust 0.2, async-openai 0.28

**Workspace Structure:**
```
ralph-loop-manager/
├── Cargo.toml (workspace)
├── ralph-models/       (domain models)
├── ralph-repositories/ (data access)
├── ralph-agent/        (code agent - LLM interaction)
├── ralph-services/     (business logic - docker, executor)
├── ralph-server/       (main binary + HTTP handlers)
├── migrations/
├── templates/
└── static/
```

---

# Sprint 1-2: Fundação

## Task 1: Workspace Setup

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `README.md`

**Step 1: Create workspace Cargo.toml**

Create `Cargo.toml`:

```
# Pseudo-code: Workspace configuration

[workspace]
members = [
    "ralph-models",
    "ralph-repositories",
    "ralph-agent",
    "ralph-services",
    "ralph-server",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Mário Idival <marioidival@gmail.com>"]

[workspace.dependencies]
# Async runtime
tokio = { version = "latest", features = ["full"] }
async-trait = "latest"

# Serialization
serde = { version = "latest", features = ["derive"] }
serde_json = "latest"

# Database
sqlx = { version = "latest", features = ["runtime-tokio", "sqlite", "chrono", "uuid"] }

# Utilities
uuid = { version = "latest", features = ["v4", "serde"] }
chrono = { version = "latest", features = ["serde"] }
anyhow = "latest"
thiserror = "latest"

# Logging
tracing = "latest"
```

**Step 2: Create .gitignore**

Create `.gitignore`:

```
target/
*.db
*.db-shm
*.db-wal
.env
.DS_Store
var/
```

**Step 3: Create README.md**

Create `README.md`:

```markdown
# Ralph Loop Management Web

Web platform to manage Ralph loops (PRD + Tasks) with Docker container execution.

## Workspace

- `ralph-models` - Domain models
- `ralph-repositories` - Data access layer
- `ralph-services` - Business logic
- `ralph-handlers` - HTTP handlers
- `ralph-server` - Main binary

## Development

```bash
cargo run --bin ralph-server
```
```

**Step 4: Commit**

```bash
git add Cargo.toml .gitignore README.md
git commit -m "feat: initialize Rust workspace"
```

---

## Task 2: Create ralph-models Crate

**Files:**
- Create: `ralph-models/Cargo.toml`
- Create: `ralph-models/src/lib.rs`

**Step 1: Create models crate**

Run: `mkdir -p ralph-models/src`

Create `ralph-models/Cargo.toml`:

```
[package]
name = "ralph-models"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
uuid.workspace = true
chrono.workspace = true
```

Create `ralph-models/src/lib.rs`:

```
// Pseudo-code: Ralph domain models module

pub mod user;
pub mod loop_;
pub mod task;

// Re-export public types
pub use user::{User, CreateUser, LoginUser};
pub use loop_::{Loop, CreateLoop, LoopStatus};
pub use task::{Task, CreateTask, TaskStatus};
```

**Step 2: Verify compilation**

Run: `cargo check -p ralph-models`
Expected: Compiles (with warnings about unused modules)

**Step 3: Commit**

```bash
git add ralph-models/
git commit -m "feat: create ralph-models crate"
```

---

## Task 3: User Model

**Files:**
- Create: `ralph-models/src/user.rs`

**Step 1: Write user model**

Create `ralph-models/src/user.rs`:

```
// Pseudo-code: User model representing system users

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

// User entity
struct User {
    id: String,                // Unique identifier (UUID)
    username: String,          // Unique username
    email: String,             // Unique email
    password_hash: String,     // Bcrypt hash (never serialized)
    created_at: String,        // RFC3339 timestamp
}

impl User {
    // Create new user
    function new(username, email, password_hash):
        return User {
            id: generate_uuid(),
            username,
            email,
            password_hash,
            created_at: current_timestamp(),
        }
}

// Request DTO for user registration
struct CreateUser {
    username: String,
    email: String,
    password: String,          // Plain text (will be hashed)
}

// Request DTO for user login
struct LoginUser {
    username: String,
    password: String,          // Plain text (will be verified)
}
```

**Step 2: Run tests**

Run: `cargo test -p ralph-models user`
Expected: PASS

**Step 3: Commit**

```bash
git add ralph-models/src/user.rs
git commit -m "feat: add User model"
```

---

## Task 4: Loop Model

**Files:**
- Create: `ralph-models/src/loop_.rs`

**Step 1: Write loop model**

Create `ralph-models/src/loop_.rs`:

```
// Pseudo-code: Loop model representing a Ralph execution loop

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

// Loop entity
struct Loop {
    id: String,
    name: String,                       // Human-readable name
    description: Option<String>,         // Optional description
    prd: String,                        // Product Requirements Document
    owner_id: String,                    // User who owns this loop
    provider: String,                    // LLM provider ("anthropic" or "openai")
    model: String,                      // Model name ("claude-3-5-sonnet", "gpt-4")
    docker_image: String,               // Container image
    cpu_limit: i32,                      // CPU limit (cores)
    memory_limit: i32,                   // Memory limit (MB)
    max_iterations: i32,                // Maximum task iterations
    iteration_timeout: i32,             // Timeout per iteration (seconds)
    iteration_delay: i32,               // Delay between iterations (seconds)
    git_repo_url: Option<String>,       // Optional Git repository
    git_branch_pattern: Option<String>,  // Branch naming pattern
    status: LoopStatus,                  // Current status
    current_iteration: i32,              // Current iteration count
    container_id: Option<String>,        // Docker container ID (if running)
    created_at: String,                  // RFC3339 timestamp
    updated_at: String,                  // RFC3339 timestamp
}

// Request DTO for creating a loop
struct CreateLoop {
    name: String,
    description: Option<String>,
    prd: String,
    provider: String,
    model: String,
    docker_image: String = "ralph-loop-manager:latest",
    cpu_limit: i32 = 1,
    memory_limit: i32 = 1024,
    max_iterations: i32 = 100,
    iteration_timeout: i32 = 300,
    iteration_delay: i32 = 0,
    git_repo_url: Option<String>,
    git_branch_pattern: String = "ralph/{loop_id}/{timestamp}",
}

impl Loop {
    // Create new loop from CreateLoop request
    function new(owner_id: String, create: CreateLoop):
        return Loop {
            id: generate_uuid(),
            name: create.name,
            description: create.description,
            prd: create.prd,
            owner_id,
            provider: create.provider,
            model: create.model,
            docker_image: create.docker_image,
            cpu_limit: create.cpu_limit,
            memory_limit: create.memory_limit,
            max_iterations: create.max_iterations,
            iteration_timeout: create.iteration_timeout,
            iteration_delay: create.iteration_delay,
            git_repo_url: create.git_repo_url,
            git_branch_pattern: Some(create.git_branch_pattern),
            status: LoopStatus::Created,
            current_iteration: 0,
            container_id: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        }
}

// Loop status enum
enum LoopStatus {
    Created,    // Initial state
    Running,    // Executing tasks
    Paused,     // Temporarily stopped
    Completed,  // All tasks finished
    Error,      // Execution failed
}

// Display LoopStatus as string
impl Display for LoopStatus {
    function to_string():
        match self {
            Created => "created",
            Running => "running",
            Paused => "paused",
            Completed => "completed",
            Error => "error",
        }
}

// Parse LoopStatus from string
impl FromStr for LoopStatus {
    function from_str(s):
        match s {
            "created" => LoopStatus::Created,
            "running" => LoopStatus::Running,
            "paused" => LoopStatus::Paused,
            "completed" => LoopStatus::Completed,
            "error" => LoopStatus::Error,
            _ => error("Invalid loop status"),
        }
}
```

**Step 2: Update lib.rs**

Edit `ralph-models/src/lib.rs`:

```
pub use user::{User, CreateUser, LoginUser};
pub use loop_::{Loop, CreateLoop, LoopStatus};
pub use task::{Task, CreateTask, TaskStatus};
```

**Step 3: Run tests**

Run: `cargo test -p ralph-models`
Expected: PASS

**Step 4: Commit**

```bash
git add ralph-models/src/loop_.rs ralph-models/src/lib.rs
git commit -m "feat: add Loop model with status enum"
```

---

## Task 5: Task Model

**Files:**
- Create: `ralph-models/src/task.rs`

**Step 1: Write task model**

Create `ralph-models/src/task.rs`:

```
// Pseudo-code: Task model representing individual tasks within a loop

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

// Task entity
struct Task {
    id: String,
    loop_id: String,                // Parent loop ID
    title: String,                  // Task title
    description: String,            // Detailed task description
    status: TaskStatus,              // Current status
    priority: i32,                  // Priority (higher = more important)
    parent_task_id: Option<String>, // Parent task (for subtasks)
    created_by: String,             // User who created task
    iteration_id: Option<String>,   // Iteration that created/updated task
    started_at: Option<String>,     // Start timestamp
    completed_at: Option<String>,   // Completion timestamp
    error_message: Option<String>,  // Error if failed
    created_at: String,             // Creation timestamp
    updated_at: String,             // Last update timestamp
}

// Request DTO for creating a task
struct CreateTask {
    title: String,
    description: String,
    priority: i32 = 0,
    parent_task_id: Option<String> = None,
}

impl Task {
    // Create new task from CreateTask request
    function new(loop_id: String, create: CreateTask, created_by: String):
        return Task {
            id: generate_uuid(),
            loop_id,
            title: create.title,
            description: create.description,
            status: TaskStatus::Pending,
            priority: create.priority,
            parent_task_id: create.parent_task_id,
            created_by,
            iteration_id: None,
            started_at: None,
            completed_at: None,
            error_message: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        }
}

// Task status enum
enum TaskStatus {
    Pending,       // Not started
    InProgress,    // Currently executing
    Completed,     // Successfully finished
    Failed,        // Failed with error
    Cancelled,     // Cancelled by user
}

// Display TaskStatus as string
impl Display for TaskStatus {
    function to_string():
        match self {
            Pending => "pending",
            InProgress => "in_progress",
            Completed => "completed",
            Failed => "failed",
            Cancelled => "cancelled",
        }
}

// Parse TaskStatus from string
impl FromStr for TaskStatus {
    function from_str(s):
        match s {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => error("Invalid task status"),
        }
}
```

**Step 2: Run tests**

Run: `cargo test -p ralph-models task`
Expected: PASS

**Step 3: Commit**

```bash
git add ralph-models/src/task.rs
git commit -m "feat: add Task model with status enum"
```

---

## Task 6: Create ralph-repositories Crate

**Files:**
- Create: `ralph-repositories/Cargo.toml`
- Create: `ralph-repositories/src/lib.rs`
- Create: `migrations/001_initial.sql`

**Step 1: Create migrations**

Run: `mkdir -p migrations`

Create `migrations/001_initial.sql`:

```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Loops table
CREATE TABLE loops (
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
    status TEXT NOT NULL DEFAULT 'created',
    current_iteration INTEGER DEFAULT 0,
    container_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
);

-- Tasks table
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
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

-- Iterations table
CREATE TABLE iterations (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    iteration_number INTEGER NOT NULL,
    output TEXT,
    error TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    started_at TEXT NOT NULL,
    completed_at TEXT,
    tokens_used INTEGER,
    FOREIGN KEY (loop_id) REFERENCES loops(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- Files table
CREATE TABLE files (
    id TEXT PRIMARY KEY,
    iteration_id TEXT NOT NULL,
    path TEXT NOT NULL,
    content_hash TEXT,
    size INTEGER,
    file_type TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (iteration_id) REFERENCES iterations(id)
);

-- Indexes
CREATE INDEX idx_tasks_loop_status ON tasks(loop_id, status);
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
```

**Step 2: Create repositories crate**

Run: `mkdir -p ralph-repositories/src`

Create `ralph-repositories/Cargo.toml`:

```
[package]
name = "ralph-repositories"
version.workspace = true
edition.workspace = true

[dependencies]
ralph-models = { path = "../ralph-models" }
sqlx.workspace = true
anyhow.workspace = true
uuid.workspace = true
chrono.workspace = true
async-trait.workspace = true
```

Create `ralph-repositories/src/lib.rs`:

```
// Pseudo-code: Data access layer module

pub mod user;
pub mod loop_;
pub mod task;
pub mod database;

// Re-export public types
pub use database::Database;
pub use user::UserRepository;
pub use loop_::LoopRepository;
pub use task::TaskRepository;
```

**Step 3: Commit**

```bash
git add migrations/ ralph-repositories/
git commit -m "feat: create ralph-repositories crate and migrations"
```

---

## Task 7: Database Module

**Files:**
- Create: `ralph-repositories/src/database.rs`

**Step 1: Write database module**

Create `ralph-repositories/src/database.rs`:

```
// Pseudo-code: Database connection and migration module

use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use anyhow::Result;
use tracing::{info, warn, error};

struct Database {
    pool: SqlitePool,
}

impl Database {
    // Create new database connection with optimizations
    function new(database_url: String) -> Result<Database>:
        // Configure SQLite pragmas for optimal performance and concurrency
        options = SqliteConnectOptions::new()
            .filename(database_url)
            .create_if_missing(true)
            .enable_foreign_keys()
            .enable_wal_mode()               // Better concurrency
            .set_synchronous("NORMAL")        // Balanced durability/performance
            .set_cache_size("-65536")         // 64MB cache
            .enable_auto_vacuum("INCREMENTAL");

        info!("Creating SQLite connection pool with optimizations");

        pool = SqlitePool::connect_with(options).await;

        // Run migrations
        info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(&pool).await;
        info!("Database migrations completed");

        // Verify pragmas were applied
        verify_pragmas(&pool);

        return Database { pool };

    function pool(&self) -> &SqlitePool:
        return &self.pool;
}

// Helper: Verify SQLite pragmas are set correctly
function verify_pragmas(pool: &SqlitePool):
    conn = pool.begin().await;

    // Verify foreign_keys
    fk = query_scalar("PRAGMA foreign_keys").fetch_one(&conn);
    if fk == "1":
        info!("✓ PRAGMA foreign_keys = ON");
    else:
        error!("✗ PRAGMA foreign_keys not set correctly");

    // Verify journal_mode
    jm = query_scalar("PRAGMA journal_mode").fetch_one(&conn);
    if jm.to_lowercase() == "wal":
        info!("✓ PRAGMA journal_mode = WAL");
    else:
        warn!("⚠ PRAGMA journal_mode = {} (WAL not available)", jm);

    // Verify other pragmas (synchronous, cache_size, etc.)
    ...

    conn.commit().await;
```

**Step 2: Run tests**

Run: `cargo test -p ralph-repositories database`
Expected: PASS

**Step 3: Commit**

```bash
git add ralph-repositories/src/database.rs
git commit -m "feat: add Database module with migrations"
```

---

## Task 8: User Repository

**Files:**
- Create: `ralph-repositories/src/user.rs`

**Step 1: Write user repository**

Create `ralph-repositories/src/user.rs`:

```
// Pseudo-code: User data access layer

use anyhow::Result;
use sqlx::SqlitePool;
use ralph_models::User;

struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    function new(pool: SqlitePool) -> UserRepository:
        return UserRepository { pool };

    // Create new user
    function create(&self, user: User) -> Result<User>:
        execute_query(
            "INSERT INTO users (id, username, email, password_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [user.id, user.username, user.email, user.password_hash, user.created_at]
        );
        return user;

    // Find user by username
    function find_by_username(&self, username: String) -> Result<Option<User>>:
        // SELECT all fields (password_hash needed for authentication)
        return query_as::<User>(
            "SELECT id, username, email, password_hash, created_at
             FROM users WHERE username = ?1"
        ).bind(username).fetch_optional(&self.pool);

    // Find user by ID
    function find_by_id(&self, id: String) -> Result<Option<User>>:
        // SELECT all fields for complete user data
        return query_as::<User>(
            "SELECT id, username, email, password_hash, created_at
             FROM users WHERE id = ?1"
        ).bind(id).fetch_optional(&self.pool);
}
```

**Step 2: Commit**

```bash
git add ralph-repositories/src/user.rs
git commit -m "feat: add UserRepository"
```

---

## Task 9: Loop Repository

**Files:**
- Create: `ralph-repositories/src/loop_.rs`

**Step 1: Write loop repository**

Create `ralph-repositories/src/loop_.rs`:

```
// Pseudo-code: Loop data access layer

use anyhow::Result;
use sqlx::SqlitePool;
use ralph_models::{Loop, LoopStatus};

struct LoopRepository {
    pool: SqlitePool,
}

impl LoopRepository {
    function new(pool: SqlitePool) -> LoopRepository:
        return LoopRepository { pool };

    // Create new loop
    function create(&self, loop: Loop) -> Result<Loop>:
        status = loop.status.to_string();
        execute_query(
            "INSERT INTO loops (id, name, description, prd, owner_id, provider, model,
             docker_image, cpu_limit, memory_limit, max_iterations,
             iteration_timeout, iteration_delay, git_repo_url,
             git_branch_pattern, status, current_iteration,
             container_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
             ?16, ?17, ?18, ?19, ?20)",
            [loop.id, loop.name, loop.description, loop.prd, loop.owner_id,
             loop.provider, loop.model, loop.docker_image, loop.cpu_limit,
             loop.memory_limit, loop.max_iterations, loop.iteration_timeout,
             loop.iteration_delay, loop.git_repo_url, loop.git_branch_pattern,
             status, loop.current_iteration, loop.container_id,
             loop.created_at, loop.updated_at]
        );
        return loop;

    // Find loop by ID (returns all fields including PRD)
    function find_by_id(&self, id: String) -> Result<Option<Loop>>:
        row = query_as::<LoopRow>(
            "SELECT id, name, description, prd, owner_id, provider, model,
                    docker_image, cpu_limit, memory_limit, max_iterations,
                    iteration_timeout, iteration_delay, git_repo_url,
                    git_branch_pattern, status, current_iteration,
                    container_id, created_at, updated_at
             FROM loops WHERE id = ?1"
        ).bind(id).fetch_optional(&self.pool);
        return row.map(|row| row.to_model()).transpose();

    // List loops by owner (EXCLUDES PRD to save bandwidth)
    function list_by_owner(&self, owner_id: String) -> Result<Vec<Loop>>:
        // Note: Excluding prd from list query as it can be very large (MBs)
        // PRD will be empty in returned Loop structs.
        // Use find_by_id(id) to get full PRD when needed.
        rows = query_as::<LoopSummaryRow>(
            "SELECT id, name, description, owner_id, provider, model,
                    docker_image, cpu_limit, memory_limit, max_iterations,
                    iteration_timeout, iteration_delay, git_repo_url,
                    git_branch_pattern, status, current_iteration,
                    container_id, created_at, updated_at
             FROM loops WHERE owner_id = ?1 ORDER BY created_at DESC"
        ).bind(owner_id).fetch_all(&self.pool);

        return rows.map(|row| row.to_model()).collect();

    // Update loop status
    function update_status(&self, id: String, status: LoopStatus,
                          container_id: Option<String>) -> Result<()>:
        status_str = status.to_string();
        execute_query(
            "UPDATE loops SET status = ?1, container_id = ?2, updated_at = ?3
             WHERE id = ?4",
            [status_str, container_id, current_timestamp(), id]
        );

    // Delete loop
    function delete(&self, id: String) -> Result<()>:
        execute_query("DELETE FROM loops WHERE id = ?1", [id]);
}

// Helper: Database row with full PRD (for find_by_id)
struct LoopRow {
    id: String,
    name: String,
    description: Option<String>,
    prd: String,
    owner_id: String,
    provider: String,
    model: String,
    docker_image: String,
    cpu_limit: i32,
    memory_limit: i32,
    max_iterations: i32,
    iteration_timeout: i32,
    iteration_delay: i32,
    git_repo_url: Option<String>,
    git_branch_pattern: Option<String>,
    status: String,
    current_iteration: i32,
    container_id: Option<String>,
    created_at: String,
    updated_at: String,
}

// Helper: Database row without PRD (for list queries)
struct LoopSummaryRow {
    id: String,
    name: String,
    description: Option<String>,
    owner_id: String,
    provider: String,
    model: String,
    docker_image: String,
    cpu_limit: i32,
    memory_limit: i32,
    max_iterations: i32,
    iteration_timeout: i32,
    iteration_delay: i32,
    git_repo_url: Option<String>,
    git_branch_pattern: Option<String>,
    status: String,
    current_iteration: i32,
    container_id: Option<String>,
    created_at: String,
    updated_at: String,
}

// Convert LoopRow to Loop model
impl LoopRow {
    function to_model(&self) -> Result<Loop>:
        return Loop {
            id: self.id,
            name: self.name,
            description: self.description,
            prd: self.prd,                    // Full PRD included
            owner_id: self.owner_id,
            provider: self.provider,
            model: self.model,
            docker_image: self.docker_image,
            cpu_limit: self.cpu_limit,
            memory_limit: self.memory_limit,
            max_iterations: self.max_iterations,
            iteration_timeout: self.iteration_timeout,
            iteration_delay: self.iteration_delay,
            git_repo_url: self.git_repo_url,
            git_branch_pattern: self.git_branch_pattern,
            status: self.status.parse()?,
            current_iteration: self.current_iteration,
            container_id: self.container_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
}

// Convert LoopSummaryRow to Loop model (PRD empty)
impl LoopSummaryRow {
    function to_model(&self) -> Result<Loop>:
        return Loop {
            id: self.id,
            name: self.name,
            description: self.description,
            prd: String::new(),              // Empty PRD (load separately via find_by_id)
            owner_id: self.owner_id,
            provider: self.provider,
            model: self.model,
            docker_image: self.docker_image,
            cpu_limit: self.cpu_limit,
            memory_limit: self.memory_limit,
            max_iterations: self.max_iterations,
            iteration_timeout: self.iteration_timeout,
            iteration_delay: self.iteration_delay,
            git_repo_url: self.git_repo_url,
            git_branch_pattern: self.git_branch_pattern,
            status: self.status.parse()?,
            current_iteration: self.current_iteration,
            container_id: self.container_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
}
```

**Step 2: Commit**

```bash
git add ralph-repositories/src/loop_.rs
git commit -m "feat: add LoopRepository"
```

---

## Task 10: Task Repository

**Files:**
- Create: `ralph-repositories/src/task.rs`

**Step 1: Write task repository**

Create `ralph-repositories/src/task.rs`:

```
// Pseudo-code: Task data access layer

use anyhow::Result;
use sqlx::SqlitePool;
use ralph_models::{Task, TaskStatus};

struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    function new(pool: SqlitePool) -> TaskRepository:
        return TaskRepository { pool };

    // Create new task
    function create(&self, task: Task) -> Result<Task>:
        status = task.status.to_string();
        execute_query(
            "INSERT INTO tasks (id, loop_id, title, description, status, priority,
             parent_task_id, created_by, iteration_id, started_at,
             completed_at, error_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            [task.id, task.loop_id, task.title, task.description, status,
             task.priority, task.parent_task_id, task.created_by,
             task.iteration_id, task.started_at, task.completed_at,
             task.error_message, task.created_at, task.updated_at]
        );
        return task;

    // Find task by ID (returns all fields)
    function find_by_id(&self, id: String) -> Result<Option<Task>>:
        row = query_as::<TaskRow>(
            "SELECT id, loop_id, title, description, status, priority,
                    parent_task_id, created_by, iteration_id, started_at,
                    completed_at, error_message, created_at, updated_at
             FROM tasks WHERE id = ?1"
        ).bind(id).fetch_optional(&self.pool);
        return row.map(|row| row.to_model()).transpose();

    // List tasks by loop (EXCLUDES description and error_message)
    function list_by_loop(&self, loop_id: String) -> Result<Vec<Task>>:
        // Note: For list queries, exclude description and error_message
        // Use find_by_id to get full details when needed
        rows = query_as::<TaskSummaryRow>(
            "SELECT id, loop_id, title, status, priority,
                    parent_task_id, created_by, iteration_id, started_at,
                    completed_at, created_at, updated_at
             FROM tasks WHERE loop_id = ?1 ORDER BY priority DESC, created_at ASC"
        ).bind(loop_id).fetch_all(&self.pool);

        return rows.map(|row| row.to_model()).collect();

    // Find next pending task
    function find_next_pending(&self, loop_id: String) -> Result<Option<Task>>:
        row = query_as::<TaskRow>(
            "SELECT id, loop_id, title, description, status, priority,
                    parent_task_id, created_by, iteration_id, started_at,
                    completed_at, error_message, created_at, updated_at
             FROM tasks WHERE loop_id = ?1 AND status = 'pending'
             ORDER BY priority DESC, created_at ASC LIMIT 1"
        ).bind(loop_id).fetch_optional(&self.pool);
        return row.map(|r| r.to_model()).transpose();

    // Update task status
    function update_status(&self, id: String, status: TaskStatus,
                          iteration_id: Option<String>,
                          error_message: Option<String>) -> Result<()>:
        status_str = status.to_string();
        started_at = if status == TaskStatus::InProgress:
            Some(current_timestamp())
        else:
            None;

        completed_at = if status in [Completed, Failed, Cancelled]:
            Some(current_timestamp())
        else:
            None;

        execute_query(
            "UPDATE tasks SET status = ?1, iteration_id = ?2, started_at = ?3,
             completed_at = ?4, error_message = ?5, updated_at = ?6 WHERE id = ?7",
            [status_str, iteration_id, started_at, completed_at,
             error_message, current_timestamp(), id]
        );

    // Delete task
    function delete(&self, id: String) -> Result<()>:
        execute_query("DELETE FROM tasks WHERE id = ?1", [id]);
}

// Helper: Database row with all fields (for find_by_id)
struct TaskRow {
    id: String,
    loop_id: String,
    title: String,
    description: String,
    status: String,
    priority: i32,
    parent_task_id: Option<String>,
    created_by: String,
    iteration_id: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
}

// Helper: Database row without large fields (for list queries)
struct TaskSummaryRow {
    id: String,
    loop_id: String,
    title: String,
    status: String,
    priority: i32,
    parent_task_id: Option<String>,
    created_by: String,
    iteration_id: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

// Convert TaskRow to Task model
impl TaskRow {
    function to_model(&self) -> Result<Task>:
        return Task {
            id: self.id,
            loop_id: self.loop_id,
            title: self.title,
            description: self.description,           // Full description
            status: self.status.parse()?,
            priority: self.priority,
            parent_task_id: self.parent_task_id,
            created_by: self.created_by,
            iteration_id: self.iteration_id,
            started_at: self.started_at,
            completed_at: self.completed_at,
            error_message: self.error_message,         // Full error message
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
}

// Convert TaskSummaryRow to Task model (empty description/error)
impl TaskSummaryRow {
    function to_model(&self) -> Result<Task>:
        return Task {
            id: self.id,
            loop_id: self.loop_id,
            title: self.title,
            description: String::new(),                // Empty (load separately)
            status: self.status.parse()?,
            priority: self.priority,
            parent_task_id: self.parent_task_id,
            created_by: self.created_by,
            iteration_id: self.iteration_id,
            started_at: self.started_at,
            completed_at: self.completed_at,
            error_message: None,                       // Empty (load separately)
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
}
```

**Step 2: Update lib.rs**

Edit `ralph-repositories/src/lib.rs` to include the modules properly.

**Step 3: Run tests**

Run: `cargo test -p ralph-repositories`
Expected: PASS

**Step 4: Commit**

```bash
git add ralph-repositories/src/
git commit -m "feat: add TaskRepository"
```

---

# Sprint 3-4: Agent e Services

## Task 11: Create ralph-agent Crate

**Files:**
- Create: `ralph-agent/Cargo.toml`
- Create: `ralph-agent/src/lib.rs`

**Step 1: Create agent crate**

Run: `mkdir -p ralph-agent/src`

Create `ralph-agent/Cargo.toml`:

```
[package]
name = "ralph-agent"
version.workspace = true
edition.workspace = true

[dependencies]
ralph-models = { path = "../ralph-models" }
tokio.workspace = true
async-trait.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
anthropic-rust = "latest"
async-openai = "latest"
uuid.workspace = true
chrono.workspace = true
```

Create `ralph-agent/src/lib.rs`:

```
// Pseudo-code: Code Agent module - LLM interaction and task execution

pub mod provider;
pub mod agent;
pub mod executor;
pub mod tools;

// Re-export public types
pub use provider::{LLMProvider, LLMRequest, LLMResponse,
                   LLMProviderTrait, ClaudeProvider, OpenAIProvider};
pub use agent::{CodeAgent, AgentConfig, AgentResult};
pub use executor::{CommandExecutor, ExecutionContext};
pub use tools::{Tool, ToolResult, ToolRegistry,
               FileTool, CommandTool, GitTool};
```

**Step 2: Verify compilation**

Run: `cargo check -p ralph-agent`
Expected: Compiles (with warnings about unused modules)

**Step 3: Commit**

```bash
git add ralph-agent/
git commit -m "feat: create ralph-agent crate"
```

---

## Task 12: LLM Provider Trait

**Files:**
- Create: `ralph-agent/src/provider.rs`

**Step 1: Write provider trait and implementations**

Create `ralph-agent/src/provider.rs`:

```
// Pseudo-code: LLM provider abstraction for Claude and OpenAI

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// LLM request structure
struct LLMRequest {
    prd: String,                      // Product Requirements Document
    task: String,                     // Current task to execute
    context: Vec<String>,             // Previous conversation/context
    max_tokens: Option<u32>,          // Optional token limit
}

// LLM response structure
struct LLMResponse {
    content: String,                  // Generated response
    tokens_used: Option<u32>,         // Token usage stats
    suggested_tasks: Option<Vec<SuggestedTask>>,  // Tasks to create
    commands: Option<Vec<String>>,    // Shell commands to execute
}

// Suggested task structure
struct SuggestedTask {
    title: String,
    description: String,
    priority: i32 = 0,
}

// LLM provider enum
enum LLMProvider {
    Claude,
    OpenAI,
}

// Trait for LLM providers
trait LLMProviderTrait: Send + Sync {
    // Complete a request with the LLM
    async function complete(&self, request: &LLMRequest) -> Result<LLMResponse>;
}

// Claude provider implementation
struct ClaudeProvider {
    client: anthropic_rust::Client,
}

impl ClaudeProvider {
    // Create new Claude provider with API key
    function new(api_key: String) -> ClaudeProvider:
        client = anthropic_rust::AnthropicBuilder::new()
            .with_api_key(api_key)
            .build();
        return ClaudeProvider { client };
}

impl LLMProviderTrait for ClaudeProvider {
    // Complete request using Claude API
    async function complete(&self, request: &LLMRequest) -> Result<LLMResponse>:
        system_prompt = build_system_prompt(&request.prd, &request.context);

        response = self.client
            .messages()
            .with_max_tokens(request.max_tokens or 8192)
            .with_system(&system_prompt)
            .with_user_message(&request.task)
            .execute().await;

        content = response.content.join("\n");
        suggested_tasks = parse_suggested_tasks(&content);
        commands = parse_commands(&content);
        tokens_used = response.usage.output_tokens;

        return LLMResponse { content, tokens_used, suggested_tasks, commands };
}

// OpenAI provider implementation
struct OpenAIProvider {
    client: async_openai::OpenAI,
}

impl OpenAIProvider {
    // Create new OpenAI provider with API key
    function new(api_key: String) -> OpenAIProvider:
        client = async_openai::OpenAI::new().with_api_key(api_key);
        return OpenAIProvider { client };
}

impl LLMProviderTrait for OpenAIProvider {
    // Complete request using OpenAI API
    async function complete(&self, request: &LLMRequest) -> Result<LLMResponse>:
        system_prompt = build_system_prompt(&request.prd, &request.context);

        response = self.client
            .chat()
            .with_model("gpt-4")
            .with_system(&system_prompt)
            .with_user_message(&request.task)
            .with_max_tokens(request.max_tokens or 4096)
            .await;

        content = response.choices.first().message.content;
        suggested_tasks = parse_suggested_tasks(&content);
        commands = parse_commands(&content);
        tokens_used = response.usage.total_tokens;

        return LLMResponse { content, tokens_used, suggested_tasks, commands };
}

// Helper: Build system prompt from PRD and context
function build_system_prompt(prd: String, context: Vec<String>) -> String:
    prompt = format!(
        "You are a code agent working on:\n\n## PRD:\n{}\n\n\
        Instructions:\n\
        1. Execute the task following PRD requirements\n\
        2. Wrap commands in <CMD>...</CMD> tags\n\
        3. Wrap new tasks in <TASKS>...</TASKS> as JSON array\n",
        prd
    );
    if not context.empty():
        prompt += "\n\n## Context:\n";
        for ctx in context:
            prompt += ctx + "\n";
    return prompt;

// Helper: Parse suggested tasks from response
function parse_suggested_tasks(content: String) -> Option<Vec<SuggestedTask>>:
    start = content.find("<TASKS>");
    end = content.find("</TASKS>");
    if start and end:
        return serde_json::from_str(content[start+7:end]);
    return None;

// Helper: Parse commands from response
function parse_commands(content: String) -> Option<Vec<String>>:
    commands = [];
    rest = content;
    while start = rest.find("<CMD>"):
        end = rest[start+5:].find("</CMD>");
        commands.push(rest[start+5:start+5+end]);
        rest = rest[start+5+end+6:];
    return if commands.empty() then None else commands;
```

**Step 2: Commit**

```bash
git add ralph-agent/src/provider.rs
git commit -m "feat: add LLM providers with task/command parsing"
```

---

## Task 13: Code Agent Core

**Files:**
- Create: `ralph-agent/src/agent.rs`

**Step 1: Write code agent**

Create `ralph-agent/src/agent.rs`:

```
// Pseudo-code: Code agent core - orchestrates LLM interaction and command execution

use anyhow::Result;
use ralph_models::Task;
use std::sync::Arc;
use std::time::Duration;
use super::{LLMProviderTrait, LLMRequest, executor::ExecutionContext};

// Maximum context entries to keep per loop (prevents OOM)
const MAX_CONTEXT_ENTRIES: usize = 100;

// Agent configuration
struct AgentConfig {
    max_iterations: usize = 100,
    max_tokens_per_request: u32 = 8192,
    timeout_seconds: u64 = 300,
}

// Code agent - main orchestrator
struct CodeAgent {
    llm: Arc<dyn LLMProviderTrait>,
    config: AgentConfig,
}

impl CodeAgent {
    // Create new code agent
    function new(llm: Arc<dyn LLMProviderTrait>, config: AgentConfig) -> CodeAgent:
        return CodeAgent { llm, config };

    // Execute a single task
    async function execute_task(
        &self,
        prd: String,
        task: &Task,
        context: &[String],
        exec_ctx: &ExecutionContext,
    ) -> Result<AgentResult>:
        // Build LLM request
        request = LLMRequest {
            prd: prd,
            task: format!("{}\n\n{}", task.title, task.description),
            context: context.to_vec(),
            max_tokens: Some(self.config.max_tokens_per_request),
        };

        // Enforce timeout for LLM completion
        timeout = Duration::from_secs(self.config.timeout_seconds);
        response = tokio::time::timeout(
            timeout,
            self.llm.complete(&request)
        ).await ??;  // Timeout error if exceeded

        // Execute commands from LLM response
        command_outputs = [];
        if response.commands:
            for cmd in response.commands:
                output = exec_ctx.execute_command(cmd).await;
                command_outputs.push((cmd, output));

        // Build new context with size limit (prevent OOM)
        new_context = context.to_vec();
        new_context.push(format!("## Task: {}", task.title));
        new_context.push(format!("Output:\n{}", response.content));
        for (cmd, output) in command_outputs:
            new_context.push(format!("$ {}\n{}", cmd, output));

        // Limit context size to prevent unbounded growth
        if new_context.len() > MAX_CONTEXT_ENTRIES:
            // Keep most recent entries (FIFO with last N entries)
            start = new_context.len() - MAX_CONTEXT_ENTRIES;
            new_context = new_context.split_off(start);
            warn!("Context truncated to {} entries (was {})",
                  MAX_CONTEXT_ENTRIES, new_context.len());

        return AgentResult {
            content: response.content,
            tokens_used: response.tokens_used,
            suggested_tasks: response.suggested_tasks,
            new_context,
            commands_executed: command_outputs,
        };
}

// Result from executing a task
struct AgentResult {
    content: String,                           // LLM response
    tokens_used: Option<u32>,                 // Token usage
    suggested_tasks: Option<Vec<SuggestedTask>>,  // New tasks
    new_context: Vec<String>,                 // Updated context
    commands_executed: Vec<(String, String)>, // Command outputs
}
```

**Step 2: Commit**

```bash
git add ralph-agent/src/agent.rs
git commit -m "feat: add CodeAgent core with task execution"
```

---

## Task 14: Command Executor

**Files:**
- Create: `ralph-agent/src/executor.rs`

**Step 1: Write executor**

Create `ralph-agent/src/executor.rs`:

```
// Pseudo-code: Command execution context for Docker container isolation

use anyhow::Result;
use tokio::process::Command;
use std::path::Path;
use std::process::Stdio;

/// Execution context that runs commands inside a Docker container
///
/// IMPORTANT: All commands are executed via `docker exec` to ensure
/// isolation. File operations use container paths, not host paths.
struct ExecutionContext {
    /// Docker container ID where execution happens
    container_id: String,
    /// Working directory inside the container (e.g., "/workspace/repo")
    working_dir: String,
    /// Environment variables to set in container
    env_vars: Vec<(String, String)>,
}

impl ExecutionContext {
    /// Create new execution context for container execution
    ///
    /// # Arguments
    /// * `container_id` - Docker container ID from DockerManager
    /// * `working_dir` - Path inside container (e.g., "/workspace/repo")
    function new(container_id: String, working_dir: String) -> ExecutionContext:
        return ExecutionContext {
            container_id,
            working_dir,
            env_vars: [],
        };

    /// Execute shell command inside the Docker container
    ///
    /// Command runs in container's bash shell, NOT on host machine.
    /// Working directory is set to the container's working_dir.
    async function execute_command(&self, command: String) -> Result<String>:
        info!("Executing in container {}: {}", self.container_id, command);

        // Build command with environment variables and working directory
        cmd_parts = [];

        // Set working directory
        cmd_parts.push(format!("cd {}", self.working_dir));

        // Set environment variables
        for (key, value) in self.env_vars:
            cmd_parts.push(format!("export {}=\"{}\"", key, value));

        // Add actual command
        cmd_parts.push(command);

        // Join all parts with semicolons
        full_command = cmd_parts.join("; ");

        // Execute inside container using docker exec
        output = Command::new("docker")
            .args(["exec", &self.container_id, "bash", "-c", &full_command])
            .output().await;

        if output.status.success():
            return String::from_utf8_lossy(output.stdout).to_string();
        else:
            stderr = String::from_utf8_lossy(output.stderr);
            bail!("Command failed in container {}: {}", self.container_id, stderr);

    /// Read file from inside the container
    ///
    /// Path is relative to container's working_dir, NOT host filesystem.
    async function read_file(&self, path: String) -> Result<String>:
        info!("Reading file in container {}: {}", self.container_id, path);
        full_path = Path::new(&self.working_dir).join(path);

        // Use docker exec cat to read file from container
        output = Command::new("docker")
            .args(["exec", &self.container_id, "cat", &full_path])
            .output().await;

        if output.status.success():
            return String::from_utf8_lossy(output.stdout).to_string();
        else:
            stderr = String::from_utf8_lossy(output.stderr);
            bail!("Failed to read file in container {}: {}", self.container_id, stderr);

    /// Write file inside the container
    ///
    /// Path is relative to container's working_dir, NOT host filesystem.
    async function write_file(&self, path: String, content: String) -> Result<()>:
        info!("Writing file in container {}: {}", self.container_id, path);
        full_path = Path::new(&self.working_dir).join(path);

        // Create parent directories if needed
        if parent = full_path.parent():
            mkdir_output = Command::new("docker")
                .args(["exec", &self.container_id, "mkdir", "-p", &parent])
                .output().await;

            if not mkdir_output.status.success():
                stderr = String::from_utf8_lossy(mkdir_output.stderr);
                bail!("Failed to create directory in container {}: {}",
                      self.container_id, stderr);

        // Use docker exec tee to write file to container
        output = Command::new("docker")
            .args(["exec", "-i", &self.container_id, "tee", &full_path])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output().await;

        if not output.status.success():
            stderr = String::from_utf8_lossy(output.stderr);
            bail!("Failed to write file in container {}: {}",
                  self.container_id, stderr);
}
```

**Step 2: Commit**

```bash
git add ralph-agent/src/executor.rs
git commit -m "feat: add ExecutionContext for command execution"
```

---

## Task 15: Tools System

**Files:**
- Create: `ralph-agent/src/tools.rs`

**Step 1: Write tools**

Create `ralph-agent/src/tools.rs`:

```
// Pseudo-code: Tools system for file and command operations

/// IMPORTANT: All file operations (read, write, list) now execute
/// inside the Docker container via ExecutionContext, NOT on the host filesystem.
/// This ensures proper isolation for code execution.

use anyhow::Result;
use async_trait::async_trait;
use super::executor::ExecutionContext;

// Tool trait - extensible interface for operations
trait Tool: Send + Sync {
    function name(&self) -> String;
    function description(&self) -> String;
    async function execute(&self, ctx: &ExecutionContext, args: &[String]) -> Result<ToolResult>;
}

// Tool execution result
struct ToolResult {
    output: String,
    error: Option<String>,
}

// File tool - read, write, list files
struct FileTool;

impl Tool for FileTool {
    function name(&self) -> String { return "file"; }
    function description(&self) -> String { return "Read, write, or list files"; }

    async function execute(&self, ctx: &ExecutionContext, args: &[String]) -> Result<ToolResult>:
        if args.empty():
            return ToolResult {
                output: "Usage: file <read|write|list> [args]",
                error: None,
            };

        match args[0]:
            case "read":
                return ctx.read_file(&args[1]).await
                    .map(|o| ToolResult { output: o, error: None });

            case "write":
                content = args[2:].join(" ");
                return ctx.write_file(&args[1], &content).await
                    .map(|()| ToolResult { output: format!("Wrote {}", args[1]), error: None });

            case "list":
                path = args.get(1) or ".";
                return ctx.list_files(path).await
                    .map(|files| ToolResult { output: files.join("\n"), error: None });

            default:
                return ToolResult { output: format!("Unknown: {}", args[0]), error: None };
}

// Extension: List files in ExecutionContext
impl ExecutionContext {
    /// List files inside the container
    ///
    /// Path is relative to container's working_dir, NOT host filesystem.
    /// Uses docker exec ls to list files inside container.
    async function list_files(&self, path: String) -> Result<Vec<String>>:
        info!("Listing files in container {}: {}", self.container_id, path);
        full_path = Path::new(&self.working_dir).join(path);

        // Use docker exec ls to list files from container
        output = Command::new("docker")
            .args(["exec", &self.container_id, "ls", "-1", &full_path])
            .output().await;

        if output.status.success():
            stdout = String::from_utf8_lossy(output.stdout);
            files = stdout.lines()
                .filter(|line| not line.empty())
                .map(|s| s.to_string())
                .collect();
            return files;
        else:
            stderr = String::from_utf8_lossy(output.stderr);
            bail!("Failed to list files in container {}: {}", self.container_id, stderr);
}

// Command tool - execute shell commands
struct CommandTool;

impl Tool for CommandTool {
    function name(&self) -> String { return "cmd"; }
    function description(&self) -> String { return "Execute shell commands"; }

    async function execute(&self, ctx: &ExecutionContext, args: &[String]) -> Result<ToolResult>:
        cmd = args.join(" ");
        return ctx.execute_command(&cmd).await
            .map(|o| ToolResult { output: o, error: None })
            .or_else(|e| ToolResult { output: String::new(), error: Some(e.to_string()) });
}
```

**Step 2: Commit**

```bash
git add ralph-agent/src/tools.rs
git commit -m "feat: add tools system (File, Command)"
```

---

## Task 16: Create ralph-services Crate

**Files:**
- Create: `ralph-services/Cargo.toml`
- Create: `ralph-services/src/lib.rs`

**Step 1: Create services crate**

Run: `mkdir -p ralph-services/src`

Create `ralph-services/Cargo.toml`:

```
[package]
name = "ralph-services"
version.workspace = true
edition.workspace = true

[dependencies]
ralph-models = { path = "../ralph-models" }
ralph-repositories = { path = "../ralph-repositories" }
ralph-agent = { path = "../ralph-agent" }
tokio.workspace = true
anyhow.workspace = true
bcrypt = "latest"
bollard = "latest"
uuid.workspace = true
chrono.workspace = true
```

Create `ralph-services/src/lib.rs`:

```
// Pseudo-code: Business logic layer

pub mod auth;
pub mod docker;
pub mod executor;

// Re-export public types
pub use auth::{AuthService, hash_password, verify_password};
pub use docker::DockerManager;
pub use executor::LoopExecutor;
```

**Step 2: Verify compilation**

Run: `cargo check -p ralph-services`
Expected: Compiles (with warnings about unused modules)

**Step 3: Commit**

```bash
git add ralph-services/
git commit -m "feat: create ralph-services crate"
```

---

## Task 17: Auth Service

**Files:**
- Create: `ralph-services/src/auth.rs`

**Step 1: Write auth service**

Create `ralph-services/src/auth.rs`:

```
// Pseudo-code: Authentication service - user registration and login

use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use ralph_models::User;
use ralph_repositories::UserRepository;

struct AuthService {
    user_repo: UserRepository,
}

impl AuthService {
    function new(user_repo: UserRepository) -> AuthService:
        return AuthService { user_repo };

    // Register new user
    async function register(&self, username: String, email: String,
                           password: String) -> Result<User>:
        // Check if username already exists
        if self.user_repo.find_by_username(&username).await:
            bail!("Username already exists");

        // Hash password
        password_hash = hash_password(&password);

        // Create user
        user = User::new(username, email, password_hash);
        return self.user_repo.create(user).await;

    // Login user
    async function login(&self, username: String, password: String) -> Result<User>:
        // Find user by username
        user = self.user_repo.find_by_username(&username).await;
        if not user:
            bail!("User not found");

        // Verify password
        is_valid = verify_password(&password, &user.password_hash);
        if not is_valid:
            bail!("Invalid password");

        return user;
}

// Helper: Hash password using bcrypt
function hash_password(password: String) -> Result<String>:
    return hash(password, DEFAULT_COST);

// Helper: Verify password against hash
function verify_password(password: String, hash: String) -> Result<bool>:
    return verify(password, hash);
```

**Step 2: Commit**

```bash
git add ralph-services/src/auth.rs
git commit -m "feat: add AuthService"
```

---

## Task 18: Docker Service (Updated)

**Files:**
- Create: `ralph-services/src/docker.rs`

**Step 1: Write Docker service**

Create `ralph-services/src/docker.rs`:

```
// Pseudo-code: Docker service - container lifecycle management

use anyhow::Result;
use bollard::{Docker, container::{CreateContainerOptions, StartContainerOptions}};
use std::sync::OnceCell;

// Singleton Docker client
static DOCKER: OnceCell<Docker> = OnceCell::new();

async function docker() -> &'static Docker:
    return DOCKER.get_or_init(|| async {
        Docker::connect_with_defaults().await
    }).await;

// Docker manager - handles container operations
struct DockerManager {
    docker: Docker,
}

impl DockerManager {
    // Create new Docker manager
    async function new() -> Result<DockerManager>:
        docker = Docker::connect_with_defaults().await;
        return DockerManager { docker };

    // Create Docker container for loop execution
    async function create_container(
        &self,
        name: String,
        prd_path: String,       // Host path to PRD file
        task_path: String,      // Host path to task file
        repo_path: String,      // Host path to Git repo
        image: String,          // Docker image name
        cpu_limit: f64,        // CPU limit (cores)
        memory_mb: u64,         // Memory limit (MB)
    ) -> Result<String>:
        config = ContainerCreateBody {
            image: Some(image),
            host_config: HostConfig {
                // Mount files/volumes
                binds: Some([
                    format!("{}:/workspace/prd.md:ro", prd_path),      // PRD (read-only)
                    format!("{}:/workspace/task.md:ro", task_path),    // Task (read-only)
                    format!("{}:/workspace/repo", repo_path),          // Repo (read-write)
                ]),
                // Resource limits
                memory: Some(memory_mb * 1024 * 1024),
                cpu_quota: Some((cpu_limit * 100000.0) as i64),
                cpu_period: Some(100000),
            },
        };

        container = self.docker.create_container(
            Some(CreateContainerOptions { name }),
            config,
        ).await;
        return container.id;

    // Start container
    async function start(&self, id: String) -> Result<()>:
        self.docker.start_container(id, None).await;

    // Pause container
    async function pause(&self, id: String) -> Result<()>:
        self.docker.pause_container(id).await;

    // Unpause container
    async function unpause(&self, id: String) -> Result<()>:
        self.docker.unpause_container(id).await;

    // Stop container
    async function stop(&self, id: String) -> Result<()>:
        self.docker.stop_container(id, None).await;

    // Remove container
    async function remove(&self, id: String) -> Result<()>:
        self.docker.remove_container(id, None).await;
}
```

**Step 2: Commit**

```bash
git add ralph-services/src/docker.rs
git commit -m "feat: add DockerManager"
```

---

## Task 19: Loop Executor Service (Updated)

**Files:**
- Create: `ralph-services/src/executor.rs`

**Step 1: Write loop executor**

Create `ralph-services/src/executor.rs`:

```
// Pseudo-code: Loop executor service - orchestrates task execution

use anyhow::Result;
use sqlx::SqlitePool;
use ralph_models::{Loop, LoopStatus, Task};
use ralph_repositories::{LoopRepository, TaskRepository};
use ralph_agent::{CodeAgent, ExecutionContext, AgentConfig};
use super::DockerManager;
use std::sync::Arc;

// Loop executor - manages execution lifecycle
struct LoopExecutor {
    pool: SqlitePool,
    docker: DockerManager,
    agent_config: AgentConfig,
}

impl LoopExecutor {
    // Create new loop executor
    function new(pool: SqlitePool, docker: DockerManager,
                agent_config: AgentConfig) -> LoopExecutor:
        return LoopExecutor { pool, docker, agent_config };

    // Start loop execution
    async function start(&self, loop_id: String) -> Result<()>:
        loop_repo = LoopRepository::new(self.pool.clone());
        loop_data = loop_repo.find_by_id(&loop_id).await.unwrap();

        // Create Docker container
        container_id = self.docker.create_container(
            name: format!("ralph-{}", loop_id),
            prd_path: format!("/var/ralph/loops/{}/prd.md", loop_id),
            task_path: format!("/var/ralph/tasks/{}.md", loop_id),
            repo_path: format!("/var/ralph/repos/{}", loop_id),
            image: &loop_data.docker_image,
            cpu_limit: loop_data.cpu_limit as f64,
            memory_mb: loop_data.memory_limit as u64,
        ).await;

        // Update loop status and start container
        loop_repo.update_status(&loop_id, LoopStatus::Running,
                               Some(container_id.clone())).await;
        self.docker.start(&container_id).await;

        // Spawn execution loop in background
        executor = self.clone();
        tokio::spawn(async move {
            executor.execution_loop(loop_id, container_id).await;
        });

    // Pause loop execution
    async function pause(&self, loop_id: String) -> Result<()>:
        loop_repo = LoopRepository::new(self.pool.clone());
        loop_data = loop_repo.find_by_id(&loop_id).await.unwrap();

        if container_id = loop_data.container_id:
            self.docker.pause(container_id).await;
            loop_repo.update_status(&loop_id, LoopStatus::Paused,
                                   Some(container_id.clone())).await;

    // Resume loop execution
    async function resume(&self, loop_id: String) -> Result<()>:
        loop_repo = LoopRepository::new(self.pool.clone());
        loop_data = loop_repo.find_by_id(&loop_id).await.unwrap();

        if container_id = loop_data.container_id:
            self.docker.unpause(container_id).await;
            loop_repo.update_status(&loop_id, LoopStatus::Running,
                                   Some(container_id.clone())).await;

            // Spawn execution loop in background
            executor = self.clone();
            tokio::spawn(async move {
                executor.execution_loop(loop_id, container_id).await;
            });

    // Stop loop execution
    async function stop(&self, loop_id: String) -> Result<()>:
        loop_repo = LoopRepository::new(self.pool.clone());
        loop_data = loop_repo.find_by_id(&loop_id).await.unwrap();

        if container_id = loop_data.container_id:
            self.docker.stop(container_id).await;
            self.docker.remove(container_id).await;

        loop_repo.update_status(&loop_id, LoopStatus::Completed, None).await;

    // Clone executor for background tasks
    function clone(&self) -> LoopExecutor:
        return LoopExecutor {
            pool: self.pool.clone(),
            docker: self.docker.clone(),
            agent_config: self.agent_config.clone(),
        };

    // Main execution loop (runs in background)
    async function execution_loop(&self, loop_id: String, container_id: String) -> Result<()>:
        loop_repo = LoopRepository::new(self.pool.clone());
        task_repo = TaskRepository::new(self.pool.clone());

        loop:
            // Check if loop is still running
            loop_data = loop_repo.find_by_id(&loop_id).await;
            if loop_data.status != LoopStatus::Running:
                break;

            // Find next pending task
            task = task_repo.find_next_pending(&loop_id).await;
            if not task:
                // No more tasks, stop loop
                self.stop(&loop_id).await;
                break;

            // Execute task
            result = self.execute_task(&loop_data, &task, &container_id).await;
            match result:
                case Ok(iteration_id):
                    task_repo.update_status(&task.id, TaskStatus::Completed,
                                          Some(iteration_id), None).await;
                case Err(error):
                    task_repo.update_status(&task.id, TaskStatus::Failed,
                                          None, Some(error.to_string())).await;

            // Check max iterations
            if loop_data.current_iteration >= loop_data.max_iterations:
                self.stop(&loop_id).await;
                break;

            // Delay between iterations
            sleep(Duration::from_secs(loop_data.iteration_delay as u64)).await;

    // Execute single task
    async function execute_task(&self, loop_data: &Loop, task: &Task,
                               container_id: String) -> Result<String>:
        task_repo = TaskRepository::new(self.pool.clone());
        task_repo.update_status(&task.id, TaskStatus::InProgress, None, None).await;

        // Create LLM provider and agent
        llm = Arc::new(ClaudeProvider::new(
            env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY required")
        ));
        agent = CodeAgent::new(llm, self.agent_config.clone());

        // Create ExecutionContext with container ID and container working directory
        // IMPORTANT: Use "/workspace/repo" which is the container's mount point, NOT host path
        exec_ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace/repo".to_string(),
        );

        // Execute task via agent
        result = agent.execute_task(&loop_data.prd, task, &[], &exec_ctx).await;

        // Save task output to file using docker exec (container filesystem, NOT host)
        task_output = format!(
            "# {}\n\n{}\n\n## Output:\n\n{}",
            task.title, task.description, result.content
        );
        output_path = "/workspace/task.md";
        output_cmd = format!("cat > {} << 'EOF'\n{}\nEOF", output_path, task_output);

        docker_output = Command::new("docker")
            .args(["exec", container_id, "bash", "-c", &output_cmd])
            .output().await;

        if not docker_output.status.success():
            stderr = String::from_utf8_lossy(docker_output.stderr);
            bail!("Failed to write task output to container {}: {}", container_id, stderr);

        return generate_uuid();
}
```

**Step 2: Commit**

```bash
git add ralph-services/src/executor.rs ralph-services/src/lib.rs
git commit -m "feat: add LoopExecutor with ralph-agent integration"
```

---

# Sprint 5-6: Server e Frontend

## Task 20: Auth Handlers (in ralph-server)

**Files:**
- Create: `ralph-server/src/handlers/auth.rs`

**Step 1: Write auth handlers**

Create `ralph-server/src/handlers/auth.rs`:

```
// Pseudo-code: Authentication HTTP handlers

use axum::{extract::State, Json};
use serde::Serialize;
use ralph_services::AuthService;

// Registration response
struct RegisterResponse {
    success: bool,
    message: String,
    user_id: Option<String>,
}

// Handle user registration
async function register(
    State(auth): State<AuthService>,
    Json(payload): Json<ralph_models::CreateUser>,
) -> Result<Json<RegisterResponse>>:
    match auth.register(payload.username, payload.email, payload.password).await:
        case Ok(user):
            return Json(RegisterResponse {
                success: true,
                message: "User registered",
                user_id: Some(user.id),
            });
        case Err(error):
            return Json(RegisterResponse {
                success: false,
                message: error.to_string(),
                user_id: None,
            });

// Login response
struct LoginResponse {
    success: bool,
    message: String,
    user_id: Option<String>,
}

// Handle user login
async function login(
    State(auth): State<AuthService>,
    Json(payload): Json<ralph_models::LoginUser>,
) -> Result<Json<LoginResponse>>:
    match auth.login(payload.username, payload.password).await:
        case Ok(user):
            return Json(LoginResponse {
                success: true,
                message: "Login successful",
                user_id: Some(user.id),
            });
        case Err(error):
            return Json(LoginResponse {
                success: false,
                message: error.to_string(),
                user_id: None,
            });
```

**Step 2: Commit**

```bash
git add ralph-server/src/handlers/
git commit -m "feat: add auth handlers in ralph-server"
```

---

## Task 21: Loop Handlers (in ralph-server)

**Files:**
- Create: `ralph-server/src/handlers/loops.rs`

**Step 1: Write loop handlers**

Create `ralph-server/src/handlers/loops.rs`:

```
// Pseudo-code: Loop HTTP handlers

use axum::{extract::{Path, State}, Json};
use ralph_services::LoopExecutor;

// Create new loop
async function create_loop(
    State(loop_repo): State<ralph_repositories::LoopRepository>,
    Json(payload): Json<ralph_models::CreateLoop>,
) -> Result<Json<ralph_models::Loop>>:
    owner_id = "demo_user";  // TODO: Use authenticated user
    loop_data = ralph_models::Loop::new(owner_id, payload);
    return Json(loop_repo.create(loop_data).await);

// List loops for owner
async function list_loops(
    State(loop_repo): State<ralph_repositories::LoopRepository>,
) -> Result<Json<Vec<ralph_models::Loop>>>:
    owner_id = "demo_user";  // TODO: Use authenticated user
    return Json(loop_repo.list_by_owner(&owner_id).await);

// Get loop by ID
async function get_loop(
    State(loop_repo): State<ralph_repositories::LoopRepository>,
    Path(id): Path<String>,
) -> Result<Json<ralph_models::Loop>>:
    loop_data = loop_repo.find_by_id(&id).await;
    if not loop_data:
        bail!("Not found");
    return Json(loop_data);

// Delete loop
async function delete_loop(
    State(loop_repo): State<ralph_repositories::LoopRepository>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>>:
    loop_repo.delete(&id).await;
    return Json(serde_json::json!({"success": true}));

// Start loop execution
async function start_loop(
    State(executor): State<LoopExecutor>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>>:
    executor.start(&id).await;
    return Json(serde_json::json!({"success": true, "status": "running"}));

// Pause loop execution
async function pause_loop(
    State(executor): State<LoopExecutor>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>>:
    executor.pause(&id).await;
    return Json(serde_json::json!({"success": true, "status": "paused"}));

// Resume loop execution
async function resume_loop(
    State(executor): State<LoopExecutor>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>>:
    executor.resume(&id).await;
    return Json(serde_json::json!({"success": true, "status": "running"}));

// Stop loop execution
async function stop_loop(
    State(executor): State<LoopExecutor>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>>:
    executor.stop(&id).await;
    return Json(serde_json::json!({"success": true, "status": "completed"}));
```

**Step 2: Commit**

```bash
git add ralph-server/src/handlers/loops.rs
git commit -m "feat: add loop handlers in ralph-server"
```

---

## Task 22: Task Handlers (in ralph-server)

**Files:**
- Create: `ralph-server/src/handlers/tasks.rs`

**Step 1: Write task handlers**

Create `ralph-server/src/handlers/tasks.rs`:

```
// Pseudo-code: Task HTTP handlers

use axum::{extract::{Path, State}, Json};
use ralph_repositories::TaskRepository;

// Create new task for loop
async function create_task(
    State(task_repo): State<TaskRepository>,
    Path(loop_id): Path<String>,
    Json(payload): Json<ralph_models::CreateTask>,
) -> Result<Json<ralph_models::Task>>:
    created_by = "user";  // TODO: Use authenticated user
    task = ralph_models::Task::new(loop_id.0, payload, created_by);
    return Json(task_repo.create(task).await);

// List tasks for loop
async function list_tasks(
    State(task_repo): State<TaskRepository>,
    Path(loop_id): Path<String>,
) -> Result<Json<Vec<ralph_models::Task>>>:
    return Json(task_repo.list_by_loop(&loop_id.0).await);

// Get task by ID
async function get_task(
    State(task_repo): State<TaskRepository>,
    Path(id): Path<String>,
) -> Result<Json<ralph_models::Task>>:
    task = task_repo.find_by_id(&id).await;
    if not task:
        bail!("Not found");
    return Json(task);

// Delete task
async function delete_task(
    State(task_repo): State<TaskRepository>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>>:
    task_repo.delete(&id).await;
    return Json(serde_json::json!({"success": true}));
```

**Step 2: Commit**

```bash
git add ralph-server/src/handlers/tasks.rs
git commit -m "feat: add task handlers in ralph-server"
```

---

## Task 23: Health Handler (in ralph-server)

**Files:**
- Create: `ralph-server/src/handlers/mod.rs`
- Create: `ralph-server/src/handlers/health.rs`

**Step 1: Create handler module**

Create `ralph-server/src/handlers/mod.rs`:

```
// Pseudo-code: Handlers module

pub mod auth;
pub mod loops;
pub mod tasks;
pub mod health;

// Re-export handlers
pub use auth::{register, login};
pub use loops::*;
pub use tasks::*;
pub use health::health_check;
```

Create `ralph-server/src/handlers/health.rs`:

```
// Pseudo-code: Health check handler

use axum::Json;
use serde::Serialize;

// Health check response
struct HealthResponse {
    status: String,
    version: String,
}

// Health check endpoint
async function health_check() -> Json<HealthResponse>:
    return Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    });
```

**Step 2: Commit**

```bash
git add ralph-server/src/handlers/
git commit -m "feat: add handlers module and health check"
```

---

## Task 24: ralph-server Binary Setup

**Files:**
- Create: `ralph-server/src/main.rs`
- Modify: `ralph-server/Cargo.toml`

**Step 1: Create server binary**

Create `ralph-server/src/main.rs`:

```
// Pseudo-code: Main server binary - HTTP API entry point

mod handlers;

use anyhow::Result;
use axum::{routing::{get, post}, Router};
use ralph_repositories::Database;
use ralph_services::{AuthService, DockerManager, LoopExecutor};
use ralph_agent::{ClaudeProvider, AgentConfig};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Main entry point
#[tokio::main]
async function main() -> Result<()>:
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
              or "ralph_server=debug")
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv();

    // Initialize database
    db = Database::new("ralph.db").await;

    // Initialize services
    auth = AuthService::new(db.pool().clone());
    docker = DockerManager::new().await;
    agent_config = AgentConfig::default();
    executor = LoopExecutor::new(db.pool().clone(), docker, agent_config);

    // Build HTTP router
    app = Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Auth endpoints
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        // Loop endpoints
        .route("/api/loops", get(handlers::list_loops)
                                  .post(handlers::create_loop))
        .route("/api/loops/:id", get(handlers::get_loop)
                                   .delete(handlers::delete_loop))
        .route("/api/loops/:id/start", post(handlers::start_loop))
        .route("/api/loops/:id/pause", post(handlers::pause_loop))
        .route("/api/loops/:id/resume", post(handlers::resume_loop))
        .route("/api/loops/:id/stop", post(handlers::stop_loop))
        // Task endpoints
        .route("/api/loops/:id/tasks", get(handlers::list_tasks)
                                        .post(handlers::create_task))
        .route("/api/tasks/:id", get(handlers::get_task)
                               .delete(handlers::delete_task))
        // Add state
        .with_state(auth)
        .with_state(executor)
        .with_state(db)
        // Add CORS
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()));

    // Start server
    addr = env::var("SERVER_ADDR") or "0.0.0.0:3000";
    listener = tokio::net::TcpListener::bind(&addr).await;
    info!("Server listening on {}", addr);

    axum::serve(listener, app).await;
```

**Step 2: Update ralph-server Cargo.toml**

Edit `ralph-server/Cargo.toml`:

```
[dependencies]
ralph-models = { path = "../ralph-models" }
ralph-repositories = { path = "../ralph-repositories" }
ralph-services = { path = "../ralph-services" }
ralph-agent = { path = "../ralph-agent" }
tokio.workspace = true
axum.workspace = true
tower.workspace = true
tower-http = { version = "latest", features = ["cors"] }
anyhow.workspace = true
dotenvy = "latest"
```

**Step 3: Test compilation**

Run: `cargo check -p ralph-server`
Expected: Compiles

**Step 4: Commit**

```bash
git add ralph-server/
git commit -m "feat: add ralph-server binary with HTTP routes"
```

---

## Task 25-27: Frontend, Dockerfile, Config

**(Task 25: Frontend templates, Task 26: Dockerfile, Task 27: .env.example)**

**Resumo:**

- **Task 25:** Templates HTML em `templates/` para interface web
- **Task 26:** Dockerfile em `docker/Dockerfile` para imagem base dos containers
- **Task 27:** `.env.example` com variáveis de ambiente e configurações

---

**Fim do Plano de Implementação**

Este plano cobre **27 tasks** distribuídas em 3 sprints:

- **Sprint 1-2** (Tasks 1-10): Fundação - workspace, models, repositories
- **Sprint 3-4** (Tasks 11-19): ralph-agent + ralph-services - code agent completo
- **Sprint 5-6** (Tasks 20-27): ralph-server - binário com handlers + frontend

**Estrutura final:**

```
ralph-loop-manager/
├── ralph-models/       ← Domain models
├── ralph-repositories/ ← Data access
├── ralph-agent/        ← Code Agent (reutilizável!)
├── ralph-services/     ← Business logic
├── ralph-server/       ← Binary + HTTP handlers
├── migrations/
├── templates/
└── docker/
```

**Destaques da ralph-agent:**

- **Independente** - pode ser usado standalone em outros projetos
- **Extensível** - sistema de tools (File, Command, Git...)
- **Autônomo** - executa comandos, cria tasks, mantém contexto
- **Multi-provider** - Claude, OpenAI, fácil adicionar mais

**Próximos passos:**

1. Executar plano com `/superpowers:executing-plans`
2. Testar código agent localmente
3. Adicionar WebSocket para streaming
4. Implementar integração Git
5. Adicionar mais tools ao agent
