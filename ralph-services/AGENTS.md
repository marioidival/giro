# Ralph Services - Intent Layer

## Purpose

This crate contains the core business logic of the Ralph Loop Manager. It implements services that coordinate complex operations between repositories, LLM agents, and Docker containers.

**What this area does:**
- **AuthService**: Manages user authentication (registration, login, password hashing)
- **DockerManager**: Manages Docker container lifecycle (creation, start, pause, stop, remove)
- **LoopExecutor**: Orchestrates Ralph loop execution (task processing, state management, LLM integration)
- Coordinates integration between `ralph-repositories`, `ralph-agent`, and Docker
- Applies business rules and validations

**What this area does NOT do:**
- Does not access database directly (uses `ralph-repositories`)
- Does not handle HTTP requests/responses (that's `ralph-server`'s responsibility)
- Does not implement data models (that's `ralph-models`' responsibility)
- Does not contain templates or HTML (that's `ralph-server`'s responsibility)

## Structure

```
ralph-services/
├── src/
│   ├── lib.rs       # Public re-exports of services
│   ├── auth.rs      # AuthService (login, registration, password hashing)
│   ├── docker.rs    # DockerManager (container lifecycle)
│   └── executor.rs  # LoopExecutor (loop orchestration)
└── Cargo.toml      # Dependencies (bcrypt, bollard, tokio, etc)
```

## Critical Invariants

### AuthService

**Password Security:**
- **ALL** passwords must be hashed with bcrypt (cost 12 - DEFAULT_COST)
- **NEVER** store passwords in plain text
- Password validation: minimum 8 characters
- Username validation: 3-50 characters, alphanumeric only

**Username Uniqueness:**
- Username must be unique before creating user
- Verify via `user_repo.find_by_username()` before `user_repo.create()`

**Login Security:**
- **ALWAYS** use same error message for "invalid username" and "invalid password"
- Prevents user enumeration (user enumeration attack)

### DockerManager

**Mandatory Resource Limits:**
```rust
cpu_quota = 1_000_000_000i64     // 1 CPU core
cpu_period = 1_000_000i64        // 1ms period
memory = 1_073_741_824i64        // 1GB (1024^3)
memory_swap = memory               // No swap (security)
```

**Mandatory Volume Mounts:**
```rust
binds = vec![
    format!("{}:/workspace/prd.md:ro", prd_path),    // Read-only
    format!("{}:/workspace/task.md:ro", task_path),   // Read-only
    format!("{}:/workspace/repo", repo_path),          // Read-write
]
```

**Container Naming:**
- Ralph containers must have prefix `ralph-loop-{loop_id}`
- Facilitates cleanup and identification

**Cleanup Pattern:**
- Orphaned containers (prefix `ralph-`, status `exited` or `dead`) must be removed automatically
- Running containers are **NEVER** removed during cleanup

### LoopExecutor

**State Transitions:**
```
Loop: Created → Running → Paused → Completed/Error
Task: Pending → InProgress → Completed/Failed
Iteration: Running → Completed/Error
```

**Execution Loop:**
- Loop must **NEVER** be blocking in HTTP handler
- Use `tokio::spawn()` for async execution
- Loop continues until: no more tasks, max_iterations, or manual stop

**Task Processing:**
- Tasks are processed in priority order (higher = more priority)
- LLM can suggest new tasks (created automatically)
- Task failure does **NOT** stop the loop (continues with next task)

**Error Handling:**
- Errors in non-critical tasks are logged and loop continues
- Only fatal errors (no tasks, max_iterations) stop the loop
- Container is cleaned up when stopped (stop + remove + volumes)

## Usage Patterns

### AuthService - Registration

```rust
use ralph_services::{AuthService, hash_password};
use ralph_repositories::{Database, UserRepository};

let db = Database::new("sqlite:ralph.db").await?;
let user_repo = UserRepository::new(db.pool().clone());
let auth_service = AuthService::new(user_repo);

let create_user = CreateUser {
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
    password: "secure_password".to_string(),
};

let user = auth_service.register(create_user).await?;
// user.password_hash is bcrypt hash (NOT the original password!)
```

### AuthService - Login

```rust
use ralph_services::AuthService;
use ralph_models::LoginUser;

let login_user = LoginUser {
    username: "alice".to_string(),
    password: "secure_password".to_string(),
};

let user = auth_service.login(login_user).await?;
// User successfully authenticated
```

### DockerManager - Create Container

```rust
use ralph_services::DockerManager;

let docker_manager = DockerManager::new();

let container_id = docker_manager
    .create_container(
        "alpine:latest",
        "/tmp/prd.md",
        "/tmp/task.md",
        "/tmp/repo",
        Some("ralph-loop-123"),  // container name
    )
    .await?;
```

