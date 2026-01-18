# Ralph Loop Manager - Sprint Breakdown (Revised)

> Atomic, commitable tasks with tests. Each sprint = demoable software.
> Applied Oracle review feedback (P0+P1 fixes).

---

<!-- ## Prerequisites -->

<!-- **Before starting:** -->
<!-- 1. Docker daemon installed and running -->
<!-- 2. Obtain ANTHROPIC_API_KEY from https://console.anthropic.com/ -->
<!-- 3. Obtain OPENAI_API_KEY from https://platform.openai.com/ -->
<!-- 4. Rust 1.85+ installed -->

---

## Sprint 1: Foundation - Workspace & Database
**Goal:** Working workspace, models, and database with full CRUD including iterations and files.

### Task 1.1: Initialize Rust Workspace
**Files:** `Cargo.toml`, `.gitignore`, `README.md`

**Steps:**
1. Create workspace `Cargo.toml` with members: ralph-models, ralph-repositories, ralph-agent, ralph-services, ralph-server
2. Configure workspace dependencies (tokio, serde, sqlx, uuid, chrono, anyhow, thiserror, tracing)
3. Create `.gitignore` (target/, *.db, .env, var/, .DS_Store)
4. Create basic README.md with project description and prerequisites

**Tests:**
- Run `cargo check`
- Expected: Compiles successfully

**Validation:** Workspace compiles, all members declared.

**Commit:** `feat: initialize Rust workspace with dependencies`

---

### Task 1.2: Create ralph-models Crate
**Files:** `ralph-models/Cargo.toml`, `ralph-models/src/lib.rs`

**Steps:**
1. Create `ralph-models/Cargo.toml` with workspace dependencies
2. Create `ralph-models/src/lib.rs` with module declarations (user, loop_, task)

**Tests:**
- Run `cargo check -p ralph-models`
- Expected: Compiles with unused module warnings

**Validation:** Crate structure in place.

**Commit:** `feat: create ralph-models crate structure`

---

### Task 1.3: Implement User Model
**Files:** `ralph-models/src/user.rs`

**Steps:**
1. Create `User` struct with fields: id, username, email, password_hash, created_at
2. Implement `User::new()` constructor with UUID generation
3. Create `CreateUser` and `LoginUser` structs as DTOs
4. Derive Serialize, Deserialize, Debug traits

**Tests:**
- Add unit tests for User::new() validation
- Add unit tests for UUID generation
- Run `cargo test -p ralph-models user`
- Expected: All tests pass

**Validation:** User model compiles, tests pass.

**Commit:** `feat: add User model with DTOs and tests`

---

### Task 1.4: Implement Loop Model
**Files:** `ralph-models/src/loop_.rs`

**Steps:**
1. Create `Loop` struct with all fields (id, name, description, prd, owner_id, provider, model, docker_image, limits, status, iteration info, git config, timestamps)
2. Create `CreateLoop` DTO with defaults (docker_image, cpu_limit, memory_limit, max_iterations, iteration_timeout, iteration_delay, git_branch_pattern)
3. Implement `Loop::new()` constructor
4. Create `LoopStatus` enum (Created, Running, Paused, Completed, Error)
5. Implement `Display` for LoopStatus
6. Implement `FromStr` for LoopStatus

**Tests:**
- Add unit tests for Loop::new() with defaults
- Add unit tests for LoopStatus serialization/deserialization
- Run `cargo test -p ralph-models loop`
- Expected: All tests pass

**Validation:** Loop model compiles, status enum works, tests pass.

**Commit:** `feat: add Loop model with status enum and tests`

---

### Task 1.5: Implement Task Model
**Files:** `ralph-models/src/task.rs`

**Steps:**
1. Create `Task` struct with fields (id, loop_id, title, description, status, priority, parent_task_id, created_by, iteration_id, timestamps, error_message)
2. Create `CreateTask` DTO with defaults (priority, parent_task_id)
3. Implement `Task::new()` constructor
4. Create `TaskStatus` enum (Pending, InProgress, Completed, Failed, Cancelled)
5. Implement `Display` for TaskStatus
6. Implement `FromStr` for TaskStatus

**Tests:**
- Add unit tests for Task::new() with defaults
- Add unit tests for TaskStatus serialization/deserialization
- Run `cargo test -p ralph-models task`
- Expected: All tests pass

**Validation:** Task model compiles, status enum works, tests pass.

**Commit:** `feat: add Task model with status enum and tests`

---

### Task 1.6: Update Models Exports
**Files:** `ralph-models/src/lib.rs`

**Steps:**
1. Update lib.rs to export User, CreateUser, LoginUser
2. Export Loop, CreateLoop, LoopStatus
3. Export Task, CreateTask, TaskStatus

**Tests:**
- Run `cargo test -p ralph-models`
- Expected: All tests pass

**Validation:** All models public and accessible.

**Commit:** `refactor: export all public models from ralph-models`

---

### Task 1.7a: Create Users Table Migration
**Files:** `migrations/001_users.sql`

**Steps:**
1. Create `migrations/001_users.sql` with:
   - users table (id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, email TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, created_at TEXT NOT NULL)
   - Index on username for faster lookups

**Tests:**
- Manually verify SQL syntax
- Check table structure
- Expected: Valid SQL, no syntax errors

**Validation:** Users table migration is valid.

**Commit:** `feat: add users table migration`

---

### Task 1.7b: Create Loops Table Migration
**Files:** `migrations/002_loops.sql`

**Steps:**
1. Create `migrations/002_loops.sql` with:
   - loops table (all Loop fields with FK to users)
   - Index on owner_id, status

**Tests:**
- Manually verify SQL syntax
- Check FK relationship
- Expected: Valid SQL, FKs defined correctly

**Validation:** Loops table migration is valid.

**Commit:** `feat: add loops table migration`

---

### Task 1.7c: Create Tasks Table Migration
**Files:** `migrations/003_tasks.sql`

**Steps:**
1. Create `migrations/003_tasks.sql` with:
   - tasks table (all Task fields with FK to loops and tasks)
   - Indexes: (loop_id, status), (status, priority DESC), (parent_task_id)

**Tests:**
- Manually verify SQL syntax
- Check FK relationships
- Verify indexes
- Expected: Valid SQL, FKs and indexes defined

**Validation:** Tasks table migration is valid.

**Commit:** `feat: add tasks table migration`

---

### Task 1.7d: Create Iterations and Files Tables Migration
**Files:** `migrations/004_iterations_files.sql`

**Steps:**
1. Create `migrations/004_iterations_files.sql` with:
   - iterations table (id, loop_id, task_id, iteration_number, output TEXT, error TEXT, status, started_at, completed_at, tokens_used, FKs to loops and tasks)
   - files table (id, iteration_id, path, content_hash, size, file_type, created_at, FK to iterations)
   - Index: (loop_id, iteration_number), (iteration_id)

