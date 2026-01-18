# Ralph Models - Intent Layer

## Purpose

This crate contains all data models used in Ralph Loop Manager. They are pure structures without external dependencies that define the shape of data flowing through the application.

**What this area does:**
- Defines domain models (User, Loop, Task, Iteration, File)
- Provides DTOs for creation/editing (CreateUser, CreateLoop, CreateTask)
- Implements status enums with to/from string conversion
- Basic validations and unit tests for models

**What this area does NOT do:**
- Does not access database (that's `ralph-repositories`' responsibility)
- Does not contain business logic (that's `ralph-services`' responsibility)
- Does not handle HTTP requests/responses (that's `ralph-server`' responsibility)

## Model Structure

### Main Models

```
ralph-models/
├── user.rs        # User, CreateUser, LoginUser
├── loop_.rs       # Loop, CreateLoop, LoopStatus
├── task.rs        # Task, CreateTask, TaskStatus
├── iteration.rs   # Iteration, IterationStatus
├── file.rs        # File (generated artifacts)
└── lib.rs        # Re-export of all models
```

### Relationship Hierarchy

```
User
  └── Loop (owner_id)
        ├─ Task (loop_id)
        │   └─ Iteration (task_id)
        │         └─ File (iteration_id)
        │
        └─ Iteration (loop_id, direct for global logs)
```

## Critical Invariants

### Business Rules

**User:**
- `id`: UUID v4 generated automatically
- `username`: Must be unique (validated in repository)
- `email`: Must be unique (validated in repository)
- `password_hash`: Bcrypt hash (never store password in plain text)

**Loop:**
- `id`: UUID v4 generated automatically
- `owner_id`: Must reference existing User (FK)
- `status`: Must follow state machine (Created → Running → Paused → Completed/Error)
- `container_id`: None when not running
- `current_iteration`: Incremented on each executed iteration
- `prd`: Markdown defining project context

**Task:**
- `id`: UUID v4 generated automatically
- `loop_id`: Must reference existing Loop (FK with CASCADE)
- `status`: Must follow state machine (Pending → InProgress → Completed/Failed/Cancelled)
- `parent_task_id`: For subtask support (CASCADE SET NULL)
- `created_by`: Must be 'user' or 'llm'
- `priority`: Higher value = more priority

**Iteration:**
- `id`: UUID v4 generated automatically
- `loop_id`: Must reference existing Loop (FK)
- `task_id`: Must reference existing Task (FK)
- `iteration_number`: Sequential within loop
- `status`: Running → Completed/Error
- `started_at`: Always set when creating iteration
- `completed_at`: Set when status ≠ Running

### Mandatory Defaults

**CreateLoop → Loop:**
```rust
docker_image: "ralph-loop-manager:latest"
cpu_limit: 1
memory_limit: 1024  // MB
max_iterations: 100
iteration_timeout: 300  // seconds
iteration_delay: 0
git_branch_pattern: "ralph/{loop_id}/{timestamp}"
status: Created
current_iteration: 0
container_id: None
```

**CreateTask → Task:**
```rust
status: Pending
priority: 0
iteration_id: None
started_at: None
completed_at: None
error_message: None
```

**Iteration::new():**
```rust
status: Running
output: None
error: None
tokens_used: None
completed_at: None
```

## Status Enums

### LoopStatus

```
Created    → Initial state, container doesn't exist
Running    → Container running, executing iterations
Paused      → Container paused (docker pause)
Completed   → Loop finished successfully
Error       → Loop finished with error
```

### TaskStatus

```
Pending     → Waiting for execution
InProgress  → Currently being executed
Completed   → Execution successful
Failed      → Execution failed
Cancelled   → Cancelled by user
```

### IterationStatus

```
Running     → In execution
Completed   → Completed successfully
Error       → Error during execution
```

## Usage Patterns

### Create a New Loop

```rust
use ralph_models::{Loop, CreateLoop};

let create_loop = CreateLoop {
    name: "My First Loop".to_string(),
    description: Some("Test loop for demo".to_string()),
    prd: "Build a simple calculator".to_string(),
    owner_id: user_id.clone(),
    provider: "claude".to_string(),
    model: "claude-3-opus".to_string(),
    docker_image: None,  // uses default
    cpu_limit: None,    // uses default
    memory_limit: None, // uses default
    max_iterations: None,  // uses default
    iteration_timeout: None,  // uses default
    iteration_delay: None,  // uses default
    git_repo_url: None,
    git_branch_pattern: None,
};

let loop_ = Loop::new(create_loop);
// loop_.id is a UUID v4
// loop_.status is Created
```

### Create a New Task

```rust
use ralph_models::{Task, CreateTask};

let create_task = CreateTask {
    loop_id: loop_id.clone(),
    title: "Add addition function".to_string(),
    description: "Implement add(a, b) -> a + b".to_string(),
    priority: Some(5),  // high priority
    parent_task_id: None,
    created_by: "user".to_string(),
};

let task = Task::new(create_task);
// task.id is a UUID v4
// task.status is Pending
```

### Create a New Iteration

```rust
use ralph_models::Iteration;

let iteration = Iteration::new(
    loop_id.clone(),
    task_id.clone(),
    1,  // iteration_number
);
// iteration.id is a UUID v4
// iteration.status is Running
// iteration.started_at is now
```