### DockerManager - Container Lifecycle

```rust
// Start
docker_manager.start(&container_id).await?;

// Pause (keeps container running but paused)
docker_manager.pause(&container_id).await?;

// Unpause
docker_manager.unpause(&container_id).await?;

// Stop (graceful shutdown, 10s timeout)
docker_manager.stop(&container_id, Some(10)).await?;

// Remove (force + volumes)
docker_manager.remove(&container_id, true, true).await?;
```

### DockerManager - Cleanup

```rust
// Remove orphaned containers automatically
let cleaned_count = docker_manager
    .cleanup_orphaned_containers()
    .await?;

println!("Cleaned up {} orphaned containers", cleaned_count);
```

### LoopExecutor - Start Loop

```rust
use ralph_services::LoopExecutor;
use std::sync::Arc;
use ralph_agent::AgentConfig;

let executor = LoopExecutor::new(
    Arc::new(pool.clone()),
    Arc::new(docker_manager),
    AgentConfig::default(),
);

executor.start(&loop_id).await?;
// Container created, started, and execution_loop running in background
```

### LoopExecutor - Pause/Resume/Stop

```rust
// Pause (pauses container)
executor.pause(&loop_id).await?;

// Resume (unpause + restart execution loop)
executor.resume(&loop_id).await?;

// Stop (stop container + remove + update status to Completed)
executor.stop(&loop_id).await?;
```

### LoopExecutor - Task Processing Flow

```rust
// Execution loop (running in background via tokio::spawn)
async fn execution_loop(&self, loop_id: &str) -> Result<()> {
    loop {
        // 1. Check loop status
        let loop_ = loop_repo.find_by_id(loop_id).await?;
        if loop_.status != LoopStatus::Running {
            return Ok(());
        }

        // 2. Check max iterations
        if loop_.current_iteration >= loop_.max_iterations {
            self.stop(loop_id).await?;
            return Ok(());
        }

        // 3. Find next pending task (highest priority)
        let task = task_repo.find_next_pending(loop_id).await?;

        if task.is_none() {
            self.stop(loop_id).await?;  // No more tasks
            return Ok(());
        }

        let task = task.unwrap();

        // 4. Execute task
        match self.execute_task(&task).await {
            Ok(_) => {
                task_repo.update_status(&task.id, TaskStatus::Completed, ...).await?;
            }
            Err(e) => {
                task_repo.update_status(&task.id, TaskStatus::Failed, ...).await?;
                // Continue with next task
            }
        }

        // 5. Sleep for iteration_delay
        tokio::time::sleep(Duration::from_secs(loop_.iteration_delay as u64)).await;
    }
}
```

## Anti-patterns

### NEVER DO

**1. Store password in plain text**
```rust
// ❌ WRONG - SECURITY DANGER
User {
    password: "password123".to_string(),
}

// ✅ CORRECT - Hash bcrypt
let password_hash = hash_password("password123")?;
User {
    password_hash,
}
```

**2. Create container without resource limits**
```rust
// ❌ WRONG - Container without limits can consume entire host
let config = Config {
    image: Some("alpine:latest".to_string()),
    host_config: None,  // NO!
    ..
};

// ✅ CORRECT - With mandatory limits
let host_config = HostConfig {
    cpu_quota: Some(1_000_000_000),
    cpu_period: Some(1_000_000),
    memory: Some(1_073_741_824),
    memory_swap: Some(1_073_741_824),
    ..
};

let config = Config {
    image: Some("alpine:latest".to_string()),
    host_config: Some(host_config),
    ..
};
```

**3. Block HTTP handler with execution_loop**
```rust
// ❌ WRONG - Handler blocked indefinitely
async fn handler(State(executor): State<LoopExecutor>) {
    executor.execution_loop(&loop_id).await?;  // Blocks!
    Ok(())
}

// ✅ CORRECT - Spawn in background
async fn handler(State(executor): State<LoopExecutor>) {
    tokio::spawn(async move {
        let _ = executor.execution_loop(&loop_id).await;
    });
    Ok(())  // Handler returns immediately
}
```

**4. Not doing cleanup of containers**
```rust
// ❌ WRONG - Container keeps running if error occurs
let container_id = docker.create_container(...).await?;
docker.start(&container_id).await?;
// If error, container runs forever!

// ✅ CORRECT - Cleanup pattern (manual or via DockerManager)
let container_id = docker.create_container(...).await?;
docker.start(&container_id).await?;

// On error, cleanup:
if let Err(e) = some_operation().await {
    docker.stop(&container_id, Some(10)).await.ok();
    docker.remove(&container_id, true, true).await.ok();
    return Err(e);
}
```