**Tests:**
- Manually verify SQL syntax
- Check FK relationships
- Verify indexes
- Expected: Valid SQL, FKs and indexes defined

**Validation:** Iterations and files tables migrations are valid.

**Commit:** `feat: add iterations and files tables migration`

---

### Task 1.8: Create ralph-repositories Crate
**Files:** `ralph-repositories/Cargo.toml`, `ralph-repositories/src/lib.rs`

**Steps:**
1. Create `ralph-repositories/Cargo.toml` with dependencies (ralph-models, sqlx, anyhow, uuid, chrono, async-trait)
2. Create `ralph-repositories/src/lib.rs` with module declarations (user, loop_, task, iteration, file, database)

**Tests:**
- Run `cargo check -p ralph-repositories`
- Expected: Compiles with unused module warnings

**Validation:** Repository crate structure in place.

**Commit:** `feat: create ralph-repositories crate structure`

---

### Task 1.9: Implement Database Module
**Files:** `ralph-repositories/src/database.rs`

**Steps:**
1. Create `Database` struct with SqlitePool
2. Implement `Database::new()` with SQLite pragmas:
   - foreign_keys=ON
   - journal_mode=WAL
   - synchronous=NORMAL
   - cache_size=64MB
   - auto_vacuum=INCREMENTAL
3. Run migrations using sqlx::migrate!
4. Add `verify_pragmas()` helper function
5. Implement `pool()` accessor

**Tests:**
- Add integration test for database connection
- Add test for pragma verification
- Run `cargo test -p ralph-repositories database`
- Expected: Database connects, migrations run, pragmas verified

**Validation:** Database connects, migrations apply, pragmas correct.

**Commit:** `feat: add Database module with SQLite optimizations`

---

### Task 1.10: Implement UserRepository
**Files:** `ralph-repositories/src/user.rs`

**Steps:**
1. Create `UserRepository` struct with SqlitePool
2. Implement `UserRepository::new()` constructor
3. Implement `create()` method (INSERT users)
4. Implement `find_by_username()` method (SELECT all fields including password_hash)
5. Implement `find_by_id()` method (SELECT all fields)

**Tests:**
- Add integration test: create user, find by username
- Add integration test: create user, find by id
- Add integration test: find non-existent user returns None
- Run `cargo test -p ralph-repositories user`
- Expected: All tests pass

**Validation:** Can create users, retrieve by username and id.

**Commit:** `feat: add UserRepository with CRUD operations`

---

### Task 1.11: Implement LoopRepository
**Files:** `ralph-repositories/src/loop_.rs`

**Steps:**
1. Create `LoopRepository` struct with SqlitePool
2. Implement `LoopRepository::new()` constructor
3. Implement `create()` method (INSERT loops with all fields)
4. Create `LoopRow` struct (with prd field) for find_by_id
5. Create `LoopSummaryRow` struct (without prd) for list queries
6. Implement `find_by_id()` returning full Loop with prd
7. Implement `list_by_owner()` returning Loops with empty prd
8. Implement `update_status()` with container_id
9. Implement `delete()` method

**Tests:**
- Add integration test: create loop, find by id (with prd)
- Add integration test: create loops, list by owner (without prd)
- Add integration test: update status
- Add integration test: delete loop
- Run `cargo test -p ralph-repositories loop`
- Expected: All tests pass

**Validation:** Can CRUD loops, list excludes prd, find includes prd.

**Commit:** `feat: add LoopRepository with optimized queries`

---

### Task 1.12: Implement TaskRepository
**Files:** `ralph-repositories/src/task.rs`

**Steps:**
1. Create `TaskRepository` struct with SqlitePool
2. Implement `TaskRepository::new()` constructor
3. Implement `create()` method (INSERT tasks)
4. Create `TaskRow` struct (with description, error_message)
5. Create `TaskSummaryRow` struct (without description, error_message)
6. Implement `find_by_id()` returning full Task
7. Implement `list_by_loop()` returning Tasks with empty description/error
8. Implement `find_next_pending()` ordered by priority DESC, created_at ASC
9. Implement `update_status()` with started_at, completed_at, error_message
10. Implement `delete()` method

**Tests:**
- Add integration test: create task, find by id (full)
- Add integration test: create tasks, list by loop (summary)
- Add integration test: find next pending (ordered)
- Add integration test: update status with timestamps
- Run `cargo test -p ralph-repositories task`
- Expected: All tests pass

**Validation:** Can CRUD tasks, list excludes large fields, find includes all.

**Commit:** `feat: add TaskRepository with optimized queries`

---

### Task 1.13: Implement IterationRepository
**Files:** `ralph-repositories/src/iteration.rs`

**Steps:**
1. Create `Iteration` struct mirroring iteration table
2. Create `IterationRepository` struct with SqlitePool
3. Implement `IterationRepository::new()` constructor
4. Implement `create()` method (INSERT iterations)
5. Implement `find_by_id()` returning full Iteration
6. Implement `list_by_loop()` returning Iterations for a loop
7. Implement `list_by_task()` returning Iterations for a task
8. Implement `update_status()` with completed_at, tokens_used
9. Implement `delete()` method

**Tests:**
- Add integration test: create iteration, find by id
- Add integration test: list iterations by loop
- Add integration test: list iterations by task
- Add integration test: update status
- Run `cargo test -p ralph-repositories iteration`
- Expected: All tests pass

**Validation:** Can CRUD iterations, query by loop/task.

**Commit:** `feat: add IterationRepository with queries`

---

### Task 1.14: Implement FileRepository
**Files:** `ralph-repositories/src/file.rs`

**Steps:**
1. Create `File` struct mirroring files table
2. Create `FileRepository` struct with SqlitePool
3. Implement `FileRepository::new()` constructor
4. Implement `create()` method (INSERT files)
5. Implement `find_by_id()` returning full File
6. Implement `list_by_iteration()` returning Files for an iteration
7. Implement `delete()` method

**Tests:**
- Add integration test: create file, find by id
- Add integration test: list files by iteration
- Run `cargo test -p ralph-repositories file`
- Expected: All tests pass

**Validation:** Can CRUD files, query by iteration.

**Commit:** `feat: add FileRepository with queries`

---

### Task 1.15: Update Repositories Exports
**Files:** `ralph-repositories/src/lib.rs`

**Steps:**
1. Update lib.rs to export Database, UserRepository, LoopRepository, TaskRepository, IterationRepository, FileRepository

**Tests:**
- Run `cargo test -p ralph-repositories`
- Expected: All tests pass

**Validation:** All repositories public and accessible.

**Commit:** `refactor: export all repositories from ralph-repositories`

---

