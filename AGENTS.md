# Ralph Loop Manager - Intent Layer (Root)

## Purpose

Ralph Loop Manager is a web-based platform for managing, monitoring, and orchestrating **Ralph loops** - AI-driven development workflows that execute continuously in Docker containers.

**What this area does:**
- Complete platform for managing AI development loops (Ralph)
- Isolated execution in Docker containers
- Real-time monitoring via WebSocket
- Complete history of iterations and tasks
- Integration with Git (auto PRs/MRs)
- Support for multiple LLM providers (Claude, OpenAI, Sourcegraph Amp)

**What this area does NOT do:**
- Not an IDE or code editor
- Does not execute code directly on host (always in containers)
- Does not replace complete CI/CD systems (focuses on iterative development)

## General Architecture

### Technology Stack
- **Backend:** Rust 1.85+ (Axum web framework, Tokio async runtime)
- **Database:** SQLite (with PostgreSQL migration path)
- **Frontend:** HTMX + Tailwind CSS (Askama templates)
- **Containers:** Docker 27.x+ for execution isolation

### Workspace Structure (Cargo)

```
giro/
├── ralph-models/          # Data models (User, Loop, Task, Iteration, File)
├── ralph-repositories/     # Database repositories (SQLx)
├── ralph-agent/          # LLM agent with adapters for multiple providers
├── ralph-services/       # Business logic (Auth, Docker, Loop execution)
├── ralph-server/         # HTTP server, handlers, middleware, WebSocket, templates
├── migrations/           # Database schema migrations
└── templates/           # HTMX frontend templates
```

### Data Flow

```
┌─────────────┐
│   Frontend  │ (HTMX + Tailwind)
└──────┬──────┘
       │ HTTP/WebSocket
       ▼
┌─────────────────────────────────────────┐
│         ralph-server                  │
│  - HTTP handlers (auth, loops, tasks)│
│  - Middleware (auth, CSRF, rate limit)│
│  - WebSocket streaming               │
└──────┬───────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│         ralph-services               │
│  - AuthService                       │
│  - DockerManager (containers)        │
│  - LoopExecutor (orchestration)      │
└──────┬───────────────────────────────┘
       │
       ├──────────────────┬─────────────┐
       ▼                  ▼             ▼
┌──────────────┐   ┌─────────────┐  ┌──────────┐
│ ralph-agent │   │  SQLite DB  │  │  Docker  │
│ (LLM calls) │   │(SQLx/Repo) │  │ (bollard)│
└──────────────┘   └─────────────┘  └──────────┘
```

## Usage Patterns

### How to Run the Project

```bash
# 1. Configure environment
cp .env.example .env
# Edit .env and add API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY)

# 2. Build
cargo build --release

# 3. Run server
cargo run --bin ralph-server
# Server available at http://localhost:3000

# 4. Test
cargo test
```

### How to Add New Feature

1. **Model**: If involves data, first create/edit in `ralph-models/`
2. **Repository**: Create methods in `ralph-repositories/` for database ops
3. **Service**: Add business logic in `ralph-services/`
4. **Handler**: Create handler in `ralph-server/src/handlers/`
5. **Route**: Add route in `ralph-server/src/router.rs`
6. **Template**: If needed, create template in `ralph-server/templates/`

### How to Debug

```rust
// Tracing is configured in main.rs
info!("Informational message");
warn!("Warning");
error!("Error: {}", e);
debug!("Debug info (requires RUST_LOG=debug)");
```

Environment variable for debug:
```bash
RUST_LOG=debug cargo run
```

## Critical Invariants

### Database
- **ALL** database operations must use SQLx with type-safe queries
- **NEVER** construct queries with string formatting (SQL injection risk)
- **ALWAYS** use prepared queries via `query!()`, `query_as!()`, etc.
- Migrations must be idempotent

### Async/Await
- Tokio runtime is mandatory in all async code
- Use `.await` correctly - **never** block in async context
- Prefer `Arc<T>` for sharing state between tasks