**5. Reveal username existence in login**
```rust
// ❌ WRONG - User enumeration attack
pub async fn login(&self, username: &str, password: &str) -> Result<User> {
    let user = self.user_repo.find_by_username(username).await?;

    if user.is_none() {
        bail!("Username not found");  // ❌ Leaked that username doesn't exist
    }

    let user = user.unwrap();
    if !verify_password(password, &user.password_hash)? {
        bail!("Invalid password");  // ❌ Different message
    }

    Ok(user)
}

// ✅ CORRECT - Same message for both cases
pub async fn login(&self, login_user: LoginUser) -> Result<User> {
    let user = self
        .user_repo
        .find_by_username(&login_user.username)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Invalid username or password"))?;

    let is_valid = verify_password(&login_user.password, &user.password_hash)?;

    if !is_valid {
        bail!("Invalid username or password");  // ✅ Same message
    }

    Ok(user)
}
```

**6. Ignore task execution errors**
```rust
// ❌ WRONG - Silently swallowing errors
match self.execute_task(&task).await {
    Ok(_) => { /* success */ }
    Err(e) => { /* ignore error */ }
}

// ✅ CORRECT - Log and update status
match self.execute_task(&task).await {
    Ok(_) => {
        task_repo.update_status(&task.id, TaskStatus::Completed, ...).await?;
    }
    Err(e) => {
        error!("Task {} failed: {:?}", task.id, e);
        task_repo.update_status(
            &task.id,
            TaskStatus::Failed,
            Some(Utc::now()),
            None,
            Some(e.to_string())
        ).await?;
        // Continue with next task (non-critical error)
    }
}
```

**7. Create multiple Docker clients**
```rust
// ❌ WRONG - Multiple connections
let manager1 = DockerManager::new();
let manager2 = DockerManager::new();  // NEW CLIENT!

// ✅ CORRECT - Clone singleton (same connection)
let manager = DockerManager::new();
let manager_clone = manager.clone();  // Cheap clone (Docker is Arc)
```

**8. Volume mounts without :ro for read-only files**
```rust
// ❌ WRONG - PRD can be modified in container
let binds = vec![
    format!("{}:/workspace/prd.md", prd_path),  // RW by default
];

// ✅ CORRECT - Read-only for files that shouldn't change
let binds = vec![
    format!("{}:/workspace/prd.md:ro", prd_path),  // Read-only
    format!("{}:/workspace/task.md:ro", task_path),
    format!("{}:/workspace/repo", repo_path),  // RW (can write)
];
```

## Dependencies

### Dependencies (Cargo.toml)

```toml
[dependencies]
ralph-models = { path = "../ralph-models" }      # Data models
ralph-repositories = { path = "../ralph-repositories" }  # Database ops
ralph-agent = { path = "../ralph-agent" }        # LLM providers
tokio.workspace = true                              # Async runtime
anyhow.workspace = true                            # Error handling
uuid.workspace = true                              # UUID generation
chrono.workspace = true                            # DateTime
tracing.workspace = true                            # Logging
sqlx.workspace = true                             # Database queries
bcrypt = "0.16"                                 # Password hashing
bollard = "0.18"                                # Docker client
once_cell = "1.20"                               # Singleton pattern
tempfile = "3.14"                                # Test fixtures
```

### Key External Libraries

**bcrypt (0.16)**
- Password hashing with adaptive cost factor
- DEFAULT_COST = 12 (balance between security and performance)
- Secure verification against timing attacks

**bollard (0.18)**
- Rust Docker client (async)
- Abstraction for Docker API
- Platform-aware connections (Unix named socket, Windows named pipe)

**once_cell (1.20)**
- Singleton pattern for Docker client
- `OnceCell<Docker>` global in `docker.rs`

**tokio**
- Async runtime for entire application
- `tokio::spawn()` for background tasks
- `tokio::time::sleep()` for delays

## Downstreams (who depends on this crate)

- `ralph-server` → Uses AuthService, DockerManager, LoopExecutor in handlers

## Downlinks (Additional Context)

**For better understanding:**
- `/AGENTS.md` - General architecture and project patterns
- `/ralph-models/AGENTS.md` - Models that these services use
- `/ralph-repositories/AGENTS.md` - How these services persist data
- `/ralph-agent/AGENTS.md` - LLM providers and agents (when exists)

## Pitfalls

### Common Confusions

**1. Docker client singleton vs multiple instances**
```rust
// DockerManager uses global singleton (DOCKER OnceCell)
// But DockerManager::new() can be called multiple times
let manager1 = DockerManager::new();  // Uses DOCKER.get_or_init()
let manager2 = DockerManager::new();  // Uses SAME DOCKER instance

// Cloning DockerManager is cheap (Arc<Docker>)
let manager_clone = manager1.clone();  // Share same Docker connection
```