## Sprint 1 Demo
Run integration tests to verify:
- Database initializes with migrations and pragmas
- Users can be created and retrieved
- Loops can be created, listed, and retrieved
- Tasks can be created, listed, and found
- Iterations can be created and queried
- Files can be created and queried

Command: `cargo test -p ralph-repositories`

---

## Sprint 2: LLM Agent & Services
**Goal:** Working LLM agent that can execute tasks in Docker containers with error recovery.

### Task 2.1: Create ralph-agent Crate
**Files:** `ralph-agent/Cargo.toml`, `ralph-agent/src/lib.rs`

**Steps:**
1. Create `ralph-agent/Cargo.toml` with dependencies (anthropic-rust, async-openai, tokio, async-trait, serde, serde_json, uuid, chrono)
2. Create `ralph-agent/src/lib.rs` with module declarations (provider, agent, executor, tools, mocks)

**Tests:**
- Run `cargo check -p ralph-agent`
- Expected: Compiles with unused module warnings

**Validation:** Agent crate structure in place.

**Commit:** `feat: create ralph-agent crate structure`

---

### Task 2.2: Implement LLM Provider Trait
**Files:** `ralph-agent/src/provider.rs` (part 1)

**Steps:**
1. Create `LLMRequest` struct (prd, task, context, max_tokens)
2. Create `LLMResponse` struct (content, tokens_used, suggested_tasks, commands)
3. Create `SuggestedTask` struct (title, description, priority)
4. Define `LLMProviderTrait` trait with `complete()` method

**Tests:**
- Add unit tests for LLMRequest/Response serialization
- Run `cargo test -p ralph-agent provider`
- Expected: All tests pass

**Validation:** Provider types defined, trait in place.

**Commit:** `feat: add LLM provider trait and data structures`

---

### Task 2.3: Implement Mock LLM Provider
**Files:** `ralph-agent/src/mocks.rs`

**Steps:**
1. Create `MockLLMProvider` struct implementing `LLMProviderTrait`
2. Implement `complete()` to return predefined response
3. Add field for response injection (for testing)

**Tests:**
- Add unit test: MockLLMProvider returns injected response
- Run `cargo test -p ralph-agent mocks`
- Expected: All tests pass

**Validation:** Mock provider works for offline testing.

**Commit:** `feat: add MockLLMProvider for offline testing`

---

### Task 2.4: Implement Claude Provider
**Files:** `ralph-agent/src/provider.rs` (part 2)

**Steps:**
1. Create `ClaudeProvider` struct with anthropic-rust::Client
2. Implement `ClaudeProvider::new()` with API key
3. Implement `LLMProviderTrait` for ClaudeProvider:
   - Build system prompt from PRD and context
   - Call Claude API with messages
   - Extract content, tokens_used
4. Implement `build_system_prompt()` helper

**Tests:**
- Mock anthropic client or use integration test with real API key
- Add test for system prompt building
- Run `cargo test -p ralph-agent provider`
- Expected: Tests pass (or skip if no API key)

**Validation:** Claude provider can call API, build prompts.

**Commit:** `feat: add Claude LLM provider implementation`

---

### Task 2.5: Implement OpenAI Provider
**Files:** `ralph-agent/src/provider.rs` (part 3)

**Steps:**
1. Create `OpenAIProvider` struct with async-openai::OpenAI
2. Implement `OpenAIProvider::new()` with API key
3. Implement `LLMProviderTrait` for OpenAIProvider:
   - Build system prompt from PRD and context
   - Call OpenAI API with chat completion
   - Extract content, tokens_used

**Tests:**
- Mock OpenAI client or use integration test with real API key
- Run `cargo test -p ralph-agent provider`
- Expected: Tests pass (or skip if no API key)

**Validation:** OpenAI provider can call API, build prompts.

**Commit:** `feat: add OpenAI LLM provider implementation`

---

### Task 2.6: Implement Response Parsers
**Files:** `ralph-agent/src/provider.rs` (part 4)

**Steps:**
1. Implement `parse_suggested_tasks()` to extract <TASKS>...</TASKS> JSON array
2. Implement `parse_commands()` to extract all <CMD>...</CMD> tags
3. Handle missing tags gracefully (return None)

**Tests:**
- Add unit test: parse suggested tasks from LLM response
- Add unit test: parse multiple commands from LLM response
- Add unit test: handle missing tags gracefully
- Run `cargo test -p ralph-agent provider`
- Expected: All tests pass

**Validation:** Can parse tasks and commands from LLM responses.

**Commit:** `feat: add LLM response parsers for tasks and commands`

---

### Task 2.7: Implement CodeAgent Structure
**Files:** `ralph-agent/src/agent.rs` (part 1)

**Steps:**
1. Define `MAX_CONTEXT_ENTRIES` constant (100)
2. Create `AgentConfig` struct (max_iterations, max_tokens_per_request, timeout_seconds)
3. Create `CodeAgent` struct with LLM provider and config
4. Implement `CodeAgent::new()` constructor
5. Implement `execute_task()` method stub (build LLM request, execute commands, build new context)
6. Create `AgentResult` struct (content, tokens_used, suggested_tasks, new_context, commands_executed)

**Tests:**
- Add unit test: CodeAgent::new() creates instance
- Add unit test: execute_task() builds correct LLM request
- Run `cargo test -p ralph-agent agent`
- Expected: All tests pass

**Validation:** Agent structure in place, request building works.

**Commit:** `feat: add CodeAgent structure and request building`

---

### Task 2.8: Implement Timeout Enforcement
**Files:** `ralph-agent/src/agent.rs` (part 2)

**Steps:**
1. Add `tokio::time::timeout()` around LLM completion in `execute_task()`
2. Return error if timeout exceeded

**Tests:**
- Add unit test: timeout is enforced (use MockLLMProvider with delay)
- Add unit test: normal completion doesn't timeout
- Run `cargo test -p ralph-agent agent`
- Expected: All tests pass

**Validation:** Agent enforces timeout on LLM requests.

**Commit:** `feat: add timeout enforcement to CodeAgent`

---

### Task 2.9: Implement Context Truncation
**Files:** `ralph-agent/src/agent.rs` (part 3)

**Steps:**
1. Add context building logic in `execute_task()`
2. Implement FIFO truncation when context exceeds MAX_CONTEXT_ENTRIES
3. Keep most recent 100 entries (oldest dropped first)
4. Log warning when context is truncated

**Tests:**
- Add unit test: context truncation works correctly (keeps last N)
- Add unit test: warning logged when truncated
- Add unit test: small context not truncated
- Run `cargo test -p ralph-agent agent`
- Expected: All tests pass

**Validation:** Agent truncates context to prevent OOM.

**Commit:** `feat: add context truncation (FIFO) to CodeAgent`

---

### Task 2.10: Implement ExecutionContext
**Files:** `ralph-agent/src/executor.rs`