### Serialization/Deserialization

All models implement `Serialize` and `Deserialize` from Serde:

```rust
use serde_json;

let json = serde_json::to_string(&loop_)?;
let deserialized: Loop = serde_json::from_str(&json)?;
```

### Status Conversion

```rust
use ralph_models::LoopStatus;

// Enum → String
let status_string = format!("{}", LoopStatus::Running);  // "running"

// String → Enum
let status = LoopStatus::from_str("running")?;
```

## Anti-patterns

### NEVER DO

**1. Modify models after creation**
```rust
// ❌ WRONG - Violate immutability
let mut loop_ = Loop::new(create_loop);
loop_.id = "custom-id".to_string();  // NO

// ✅ CORRECT - Create new loop with updated CreateLoop
let create_loop = CreateLoop { ... };
let loop_ = Loop::new(create_loop);
```

**2. Store password in plain text**
```rust
// ❌ WRONG
User {
    password: "password123".to_string(),  // DANGER
}

// ✅ CORRECT
User {
    password_hash: bcrypt::hash("password123", 12)?,
}
```

**3. Complex validations in models**
```rust
// ❌ WRONG - Models should be simple
impl User {
    pub fn validate(&self) -> Result<(), Error> {
        // Complex validations
        if self.username.len() < 3 { ... }
        if !self.email.contains('@') { ... }
    }
}

// ✅ CORRECT - Validations in repository or service
impl UserRepository {
    pub async fn create(&self, create_user: CreateUser) -> Result<User> {
        // Validate before persisting
        validate_username(&create_user.username)?;
        validate_email(&create_user.email)?;
        // ...
    }
}
```

**4. Override timestamps**
```rust
// ❌ WRONG - Overriding timestamps
let mut user = User::new(...);
user.created_at = Utc::now() - Duration::days(1);  // NO

// ✅ CORRECT - Let models define timestamps
let user = User::new(...);  // created_at is Utc::now() automatically
```

**5. Not define defaults correctly**
```rust
// ❌ WRONG - Optional without default
impl Loop {
    pub fn new(create_loop: CreateLoop) -> Self {
        Self {
            cpu_limit: create_loop.cpu_limit,  // can be None!
        }
    }
}

// ✅ CORRECT - Provide defaults
impl Loop {
    pub fn new(create_loop: CreateLoop) -> Self {
        Self {
            cpu_limit: create_loop.cpu_limit.unwrap_or(1),
        }
    }
}
```

## Dependencies

### Dependencies (Cargo.toml)

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }  # Serialize/Deserialize
serde_json = "1.0"  # JSON serialization
chrono = { version = "0.4", features = ["serde"] }  # DateTime<Utc>
uuid = { version = "1.11", features = ["serde"] }  # UUID generation
```

### Downstreams (who depends on this crate)

- `ralph-repositories` → Uses models for database queries
- `ralph-agent` → Uses models for LLM interaction
- `ralph-services` → Uses models for business logic
- `ralph-server` → Uses models in handlers and templates

## Tests

Each model has comprehensive unit tests:

```bash
# Run all tests
cargo test --package ralph-models

# Run tests for specific model
cargo test --package ralph-models user
cargo test --package ralph-models loop
cargo test --package ralph-models task
cargo test --package ralph-models iteration
```

### Expected Coverage

- **User**: UUID generation, serialization, filled fields
- **Loop**: Defaults, custom values, status enum, timestamps
- **Task**: Defaults, priority, parent_task, status enum
- **Iteration**: Creation, status enum, timestamps

## Pitfalls

### Common Confusions

**1. UUID as String vs Uuid type**
```rust
// Models use String for IDs to facilitate serialization
pub struct Loop {
    pub id: String,  // String containing UUID
}

// To validate if it's valid UUID:
use uuid::Uuid;
Uuid::parse_str(&loop_.id).is_ok()
```

**2. Option vs Default**
```rust
// CreateLoop uses Option for optional parameters
pub struct CreateLoop {
    pub cpu_limit: Option<i32>,
    pub memory_limit: Option<i32>,
}

// Loop uses concrete values (with defaults applied)
pub struct Loop {
    pub cpu_limit: i32,        // always defined
    pub memory_limit: i32,     // always defined
}
```

**3. Status Enums aren't strings**
```rust
// ❌ WRONG - Compare with string
if loop_.status == "running" { ... }

// ✅ CORRECT - Compare with enum
if loop_.status == LoopStatus::Running { ... }

// Or convert when needed
let status_str = loop_.status.to_string();  // "running"
```

**4. Timestamps are UTC**
```rust
// All timestamps are DateTime<Utc>
pub struct Loop {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// To convert to local:
use chrono::Local;
let local_time = loop_.created_at.with_timezone(&Local);
```

## Downlinks (Additional Context)

**For better understanding:**
- `/AGENTS.md` - General architecture and project patterns
- `/ralph-repositories/AGENTS.md` - How these models are persisted in database
- `/ralph-services/AGENTS.md` - How these models are used in business logic
- `/ralph-server/AGENTS.md` - How these models flow through HTTP API

---

**Last updated:** 2026-01-18
**Version:** 1.0