**2. LoopExecutor::start() vs execution_loop()**
```rust
// start() is public - creates container, starts, and spawns execution_loop
executor.start(&loop_id).await?;

// execution_loop() is private - runs in background via tokio::spawn
// DO NOT call execution_loop directly in handlers!
```

**3. Task status updates**
```rust
// execution_loop updates task status automatically:
// - Pending → InProgress (before executing)
// - InProgress → Completed/Failed (after execution)

// DO NOT update manually in HTTP handler!
// Use executor.start() and let the loop manage it
```

**4. Container cleanup timing**
```rust
// Container is removed ONLY in:
// 1. executor.stop() → stop + remove
// 2. docker_manager.cleanup_orphaned_containers() → exited/dead containers

// Container is NOT removed in:
// - executor.pause() → just pause (continues existing)
// - executor.resume() → just unpause
// - If error during create_container (container doesn't even exist)
```

**5. bcrypt cost factor**
```rust
// DEFAULT_COST = 12 (in bcrypt crate)
// This value is used automatically in hash_password()

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;  // Cost 12
    Ok(hashed)
}

// DO NOT change DEFAULT_COST without good reason:
// - Cost 10: 2x faster, 4x less secure
// - Cost 12: current balance (industry standard)
// - Cost 14: 4x slower, 16x more secure
```

### Unexpected Behaviors

**1. Loop continues even after task failure**
```rust
// execution_loop does NOT stop if a task fails!
match self.execute_task(&task).await {
    Ok(_) => { /* task.completed */ }
    Err(e) => {
        task_repo.update_status(&task.id, TaskStatus::Failed, ...).await?;
        // Continues with next task!
    }
}

// Only stops if:
// - No more tasks
// - max_iterations reached
// - Loop status changed (manual stop)
// - Fatal error (but this is rare)
```

**2. Execution loop is infinite until conditions**
```rust
async fn execution_loop(&self, loop_id: &str) -> Result<()> {
    loop {  // Infinite loop!
        // Check conditions
        if loop_.status != LoopStatus::Running {
            return Ok(());  // Exits here
        }

        // Process task
        // Sleep

        // Repeat...
    }
}
// No return in loop = infinite loop (not a bug, it's design!)
```

**3. tokio::spawn error handling**
```rust
// tokio::spawn() does NOT propagate errors
tokio::spawn(async move {
    if let Err(e) = executor.execution_loop(&loop_id_owned).await {
        tracing::error!("Execution loop error: {:?}", e);
        // Error is logged, but NOT propagated!
    }
});

// Handler continues normally even if execution_loop fails
// Use logs or events to monitor loop health
```

**4. Docker container name collision**
```rust
// If container with same name already exists, create_container() returns error
let container_id = docker_manager
    .create_container(
        "alpine:latest",
        "/tmp/prd.md",
        "/tmp/task.md",
        "/tmp/repo",
        Some("ralph-loop-123"),  // If exists → Error!
    )
    .await?;

// Solution: Stop/remove before or use unique name
// LoopExecutor uses format: ralph-loop-{loop_id} (loop_id is UUID, so unique)
```

**5. Password hash verification is not case-sensitive**
```rust
// bcrypt verification is NOT case-sensitive for original password
let hash1 = hash_password("Password123")?;
let hash2 = hash_password("password123")?;

// Hashes are DIFFERENT (bcrypt uses random salt)
assert_ne!(hash1, hash2);

// But both verify correctly
assert!(verify_password("Password123", &hash1)?);  // True
assert!(verify_password("password123", &hash1)?);  // False
assert!(verify_password("password123", &hash2)?);  // True
```

## Tests

### Unit Tests

```bash
# Password hash/verify tests
cargo test --package ralph-services test_hash_and_verify

# Docker singleton tests
cargo test --package ralph-services test_docker_singleton
```

### Integration Tests

```bash
# All tests (some require Docker daemon)
cargo test --package ralph-services

# Skip tests requiring Docker
cargo test --package ralph-services -- --ignore

# Specific tests
cargo test --package ralph-services test_register_new_user
cargo test --package ralph-services test_create_container_with_volumes
```

### Tests Requiring Docker Daemon

Most integration tests in `docker.rs` and `executor.rs` require Docker running:

```bash
# Start Docker before running tests
docker info  # Check if Docker is running

# Run tests
cargo test --package ralph-services

# If Docker is not running, tests are ignored (#[ignore])
# Use --ignored to see which tests were skipped
cargo test --package ralph-services -- --list --ignored
```

---

**Last updated:** 2026-01-18
**Version:** 1.0