**Steps:**
1. Create `ExecutionContext` struct (container_id, working_dir, env_vars)
2. Implement `ExecutionContext::new()` with container_id and working_dir
3. Implement `execute_command()` using docker exec:
   - cd to working_dir
   - export env_vars
   - execute command
4. Implement `read_file()` using docker exec cat
5. Implement `write_file()` using docker exec tee with mkdir -p
6. Implement `list_files()` using docker exec ls -1

**Tests:**
- Add integration test: execute command in container
- Add integration test: read file from container
- Add integration test: write file to container
- Add integration test: list files in container directory
- Add integration test: command timeout handling
- Add integration test: large output handling
- Add integration test: failed commands return error
- Run `cargo test -p ralph-agent executor`
- Expected: All tests pass (requires Docker running)

**Validation:** Commands execute inside container, files read/write work.

**Commit:** `feat: add ExecutionContext for containerized command execution`

---

### Task 2.11: Implement Tool Trait
**Files:** `ralph-agent/src/tools.rs` (part 1)

**Steps:**
1. Create `Tool` trait with methods: name(), description(), execute()
2. Create `ToolResult` struct (output, error)

**Tests:**
- No tests needed (trait only)

**Validation:** Tool trait defined.

**Commit:** `feat: add Tool trait for extensible operations`

---

### Task 2.12: Implement FileTool
**Files:** `ralph-agent/src/tools.rs` (part 2)

**Steps:**
1. Create `FileTool` struct
2. Implement `Tool` for FileTool:
   - Execute "file read <path>" -> ctx.read_file()
   - Execute "file write <path> <content>" -> ctx.write_file()
   - Execute "file list [path]" -> ctx.list_files()
3. Add usage message for invalid commands

**Tests:**
- Add unit test: file read command
- Add unit test: file write command
- Add unit test: file list command
- Add unit test: invalid command returns usage
- Run `cargo test -p ralph-agent tools`
- Expected: All tests pass

**Validation:** FileTool can read, write, and list files via ExecutionContext.

**Commit:** `feat: add FileTool for container file operations`

---

### Task 2.13: Implement CommandTool
**Files:** `ralph-agent/src/tools.rs` (part 3)

**Steps:**
1. Create `CommandTool` struct
2. Implement `Tool` for CommandTool:
   - Execute args joined as shell command via ctx.execute_command()
   - Return output or error

**Tests:**
- Add unit test: command tool executes shell command
- Add unit test: command tool returns error on failure
- Run `cargo test -p ralph-agent tools`
- Expected: All tests pass

**Validation:** CommandTool executes shell commands via ExecutionContext.

**Commit:** `feat: add CommandTool for shell command execution`

---

### Task 2.14: Update Agent Exports
**Files:** `ralph-agent/src/lib.rs`

**Steps:**
1. Update lib.rs to export provider types, CodeAgent, AgentConfig, AgentResult, ExecutionContext, Tool, ToolResult, FileTool, CommandTool, MockLLMProvider

**Tests:**
- Run `cargo test -p ralph-agent`
- Expected: All tests pass

**Validation:** All agent components public and accessible.

**Commit:** `refactor: export all agent components from ralph-agent`

---

### Task 2.15: Create ralph-services Crate
**Files:** `ralph-services/Cargo.toml`, `ralph-services/src/lib.rs`

**Steps:**
1. Create `ralph-services/Cargo.toml` with dependencies (ralph-models, ralph-repositories, ralph-agent, tokio, anyhow, bcrypt, bollard, uuid, chrono)
2. Create `ralph-services/src/lib.rs` with module declarations (auth, docker, executor)

**Tests:**
- Run `cargo check -p ralph-services`
- Expected: Compiles with unused module warnings

**Validation:** Services crate structure in place.

**Commit:** `feat: create ralph-services crate structure`

---

### Task 2.16: Implement Password Utilities
**Files:** `ralph-services/src/auth.rs` (part 1)

**Steps:**
1. Implement `hash_password()` using bcrypt with DEFAULT_COST
2. Implement `verify_password()` using bcrypt verify

**Tests:**
- Add unit test: hash and verify same password
- Add unit test: verify fails with wrong password
- Run `cargo test -p ralph-services auth`
- Expected: All tests pass

**Validation:** Can hash and verify passwords securely.

**Commit:** `feat: add password hashing and verification utilities`

---

### Task 2.17: Implement AuthService
**Files:** `ralph-services/src/auth.rs` (part 2)

**Steps:**
1. Create `AuthService` struct with UserRepository
2. Implement `AuthService::new()` constructor
3. Implement `register()` method:
   - Validate password (not empty, min length)
   - Validate username format
   - Check username exists, bail if duplicate
   - Hash password
   - Create user
4. Implement `login()` method:
   - Find user by username
   - Verify password
   - Return user if valid

**Tests:**
- Add integration test: register new user
- Add integration test: register duplicate user fails
- Add integration test: register with empty password fails
- Add integration test: login with valid credentials
- Add integration test: login with invalid password fails
- Run `cargo test -p ralph-services auth`
- Expected: All tests pass

**Validation:** Can register and authenticate users with validation.

**Commit:** `feat: add AuthService with user validation`

---

### Task 2.18: Implement DockerManager
**Files:** `ralph-services/src/docker.rs`

**Steps:**
1. Create static DOCKER OnceCell for singleton Docker client
2. Implement `docker()` function to get or initialize client
3. Create `DockerManager` struct
4. Implement `DockerManager::new()` constructor
5. Implement `create_container()` with:
   - Binds: prd_path:/workspace/prd.md:ro, task_path:/workspace/task.md:ro, repo_path:/workspace/repo
   - Memory and CPU limits
6. Implement `start()`, `pause()`, `unpause()`, `stop()`, `remove()` methods

**Tests:**
- Add integration test: create container with volume mounts
- Add integration test: verify volume mount correctness
- Add integration test: enforce resource limits (CPU/memory)
- Add integration test: start and stop container
- Add integration test: pause and unpause container
- Add integration test: verify container state transitions
- Add integration test: orphaned container cleanup after failed start
- Run `cargo test -p ralph-services docker`
- Expected: All tests pass (requires Docker daemon)

**Validation:** Can create, manage Docker containers with volumes and limits.

**Commit:** `feat: add DockerManager for container lifecycle management`

---

### Task 2.19: Implement LoopExecutor Structure
**Files:** `ralph-services/src/executor.rs` (part 1)

**Steps:**
1. Create `LoopExecutor` struct (pool: Arc<SqlitePool>, docker: Arc<DockerManager>, agent_config: AgentConfig)
2. Implement `LoopExecutor::new()` constructor
3. Implement `clone()` method for background tasks
4. Implement `start()` method:
   - Create container
   - Update loop status to Running
   - Start container
   - Spawn execution_loop in background tokio task
