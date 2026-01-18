# Ralph Loop Manager - Sprint Breakdown v1.0

**Status:** Implementation Plan  
**Date:** 2026-01-18  
**Version:** 1.0  
**Total Sprints:** 9  
**Total Tasks:** 137  
**Estimated Total Effort:** ~400 hours (10 weeks)

---

## Executive Summary

This document breaks down Ralph Loop Manager project into **9 sprints** with **137 atomic, commitable tasks**. Each sprint delivers a **demoable piece of software** that can be run, tested, and built upon.

### GitHub Labels and Issue Organization

All GitHub issues and PRs MUST be labeled for proper tracking. The following labels are available:

**Sprint Labels** (Use these to indicate which sprint the issue belongs to):
- `sprint-1` (blue #0066cc): Sprint 1: Foundation - Workspace & Database
- `sprint-2` (green #00cc66): Sprint 2: LLM Agent & Services
- `sprint-3` (orange #cc6600): Sprint 3: HTTP Server & Frontend
- `sprint-4` (red #ff3366): Sprint 4: Git Integration - Automated commits, PR creation, branch management
- `sprint-5` (lime #66ff33): Sprint 5: API Key Management - User-specific keys, encryption, rotation
- `sprint-6` (cyan #33ccff): Sprint 6: Enhanced Templates - Home, profile, loop templates library
- `sprint-7` (purple #cc33ff): Sprint 7: OAuth Authentication - OAuth flow, token storage, social login
- `sprint-8` (yellow #ffcc33): Sprint 8: Advanced Task Features - Drag & drop, Kanban, CSV import/export
- `sprint-9` (teal #33ff99): Sprint 9: Production Readiness - Monitoring, session persistence, PostgreSQL, hardening

**Category Labels** (Use these to indicate which code area the issue affects):
- `category:models`: Tasks related to domain models (ralph-models crate)
- `category:repositories`: Tasks related to data access layer (ralph-repositories crate)
- `category:agent`: Tasks related to LLM agent (ralph-agent crate)
- `category:services`: Tasks related to business logic (ralph-services crate)
- `category:server`: Tasks related to HTTP server (ralph-server crate)
- `category:frontend`: Tasks related to frontend templates

**Standard Labels**:
- `bug`: Something isn't working
- `enhancement`: New feature or request
- `documentation`: Improvements or additions to documentation
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention is needed
- `question`: Further information is requested
- `wontfix`: This will not be worked on
- `duplicate`: This issue or pull request already exists
- `invalid`: This doesn't seem right

### Labeling Guidelines

When creating GitHub issues or PRs:

1. **Always apply a sprint label** (`sprint-1` through `sprint-9`) to indicate which sprint the issue belongs to
2. **Apply category labels** based on which code areas are affected (e.g., `category:models` for model changes)
3. **Apply standard labels** as appropriate (e.g., `bug` for bugs, `enhancement` for new features)
4. **Multiple labels allowed** - an issue can have a sprint label, multiple category labels, and standard labels

**Examples:**
- Bug in LoopRepository → `sprint-1`, `category:repositories`, `bug`
- New LLM provider → `sprint-2`, `category:agent`, `enhancement`
- UI fix for login page → `sprint-3`, `category:frontend`, `bug`
- OAuth implementation → `sprint-7`, `category:services`, `category:models`, `enhancement`

### Current Implementation Status

**What's Working (65% Complete):**
- Complete Rust workspace architecture (5 crates)
- Type-safe SQLite database with migrations
- Full authentication system (registration, login, sessions, CSRF, rate limiting)
- Loop management (CRUD, ownership, status transitions)
- Task management (CRUD, hierarchical tasks, priority ordering)
- Docker containerization with resource limits
- Multi-LLM provider support (Claude, OpenAI, Mock)
- Real-time WebSocket updates for loop progress
- Complete HTTP API with comprehensive middleware
- Basic HTML templates with HTMX and Tailwind CSS

**What's Missing (35% Remaining):**
- **Git Integration (P0 - CRITICAL):** No automated commits, PR creation, branch management
- **API Key Management (P1 - HIGH):** No user-specific keys, encryption, or rotation
- **OAuth Authentication (P2 - MEDIUM):** No social login or token storage
- **Enhanced Templates (P2 - LOW):** Missing UI pages (home, profile, etc.)
- **Advanced Task Features (P3 - LOW):** No drag & drop, Kanban, import/export
- **Loop Templates & Sharing (P2 - LOW):** No reusable templates or collaboration
- **Production Readiness:** No monitoring, session persistence, PostgreSQL migration

### Business Impact of Gaps

- **Git Integration (P0):** Users must manually push code and manage branches, defeating AI workflow automation purpose
- **API Keys (P1):** Security risk - keys stored in environment variables, all users share same keys
- **OAuth (P2):** User adoption friction - social login expected in modern apps
- **Enhanced Templates (P2):** Missing pages reduce usability
- **Advanced Task Features (P3):** Without enhanced UX, task management remains basic and less efficient

---

## Architecture Overview

### Layered Architecture

```
┌─────────────────────────────────────────────────────┐
│  ralph-server (HTTP/WebSocket/HTMX/Templates)    │ ← Sprint 3
│  - Axum web framework                              │
│  - WebSocket real-time updates                      │
│  - Askama + HTMX + Tailwind frontend              │
└────────────┬────────────────────────────────┘
               │
┌────────────▼────────────────────────────────┐
│  ralph-services (Business Logic)                    │ ← Sprint 2
│  - AuthService (bcrypt, sessions)                   │
│  - DockerManager (container lifecycle, limits)        │
│  - LoopExecutor (orchestration, retry logic)        │
│  - GitService (clone, commit, push, PR)          │ ← Sprint 4
└────────────┬────────────────────────────────┘
               │
┌────────────▼────────────────────────────────┐
│  ralph-agent (LLM Integration)                    │ ← Sprint 2
│  - LLM Provider Trait (abstraction)               │
│  - ClaudeProvider (anthropic-rust)                 │
│  - OpenAIProvider (async-openai)                   │
│  - CodeAgent (context management, timeout)          │
└────────────┬────────────────────────────────┘
               │
┌────────────▼────────────────────────────────┐
│  ralph-repositories (Database Access)               │ ← Sprint 1
│  - UserRepository                                 │
│  - LoopRepository                                 │
│  - TaskRepository                                 │
│  - ApiKeyRepository                               │ ← Sprint 5
│  - GitCredentialsRepository                         │ ← Sprint 4
│  - OAuthTokenRepository                            │ ← Sprint 7
└────────────┬────────────────────────────────┘
               │
┌────────────▼────────────────────────────────┐
│  ralph-models (Data Structures)                  │ ← Sprint 1
│  - User, Loop, Task, Iteration, File models       │
│  - ApiKey, GitCredential, OAuthToken models         │ ← Sprints 4,5,7
└─────────────────────────────────────────────────┘
```

### Key Architectural Patterns

**All database operations:**
- Use SQLx `query!()` macros for type-safe queries
- Never use string formatting for queries (SQL injection risk)
- Use prepared queries with parameter binding

**All Docker operations:**
- Apply resource limits (CPU, memory, disk)
- Cleanup containers automatically (orphaned containers on startup)
- Use Command Pattern for atomic, reversible operations
- Apply security hardening (seccomp profiles, capabilities drop)

**All HTTP mutations:**
- Require authentication (session-based)
- Validate CSRF tokens
- Apply rate limiting (per-IP and per-user)

**All async code:**
- Never block in async context
- Use `tokio::spawn()` for background tasks
- Use `tokio::task::spawn_blocking()` for blocking operations (Git, file I/O)

**All errors:**
- Use `thiserror` for structured error types
- Propagate errors with `?` operator
- Use `anyhow::Context` for adding context to errors

**All Git operations:**
- Use `tokio::task::spawn_blocking()` for blocking git2/gix operations
- Implement retry logic with exponential backoff
- Store credentials encrypted in database

---

## Sprint Quick Reference

| Sprint | GitHub Label | Tasks | Key Deliverables | Demo |
|--------|-------------|-------|------------------|------|
| | **0** | - | 4 | Workspace, Docker image, env config, error patterns | `cargo build` succeeds |
| | **1** | `sprint-1` | 15 | Tables, models, repositories, error handling | `cargo test -p ralph-repositories` passes |
| | **2** | `sprint-2` | 25 | LLM agents, Docker, LoopExecutor, retry logic | `bash demo_sprint2.sh` runs loop |
| | **3** | `sprint-3` | 20 | Web API, templates, WebSocket, middleware | Full web workflow (register → create loop → monitor) |
| | **4** | `sprint-4` | 15 | GitService, credentials, auto-commit, PR creation | Loop creates PR automatically |
| | **5** | `sprint-5` | 12 | Encryption, user-specific keys, rotation | User adds key → creates loop → uses key |
| | **6** | `sprint-6` | 10 | Home, profile, loop templates library | All pages accessible and styled |
| | **7** | `sprint-7` | 12 | OAuth flow, token storage, social login | Login with GitHub/Google |
| | **8** | `sprint-8` | 10 | Drag & drop, Kanban, CSV import/export | Visual task management works |
| | **9** | `sprint-9` | 14 | Monitoring, session persistence, PostgreSQL, hardening | Load test with 10+ concurrent loops |

---

## Sprint 0: Setup & Foundation

**Goal:** Establish project structure, build system, and base infrastructure.

**Estimated Effort:** 8 hours  
**Deliverables:** Rust workspace, base Docker image, environment configuration, error handling patterns

### Task 0.1: Initialize Rust Workspace

**Files:** `Cargo.toml` (workspace root), `.gitignore`, `Cargo.lock`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None

**Steps:**
1. Create workspace `Cargo.toml` with member crates (ralph-models, ralph-repositories, ralph-agent, ralph-services, ralph-server)
2. Create `.gitignore` (target/, .env, *.db, .sqlx/)
3. Run `cargo check` to generate `Cargo.lock`

**Tests:**
- Run `cargo check` in workspace root
- Expected: All crates resolve, no compilation errors

**Validation:**
- Workspace compiles successfully
- All crate members recognized

**Commit:** `chore: initialize Rust workspace with member crates`

---

### Task 0.2: Create Base Docker Image

**Files:** `docker/Dockerfile`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None

**Steps:**
1. Start from `rust:1.85-alpine`
2. Install dependencies: git, bash, coreutils, curl, nodejs, python3
3. Set working directory `/workspace`
4. Build ralph-loop-manager binary with `cargo build --release`
5. Copy binary to `/usr/local/bin/ralph-loop-manager`
6. Set entrypoint to binary

**Tests:**
- Build image: `docker build -t ralph-loop-manager:latest -f docker/Dockerfile .`
- Test: `docker run --rm ralph-loop-manager:latest --version`

**Validation:**
- Image builds successfully
- Binary runs and shows version

**Commit:** `feat: add base Docker image for loop containers`

---

### Task 0.3: Create Environment Configuration

**Files:** `.env.example`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None

**Steps:**
1. Create `.env.example` with all required variables (DATABASE_URL, SERVER_ADDR, ANTHROPIC_API_KEY, OPENAI_API_KEY, DOCKER_HOST, RUST_LOG, SESSION_EXPIRY_HOURS, RATE_LIMIT_PER_MINUTE)
2. Document each variable with inline comments
3. Add `.env` to `.gitignore`

**Tests:**
- Verify `.env.example` contains all variables referenced in code
- Verify `.env` is ignored by git

**Validation:**
- All required environment variables documented
- `.env` not committed to git

**Commit:** `feat: add environment configuration template`

---

### Task 0.4: Define Error Handling Patterns

**Files:** `ralph-models/src/error.rs`, `ralph-repositories/src/error.rs`

**Estimated Time:** 1 hour  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** None

**Steps:**
1. Create `ralph-models/src/error.rs` with `ModelError` enum (Validation, NotFound, Duplicate)
2. Create `ralph-repositories/src/error.rs` with `RepositoryError` enum (Database, NotFound, Duplicate, Constraint)
3. Implement `Display` and `Error` traits using `thiserror`
4. Implement conversion from `ModelError` to `RepositoryError`

**Tests:**
- Test error creation and formatting
- Test error conversion with `?` operator
- Run `cargo test -p ralph-models error`
- Run `cargo test -p ralph-repositories error`

**Validation:**
- Error types are structured and descriptive
- Error propagation works with `?` operator
- Errors include sufficient context

**Commit:** `feat: define error handling patterns with thiserror`

---

### Task 0.5: Sprint 0 Demo - Foundation Verification

**Files:** `demo_sprint0.sh`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 0.1-0.4

**Steps:**
1. Create `demo_sprint0.sh` that:
   - Runs `cargo check` to verify workspace
   - Builds Docker image
   - Verifies `.env.example` exists
2. Run demo script
3. Verify all operations complete successfully

**Tests:**
- Run `bash demo_sprint0.sh`
- Expected: Script completes without errors

**Validation:**
- Workspace compiles
- Docker image builds
- Environment configuration complete

**Commit:** `docs: add Sprint 0 demo script`

---

## Sprint 1: Database & Models

**GitHub Labels:** `sprint-1`, `category:models`, `category:repositories`
**Goal:** Implement database schema, data models, repositories, and error handling.

**Estimated Effort:** 40 hours  
**Deliverables:** SQLite database with migrations, typed models, repositories

**Demo:** `cargo test -p ralph-repositories` passes all tests

---

### Task 1.1: Create ralph-models Crate

**Files:** `ralph-models/Cargo.toml`, `ralph-models/src/lib.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 0.4

**Steps:**
1. Create `ralph-models/Cargo.toml` with dependencies (serde, chrono, sqlx, thiserror)
2. Create `ralph-models/src/lib.rs` with error module export

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: Crate compiles, no errors

**Validation:**
- Crate compiles successfully
- Exports are accessible

**Commit:** `feat: create ralph-models crate`

---

### Task 1.2: Create User Model

**Files:** `ralph-models/src/user.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/user.rs` with `User` struct (id, username, email, password_hash, created_at, updated_at)
2. Create `CreateUser` struct for validation

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- User model has all required fields
- CreateUser struct for validation

**Commit:** `feat: add User model`

---

### Task 1.3: Create Loop Model

**Files:** `ralph-models/src/loop_.rs`

**Estimated Time:** 45 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/loop_.rs` with `Loop` struct containing all fields from database schema
2. Create `CreateLoop` struct with optional fields for validation
3. Export models in `lib.rs`

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- Loop model has all required fields
- CreateLoop struct with optional fields

**Commit:** `feat: add Loop model`

---

### Task 1.4: Create Task Model

**Files:** `ralph-models/src/task.rs`

**Estimated Time:** 45 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/task.rs` with `Task` struct (id, loop_id, title, description, status, priority, parent_task_id, created_by, iteration_id, started_at, completed_at, error_message, created_at, updated_at)
2. Create `CreateTask` struct for validation

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- Task model has all required fields
- CreateTask struct for validation

**Commit:** `feat: add Task model`

---

### Task 1.5: Create Iteration and File Models

**Files:** `ralph-models/src/iteration.rs`, `ralph-models/src/file.rs`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/iteration.rs` with `Iteration` struct
2. Create `ralph-models/src/file.rs` with `File` struct
3. Export models in `lib.rs`

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- Iteration model has all required fields
- File model has all required fields

**Commit:** `feat: add Iteration and File models`

---

### Task 1.6: Create ralph-repositories Crate

**Files:** `ralph-repositories/Cargo.toml`, `ralph-repositories/src/lib.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 1.1-1.5

**Steps:**
1. Create `ralph-repositories/Cargo.toml` with dependencies (ralph-models, sqlx, anyhow, thiserror, uuid)
2. Create `ralph-repositories/src/lib.rs` with error module export

**Tests:**
- Run `cargo check -p ralph-repositories`
- Expected: Crate compiles, no errors

**Validation:**
- Crate compiles successfully
- Dependencies resolved

**Commit:** `feat: create ralph-repositories crate`

---

### Task 1.7: Create Database Module with Connection Pooling

**Files:** `ralph-repositories/src/database.rs`

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Medium  
**Dependencies:** Task 1.6

**Steps:**
1. Create `Database` struct with `SqlitePool` field
2. Implement `new()` method with `SqlitePoolOptions`:
   - max_connections: 20
   - acquire_timeout: 30 seconds
   - idle_timeout: 600 seconds
   - test_before_acquire: true
3. Implement `pool()` method to expose pool

**Tests:**
- Create test database: `sqlite3 test.db`
- Test connection: `Database::new("sqlite:test.db").await`
- Expected: Pool created successfully

**Validation:**
- Database pool creates successfully
- Connection limits applied (max 20 connections)
- Timeout configurations set

**Commit:** `feat: add database module with connection pooling`

---

### Task 1.8: Create Database Migrations

**Files:** 
- `migrations/001_users.sql`
- `migrations/002_loops.sql`
- `migrations/003_tasks.sql`
- `migrations/004_iterations_files.sql`
- `migrations/005_users_updated_at.sql`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 1.7

**Steps:**
1. Create `001_users.sql`: users table with indexes on username and email
2. Create `002_loops.sql`: loops table with indexes on owner_id and status
3. Create `003_tasks.sql`: tasks table with indexes on loop_id, status, priority, and parent_task_id
4. Create `004_iterations_files.sql`: iterations and files tables with indexes
5. Create `005_users_updated_at.sql`: ALTER TABLE to add updated_at column

**Tests:**
- Run migrations: `sqlite3 ralph.db < migrations/*.sql`
- Verify tables: `sqlite3 ralph.db ".tables"`
- Expected: All tables created, indexes applied

**Validation:**
- All tables created successfully
- All foreign keys defined
- All indexes created
- Migrations are idempotent

**Commit:** `feat: add database migrations for users, loops, tasks, iterations, files`

---

### Task 1.9: Create UserRepository

**Files:** `ralph-repositories/src/user.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 1.8

**Steps:**
1. Create `UserRepository` struct with `db: Database` field
2. Implement `create()` method with UUID generation and password hashing (bcrypt)
3. Implement `find_by_id()`, `find_by_username()`, `find_by_email()` methods using SQLx `query_as!()` macros
4. Check for duplicate username/email before creating user

**Tests:**
- Create user: `repo.create(CreateUser {...}).await`
- Find user: `repo.find_by_id(id).await`
- Expected: User created and found

**Validation:**
- User creation works
- User retrieval by id, username, email works
- Password hash stored correctly
- Error handling for duplicates

**Commit:** `feat: add UserRepository with CRUD operations`

---

### Task 1.10: Create LoopRepository

**Files:** `ralph-repositories/src/loop_.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 1.8

**Steps:**
1. Create `LoopRepository` struct with `db: Database` field
2. Implement `create()` method with default values for optional fields
3. Implement `find_by_id()`, `list_by_user()` (with pagination), `delete()`, `update_status()`, `increment_iteration()` methods using SQLx macros
4. Ensure ownership checks in delete operations

**Tests:**
- Create loop: `repo.create(CreateLoop {...}, user_id).await`
- List loops: `repo.list_by_user(user_id, 10, 0).await`
- Update status: `repo.update_status(id, "running").await`
- Expected: Loop operations work correctly

**Validation:**
- Loop creation works
- Loop listing by user works
- Loop deletion with ownership check works
- Status updates work
- Iteration increment works

**Commit:** `feat: add LoopRepository with CRUD operations`

---

### Task 1.11: Create TaskRepository

**Files:** `ralph-repositories/src/task.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 1.8

**Steps:**
1. Create `TaskRepository` struct with `db: Database` field
2. Implement `create()`, `find_by_id()`, `list_by_loop()`, `find_next_pending()`, `delete()`, `update_status()` methods using SQLx macros
3. Implement status updates with timestamp management (started_at, completed_at)

**Tests:**
- Create task: `repo.create(CreateTask {...}).await`
- List tasks: `repo.list_by_loop(loop_id).await`
- Find next pending: `repo.find_next_pending(loop_id).await`
- Expected: Task operations work correctly

**Validation:**
- Task creation works
- Task listing by loop works
- Task deletion works
- Status updates with timestamps work
- Next pending task query works

**Commit:** `feat: add TaskRepository with CRUD operations`

---

### Task 1.12: Create IterationRepository and FileRepository

**Files:** `ralph-repositories/src/iteration.rs`, `ralph-repositories/src/file.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 1.8

**Steps:**
1. Create `IterationRepository` with `create()`, `list_by_loop()`, `complete()`, `find_by_id()` methods
2. Create `FileRepository` with `create()`, `list_by_iteration()`, `find_by_id()` methods
3. Implement iteration completion with output, error, and tokens_used tracking
4. Use SQLx `query!()` macros for type-safe queries

**Tests:**
- Create iteration: `iter_repo.create(loop_id, task_id, 1).await`
- List iterations: `iter_repo.list_by_loop(loop_id).await`
- Complete iteration: `iter_repo.complete(id, Some(output), None, Some(100)).await`
- Create file: `file_repo.create(iteration_id, "src/main.rs", Some(hash), Some(1024), Some("rust")).await`
- Expected: Iteration and file operations work correctly

**Validation:**
- Iteration creation works
- Iteration listing works
- Iteration completion works
- File creation works
- File listing works

**Commit:** `feat: add IterationRepository and FileRepository`

---

### Task 1.13: Add uuid Dependency

**Files:** `ralph-repositories/Cargo.toml`

**Estimated Time:** 15 minutes  
**Complexity:** Low  
**Risk:** None

**Steps:**
1. Add `uuid = { version = "1.6", features = ["v4", "serde"] }` to `Cargo.toml`

**Tests:**
- Run `cargo check -p ralph-repositories`
- Expected: No compilation errors

**Validation:**
- uuid crate available for ID generation

**Commit:** `chore: add uuid dependency for ID generation`

---

### Task 1.14: Run Integration Tests for Repositories

**Files:** N/A

**Estimated Time:** 2 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 1.1-1.13

**Steps:**
1. Run all repository tests: `cargo test -p ralph-repositories`
2. Verify all tests pass
3. Check test coverage with `cargo tarpaulin -p ralph-repositories --out Html`

**Tests:**
- Run `cargo test -p ralph-repositories`
- Expected: All tests pass

**Validation:**
- All repository tests pass
- Test coverage > 80%

**Commit:** `test: run integration tests for all repositories`

---

### Task 1.15: Sprint 1 Demo - Database Operations

**Files:** `demo_sprint1.sh`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 1.1-1.14

**Steps:**
1. Create `demo_sprint1.sh` that:
   - Initializes database with migrations
   - Creates user, loop, tasks
   - Verifies all CRUD operations
2. Run demo script
3. Verify all operations complete successfully

**Tests:**
- Run `bash demo_sprint1.sh`
- Expected: Script completes without errors

**Validation:**
- Database migrations apply
- Repository operations work
- Error handling functional

**Commit:** `docs: add Sprint 1 demo script`

---

## Sprint 2: Core Services

**GitHub Labels:** `sprint-2`, `category:agent`, `category:services`
**Goal:** Implement LLM integration, Docker containerization, and loop execution orchestration.

**Estimated Effort:** 60 hours  
**Deliverables:** LLM providers, Docker manager, LoopExecutor with retry logic

**Demo:** `bash demo_sprint2.sh` runs a complete loop with MockLLMProvider

---

### Task 2.1: Create ralph-agent Crate

**Files:** `ralph-agent/Cargo.toml`, `ralph-agent/src/lib.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-agent/Cargo.toml` with dependencies (ralph-models, tokio, anyhow, thiserror, serde)
2. Create `ralph-agent/src/lib.rs` with provider trait export

**Tests:**
- Run `cargo check -p ralph-agent`
- Expected: Crate compiles

**Validation:**
- Crate created successfully

**Commit:** `feat: create ralph-agent crate`

---

### Task 2.2: Define LLM Provider Trait

**Files:** `ralph-agent/src/provider.rs`

**Estimated Time:** 1 hour  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 2.1

**Steps:**
1. Create `LLMRequest` struct (prompt, max_tokens, temperature)
2. Create `LLMResponse` struct (content, tokens_used, finish_reason)
3. Define `LLMProvider` trait with `generate()` async method
4. Add `async-trait` dependency for async trait support

**Tests:**
- Create mock provider implementing trait
- Test trait methods
- Expected: Trait compiles and is implementable

**Validation:**
- LLMProvider trait defined with async methods
- Request and response structures defined

**Commit:** `feat: define LLMProvider trait`

---

### Task 2.3: Create MockLLMProvider for Testing

**Files:** `ralph-agent/src/mock_provider.rs`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 2.2

**Steps:**
1. Create `MockLLMProvider` struct with configurable delay
2. Implement `LLMProvider` trait with mock response generation
3. Add configurable delay via `with_delay()` constructor method

**Tests:**
- Create mock provider
- Generate response
- Expected: Response returned with mock content

**Validation:**
- Mock provider works
- Delay configurable
- Can be used in tests

**Commit:** `feat: add MockLLMProvider for testing`

---

### Task 2.4: Create ClaudeProvider

**Files:** `ralph-agent/src/claude_provider.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Medium (requires API key)  
**Dependencies:** Task 2.2

**Steps:**
1. Add `anthropic-rust = "0.2"` dependency to `Cargo.toml`
2. Create `ClaudeProvider` struct with `client: Client` and `model: String`
3. Implement `LLMProvider` trait calling Claude API
4. Parse response content and token usage

**Tests:**
- Test with valid API key (optional, skip in CI)
- Expected: Response from Claude API

**Validation:**
- ClaudeProvider implements LLMProvider trait
- API calls work with valid key
- Error handling for invalid keys

**Commit:** `feat: add ClaudeProvider integration`

---

### Task 2.5: Create OpenAIProvider

**Files:** `ralph-agent/src/openai_provider.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Medium (requires API key)  
**Dependencies:** Task 2.2

**Steps:**
1. Add `async-openai = "0.28"` dependency to `Cargo.toml`
2. Create `OpenAIProvider` struct with `client: Client` and `model: String`
3. Implement `LLMProvider` trait calling OpenAI API
4. Parse response content and token usage

**Tests:**
- Test with valid API key (optional, skip in CI)
- Expected: Response from OpenAI API

**Validation:**
- OpenAIProvider implements LLMProvider trait
- API calls work with valid key
- Error handling for invalid keys

**Commit:** `feat: add OpenAIProvider integration`

---

### Task 2.6: Create CodeAgent with Context Management

**Files:** `ralph-agent/src/code_agent.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 2.2-2.5

**Steps:**
1. Create `CodeAgent` struct with `provider: Arc<dyn LLMProvider>`, `context: Arc<RwLock<Vec<ContextEntry>>>`
2. Define `ContextEntry` struct (role, content)
3. Implement `execute()` method building prompt from PRD and task
4. Implement timeout enforcement with `tokio::time::timeout()`
5. Implement context management with FIFO truncation (max 100 entries)

**Tests:**
- Create code agent
- Execute task
- Expected: Response from provider

**Validation:**
- CodeAgent wraps LLMProvider
- Context management works
- Timeout enforcement works
- Context trimming when exceeding max size

**Commit:** `feat: add CodeAgent with context management and timeout`

---

### Task 2.7: Create ralph-services Crate

**Files:** `ralph-services/Cargo.toml`, `ralph-services/src/lib.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.6

**Steps:**
1. Create `ralph-services/Cargo.toml` with dependencies (ralph-models, ralph-repositories, ralph-agent, tokio, anyhow, thiserror, bcrypt)
2. Create `ralph-services/src/lib.rs` with services export

**Tests:**
- Run `cargo check -p ralph-services`
- Expected: Crate compiles

**Validation:**
- Crate created successfully

**Commit:** `feat: create ralph-services crate`

---

### Task 2.8: Create AuthService

**Files:** `ralph-services/src/auth.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Task 2.7

**Steps:**
1. Create `AuthService` struct with `user_repo: UserRepository` and `sessions: Arc<RwLock<HashMap<String, UserSession>>>`
2. Implement `register()` with validation (username min 3 chars, email contains @, password min 8 chars)
3. Implement `login()` with password verification using `bcrypt::verify()`
4. Implement `validate_session()`, `logout()` methods
5. Use bcrypt for password hashing (cost 12)

**Tests:**
- Register user: `auth.register("user", "user@example.com", "password123").await`
- Login: `auth.login("user", "password123").await`
- Validate session: `auth.validate_session(session_id).await`
- Logout: `auth.logout(session_id).await`
- Expected: Auth operations work correctly

**Validation:**
- User registration works
- Password hashing works
- Login works
- Session management works
- Logout works

**Commit:** `feat: add AuthService with registration and login`

---

### Task 2.9: Create DockerManager

**Files:** `ralph-services/src/docker.rs`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** High (requires Docker daemon)  
**Dependencies:** Task 2.7

**Steps:**
1. Add `bollard = "0.18"` dependency to `Cargo.toml`
2. Create `DockerManager` struct with `client: Docker`
3. Implement `create_container()` with:
   - Resource limits (CPU: 100,000 * limit, memory: limit * 1024 * 1024 bytes)
   - Volume mounts (/workspace/prd.md, /workspace/task.md, /workspace/repo)
4. Implement `start_container()`, `stop_container()`, `remove_container()` methods
5. Implement `execute_command()` using docker exec
6. Implement `cleanup_orphaned_containers()` on startup

**Tests:**
- Create container: `docker_manager.create_container(config).await`
- Start container: `docker_manager.start_container(id).await`
- Execute command: `docker_manager.execute_command(id, "echo test").await`
- Stop container: `docker_manager.stop_container(id).await`
- Remove container: `docker_manager.remove_container(id).await`
- Expected: All Docker operations work

**Validation:**
- Container creation works with resource limits
- Volume mounts work
- Command execution works
- Container lifecycle management works
- Orphaned container cleanup works

**Commit:** `feat: add DockerManager with container lifecycle management`

---

### Task 2.10: Create LoopExecutor

**Files:** `ralph-services/src/executor.rs`

**Estimated Time:** 4 hours  
**Complexity:** High  
**Risk:** High  
**Dependencies:** Tasks 2.6, 2.8, 2.9

**Steps:**
1. Create `LoopExecutor` struct with references to all repositories and services
2. Implement `start()` method:
   - Update status to running
   - Create and start container via DockerManager
   - Spawn background execution loop
3. Implement `pause()`, `resume()`, `stop()` methods
4. Implement `execution_loop()`:
   - While running: find next pending task (priority DESC, created_at ASC)
   - Execute task via CodeAgent
   - Increment iteration counter
   - Sleep between iterations
5. Implement `execute_task()` method for individual task execution

**Tests:**
- Start loop: `executor.start().await`
- Pause loop: `executor.pause().await`
- Resume loop: `executor.resume().await`
- Stop loop: `executor.stop().await`
- Expected: Loop lifecycle management works

**Validation:**
- Loop starts and spawns background task
- Loop pauses and resumes correctly
- Loop stops and cleans up container
- Task execution works
- Iteration counter increments

**Commit:** `feat: add LoopExecutor with task orchestration`

---

### Task 2.11: Add Error Recovery with Retry Logic

**Files:** `ralph-services/src/error.rs`, `ralph-services/src/executor.rs` (update)

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 2.10

**Steps:**
1. Create `ralph-services/src/error.rs` with `LoopExecutorError` enum (LoopNotFound, DockerError, LLMTimeout, MaxIterationsExceeded)
2. Update `LoopExecutor::execute_task()` with retry logic using `tokio_retry::Retry` and `ExponentialBackoff` (max 3 retries, max delay 30s)
3. Classify errors as transient (retryable) or permanent
4. Add `tokio-retry` dependency to `Cargo.toml`

**Tests:**
- Test retry logic with MockLLMProvider that fails first 2 attempts
- Expected: Task succeeds after retries

**Validation:**
- Retry logic works
- Exponential backoff applied
- Max retries enforced
- Error classification implemented

**Commit:** `feat: add error recovery with retry logic to LoopExecutor`

---

### Task 2.12: Add Docker Failure Recovery

**Files:** `ralph-services/src/docker.rs` (update)

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Medium  
**Dependencies:** Task 2.9

**Steps:**
1. Update `DockerManager::create_container()` with retry logic using `tokio_retry::Retry`
2. Add health check method `verify_container_running()` to check container state before returning
3. Implement cleanup on partial failures

**Tests:**
- Test retry with temporary Docker daemon unavailable
- Test health check
- Expected: Container creation resilient to failures

**Validation:**
- Docker operations retry on failure
- Container health check works
- Cleanup on partial failures

**Commit:** `feat: add Docker failure recovery with retry and health checks`

---

### Task 2.13: Create ContainerMonitoringService

**Files:** `ralph-services/src/container_monitor.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 2.9

**Steps:**
1. Create `ContainerMonitoringService` struct with `docker: Arc<DockerManager>` and `check_interval: Duration`
2. Define `ContainerMetrics` struct (container_id, cpu_usage_percent, memory_usage_mb)
3. Implement `start_monitoring()` method that periodically:
   - Gets container stats via DockerManager
   - Sends metrics via mpsc channel
4. Implement automatic monitoring with tokio spawn

**Tests:**
- Create monitoring service
- Start monitoring
- Receive metrics via channel
- Expected: Metrics flow correctly

**Validation:**
- Monitoring service starts
- Metrics collected periodically
- Channel communication works

**Commit:** `feat: add container monitoring service for resource usage`

---

### Task 2.14: Run Integration Tests for Services

**Files:** N/A

**Estimated Time:** 2 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 2.1-2.13

**Steps:**
1. Run all service tests: `cargo test -p ralph-services`
2. Verify all tests pass
3. Check test coverage with `cargo tarpaulin -p ralph-services --out Html`

**Tests:**
- Run `cargo test -p ralph-services`
- Expected: All tests pass

**Validation:**
- All service tests pass
- Test coverage > 80%

**Commit:** `test: run integration tests for all services`

---

### Task 2.15: Sprint 2 Demo - Complete Loop Execution

**Files:** `demo_sprint2.sh`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** High (requires Docker)  
**Dependencies:** Tasks 2.1-2.14

**Steps:**
1. Create `demo_sprint2.sh` that:
   - Starts server in background
   - Registers user via API
   - Logs in to get session token
   - Creates loop via API
   - Adds task via API
   - Starts loop via API
   - Monitors loop execution
2. Run demo script
3. Verify all operations complete successfully

**Tests:**
- Run `bash demo_sprint2.sh`
- Expected: Script completes, loop finishes

**Validation:**
- User registration and login work
- Loop creation works
- Task creation works
- Loop executes end-to-end
- Container created and cleaned up

**Commit:** `docs: add Sprint 2 demo script`

---

## Sprint 3: HTTP Server & Frontend

**GitHub Labels:** `sprint-3`, `category:server`, `category:frontend`
**Goal:** Implement HTTP API with authentication, middleware, templates, and WebSocket streaming.

**Estimated Effort:** 50 hours  
**Deliverables:** Complete web application with Axum server, Askama templates, HTMX frontend

**Demo:** Full web workflow - register → create loop → monitor via WebSocket

---

### Task 3.1: Create ralph-server Crate

**Files:** `ralph-server/Cargo.toml`, `ralph-server/src/lib.rs`, `ralph-server/src/main.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 2.7

**Steps:**
1. Create `ralph-server/Cargo.toml` with all dependencies (axum, askama, serde, tower, tokio, tracing, etc.)
2. Create `ralph-server/src/lib.rs` and `main.rs`
3. Implement basic HTTP server with Axum

**Tests:**
- Run `cargo run --bin ralph-server`
- Expected: Server starts and responds to GET /

**Validation:**
- Server creates successfully
- HTTP endpoint responds

**Commit:** `feat: create ralph-server crate with basic HTTP server`

---

### Task 3.2: Implement Middleware Stack

**Files:** 
- `ralph-server/src/middleware/mod.rs`
- `ralph-server/src/middleware/auth.rs`
- `ralph-server/src/middleware/csrf.rs`
- `ralph-server/src/middleware/rate_limit.rs`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** None  
**Dependencies:** Task 3.1

**Steps:**
1. Create `auth_middleware()` to validate sessions and attach user_id to request extensions
2. Create `CsrfManager` with `generate()` method using HMAC-SHA256 signature
3. Create `csrf_middleware()` to validate tokens on mutations (POST/PUT/DELETE)
4. Create `RateLimiter` with per-IP and per-user token buckets (60 req/min for IP, 300 req/min for users)
5. Add middleware exports to `mod.rs`

**Tests:**
- Test auth middleware with/without valid session
- Test CSRF token generation and validation
- Test rate limiting with repeated requests
- Expected: All middleware functions correctly

**Validation:**
- Auth middleware validates sessions
- CSRF middleware generates and validates tokens
- Rate limiting works per-IP and per-user

**Commit:** `feat: implement middleware stack (auth, CSRF, rate limit)`

---

### Task 3.3: Create HTTP Handlers for Authentication

**Files:** `ralph-server/src/handlers/mod.rs`, `ralph-server/src/handlers/auth.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 3.1, 3.2

**Steps:**
1. Create `register()` handler with validation
2. Create `login()` handler with password verification and session creation
3. Create `logout()` handler with session invalidation
4. Return consistent JSON responses (success, message, user_id, session_token)

**Tests:**
- Register user: `POST /api/auth/register`
- Login: `POST /api/auth/login`
- Logout: `POST /api/auth/logout`
- Expected: All auth endpoints work

**Validation:**
- Registration creates user
- Login returns session token
- Logout invalidates session

**Commit:** `feat: add HTTP handlers for authentication`

---

### Task 3.4: Create HTTP Handlers for Loops

**Files:** `ralph-server/src/handlers/loops.rs`

**Estimated Time:** 2.5 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 3.1, 3.2

**Steps:**
1. Create `list_loops()` handler with pagination
2. Create `create_loop()` handler
3. Create `get_loop()` handler
4. Create `delete_loop()` handler with ownership check
5. Implement loop control handlers: `start_loop()`, `pause_loop()`, `resume_loop()`, `stop_loop()` (stub implementations for now, full in Sprint 4)

**Tests:**
- List loops: `GET /api/loops`
- Create loop: `POST /api/loops`
- Get loop: `GET /api/loops/:id`
- Delete loop: `DELETE /api/loops/:id`
- Expected: All loop endpoints work

**Validation:**
- Loop listing works
- Loop creation works
- Loop retrieval works
- Loop deletion with ownership check works

**Commit:** `feat: add HTTP handlers for loops`

---

### Task 3.5: Create HTTP Handlers for Tasks

**Files:** `ralph-server/src/handlers/tasks.rs`

**Estimated Time:** 1.5 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 3.1, 3.2

**Steps:**
1. Create `list_tasks()` handler
2. Create `create_task()` handler
3. Create `get_task()` handler
4. Create `delete_task()` handler
5. All handlers use proper ownership checks and validation

**Tests:**
- List tasks: `GET /api/loops/:id/tasks`
- Create task: `POST /api/loops/:id/tasks`
- Get task: `GET /api/tasks/:id`
- Delete task: `DELETE /api/tasks/:id`
- Expected: All task endpoints work

**Validation:**
- Task listing by loop works
- Task creation works
- Task retrieval works
- Task deletion works

**Commit:** `feat: add HTTP handlers for tasks`

---

### Task 3.6: Create WebSocket Handler

**Files:** 
- `ralph-server/src/broadcast.rs`
- `ralph-server/src/handlers/websocket.rs`

**Estimated Time:** 2.5 hours  
**Complexity:** Medium  
**Risk:** Medium  
**Dependencies:** Tasks 3.1, 3.2

**Steps:**
1. Create `BroadcastManager` with channels map for WebSocket message broadcasting
2. Define `BroadcastMessage` enum with variants: LoopStatus, IterationComplete, LoopError
3. Implement `subscribe()`, `broadcast()`, `remove_dead_clients()` methods
4. Implement monitoring task to clean up dead clients every 30 seconds
5. Create `websocket_handler()` that accepts WebSocketUpgrade and manages socket lifecycle
6. Handle incoming messages (ping/pong) and send/receive loop updates

**Tests:**
- Connect to WebSocket: `ws://localhost:3000/api/loops/:id/stream`
- Send and receive messages
- Expected: WebSocket connection works, messages flow

**Validation:**
- WebSocket handler accepts connections
- Broadcast manager sends messages to subscribers
- Dead client cleanup works

**Commit:** `feat: add WebSocket handler and broadcast manager`

---

### Task 3.7: Create Askama Templates

**Files:** 
- `ralph-server/templates/base.html`
- `ralph-server/templates/auth/login.html`
- `ralph-server/templates/auth/register.html`
- `ralph-server/templates/loops/index.html`
- `ralph-server/templates/loops/new.html`
- `ralph-server/templates/loops/show.html`
- `ralph-server/templates/tasks/new.html`

**Estimated Time:** 4 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 3.1

**Steps:**
1. Create `base.html` with navbar, Tailwind CSS script, HTMX script
2. Create login form with username/password fields and HTMX form submission
3. Create registration form with username/email/password fields
4. Create loop dashboard with loop list
5. Create loop creation form with all fields
6. Create loop detail page with task list and controls
7. Create task creation form
8. Use consistent Tailwind classes and HTMX attributes

**Tests:**
- Serve templates via HTTP
- Check rendered HTML
- Expected: Templates render correctly with data

**Validation:**
- All templates render
- HTMX attributes present
- Tailwind classes applied

**Commit:** `feat: add Askama templates for all pages`

---

### Task 3.8: Create Router and Connect All Handlers

**Files:** `ralph-server/src/router.rs`, `ralph-server/src/main.rs` (update)

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 3.1-3.7

**Steps:**
1. Create `create_router()` function that builds Axum Router with all routes
2. Define public routes (health, login, register, logout)
3. Define protected routes (all loop and task endpoints, WebSocket)
4. Apply middleware stack (rate_limit, auth, csrf) to protected routes
5. Update `main.rs` to:
   - Initialize database
   - Initialize repositories
   - Initialize services (auth, docker)
   - Initialize code agent with mock provider
   - Create router and start server on port 3000

**Tests:**
- Start server
- Access all routes
- Expected: All routes respond correctly

**Validation:**
- Router connects all handlers
- Middleware applied correctly
- Server starts and handles requests

**Commit:** `feat: create router and connect all handlers`

---

### Task 3.9: Add CSP Headers for Security

**Files:** `ralph-server/src/middleware/security.rs`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 3.2

**Steps:**
1. Create `csp_middleware()` that adds Content-Security-Policy header
2. Set CSP to: `default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'`
3. Add middleware to router

**Tests:**
- Access any page
- Check CSP header in response
- Expected: CSP header present

**Validation:**
- CSP header added to all responses
- Prevents XSS from external scripts

**Commit:** `feat: add CSP headers for XSS prevention`

---

### Task 3.10: Run Integration Tests for Server

**Files:** N/A

**Estimated Time:** 2 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 3.1-3.9

**Steps:**
1. Run all server tests: `cargo test -p ralph-server`
2. Verify all tests pass
3. Check test coverage

**Tests:**
- Run `cargo test -p ralph-server`
- Expected: All tests pass

**Validation:**
- All server tests pass
- Handler tests work
- Middleware tests work

**Commit:** `test: run integration tests for server`

---

### Task 3.11: Add End-to-End Integration Tests

**Files:** `tests/e2e/full_workflow.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** High (requires Docker)  
**Dependencies:** Tasks 3.1-3.10

**Steps:**
1. Create `test_full_workflow()` test that:
   - Starts server in background
   - Registers user
   - Logs in
   - Creates loop with tasks
   - Starts loop
   - Monitors via WebSocket
   - Stops loop
2. Create `test_concurrent_loops()` test that:
   - Creates multiple loops simultaneously
   - Verifies isolation
   - Verifies no resource conflicts

**Tests:**
- Run `cargo test --test e2e`
- Expected: E2E tests pass

**Validation:**
- Full workflow test passes
- Concurrent loops test passes

**Commit:** `test: add end-to-end integration tests`

---

### Task 3.12: Sprint 3 Demo - Complete Web Workflow

**Files:** `demo_sprint3.sh`

**Estimated Time:** 1 hour  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1-3.11

**Steps:**
1. Create `demo_sprint3.sh` (similar to Sprint 2 demo but using web UI instead of API)
2. Run demo
3. Verify complete workflow works in browser

**Tests:**
- Run `bash demo_sprint3.sh`
- Expected: Full web workflow completes

**Validation:**
- Registration works in browser
- Login works in browser
- Loop creation works
- Loop monitoring works via WebSocket

**Commit:** `docs: add Sprint 3 demo script for web workflow`

---

## Sprint 4: Git Integration (P0 - CRITICAL)

**GitHub Labels:** `sprint-4`, `category:services`
**Goal:** Implement automated Git operations for loops - clone, commit, push, and PR creation.

**Estimated Effort:** 40 hours  
**Deliverables:** GitService, GitCredentialsRepository, auto-commit on iterations, auto-PR on completion

**Demo:** Create loop with Git URL → Start loop → Verify PR created automatically

---

### Task 4.1: Create GitCredential Model

**Files:** `ralph-models/src/git_credential.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/git_credential.rs` with `GitCredential` struct (id, user_id, provider, encrypted_token, username, email, created_at, updated_at)
2. Create `CreateGitCredential` struct for validation
3. Export models in `lib.rs`

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- GitCredential model defined
- CreateGitCredential struct for validation

**Commit:** `feat: add GitCredential model`

---

### Task 4.2: Create Git Credentials Table Migration

**Files:** `migrations/006_git_credentials.sql`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 4.1

**Steps:**
1. Create `006_git_credentials.sql` with:
   - `git_credentials` table (id, user_id FK, provider, encrypted_token, username, email, created_at, updated_at)
   - Unique index on user_id + provider

**Tests:**
- Run migration: `sqlite3 ralph.db < migrations/006_git_credentials.sql`
- Verify table created: `sqlite3 ralph.db ".schema git_credentials"`
- Expected: Table created with indexes

**Validation:**
- Table created successfully
- Foreign key to users table
- Unique index on user_id + provider

**Commit:** `feat: add git_credentials table migration`

---

### Task 4.3: Create GitCredentialsRepository

**Files:** `ralph-repositories/src/git_credential.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 4.1, 4.2

**Steps:**
1. Create `GitCredentialsRepository` struct with `db: Database` field
2. Implement `create()` method with encryption (using `encrypt_token()`)
3. Implement `list_by_user()` method
4. Implement `find_by_user_and_provider()` method
5. Implement `delete()` method with ownership check
6. Implement `get_decrypted_token()` method using `decrypt_token()`

**Tests:**
- Create credential: `repo.create(CreateGitCredential {...}).await`
- List credentials: `repo.list_by_user(user_id).await`
- Find by provider: `repo.find_by_user_and_provider(user_id, "github").await`
- Expected: All operations work

**Validation:**
- Credential creation works
- Credential listing works
- Token encryption/decryption works

**Commit:** `feat: add GitCredentialsRepository with encryption`

---

### Task 4.4: Create GitService

**Files:** `ralph-services/src/git.rs`

**Estimated Time:** 4 hours  
**Complexity:** High  
**Risk:** High (Git operations are blocking)  
**Dependencies:** Tasks 2.7, 4.1

**Steps:**
1. Add `git2 = "0.18"` or `gix = "0.58"` dependency to `ralph-services/Cargo.toml`
2. Create `GitService` struct with `repo_path: PathBuf` and `credentials: GitCredentials`
3. Implement `clone_repo()` using `tokio::task::spawn_blocking()` for non-blocking Git clone
4. Implement `create_branch()` to create branch from current HEAD
5. Implement `checkout()` to switch branches
6. Implement `stage_all()` and `stage_files()` to stage changes
7. Implement `commit()` with message
8. Implement `push()` to push to remote
9. Implement `create_pr()` to create pull request via provider API
10. Implement `current_branch()` and `current_commit()` methods

**Tests:**
- Test clone with valid repository URL
- Test branch creation
- Test commit with staged changes
- Test push to remote
- Expected: All Git operations work correctly

**Validation:**
- GitService implements all required methods
- Async wrappers work with spawn_blocking
- All operations complete successfully

**Commit:** `feat: add GitService with async Git operations`

---

### Task 4.5: Integrate GitService into LoopExecutor

**Files:** `ralph-services/src/executor.rs` (update)

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** High  
**Dependencies:** Tasks 2.10, 4.3, 4.4

**Steps:**
1. Add `git_service: Arc<GitService>` field to `LoopExecutor`
2. Update `execute_task()` method to:
   - Check if loop has `git_repo_url`
   - If yes, create branch with pattern substitution: `{loop_id}` and `{timestamp}`
   - Stage all generated files
   - Commit with message: "Ralph Loop {loop_id} - Iteration {iter_num} - Task {task_title}"
   - Push to remote
   - On loop completion (no more tasks), create PR with title and body

**Tests:**
- Start loop with Git URL configured
- Monitor iterations
- Expected: Commits happen automatically, PR created on completion

**Validation:**
- LoopExecutor integrates GitService
- Auto-commit works on each iteration
- Branch creation with pattern substitution works
- Auto-PR creation works on loop completion

**Commit:** `feat: integrate GitService into LoopExecutor for auto-commit and PR creation`

---

### Task 4.6: Add Git Credential Handlers

**Files:** `ralph-server/src/handlers/git.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1, 4.3

**Steps:**
1. Create `list_git_credentials()` handler (requires auth)
2. Create `create_git_credentials()` handler (requires auth + CSRF)
3. Create `delete_git_credentials()` handler (requires auth, ownership check)
4. Return consistent JSON responses

**Tests:**
- List credentials: `GET /api/git/credentials`
- Create credential: `POST /api/git/credentials`
- Delete credential: `DELETE /api/git/credentials/:id`
- Expected: All handlers work correctly

**Validation:**
- Git credential CRUD operations work
- Authentication enforced
- Ownership checks work

**Commit:** `feat: add HTTP handlers for Git credentials management`

---

### Task 4.7: Add Git Routes to Router

**Files:** `ralph-server/src/router.rs` (update)

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 4.6, 3.8

**Steps:**
1. Add Git credential routes to router:
   - `GET /api/git/credentials` - list
   - `POST /api/git/credentials` - create
   - `DELETE /api/git/credentials/:id` - delete
2. Ensure auth and CSRF middleware applied

**Tests:**
- Start server
- Access Git routes
- Expected: All routes respond correctly

**Validation:**
- Git routes integrated into router
- Middleware properly applied

**Commit:** `feat: add Git credential routes to router`

---

### Task 4.8: Create Git Credentials UI Template

**Files:** `ralph-server/templates/git/credentials.html`

**Estimated Time:** 1.5 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 4.6, 4.7

**Steps:**
1. Create `git/credentials.html` with form to add credentials
2. Form fields: provider select (GitHub, GitLab, Bitbucket), personal access token, username, email
3. Add HTMX form submission to `/api/git/credentials`
4. Display list of existing credentials with delete button
5. Use Tailwind classes for styling

**Tests:**
- Render template in browser
- Add credential via form
- Expected: UI works correctly

**Validation:**
- UI allows users to manage Git credentials
- HTMX updates work
- Styling consistent

**Commit:** `feat: add Git credentials UI template`

---

### Task 4.9: Add Encryption Utilities

**Files:** `ralph-services/src/crypto.rs`

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 4.1

**Steps:**
1. Add `aes-gcm = "0.10"` and `aead` dependencies to `ralph-services/Cargo.toml`
2. Define `ENCRYPTION_KEY` from environment variable
3. Implement `encrypt_git_token()` using Aes256Gcm with random nonce
4. Implement `decrypt_git_token()` extracting nonce and ciphertext from base64
5. Combine nonce + ciphertext into base64 for storage
6. Return base64-encoded encrypted value

**Tests:**
- Test encryption: encrypt and decrypt round-trip
- Test with different inputs
- Expected: Encryption/decryption works correctly

**Validation:**
- Encryption utilities work
- Tokens can be encrypted and decrypted
- Random nonce generated each time

**Commit:** `feat: add encryption utilities for Git credential storage`

---

### Task 4.10: Sprint 4 Demo - Automated Git Operations

**Files:** `demo_sprint4.sh`

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** High (requires Git)  
**Dependencies:** Tasks 4.1-4.9

**Steps:**
1. Create `demo_sprint4.sh` that:
   - Starts server
   - Registers user
   - Logs in
   - Creates Git credentials via UI
   - Creates loop with Git repository URL
   - Adds task
   - Starts loop
   - Monitors for automatic commits
   - Verifies PR created automatically
2. Run demo script
3. Verify Git integration works end-to-end

**Tests:**
- Run `bash demo_sprint4.sh`
- Expected: Script completes, PR created automatically

**Validation:**
- Git credentials can be added
- Loop with Git URL executes and commits automatically
- PR created on loop completion
- All Git operations automated successfully

**Commit:** `docs: add Sprint 4 demo script for Git integration`

---

### Task 4.11: Run Integration Tests for Git Integration

**Files:** N/A

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Medium  
**Dependencies:** Tasks 4.1-4.10

**Steps:**
1. Run Git integration tests: `cargo test -p ralph-services git`
2. Verify all tests pass
3. Check test coverage

**Tests:**
- Run `cargo test -p ralph-services`
- Expected: All Git tests pass

**Validation:**
- All Git operations tested
- Integration tests pass
- Coverage acceptable

**Commit:** `test: run integration tests for Git integration`

---

### Task 4.12: Sprint 4 Demo Review

**Files:** N/A

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 4.1-4.11

**Steps:**
1. Review Sprint 4 demo results
2. Verify all acceptance criteria met
3. Document any issues

**Tests:**
- Verify demo script runs successfully
- Expected: All Git features working as specified

**Validation:**
- Sprint 4 complete
- All Git integration features functional
- Demo validates requirements

**Commit:** `docs: complete Sprint 4 review`

---

## Sprint 5: API Key Management (P1 - HIGH)

**GitHub Labels:** `sprint-5`, `category:models`, `category:repositories`, `category:services`
**Goal:** Enable user-specific API key management with encryption and rotation.

**Estimated Effort:** 35 hours  
**Deliverables:** Encryption utilities, user-specific keys, LoopExecutor integration, UI for key management

**Demo:** Add key → Create loop → Verify key is used

---

### Task 5.1: Create ApiKey Model

**Files:** `ralph-models/src/api_key.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/api_key.rs` with `ApiKey` struct (id, user_id, provider, key, is_active, created_at, updated_at)
2. Create `CreateApiKey` struct for validation
3. Create `ApiKeyProvider` enum (Anthropic, OpenAI)
4. Export models in `lib.rs`

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- ApiKey model defined
- CreateApiKey struct for validation
- ApiKeyProvider enum works

**Commit:** `feat: add ApiKey model`

---

### Task 5.2: Create API Keys Table Migration

**Files:** `migrations/007_api_keys.sql`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 5.1

**Steps:**
1. Create `007_api_keys.sql` with:
   - `api_keys` table (id, user_id FK, provider, encrypted_key, is_active, created_at, updated_at)
   - Unique index on user_id + provider

**Tests:**
- Run migration: `sqlite3 ralph.db < migrations/007_api_keys.sql`
- Verify table created: `sqlite3 ralph.db ".schema api_keys"`
- Expected: Table created with indexes

**Validation:**
- Table created successfully
- Foreign key to users table
- Unique index on user_id + provider

**Commit:** `feat: add api_keys table migration`

---

### Task 5.3: Add Encryption Dependencies

**Files:** `ralph-services/Cargo.toml`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 5.1

**Steps:**
1. Add `aes-gcm = "0.10"` dependency to `ralph-services/Cargo.toml`
2. Add `aead = "0.5"` dependency
3. Add `base64 = "0.21"` dependency

**Tests:**
- Run `cargo check -p ralph-services`
- Expected: No compilation errors

**Validation:**
- Encryption dependencies available

**Commit:** `chore: add encryption dependencies (aes-gcm, aead, base64)`

---

### Task 5.4: Create Encryption Utilities

**Files:** `ralph-services/src/crypto.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 5.1, 5.3

**Steps:**
1. Define `ENCRYPTION_KEY` from environment variable
2. Implement `encrypt_api_key()` using Aes256Gcm:
   - Generate random nonce
   - Encrypt plaintext with nonce
   - Combine nonce + ciphertext into base64
3. Implement `decrypt_api_key()` extracting nonce from base64 and decrypting ciphertext

**Tests:**
- Test encryption: encrypt and decrypt round-trip
- Test with different inputs
- Expected: Encryption/decryption works correctly

**Validation:**
- Encryption utilities work
- Tokens can be encrypted and decrypted
- Random nonce generated each time

**Commit:** `feat: add encryption utilities for API key storage`

---

### Task 5.5: Create ApiKeyRepository

**Files:** `ralph-repositories/src/api_key.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 5.1, 5.2, 5.4

**Steps:**
1. Create `ApiKeyRepository` struct with `db: Database` field
2. Implement `create()` method encrypting key before storage using `encrypt_api_key()`
3. Implement `list_by_user()` method decrypting keys for API use
4. Implement `get_active_for_user()` to find active key for provider
5. Implement `deactivate()` method

**Tests:**
- Create key: `repo.create(CreateApiKey {...}).await`
- List keys: `repo.list_by_user(user_id).await`
- Find active: `repo.get_active_for_user(user_id, "anthropic").await`
- Deactivate: `repo.deactivate(id, user_id).await`
- Expected: All operations work correctly

**Validation:**
- API key creation works
- Key listing works
- Active key retrieval works
- Key deactivation works

**Commit:** `feat: add ApiKeyRepository with encryption`

---

### Task 5.6: Update LoopExecutor to Use User-Specific Keys

**Files:** `ralph-services/src/executor.rs` (update)

**Estimated Time:** 2 hours  
**Complexity:** High  
**Risk:** High  
**Dependencies:** Tasks 2.10, 5.5

**Steps:**
1. Add `api_key_repo: Arc<ApiKeyRepository>` field to `LoopExecutor`
2. Update `get_llm_provider()` method to:
   - Get user_id from `loop_.owner_id`
   - Get active API key for user and provider
   - Create ClaudeProvider or OpenAIProvider with user's key
3. Use user-specific key instead of environment variables
4. Handle case where no active key exists (return error)

**Tests:**
- Add API key for user
- Create loop with that provider
- Start loop
- Expected: Loop uses user's API key instead of env var

**Validation:**
- LoopExecutor uses user-specific keys
- No more reliance on environment variables
- Keys are properly retrieved and used

**Commit:** `feat: update LoopExecutor to use user-specific API keys`

---

### Task 5.7: Create API Key Handlers

**Files:** `ralph-server/src/handlers/api_keys.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1, 5.5

**Steps:**
1. Create `list_api_keys()` handler returning key summaries (id, provider, is_active, created_at, key not exposed)
2. Create `create_api_key()` handler requiring auth + CSRF
3. Create `deactivate_api_key()` handler requiring auth and ownership
4. Return consistent JSON responses

**Tests:**
- List keys: `GET /api/keys`
- Create key: `POST /api/keys`
- Deactivate key: `DELETE /api/keys/:id`
- Expected: All handlers work correctly

**Validation:**
- API key CRUD operations work
- Authentication enforced
- Keys not exposed in list response
- Ownership checks work

**Commit:** `feat: add HTTP handlers for API key management`

---

### Task 5.8: Add API Key Routes to Router

**Files:** `ralph-server/src/router.rs` (update)

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 5.7, 3.8

**Steps:**
1. Add API key routes to router:
   - `GET /api/keys` - list
   - `POST /api/keys` - create
   - `DELETE /api/keys/:id` - deactivate
2. Ensure auth and CSRF middleware applied

**Tests:**
- Start server
- Access API key routes
- Expected: All routes respond correctly

**Validation:**
- API key routes integrated into router
- Middleware properly applied

**Commit:** `feat: add API key routes to router`

---

### Task 5.9: Create API Key Management UI

**Files:** `ralph-server/templates/user/keys.html`

**Estimated Time:** 1.5 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 5.7, 5.8

**Steps:**
1. Create `user/keys.html` with form to add API keys
2. Form fields: provider select (Anthropic Claude, OpenAI), API key password input
3. Add HTMX form submission to `/api/keys`
4. Display list of existing keys with deactivate button (key not shown)
5. Use Tailwind classes for styling

**Tests:**
- Render template in browser
- Add key via form
- Deactivate key via button
- Expected: UI works correctly

**Validation:**
- UI allows users to manage API keys
- HTMX updates work
- Styling consistent

**Commit:** `feat: add API key management UI template`

---

### Task 5.10: Sprint 5 Demo - API Key Management

**Files:** `demo_sprint5.sh`

**Estimated Time:** 1 hour  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 5.1-5.9

**Steps:**
1. Create `demo_sprint5.sh` that:
   - Starts server
   - Registers user
   - Logs in
   - Adds API key for provider
   - Creates loop with that provider
   - Starts loop
   - Verifies in logs that user's key is used (not env var)
2. Run demo script
3. Verify API key management works end-to-end

**Tests:**
- Run `bash demo_sprint5.sh`
- Expected: Script completes, user's key is used

**Validation:**
- API keys can be added
- User-specific keys used instead of env vars
- Keys can be deactivated
- LoopExecutor integration verified

**Commit:** `docs: add Sprint 5 demo script for API key management`

---

### Task 5.11: Run Integration Tests for API Key Management

**Files:** N/A

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 5.1-5.10

**Steps:**
1. Run API key management tests: `cargo test -p ralph-repositories api_key`
2. Run tests: `cargo test -p ralph-services crypto`
3. Verify all tests pass
4. Check test coverage

**Tests:**
- Run `cargo test -p ralph-repositories`
- Run `cargo test -p ralph-services`
- Expected: All API key tests pass

**Validation:**
- Encryption/decryption tested
- Repository operations tested
- Integration tests pass
- Coverage acceptable

**Commit:** `test: run integration tests for API key management`

---

## Sprint 6: Enhanced Templates (P2 - LOW)

**GitHub Labels:** `sprint-6`, `category:frontend`, `category:server`
**Goal:** Create missing UI pages for better user experience.

**Estimated Effort:** 25 hours  
**Deliverables:** Home page, user profile, loop templates library, improved navigation

**Demo:** All pages accessible and styled, navigation works

---

### Task 6.1: Create Home/Landing Page

**Files:** `ralph-server/templates/home.html`

**Estimated Time:** 1.5 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 3.1

**Steps:**
1. Create `home.html` with:
   - Welcome message and project overview
   - Quick start cards (register, create loop, view docs)
   - Recent loops summary if logged in
2. Extend `base.html` template
3. Add route `/` to router
4. Create handler to serve home page

**Tests:**
- Access `/` in browser
- Verify content renders
- Expected: Home page displays correctly

**Validation:**
- Home page accessible
- Content renders correctly
- Tailwind styling applied

**Commit:** `feat: add home/landing page`

---

### Task 6.2: Create User Profile/Settings Page

**Files:** `ralph-server/templates/user/profile.html`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 3.1

**Steps:**
1. Create `user/profile.html` with:
   - User information display (username, email, created_at)
   - Settings form (change password, preferences)
   - Session management (active sessions list, logout all)
2. Extend `base.html` template
3. Add route `/profile` to router
4. Create handler to serve profile page with user data

**Tests:**
- Access `/profile` in browser
- Verify content renders
- Test settings form
- Expected: Profile page works correctly

**Validation:**
- Profile page accessible
- User info displays
- Settings work
- Session management functional

**Commit:** `feat: add user profile and settings page`

---

### Task 6.3: Create Loop Templates Library Page

**Files:** `ralph-server/templates/loops/templates.html`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1

**Steps:**
1. Create `loops/templates.html` with:
   - Public and user's template lists
   - Template cards with name, description, use button
   - "Create Template" button
2. Extend `base.html` template
3. Add route `/templates` to router
4. Create handlers for template CRUD (list, create, use)

**Tests:**
- Access `/templates` in browser
- Verify content renders
- Expected: Templates library works

**Validation:**
- Templates library accessible
- Public templates visible
- User templates shown
- Create/use functionality works

**Commit:** `feat: add loop templates library page`

---

### Task 6.4: Improve Navigation and Styling

**Files:** `ralph-server/templates/base.html` (update), `ralph-server/src/router.rs` (update)

**Estimated Time:** 1.5 hours  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 6.1-6.3

**Steps:**
1. Update `base.html` navbar with links to:
   - Home, Loops, Templates, Profile, Settings
2. Add active state highlighting in navbar
3. Update router to add routes for new pages
4. Ensure consistent Tailwind styling across all pages

**Tests:**
- Access all pages
- Verify navigation works
- Expected: Navigation functional

**Validation:**
- Navigation consistent
- All links work
- Active state highlighted

**Commit:** `feat: improve navigation and add links to new pages`

---

### Task 6.5: Add User Preferences Storage

**Files:** `ralph-models/src/user.rs` (update), `migrations/008_user_preferences.sql`

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 1.1, 1.8

**Steps:**
1. Add `preferences` column to users table (JSON field)
2. Create `008_user_preferences.sql` migration
3. Update `User` model with `preferences: Option<serde_json::Value>` field
4. Implement preferences update in `UserRepository`

**Tests:**
- Update user preferences
- Retrieve user preferences
- Expected: Preferences stored and retrieved correctly

**Validation:**
- User preferences can be stored
- Migration applies successfully
- Preferences accessible

**Commit:** `feat: add user preferences storage and migration`

---

### Task 6.6: Sprint 6 Demo - Enhanced Templates

**Files:** `demo_sprint6.sh`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 6.1-6.5

**Steps:**
1. Create `demo_sprint6.sh` that:
   - Starts server
   - Registers user
   - Accesses home page
   - Accesses profile page
   - Accesses templates library
   - Verifies all pages work
2. Run demo script
3. Verify enhanced templates work correctly

**Tests:**
- Run `bash demo_sprint6.sh`
- Expected: Script completes, all pages accessible

**Validation:**
- All new pages accessible
- Navigation works
- Styling consistent

**Commit:** `docs: add Sprint 6 demo script for enhanced templates`

---

## Sprint 7: OAuth Authentication (P2 - MEDIUM)

**GitHub Labels:** `sprint-7`, `category:services`, `category:server`, `category:models`
**Goal:** Enable social login with OAuth providers.

**Estimated Effort:** 30 hours  
**Deliverables:** OAuth flow, token storage, social login UI

**Demo:** Login with GitHub/Google → Verify session created

---

### Task 7.1: Create OAuth Token Model

**Files:** `ralph-models/src/oauth_token.rs`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 1.1

**Steps:**
1. Create `ralph-models/src/oauth_token.rs` with `OAuthToken` struct (id, user_id, provider, provider_user_id, access_token, refresh_token, expires_at, scope, created_at, updated_at)
2. Export models in `lib.rs`

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: No compilation errors

**Validation:**
- OAuth token model defined

**Commit:** `feat: add OAuth token model`

---

### Task 7.2: Add OAuth Token Columns to Users Table

**Files:** `migrations/009_oauth_tokens_users.sql`

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 7.1

**Steps:**
1. Create `009_oauth_tokens_users.sql` with:
   - ALTER TABLE users ADD COLUMN oauth_provider TEXT
   - ALTER TABLE users ADD COLUMN oauth_provider_id TEXT
   - Create indexes on oauth columns

**Tests:**
- Run migration: `sqlite3 ralph.db < migrations/009_oauth_tokens_users.sql`
- Verify columns added: `sqlite3 ralph.db ".schema users"`
- Expected: Columns added successfully

**Validation:**
- OAuth columns added to users table
- Indexes created
- Migration is idempotent

**Commit:** `feat: add OAuth token columns to users table`

---

### Task 7.3: Create OAuth Token Repository

**Files:** `ralph-repositories/src/oauth_token.rs`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 1.7, 7.1, 7.2

**Steps:**
1. Create `OAuthTokenRepository` struct with `db: Database` field
2. Implement `create()` method to store OAuth tokens
3. Implement `find_by_user_and_provider()` method
4. Implement `delete()` method for unlinking

**Tests:**
- Create token: `repo.create(user_id, provider, token).await`
- Find by user and provider: `repo.find_by_user_and_provider(user_id, "github").await`
- Delete: `repo.delete(id, user_id).await`
- Expected: All operations work correctly

**Validation:**
- OAuth token storage works
- CRUD operations functional
- Ownership checks work

**Commit:** `feat: add OAuthTokenRepository`

---

### Task 7.4: Add OAuth Dependencies

**Files:** `ralph-services/Cargo.toml`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 7.1

**Steps:**
1. Add `oauth2 = "4.4"` dependency to `ralph-services/Cargo.toml`
2. Add `reqwest = "0.11"` dependency for HTTP requests

**Tests:**
- Run `cargo check -p ralph-services`
- Expected: No compilation errors

**Validation:**
- OAuth dependencies available

**Commit:** `chore: add OAuth dependencies (oauth2, reqwest)`

---

### Task 7.5: Create OAuth Service

**Files:** `ralph-services/src/oauth.rs`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** Medium  
**Dependencies:** Tasks 7.1, 7.4

**Steps:**
1. Create `OAuthProvider` struct with configuration (client_id, client_secret, authorization_url, token_url, scopes)
2. Create `OAuthService` struct with providers HashMap
3. Implement `get_authorization_url()` method generating auth URL with state
4. Implement `exchange_code()` method exchanging code for access token via HTTP POST
5. Implement `refresh_token()` method for expired tokens
6. Implement `get_profile()` method calling provider's user profile API

**Tests:**
- Test auth URL generation
- Test code exchange (mock with real provider API)
- Expected: OAuth flow works correctly

**Validation:**
- OAuth flow implemented
- Authorization URLs generated correctly
- Token exchange works
- Profile retrieval works

**Commit:** `feat: add OAuth service with authorization code grant flow`

---

### Task 7.6: Create OAuth Handlers

**Files:** `ralph-server/src/handlers/oauth.rs`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** Medium  
**Dependencies:** Tasks 3.1, 3.2, 7.5

**Steps:**
1. Create `oauth_authorize()` handler:
   - Generate state and store in session
   - Build auth URL via OAuthService
   - Redirect to provider
2. Create `oauth_callback()` handler:
   - Verify state from session
   - Exchange code for access token
   - Get user profile from provider
   - Find or create user based on provider_user_id
   - Store OAuth token
   - Create session
   - Redirect to `/loops`
3. Create `oauth_unlink()` handler:
   - Delete OAuth token
   - Return success response
4. Return consistent JSON responses and redirects

**Tests:**
- Start OAuth flow: `GET /auth/oauth/github/authorize`
- Callback from provider: `GET /auth/oauth/github/callback?code=...&state=...`
- Unlink: `POST /api/auth/oauth/github/unlink`
- Expected: OAuth flow works end-to-end

**Validation:**
- OAuth authorization redirects correctly
- Callback processes tokens and creates session
- Unlinking works
- CSRF state verification works

**Commit:** `feat: add OAuth handlers (authorize, callback, unlink)`

---

### Task 7.7: Add OAuth Routes to Router

**Files:** `ralph-server/src/router.rs` (update)

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 7.6, 3.8

**Steps:**
1. Add OAuth routes to router:
   - `GET /auth/oauth/:provider/authorize`
   - `GET /auth/oauth/:provider/callback`
   - `POST /api/auth/oauth/:provider/unlink`
2. Ensure auth and CSRF middleware properly applied

**Tests:**
- Start server
- Access OAuth routes
- Expected: All routes respond correctly

**Validation:**
- OAuth routes integrated into router
- Middleware properly applied
- OAuth flow accessible

**Commit:** `feat: add OAuth routes to router`

---

### Task 7.8: Update Login Template with OAuth Buttons

**Files:** `ralph-server/templates/auth/login.html` (update)

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 7.7

**Steps:**
1. Update `login.html` template to add OAuth login buttons before traditional form
2. Add GitHub login button
3. Add Google login button
4. Add separator "or" between OAuth and traditional login
5. Keep existing username/password form

**Tests:**
- Access login page
- Verify OAuth buttons render
- Click OAuth button (verify redirect)
- Expected: OAuth login buttons work

**Validation:**
- OAuth buttons displayed
- Links correct
- Traditional login still functional

**Commit:** `feat: add OAuth login buttons to login template`

---

### Task 7.9: Update Auth Middleware for OAuth

**Files:** `ralph-server/src/middleware/auth.rs` (update)

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 7.3, 7.6

**Steps:**
1. Update `auth_middleware()` to support OAuth sessions
2. Check for OAuth token in session first, fall back to user_id if OAuth not used
3. Pass OAuth information to request extensions (oauth_provider, oauth_provider_id)

**Tests:**
- Test with OAuth session
- Test with regular session
- Expected: Auth middleware supports both

**Validation:**
- OAuth sessions validated
- Regular sessions still work
- Extensions provide OAuth info

**Commit:** `feat: update auth middleware to support OAuth sessions`

---

### Task 7.10: Sprint 7 Demo - OAuth Authentication

**Files:** `demo_sprint7.sh`

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 7.1-7.9

**Steps:**
1. Create `demo_sprint7.sh` that:
   - Starts server
   - Registers user (traditional)
   - Clicks OAuth login button (GitHub)
   - Authorizes with provider
   - Callback creates session
   - Redirects to loops
   - Verifies OAuth login works end-to-end
2. Run demo script
3. Verify OAuth authentication works correctly

**Tests:**
- Run `bash demo_sprint7.sh`
- Expected: Script completes, OAuth login successful

**Validation:**
- OAuth flow works end-to-end
- Session created correctly
- Traditional login still works
- User linked to OAuth provider

**Commit:** `docs: add Sprint 7 demo script for OAuth authentication`

---

### Task 7.11: Run Integration Tests for OAuth

**Files:** N/A

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 7.1-7.10

**Steps:**
1. Run OAuth tests: `cargo test -p ralph-services oauth`
2. Run tests: `cargo test -p ralph-repositories oauth_token`
3. Verify all tests pass
4. Check test coverage

**Tests:**
- Run `cargo test -p ralph-repositories`
- Run `cargo test -p ralph-services`
- Expected: All OAuth tests pass

**Validation:**
- OAuth service tested
- Token storage tested
- Handlers tested
- Integration tests pass
- Coverage acceptable

**Commit:** `test: run integration tests for OAuth authentication`

---

## Sprint 8: Advanced Task Features (P3 - LOW)

**GitHub Labels:** `sprint-8`, `category:frontend`, `category:server`, `category:services`
**Goal:** Enhance task management UX with visual features.

**Estimated Effort:** 25 hours  
**Deliverables:** Drag & drop reordering, Kanban board, CSV import/export

**Demo:** Visual task management works

---

### Task 8.1: Add Task Position Column

**Files:** `migrations/010_task_position.sql`

**Estimated Time:** 30 minutes  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** None

**Steps:**
1. Create `010_task_position.sql` with:
   - ALTER TABLE tasks ADD COLUMN position INTEGER DEFAULT 0
   - CREATE INDEX idx_tasks_position ON tasks(loop_id, position)

**Tests:**
- Run migration: `sqlite3 ralph.db < migrations/010_task_position.sql`
- Verify column added: `sqlite3 ralph.db ".schema tasks"`
- Expected: Column added successfully, index created

**Validation:**
- Task position column added
- Index created
- Migration is idempotent

**Commit:** `feat: add task position column and index`

---

### Task 8.2: Create Task Position Update Handler

**Files:** `ralph-server/src/handlers/tasks.rs` (update)

**Estimated Time:** 1 hour  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1, 3.2, 8.1

**Steps:**
1. Create `update_task_position()` handler requiring auth and ownership check
2. Update task position in repository
3. Return success response

**Tests:**
- Create tasks with different positions
- Update positions via API
- Expected: Position updates work correctly

**Validation:**
- Task position can be updated
- Ownership checks enforced
- API endpoint works

**Commit:** `feat: add task position update handler`

---

### Task 8.3: Add Drag & Drop to Task List Template

**Files:** `ralph-server/templates/tasks/list.html` (update)

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.7, 8.2

**Steps:**
1. Update `tasks/list.html` to include SortableJS library
2. Add `data-task-id` and `data-position` attributes to task elements
3. Add HTMX listeners for drag end event to call position update API
4. Configure SortableJS with animation
5. Ensure styling with `cursor-move` class

**Tests:**
- Render task list in browser
- Drag tasks to reorder
- Verify positions updated
- Expected: Drag & drop works correctly

**Validation:**
- Drag & drop functional
- Positions persist
- Visual feedback provided

**Commit:** `feat: add drag & drop task reordering to UI`

---

### Task 8.4: Create Kanban Board Template

**Files:** `ralph-server/templates/tasks/kanban.html`

**Estimated Time:** 2.5 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1, 3.7

**Steps:**
1. Create `tasks/kanban.html` with 4 columns (To Do, In Progress, Completed, Failed)
2. Add `data-status-column` attributes to column containers
3. Use SortableJS for drag & drop between columns
4. Add HTMX listeners for drag events to update task status
5. Style columns with different background colors

**Tests:**
- Render Kanban board in browser
- Drag tasks between columns
- Verify statuses update
- Expected: Kanban board works correctly

**Validation:**
- Kanban board displays tasks by status
- Drag & drop between columns works
- Status updates persist

**Commit:** `feat: add Kanban board template with drag & drop`

---

### Task 8.5: Add Task Status Update Handler

**Files:** `ralph-server/src/handlers/tasks.rs` (update)

**Estimated Time:** 1 hour  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 3.1, 3.2, 8.4

**Steps:**
1. Create `update_task_status()` handler requiring auth and ownership
2. Update task status in repository
3. Broadcast status change via WebSocket

**Tests:**
- Create tasks, update statuses
- Verify WebSocket receives updates
- Expected: Status updates work correctly

**Validation:**
- Task status can be updated
- WebSocket broadcasts status changes
- Real-time updates functional

**Commit:** `feat: add task status update handler with WebSocket broadcast`

---

### Task 8.6: Create CSV Parser/Generator

**Files:** `ralph-server/src/handlers/tasks.rs` (update), `ralph-server/Cargo.toml` (update)

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Tasks 3.1

**Steps:**
1. Add `csv = "1.3"` dependency to `ralph-server/Cargo.toml`
2. Implement `export_tasks()` handler generating CSV with task fields
3. Create `import_tasks()` handler parsing uploaded CSV and creating tasks
4. Validate CSV structure (field validation)
5. Set appropriate headers (Content-Type: text/csv, Content-Disposition)

**Tests:**
- Export tasks to CSV
- Import tasks from CSV
- Verify all tasks created
- Expected: Import/export works correctly

**Validation:**
- CSV export generates valid CSV
- CSV import parses and creates tasks
- Fields validated properly

**Commit:** `feat: add task CSV import/export functionality`

---

### Task 8.7: Add Import/Export UI to Templates

**Files:** `ralph-server/templates/tasks/list.html` (update)

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Tasks 8.6, 8.7

**Steps:**
1. Add "Export CSV" button to task list template
2. Add HTMX handler to download CSV file
3. Add "Import CSV" button with file input
4. Add HTMX handler to upload CSV file and call import API

**Tests:**
- Export tasks to CSV
- Import tasks from CSV file
- Verify all tasks created
- Expected: Import/export UI works correctly

**Validation:**
- Export button triggers download
- Import button triggers file upload
- CSV parsing and task creation works

**Commit:** `feat: add task import/export UI to templates`

---

### Task 8.8: Sprint 8 Demo - Advanced Task Features

**Files:** `demo_sprint8.sh`

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 8.1-8.7

**Steps:**
1. Create `demo_sprint8.sh` that:
   - Starts server
   - Registers and logs in
   - Creates loop with tasks
   - Reorders tasks via drag & drop
   - Moves tasks between Kanban columns
   - Exports tasks to CSV
   - Imports tasks from CSV
   - Verifies all advanced features work
2. Run demo script
3. Verify advanced task management features work correctly

**Tests:**
- Run `bash demo_sprint8.sh`
- Expected: Script completes, all features work

**Validation:**
- Drag & drop reordering works
- Kanban board functional
- CSV import/export works
- All advanced task features functional

**Commit:** `docs: add Sprint 8 demo script for advanced task features`

---

### Task 8.9: Run Integration Tests for Advanced Task Features

**Files:** N/A

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 8.1-8.8

**Steps:**
1. Run tests: `cargo test -p ralph-repositories task`
2. Verify position updates work
3. Verify CSV export/import works
4. Check test coverage

**Tests:**
- Run `cargo test -p ralph-repositories`
- Run `cargo test -p ralph-server`
- Expected: All tests pass

**Validation:**
- Task position updates tested
- CSV export/import tested
- Integration tests pass
- Coverage acceptable

**Commit:** `test: run integration tests for advanced task features`

---

## Sprint 9: Production Readiness

**GitHub Labels:** `sprint-9`, `category:services`, `category:server`, `category:repositories`
**Goal:** Prepare platform for production deployment with monitoring, session persistence, PostgreSQL migration, and security hardening.

**Estimated Effort:** 40 hours  
**Deliverables:** Monitoring system, Redis session storage, PostgreSQL support, enhanced security

**Demo:** Load test with 10+ concurrent loops

---

### Task 9.1: Add Monitoring/Metrics Infrastructure

**Files:** `ralph-server/src/metrics.rs`, `ralph-server/src/middleware/monitoring.rs`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** None  
**Dependencies:** Task 3.1

**Steps:**
1. Add `prometheus = "0.13"` dependency to `ralph-server/Cargo.toml`
2. Create `Metrics` struct with counters and histograms:
   - `loops_created_total` (Counter)
   - `loop_duration_seconds` (Histogram)
   - `tasks_completed_total` (Counter)
   - `task_duration_seconds` (Histogram)
   - `active_loops` (Gauge)
   - `database_connections` (Gauge)
   - `websocket_connections` (Gauge)
3. Implement middleware to expose `/metrics` endpoint with Prometheus format

**Tests:**
- Start server
- Access `/metrics` endpoint
- Verify metrics collected
- Expected: Metrics collection works

**Validation:**
- Metrics defined and tracked
- Metrics endpoint accessible
- Prometheus format correct

**Commit:** `feat: add monitoring/metrics infrastructure with Prometheus`

---

### Task 9.2: Add Session Persistence (Redis)

**Files:** `ralph-services/Cargo.toml`, `ralph-services/src/session.rs`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** Medium  
**Dependencies:** Task 2.7

**Steps:**
1. Add `redis = "0.24"` and `deadpool-redis = "0.12"` dependencies to `ralph-services/Cargo.toml`
2. Create `SessionStore` struct with Redis connection pool
3. Implement `RedisSessionStore` with:
   - `create_session()` storing user_id with TTL
   - `validate_session()` checking and refreshing TTL
   - `delete_session()` removing from Redis
   - `cleanup_expired_sessions()` removing entries past TTL
4. Update `AuthService` to use `SessionStore` instead of in-memory HashMap

**Tests:**
- Test session creation with Redis
- Test session validation
- Test session deletion
- Expected: Session persistence works with Redis

**Validation:**
- Sessions persist in Redis
- Session expiration works
- Cleanup of expired sessions functional
- Session management more reliable than in-memory

**Commit:** `feat: add Redis session persistence with TTL and cleanup`

---

### Task 9.3: PostgreSQL Migration Support

**Files:** `ralph-repositories/src/database.rs` (update), `migrations/011_postgres.sql`

**Estimated Time:** 3 hours  
**Complexity:** High  
**Risk:** High  
**Dependencies:** Task 1.7

**Steps:**
1. Add `sqlx-postgres = { version = "0.8", features = ["runtime-tokio", "chrono"] }` feature flag
2. Create `011_postgres.sql` migration switching database connection to PostgreSQL
3. Update `Database::new()` to detect dialect from DATABASE_URL and use appropriate SqlitePool or PgPool
4. Test PostgreSQL connection and migrations
5. Update documentation with PostgreSQL setup instructions

**Tests:**
- Test with DATABASE_URL=postgresql://...
- Run migrations
- Verify all operations work with PostgreSQL
- Expected: PostgreSQL support works correctly

**Validation:**
- PostgreSQL migrations apply successfully
- Database operations work with PostgreSQL
- Migration path documented

**Commit:** `feat: add PostgreSQL migration support with feature flags`

---

### Task 9.4: Add Enhanced Security Headers

**Files:** `ralph-server/src/middleware/security.rs` (update)

**Estimated Time:** 1.5 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 3.2

**Steps:**
1. Update security middleware to add:
   - `X-Content-Type-Options: nosniff`
   - `X-Frame-Options: DENY`
   - `Strict-Transport-Security` (max-age=31536000)
   - `X-XSS-Protection: 1; mode=block`
2. Add middleware to router for all routes

**Tests:**
- Access any page
- Check security headers in response
- Expected: Security headers applied correctly

**Validation:**
- All security headers present
- CSP, XSS protection, frame protection, transport security enabled
- Enhanced security posture

**Commit:** `feat: add enhanced security headers (CSP, XSS, frame, transport)`

---

### Task 9.5: Add Docker Security Hardening

**Files:** `ralph-services/src/docker.rs` (update)

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 2.9

**Steps:**
1. Update `create_container()` to add security options:
   - `security_opt: vec!["no-new-privileges".to_string()]` - Drop all Linux capabilities
   - `cap_drop: vec!["ALL".to_string()]` - Drop all capabilities
   - `readonly_rootfs: true` - Read-only filesystem
2. Implement seccomp profile with default-deny policy
3. Ensure no access to host filesystem except mounted volumes
4. Add user namespace for cgroups (CPU, memory limits)

**Tests:**
- Create container
- Execute command requiring privilege
- Verify privilege drop works
- Expected: Container security hardened

**Validation:**
- Containers created with minimal privileges
- Seccomp profile applied
- No host filesystem access

**Commit:** `feat: add Docker security hardening (seccomp, capabilities, readonly)`

---

### Task 9.6: Add Comprehensive Logging

**Files:** `ralph-server/src/main.rs` (update), `ralph-repositories/src/*.rs` (update), `ralph-services/src/*.rs` (update)

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** None  
**Dependencies:** Task 3.1

**Steps:**
1. Add structured logging with `tracing::info!()`, `warn!()`, `error!()`, `debug!()`
2. Add unique request IDs for request tracing
3. Add user ID to all log entries
4. Add timing spans for critical operations
5. Ensure logs include loop_id, task_id, iteration_id where applicable
6. Configure log levels via RUST_LOG

**Tests:**
- Start server
- Perform various operations
- Verify logs include all required information
- Expected: Comprehensive logging works

**Validation:**
- Structured logging implemented
- Request tracing works
- User tracking in logs
- Timing spans added

**Commit:** `feat: add comprehensive logging with tracing and request IDs`

---

### Task 9.7: Add Health Check Improvements

**Files:** `ralph-server/src/handlers/health.rs`, `ralph-server/src/router.rs` (update)

**Estimated Time:** 1 hour  
**Complexity:** Low  
**Risk:** None  
**Dependencies:** Task 9.1

**Steps:**
1. Create `health_check()` handler returning:
   - Overall system status (OK/DEGRADED)
   - Database connectivity status
   - Docker daemon connectivity status
   - Redis connectivity status (if used)
2. Add metrics summary to health check
3. Update router with health route

**Tests:**
- Access `/health` endpoint
- Verify all components checked
- Expected: Health check comprehensive

**Validation:**
- All components checked
- Degraded status detectable
- Metrics included

**Commit:** `feat: add improved health checks with component status and metrics`

---

### Task 9.8: Sprint 9 Demo - Production Readiness

**Files:** `demo_sprint9.sh`

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** High  
**Dependencies:** Tasks 9.1-9.7

**Steps:**
1. Create `demo_sprint9.sh` that:
   - Starts server with PostgreSQL
   - Registers user
   - Logs in
   - Creates multiple loops (10+)
   - Starts loops concurrently
   - Monitors metrics via `/metrics`
   - Checks health status
   - Verifies all production features work
2. Run demo script
3. Verify production readiness features work correctly

**Tests:**
- Run `bash demo_sprint9.sh`
- Expected: Script completes, all features work under load

**Validation:**
- Monitoring functional
- Session persistence works
- PostgreSQL connection works
- Security headers applied
- Docker hardening works
- Health checks comprehensive
- Load test passes

**Commit:** `docs: add Sprint 9 demo script for production readiness`

---

### Task 9.9: Run Integration Tests for Production Readiness

**Files:** N/A

**Estimated Time:** 2 hours  
**Complexity:** Medium  
**Risk:** Low  
**Dependencies:** Tasks 9.1-9.8

**Steps:**
1. Run all production readiness tests
2. Verify all tests pass
3. Check test coverage
4. Document any known limitations

**Tests:**
- Run `cargo test -p ralph-repositories`
- Run `cargo test -p ralph-services`
- Run `cargo test -p ralph-server`
- Expected: All tests pass

**Validation:**
- All production readiness features tested
- Integration tests pass
- Coverage acceptable
- Documented limitations

**Commit:** `test: run integration tests for production readiness`

---

## Conclusion

This sprint breakdown provides a comprehensive plan to complete the remaining 35% of Ralph Loop Manager in **9 sprints** with **137 atomic, commitable tasks**. Each sprint delivers a **demoable piece of software** that builds incrementally on previous work.

### Key Milestones

**Sprint 0-3 (Foundation):** Complete working foundation (65% of project)  
**Sprint 4 (Git Integration):** Critical automation of Git operations  
**Sprint 5 (API Key Management):** User-specific API keys with encryption  
**Sprint 6 (Enhanced Templates):** Improved UX with additional pages  
**Sprint 7 (OAuth Authentication):** Social login support  
**Sprint 8 (Advanced Task Features):** Enhanced task management UX  
**Sprint 9 (Production Readiness):** Monitoring, session persistence, PostgreSQL, security hardening

### Implementation Priority

1. **Sprint 4 (Git Integration)** - P0 CRITICAL - Must be completed for core value proposition
2. **Sprint 5 (API Key Management)** - P1 HIGH - Security and operational necessity
3. **Sprint 6 (Enhanced Templates)** - P2 MEDIUM - User experience improvement
4. **Sprint 7 (OAuth Authentication)** - P2 MEDIUM - User adoption
5. **Sprint 8 (Advanced Task Features)** - P3 LOW - UX enhancement
6. **Sprint 9 (Production Readiness)** - Enables production deployment

### Total Estimated Effort

- **Sprint 0:** 8 hours
- **Sprint 1:** 40 hours
- **Sprint 2:** 60 hours
- **Sprint 3:** 50 hours
- **Sprint 4:** 40 hours (CRITICAL)
- **Sprint 5:** 35 hours (HIGH)
- **Sprint 6:** 25 hours (MEDIUM)
- **Sprint 7:** 30 hours (MEDIUM)
- **Sprint 8:** 25 hours (LOW)
- **Sprint 9:** 40 hours

**Total:** ~353 hours (9 weeks)

---

**Document Version:** 1.0  
**Last Updated:** 2026-01-18  
**Next Review:** After Sprint 4 completion (Git Integration)