### Docker
- **ALL** loops run in isolated containers
- Containers should not have access to host filesystem (except mounted volumes)
- Resource limits (CPU, memory) must always be applied
- Containers are removed after completion

### Auth/Security
- Protected routes use authentication middleware
- CSRF tokens are mandatory in all mutations (POST/PUT/DELETE)
- Passwords are hashed with bcrypt (cost 12)
- API keys are encrypted in database

## Anti-patterns

### NEVER DO

**Database:**
```rust
// ❌ WRONG - SQL injection
let query = format!("SELECT * FROM loops WHERE id = '{}'", id);

// ✅ CORRECT - Type-safe query
let loop = sqlx::query_as!(
    Loop,
    "SELECT * FROM loops WHERE id = ?",
    id
).fetch_one(&pool).await?;
```

**Async/Await:**
```rust
// ❌ WRONG - Blocking async context
std::thread::sleep(Duration::from_secs(1));

// ✅ CORRECT - Async sleep
tokio::time::sleep(Duration::from_secs(1)).await;
```

**Error Handling:**
```rust
// ❌ WRONG - Ignoring errors
let _ = some_operation();

// ✅ CORRECT - Propagating errors
some_operation().await?;
```

**Docker:**
```rust
// ❌ WRONG - Container without resource limits
docker.create_container(...);

// ✅ CORRECT - With limits
docker.create_container()
    .with_host_config(HostConfig {
        memory: Some(1024 * 1024 * 1024), // 1GB
        cpu_quota: Some(100000),           // 1 CPU
        ..
    })
```

**Frontend (HTMX):**
```rust
// ❌ WRONG - Returning JSON when HTML is expected
Json(data)

// ✅ CORRECT - Returning HTML for HTMX
Html(template)
```

## Dependencies

### Internal Dependencies
- `ralph-models` → None (pure models)
- `ralph-repositories` → `ralph-models`
- `ralph-agent` → `ralph-models`
- `ralph-services` → `ralph-models`, `ralph-repositories`, `ralph-agent`
- `ralph-server` → `ralph-models`, `ralph-repositories`, `ralph-services`

### Downlinks (Additional Context)

**For better understanding:**
- `/ralph-models/AGENTS.md` - Data models and validation
- `/ralph-repositories/AGENTS.md` - Database operations and SQLx patterns
- `/ralph-agent/AGENTS.md` - LLM integration and provider adapters
- `/ralph-services/AGENTS.md` - Business logic and Docker integration
- `/ralph-server/AGENTS.md` - HTTP handlers, middleware, WebSocket
- `/migrations/AGENTS.md` - Database schema and migrations

**Documentation:**
- `README.md` - Setup and usage instructions
- `PRD-ralph-loop-management.md` - Detailed product requirements
- `2026-01-17-ralph-loop-manager-sprint-breakdown.md` - Implementation roadmap

## Pitfalls

### Common Confusions

**1. Database Connection Pool**
```rust
// ❌ WRONG - Creating multiple pools
let pool1 = Database::new(&url).await?.pool();
let pool2 = Database::new(&url).await?.pool();

// ✅ CORRECT - Sharing pool via Arc
let pool = Arc::new(Database::new(&url).await?.pool());
```

**2. Handler State**
```rust
// ❌ WRONG - Unnecessary clone
async fn handler(State(repo): State<LoopRepository>) {
    let repo2 = repo.clone(); // unnecessary
}

// ✅ CORRECT - Use State directly or .clone() when needed
async fn handler(State(repo): State<Arc<LoopRepository>>) {
    // repo is already Arc, can be used directly
}
```

**3. WebSocket Connection Management**
```rust
// ❌ WRONG - No cleanup
let ws = WebSocketUpgrade::new(...);
ws.on_upgrade(move |socket| async move {
    // handle socket
});

// ✅ CORRECT - With cleanup
let ws = WebSocketUpgrade::new(...);
ws.on_upgrade(move |socket| {
    let broadcast = broadcast.clone();
    async move {
        let mut socket = socket;
        let mut rx = broadcast.subscribe();
        // handle socket
        // automatic cleanup when socket closes
    }
});
```