5. Implement `pause()` method:
   - Pause container
   - Update status to Paused

**Tests:**
- Add integration test: start loop creates and starts container
- Add integration test: pause loop pauses container
- Run `cargo test -p ralph-services executor`
- Expected: All tests pass

**Validation:** Can start and pause loops.

**Commit:** `feat: add LoopExecutor structure (start, pause)`

---

### Task 2.20: Implement LoopExecutor Resume and Stop
**Files:** `ralph-services/src/executor.rs` (part 2)

**Steps:**
1. Implement `resume()` method:
   - Unpause container
   - Update status to Running
   - Spawn execution_loop in background tokio task
2. Implement `stop()` method:
   - Stop and remove container
   - Update status to Completed

**Tests:**
- Add integration test: resume loop unpauses and restarts execution
- Add integration test: stop loop cleans up container
- Run `cargo test -p ralph-services executor`
- Expected: All tests pass

**Validation:** Can resume and stop loops.

**Commit:** `feat: add LoopExecutor (resume, stop)`

---

### Task 2.21: Implement Execution Loop
**Files:** `ralph-services/src/executor.rs` (part 3)

**Steps:**
1. Implement `execution_loop()` method:
   - Check loop status, exit if not Running
   - Find next pending task
   - Exit if no more tasks (call stop())
   - Execute task
   - Update task status based on result
   - Check max iterations, stop if exceeded
   - Sleep for iteration_delay
2. Handle errors gracefully (log, continue or stop based on severity)

**Tests:**
- Add integration test: execution loop processes tasks sequentially
- Add integration test: loop stops when no more tasks
- Add integration test: loop stops at max iterations
- Add integration test: errors in execution are logged
- Run `cargo test -p ralph-services executor`
- Expected: All tests pass

**Validation:** Execution loop processes tasks, respects limits, handles errors.

**Commit:** `feat: add execution_loop for task processing`

---

### Task 2.22: Implement Task Execution
**Files:** `ralph-services/src/executor.rs` (part 4)

**Steps:**
1. Implement `execute_task()` method:
   - Update task status to InProgress
   - Create ClaudeProvider with API key (or MockLLMProvider for tests)
   - Create CodeAgent with config
   - Create ExecutionContext with container_id and /workspace/repo
   - Execute task via agent
   - Write task output to /workspace/task.md using docker exec
   - Return iteration_id on success
2. Handle task failures gracefully

**Tests:**
- Add integration test: execute task with MockLLMProvider
- Add integration test: task output written to container
- Add integration test: task failure updates status with error
- Add integration test: malformed LLM response handled
- Add integration test: task timeout enforced
- Run `cargo test -p ralph-services executor`
- Expected: All tests pass

**Validation:** Can execute tasks via LLM, output written to container, failures handled.

**Commit:** `feat: add task execution with LLM and container integration`

---

### Task 2.23: Implement Auto-create Tasks from LLM Suggestions
**Files:** `ralph-services/src/executor.rs` (part 5)

**Steps:**
1. In `execute_task()`, after LLM response:
   - Check for suggested_tasks in AgentResult
   - For each suggested task, create new Task in database
   - Link to current iteration
2. Use TaskRepository to create tasks

**Tests:**
- Add integration test: LLM suggestions create new tasks
- Add integration test: tasks created with correct priority
- Add integration test: parent_task_id set if hierarchical
- Run `cargo test -p ralph-services executor`
- Expected: All tests pass

**Validation:** LLM suggestions auto-create tasks in database.

**Commit:** `feat: auto-create tasks from LLM suggestions`

---

### Task 2.24: Implement Container Cleanup on Startup
**Files:** `ralph-services/src/docker.rs` (update)

**Steps:**
1. Add `cleanup_orphaned_containers()` method
2. On app startup, list all containers with "ralph-" prefix
3. Remove containers that are stopped or exited
4. Log cleanup actions

**Tests:**
- Add integration test: orphaned containers cleaned on startup
- Add integration test: running containers not affected
- Run `cargo test -p ralph-services docker`
- Expected: All tests pass

**Validation:** Orphaned containers cleaned on startup.

**Commit:** `feat: add container cleanup on startup`

---

### Task 2.25: Update Services Exports
**Files:** `ralph-services/src/lib.rs`

**Steps:**
1. Update lib.rs to export AuthService, hash_password, verify_password, DockerManager, LoopExecutor

**Tests:**
- Run `cargo test -p ralph-services`
- Expected: All tests pass

**Validation:** All services public and accessible.

**Commit:** `refactor: export all services from ralph-services`

---

## Sprint 2 Demo
**Script: demo_sprint2.sh**
```bash
#!/bin/bash
# Demo script for Sprint 2

# 1. Start a container
docker run -d --name test-loop ralph-loop-manager:latest sleep 3600

# 2. Create a test loop (via DB or API)
curl -X POST http://localhost:3000/api/loops \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Loop",
    "prd": "Build a simple web server",
    "provider": "mock",
    "model": "mock"
  }'

# 3. Add a task
LOOP_ID="...from response..."
curl -X POST http://localhost:3000/api/loops/$LOOP_ID/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Create HTML file",
    "description": "Create index.html with hello world"
  }'

# 4. Start loop
curl -X POST http://localhost:3000/api/loops/$LOOP_ID/start

# 5. Monitor container logs
docker logs -f test-loop

# 6. Verify task completed
curl http://localhost:3000/api/loops/$LOOP_ID/tasks

# 7. Cleanup
docker stop test-loop && docker rm test-loop
```

Run: `bash demo_sprint2.sh`

Expected: Container executes task, output written, tasks auto-created from suggestions.

---

## Sprint 3: HTTP Server & Frontend
**Goal:** Complete web application with HTTP API, HTML interface, and real-time updates.

### Task 3.1: Create ralph-server Crate
**Files:** `ralph-server/Cargo.toml`, `ralph-server/src/lib.rs`

**Steps:**
1. Create `ralph-server/Cargo.toml` with dependencies (ralph-models, ralph-repositories, ralph-services, ralph-agent, axum, tower, tower-http, tokio, anyhow, dotenvy, validator, askama)
2. Create `ralph-server/src/lib.rs` with module declarations (handlers, middleware, templates)

**Tests:**
- Run `cargo check -p ralph-server`
- Expected: Compiles with unused module warnings

**Validation:** Server crate structure in place.

**Commit:** `feat: create ralph-server crate structure`

---

### Task 3.2: Implement Input Validation
**Files:** `ralph-server/src/validation.rs`

**Steps:**
1. Create validators using validator crate:
   - validate_username() (alphanumeric, 3-50 chars)
   - validate_email() (valid email format)
   - validate_password() (min 8 chars, mix of letters/numbers)
   - validate_loop_name() (1-100 chars)
2. Create validation error types

**Tests:**
- Add unit test: validate_username accepts valid, rejects invalid
- Add unit test: validate_email accepts valid, rejects invalid
- Add unit test: validate_password accepts valid, rejects weak
- Run `cargo test -p ralph-server validation`
- Expected: All tests pass

**Validation:** Input validation works for common cases.

**Commit:** `feat: add input validation utilities`

---

### Task 3.3: Implement Auth Middleware
**Files:** `ralph-server/src/middleware/auth.rs`

**Steps:**
1. Create session management (simple in-memory Map for now)
2. Implement `AuthMiddleware` Axum layer:
   - Check session cookie/token
   - Attach user_id to request extensions if valid
   - Return 401 if invalid
3. Implement `require_auth()` extractor

**Tests:**
- Add integration test: protected route requires auth
- Add integration test: valid session allows access
- Add integration test: invalid session returns 401
- Run `cargo test -p ralph-server middleware::auth`
- Expected: All tests pass

**Validation:** Auth middleware protects routes.

**Commit:** `feat: add authentication middleware`

---

### Task 3.4: Implement Auth Handlers
**Files:** `ralph-server/src/handlers/auth.rs`

**Steps:**
1. Create `RegisterResponse` struct (success, message, user_id)
2. Create `LoginResponse` struct (success, message, user_id, session_token)
3. Implement `register()` handler:
   - Validate inputs
   - Call auth.register()
   - Create session
   - Return RegisterResponse
4. Implement `login()` handler:
   - Validate inputs
   - Call auth.login()
   - Create session
   - Return LoginResponse with session_token
5. Implement `logout()` handler:
   - Remove session
   - Return success

**Tests:**
- Add integration test: POST /api/auth/register creates user with valid data
- Add integration test: POST /api/auth/register rejects weak password
- Add integration test: POST /api/auth/register duplicate fails
- Add integration test: POST /api/auth/login authenticates user
- Add integration test: POST /api/auth/login wrong password fails
- Add integration test: POST /api/auth/logout removes session
- Run `cargo test -p ralph-server handlers::auth`
- Expected: All tests pass

**Validation:** Can register, login, logout via HTTP API with validation.

**Commit:** `feat: add auth HTTP handlers with validation`

---

### Task 3.5: Implement Loop CRUD Handlers
**Files:** `ralph-server/src/handlers/loops.rs` (part 1)

**Steps:**
1. Implement `create_loop()` handler:
   - Validate inputs
   - Extract user_id from auth middleware
   - Create Loop from CreateLoop payload
   - Call loop_repo.create()
2. Implement `list_loops()` handler:
   - Extract user_id from auth middleware
   - Add pagination params (page, limit)
   - Call loop_repo.list_by_owner()
3. Implement `get_loop()` handler:
   - Check ownership
   - Call loop_repo.find_by_id()
   - Return 404 if not found
4. Implement `delete_loop()` handler:
   - Check ownership
   - Call loop_repo.delete()

**Tests:**
- Add integration test: POST /api/loops creates loop
- Add integration test: GET /api/loops?page=1&limit=10 paginates
- Add integration test: GET /api/loops/:id returns loop
- Add integration test: GET /api/loops/:id unauthorized for wrong user
- Add integration test: DELETE /api/loops/:id deletes loop
- Run `cargo test -p ralph-server handlers::loops`
- Expected: All tests pass

**Validation:** Can CRUD loops via HTTP API with auth and pagination.

**Commit:** `feat: add loop CRUD HTTP handlers`

---

### Task 3.6: Implement Loop Control Handlers
**Files:** `ralph-server/src/handlers/loops.rs` (part 2)

**Steps:**
1. Implement `start_loop()` handler:
   - Check ownership
   - Call executor.start()
   - Return {"success": true, "status": "running"}
2. Implement `pause_loop()` handler:
   - Check ownership
   - Call executor.pause()
   - Return {"success": true, "status": "paused"}
3. Implement `resume_loop()` handler:
   - Check ownership
   - Call executor.resume()
   - Return {"success": true, "status": "running"}
4. Implement `stop_loop()` handler:
   - Check ownership
   - Call executor.stop()
   - Return {"success": true, "status": "completed"}

**Tests:**
- Add integration test: POST /api/loops/:id/start starts loop
- Add integration test: POST /api/loops/:id/pause pauses loop
- Add integration test: POST /api/loops/:id/resume resumes loop
- Add integration test: POST /api/loops/:id/stop stops loop
- Add integration test: unauthorized control returns 401
- Run `cargo test -p ralph-server handlers::loops`
- Expected: All tests pass

**Validation:** Can control loops (start/pause/resume/stop) via HTTP with auth.

**Commit:** `feat: add loop control HTTP handlers`

---

### Task 3.7: Implement Task Handlers
**Files:** `ralph-server/src/handlers/tasks.rs`

**Steps:**
1. Implement `create_task()` handler:
   - Validate inputs
   - Check loop ownership
   - Create Task from CreateTask payload
   - Call task_repo.create()
2. Implement `list_tasks()` handler:
   - Check loop ownership
   - Add pagination params
   - Call task_repo.list_by_loop()
3. Implement `get_task()` handler:
   - Check loop ownership via task.loop_id
   - Call task_repo.find_by_id()
   - Return 404 if not found
4. Implement `delete_task()` handler:
   - Check loop ownership
   - Call task_repo.delete()

**Tests:**
- Add integration test: POST /api/loops/:id/tasks creates task
- Add integration test: GET /api/loops/:id/tasks paginates
- Add integration test: GET /api/tasks/:id returns task
- Add integration test: GET /api/tasks/:id unauthorized for wrong user
- Add integration test: DELETE /api/tasks/:id deletes task
- Run `cargo test -p ralph-server handlers::tasks`
- Expected: All tests pass

**Validation:** Can CRUD tasks via HTTP API with auth and pagination.

**Commit:** `feat: add task HTTP handlers`

---

### Task 3.8: Implement Health Check Handler
**Files:** `ralph-server/src/handlers/health.rs`

**Steps:**
1. Create `HealthResponse` struct (status, version, database_status)
2. Implement `health_check()` handler:
   - Ping database
   - Return HealthResponse with "ok" status, CARGO_PKG_VERSION, db_status

**Tests:**
- Add integration test: GET /health returns ok status
- Add integration test: GET /health includes db status
- Run `cargo test -p ralph-server handlers::health`
- Expected: All tests pass

**Validation:** Health check endpoint works with DB status.

**Commit:** `feat: add health check handler with DB status`

---

### Task 3.9: Create Handlers Module
**Files:** `ralph-server/src/handlers/mod.rs`

**Steps:**
1. Create handlers module with auth, loops, tasks, health submodules
2. Export all handlers

**Tests:**
- Run `cargo check -p ralph-server`
- Expected: Compiles