**4. Container Lifecycle**
```rust
// ❌ WRONG - Container without cleanup
let container = docker.create_container(...).await?;
docker.start_container(&id).await?;
// If error, container keeps running

// ✅ CORRECT - With cleanup pattern
let container = docker.create_container(...).await?;
docker.start_container(&id).await?;
let guard = ContainerGuard::new(docker.clone(), id);
// When guard goes out of scope, container is automatically removed
```

### Unexpected Behaviors

**1. SQLx Compile-Time Checks**
```rust
// SQLx queries are validated at compile time
// If schema changes, code DOESN'T compile
let loop = sqlx::query_as!(
    Loop,
    "SELECT * FROM loops WHERE id = ?",  // If column doesn't exist, compile-time error
    id
).fetch_one(&pool).await?;
```

**2. Axum State Cloning**
```rust
// AppState must implement Clone
#[derive(Clone)]
struct AppState {
    repo: Arc<LoopRepository>,  // Cheap clone (just the Arc)
}
```

**3. HTMX Request Detection**
```rust
// To know if request came from HTMX:
let is_htmx = req
    .headers()
    .get("HX-Request")
    .is_some();
```

**4. Async Drop doesn't exist in Rust**
```rust
// ❌ This doesn't work
struct ContainerGuard {
    docker: Docker,
    id: String,
}
impl Drop for ContainerGuard {
    fn drop(&mut self) {
        async {  // Can NOT be async!
            self.docker.remove_container(&self.id).await?;
        }
    }
}

// ✅ Use background task or explicit cleanup
tokio::spawn(async move {
    docker.remove_container(&id).await.ok();
});
```

## Configuration

### Environment Variables (Required)

```bash
# Database
DATABASE_URL=sqlite:ralph.db

# Server
SERVER_ADDR=0.0.0.0:3000
CORS_ORIGINS=http://localhost:3000

# LLM Providers (at least one required)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...

# Rate Limiting
RATE_LIMIT_PER_MINUTE=60
```

### Environment Variables (Optional)

```bash
# Docker
DOCKER_HOST=unix:///var/run/docker.sock

# Logging
RUST_LOG=info  # or debug, warn, error

# Session
SESSION_EXPIRY_HOURS=24

# CORS
CORS_ORIGINS=http://localhost:3000,https://example.com
```

## Ralph Specific Patterns

### Loop Execution Flow

```
1. Loop created (status: created)
   ↓
2. Docker container created with volumes:
   - /workspace/prd.md (Project PRD)
   - /workspace/task.md (Current task)
   - /workspace/repo/ (Git repository)
   ↓
3. Container starts with entrypoint script
   ↓
4. Ralph loop starts: while :; do cat PROMPT.md | npx amp ; done
   ↓
5. On each iteration:
   - Fetches next task (status: pending)
   - Marks as in_progress
   - Sends PRD + Task to LLM
   - LLM executes (generates code, etc)
   - Task marked as completed/failed
   - LLM can create new tasks
   ↓
6. Repeats until no tasks or max_iterations
   ↓
7. Container stops and is removed
```

### Task State Machine

```
pending → in_progress → completed
   ↓          ↓            ↑
   └─────→ failed ───────┘
         ↖
        cancelled
```

### WebSocket Streaming

- **Each loop has dedicated channel**
- **BroadcastManager manages multiple channels**
- **Message format:** JSON with `{type: "output", data: "..."}`
- **Clients subscribe to receive real-time updates**

## Pre-Commit Checklist

- [ ] `cargo fmt` (formatting)
- [ ] `cargo clippy -- -D warnings` (linting)
- [ ] `cargo test` (tests passing)
- [ ] SQLx queries are type-safe (using macros)
- [ ] No unnecessary `unwrap()` calls
- [ ] Errors are handled with `?` or `anyhow::Context`
- [ ] New models have corresponding migrations
- [ ] Protected handlers use auth middleware
- [ ] CSRF tokens included in forms
- [ ] Docker containers have resource limits
- [ ] No hardcoded secrets or URLs

---

**Last updated:** 2026-01-18
**Version:** 1.0