**Validation:** Handlers module organized.

**Commit:** `refactor: organize handlers into module`

---

### Task 3.10: Setup HTTP Router (Auth Routes)
**Files:** `ralph-server/src/router.rs` (part 1)

**Steps:**
1. Create Router builder
2. Add public routes:
   - GET /health
   - POST /api/auth/register
   - POST /api/auth/login
   - POST /api/auth/logout
3. Add CORS layer with configurable origins (from .env)

**Tests:**
- Add integration test: GET /health accessible without auth
- Add integration test: POST /api/auth/register works
- Add integration test: CORS headers present
- Run `cargo test -p ralph-server router`
- Expected: All tests pass

**Validation:** Router with public routes and CORS.

**Commit:** `feat: add HTTP router with auth routes`

---

### Task 3.11: Setup HTTP Router (Protected Routes)
**Files:** `ralph-server/src/router.rs` (part 2)

**Steps:**
1. Add protected routes with auth middleware:
   - GET/POST /api/loops
   - GET/DELETE /api/loops/:id
   - POST /api/loops/:id/start/pause/resume/stop
   - GET/POST /api/loops/:id/tasks
   - GET/DELETE /api/tasks/:id

**Tests:**
- Add integration test: protected routes require auth
- Add integration test: protected routes work with valid auth
- Run `cargo test -p ralph-server router`
- Expected: All tests pass

**Validation:** Router with protected routes and auth.

**Commit:** `feat: add HTTP router with protected routes`

---

### Task 3.12: Setup HTTP Server
**Files:** `ralph-server/src/main.rs`

**Steps:**
1. Initialize tracing subscriber with env filter
2. Load .env with dotenvy
3. Initialize Database
4. Run container cleanup (Task 2.24)
5. Initialize AuthService, DockerManager, LoopExecutor
6. Build Router (from Task 3.10-3.11)
7. Bind listener and serve

**Tests:**
- Run `cargo run --bin ralph-server`
- Test all endpoints with curl/Postman
- Expected: Server starts, all routes respond

**Validation:** Server runs with all routes.

**Commit:** `feat: add HTTP server main binary`

---

### Task 3.13: Add Rate Limiting
**Files:** `ralph-server/src/middleware/rate_limit.rs`

**Steps:**
1. Implement rate limiting middleware (token bucket or sliding window)
2. Configure limits from .env (requests per minute)
3. Apply to all API routes
4. Return 429 when limit exceeded

**Tests:**
- Add integration test: rate limit enforced
- Add integration test: rate limit resets after window
- Run `cargo test -p ralph-server middleware::rate_limit`
- Expected: All tests pass

**Validation:** API protected from abuse.

**Commit:** `feat: add rate limiting middleware`

---

### Task 3.14: Add CSRF Protection
**Files:** `ralph-server/src/middleware/csrf.rs`

**Steps:**
1. Implement CSRF token generation
2. Add CSRF token to session
3. Validate CSRF token on POST/PUT/DELETE
4. Add CSRF token to template context

**Tests:**
- Add integration test: POST without CSRF rejected
- Add integration test: POST with valid CSRF accepted
- Run `cargo test -p ralph-server middleware::csrf`
- Expected: All tests pass

**Validation:** HTMX forms protected from CSRF.

**Commit:** `feat: add CSRF protection middleware`

---

### Task 3.15: Add WebSocket for Real-time Updates
**Files:** `ralph-server/src/websocket.rs`

**Steps:**
1. Implement WebSocket handler for loop progress
2. Use axum::extract::ws::WebSocket
3. Subscribe to loop status updates
4. Broadcast status changes to connected clients
5. Implement client connection management

**Tests:**
- Add integration test: WebSocket connection established
- Add integration test: status changes broadcast to clients
- Run `cargo test -p ralph-server websocket`
- Expected: All tests pass

**Validation:** Real-time loop status updates via WebSocket.

**Commit:** `feat: add WebSocket for real-time progress updates`

---

### Task 3.16: Create Base Template
**Files:** `templates/base.html`

**Steps:**
1. Create Askama base template with:
   - HTML5 boilerplate
   - Tailwind CSS CDN
   - HTMX CDN
   - CSRF meta tag
   - Content block
   - Navigation (if logged in)

**Tests:**
- Add test: template renders valid HTML
- Add test: Tailwind and HTMX scripts included
- Run `cargo test -p ralph-server templates::base`
- Expected: All tests pass

**Validation:** Base template structure in place.

**Commit:** `feat: add base HTML template with Tailwind and HTMX`

---

### Task 3.17: Create Login/Register Templates
**Files:** `templates/auth/login.html`, `templates/auth/register.html`

**Steps:**
1. Create login template extending base:
   - Login form (username, password)
   - CSRF token input
   - hx-post to /api/auth/login
2. Create register template extending base:
   - Register form (username, email, password)
   - Validation error display
   - hx-post to /api/auth/register

**Tests:**
- Add integration test: login form renders with CSRF token
- Add integration test: register form renders with validation errors
- Add integration test: forms submit correctly (mock server)
- Run `cargo test -p ralph-server templates::auth`
- Expected: All tests pass

**Validation:** Auth forms render and submit correctly.

**Commit:** `feat: add auth templates (login, register)`

---

### Task 3.18: Create Loop List Template
**Files:** `templates/loops/index.html`

**Steps:**
1. Create loop list template extending base:
   - Loop table (name, status, created_at)
   - "Create Loop" button (hx-get to new form)
   - Loop controls (start/pause/resume/stop) with hx-post
   - Pagination controls
   - Empty state message
2. Use HTMX for interactions (hx-get, hx-post, hx-delete)

**Tests:**
- Add integration test: template renders loop list
- Add integration test: buttons trigger correct HTMX requests
- Add integration test: pagination renders
- Add integration test: empty state displays
- Run `cargo test -p ralph-server templates::loops`
- Expected: All tests pass

**Validation:** Loop list renders with HTMX interactions.

**Commit:** `feat: add loop list template with HTMX`

---

### Task 3.19: Create Loop Form Template
**Files:** `templates/loops/new.html`

**Steps:**
1. Create loop form template extending base:
   - Form fields: name, description, prd, provider, model, docker_image, limits, git config
   - Validation error display
   - CSRF token input
   - hx-post to submit form
   - hx-swap="outerHTML" to replace with detail view

**Tests:**
- Add integration test: form renders with all fields
- Add integration test: validation errors display
- Add integration test: form submission creates loop
- Run `cargo test -p ralph-server templates::loops`
- Expected: All tests pass

**Validation:** Loop form creates loop on submit.

**Commit:** `feat: add loop creation form template`

---

### Task 3.20: Create Loop Detail Template
**Files:** `templates/loops/show.html`

**Steps:**
1. Create loop detail template extending base:
   - Loop info (name, status, PRD, config)
   - Task list with status badges
   - "Add Task" button (hx-get to task form)
   - Loop controls (start/pause/resume/stop)
   - WebSocket connection for real-time updates
   - Task status indicators (pending/in-progress/completed/failed)

**Tests:**
- Add integration test: template renders loop details
- Add integration test: task list displays
- Add integration test: WebSocket connection established
- Add integration test: status updates received via WebSocket
- Run `cargo test -p ralph-server templates::loops`
- Expected: All tests pass

**Validation:** Detail page shows loop and tasks with real-time updates.

**Commit:** `feat: add loop detail template with WebSocket`

---

### Task 3.21: Create Task Form Template
**Files:** `templates/tasks/new.html`

**Steps:**
1. Create task form template:
   - Form fields: title, description, priority, parent_task_id
   - Validation error display
   - CSRF token input
   - hx-post to submit form
   - hx-swap="afterbegin" to add to task list

**Tests:**
- Add integration test: form renders with all fields
- Add integration test: form submission creates task
- Add integration test: task added to list after submit
- Run `cargo test -p ralph-server templates::tasks`
- Expected: All tests pass

**Commit:** `feat: add task creation form template`

---

### Task 3.22: Create Base Docker Image
**Files:** `docker/Dockerfile`

**Steps:**
1. Start from rust:1.85-alpine
2. Install system dependencies (git, bash, coreutils, curl)
3. Set working directory to /workspace
4. Copy ralph-loop-manager binary (from build)
5. Set entrypoint

**Tests:**
- Build docker image: `docker build -t ralph-loop-manager:latest docker/`
- Run container: `docker run --rm ralph-loop-manager:latest /ralph-loop-manager --version`
- Expected: Image builds, container runs

**Validation:** Docker image creates functional container.

**Commit:** `feat: add base Docker image for loop containers`

---

### Task 3.23: Add Environment Variables Config
**Files:** `.env.example`

**Steps:**
1. Create `.env.example` with:
   - DATABASE_URL=ralph.db
   - ANTHROPIC_API_KEY=sk-ant-...
   - OPENAI_API_KEY=sk-...
   - SERVER_ADDR=0.0.0.0:3000
   - RUST_LOG=ralph_server=debug
   - CORS_ORIGINS=http://localhost:3000
   - RATE_LIMIT=100 (requests per minute)

**Tests:**
- Copy .env.example to .env
- Fill in API keys
- Run server
- Expected: Server starts with env vars loaded

**Commit:** `feat: add environment configuration template`

---

### Task 3.24: Update README
**Files:** `README.md`

**Steps:**
1. Update README with:
   - Project description and architecture
   - Prerequisites (Docker, API keys, Rust)
   - Development setup instructions
   - Running server
   - API endpoints documentation
   - Frontend usage
   - Demo scripts
   - SQLite → PostgreSQL migration path (future Sprint 4)

**Tests:**
- Follow README instructions from scratch on fresh machine
- Expected: New developer can set up and run project

**Commit:** `docs: update README with comprehensive setup guide`

---

## Sprint 3 Demo
**Full Workflow Test:**

1. Start server: `cargo run --bin ralph-server`
2. Open browser to http://localhost:3000
3. Register new user account
4. Login
5. Create loop via web UI:
   - Name: "Hello World"
   - PRD: "Build a simple HTML page that says hello"
   - Provider: Mock (for demo)
6. Add task:
   - Title: "Create index.html"
   - Description: "Create an HTML file with hello world message"
7. Start loop execution
8. Monitor real-time progress via WebSocket
9. View task completion
10. Pause loop, then resume
11. Stop loop
12. Verify all functionality works end-to-end

**Expected:** Full workflow works without manual refreshes, real-time updates visible.

---

## Summary

**Total Tasks:** 60 atomic tasks across 3 sprints

**Sprint 1:** Foundation (Tasks 1.1-1.15)
- Working workspace, models, repositories
- SQLite database with migrations and optimizations
- Full CRUD for users, loops, tasks, iterations, files
- **Demo:** Integration tests verify all data layer

**Sprint 2:** Agent & Services (Tasks 2.1-2.25)
- LLM providers (Claude, OpenAI, Mock)
- Code agent with task execution, timeout, context truncation
- Docker container management with cleanup
- Loop executor with background processing
- Auto-create tasks from LLM suggestions
- **Demo:** Manual script shows container execution with LLM

**Sprint 3:** Server & Frontend (Tasks 3.1-3.24)
- HTTP API with all endpoints, auth, pagination
- Input validation, rate limiting, CSRF protection
- WebSocket for real-time progress
- HTML templates with HTMX and Tailwind
- Docker base image
- Complete documentation
- **Demo:** Full web UI end-to-end workflow with real-time updates

---

## Oracle Review Applied

**P0 Fixes Applied:**
- ✅ Added IterationRepository (Task 1.13)
- ✅ Added FileRepository (Task 1.14)
- ✅ Split Task 1.7 into 4 tasks (1.7a-1.7d)
- ✅ Added Auth Middleware (Task 3.3)
- ✅ Fixed dependency order (3.10-3.12 router setup before 3.12 server main)

**P1 Fixes Applied:**
- ✅ Split Task 2.17 into 2 tasks (2.19, 2.20)
- ✅ Split Task 2.6 into 3 tasks (2.7, 2.8, 2.9)
- ✅ Split Task 3.8 into 5 tasks (3.10-3.12 router, 3.12 server)
- ✅ Added Container Cleanup on Startup (Task 2.24)
- ✅ Added MockLLMProvider for offline testing (Task 2.3)
- ✅ Added WebSocket for real-time updates (Task 3.15)
- ✅ Added input validation (Task 3.2)
- ✅ Added rate limiting (Task 3.13)
- ✅ Added CSRF protection (Task 3.14)
- ✅ Auto-create tasks from LLM suggestions (Task 2.23)
- ✅ Added automated template tests (Tasks 3.16-3.21)
- ✅ Added pagination to list endpoints (Tasks 3.5, 3.7)
- ✅ Added SQLite → PostgreSQL migration note in README (Task 3.24)

**Merges Applied:**
- ✅ Task 1.13 (models exports) merged into Task 1.6
- ✅ Task 2.10 (list_files) merged into Task 2.10
- ✅ Task 3.7 (handlers module) merged into Task 3.9
- ✅ Tasks 3.13-3.15 merged into Task 3.24

**New Features Added:**
- ✅ WebSocket for real-time progress updates
- ✅ Rate limiting middleware
- ✅ CSRF protection
- ✅ Input validation
- ✅ Mock LLM provider for offline testing
- ✅ Container cleanup on startup
- ✅ Auto-create tasks from LLM suggestions

**Task Count Change:** 45 → 60 (+15 tasks, but more atomic and complete)
