# Ralph Repositories - Intent Layer

## Purpose

This crate provides the database access layer for Ralph Loop Manager. It implements the Repository pattern to abstract database operations using SQLx with type-safe queries.

**What this area does:**
- Manages database connection (SQLite pool)
- Automatically runs migrations on startup
- Provides repositories for each model (User, Loop, Task, Iteration, File)
- Implements type-safe queries with SQLx
- Validates uniqueness and foreign key constraints

**What this area does NOT do:**
- Does not contain business logic (that's `ralph-services`' responsibility)
- Does not validate complex rules (only DB constraints)
- Does not handle HTTP requests/responses

## Structure

```
ralph-repositories/
├── database.rs        # Database, connection pool, migrations
├── user.rs           # UserRepository
├── loop_.rs          # LoopRepository
├── task.rs           # TaskRepository
├── iteration.rs      # IterationRepository
├── file.rs           # FileRepository
└── lib.rs           # Re-export of all repositories
```

## Critical Invariants

### SQLx Type-Safe Queries

**ALWAYS** use SQLx type-safe queries (`query!`, `query_as!`, etc.):

```rust
// ✅ CORRECT - Type-safe query (validated at compile-time)
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE username = ?",
    username
).fetch_one(&pool).await?

// ❌ WRONG - Unsafe query (SQL injection risk)
sqlx::query(&format!(
    "SELECT * FROM users WHERE username = '{}'",
    username
))
```

**Why this is critical:**
- SQLx validates queries against schema at compile-time
- If table changes, code DOESN'T compile
- Prevents SQL injection
- Ensures correct types

### Database Pragmas (SQLite)

```rust
foreign_keys=ON          // FK constraints enabled
journal_mode=WAL         // Write-Ahead Logging for concurrency
synchronous=NORMAL        // Balance between safety and performance
cache_size=64MB          // Larger cache for performance
auto_vacuum=INCREMENTAL  // Automatic VACUUM
```

### Connection Pool

```rust
// Repository must have cheap clone (Arc<T>)
#[derive(Clone)]
pub struct UserRepository {
    pool: Pool<Sqlite>,  // Clone shares the pool
}

// Never create multiple pools:
// ❌ let pool1 = Database::new(&url).await?.pool();
// ❌ let pool2 = Database::new(&url).await?.pool();

// ✅ Create ONCE and share via Arc:
let pool = Arc::new(Database::new(&url).await?.pool());
let user_repo = UserRepository::new(pool.clone());
let loop_repo = LoopRepository::new(pool.clone());
```

## Usage Patterns

### Initialize Database

```rust
use ralph_repositories::Database;

// Creates pool, applies migrations, checks pragmas
let db = Database::new("sqlite:ralph.db").await?;

// Get pool for repositories
let pool = db.pool();
```

### Create Repository

```rust
use ralph_repositories::UserRepository;

let user_repo = UserRepository::new(pool.clone());
// UserRepository implemented Clone to share pool
```

### Query Patterns

#### SELECT Single

```rust
// Find by username (returns Option)
let user = user_repo
    .find_by_username("alice")
    .await?;

if let Some(user) = user {
    println!("Found user: {}", user.username);
}

// Find by ID (returns Result with NotFound if doesn't exist)
let user = user_repo
    .find_by_id(&user_id)
    .await?;
```

#### SELECT Multiple

```rust
// List user's loops
let loops = loop_repo
    .find_by_owner(&user_id)
    .await?;

// List tasks with status filter
let pending_tasks = task_repo
    .find_by_loop_and_status(&loop_id, TaskStatus::Pending)
    .await?;
```

#### INSERT

```rust
// Create new user
let create_user = CreateUser {
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
    password: hashed_password,
};

let user = user_repo.create(create_user).await?;
// user.id is generated UUID v4
```

#### UPDATE

```rust
// Update loop (status, current_iteration, container_id)
let updated_loop = loop_repo
    .update_status(&loop_id, LoopStatus::Running, Some(container_id), 0)
    .await?;
```

#### DELETE

```rust
// Delete loop (CASCADE deletes tasks, iterations, files)
loop_repo.delete(&loop_id).await?;

// Delete task
task_repo.delete(&task_id).await?;
```

### Transaction Support (when needed)

```rust
use sqlx::Acquire;

pool.begin().await?.transaction(|tx| {
    // Multiple operations
    user_repo.create_with_tx(create_user, tx).await?;
    loop_repo.create_with_tx(create_loop, tx).await?;
    tx.commit().await?;
    Ok(())
}).await?;
```

### Row Mapping

When DB returns different types from model:

```rust
// DB returns status as String, model uses LoopStatus enum
struct LoopRow {
    status: String,  // "running", "paused", etc
    // ... other fields
}

impl From<LoopRow> for Loop {
    fn from(row: LoopRow) -> Self {
        Self {
            status: LoopStatus::from_str(&row.status).unwrap_or(LoopStatus::Error),
            // ...
        }
    }
}
```

## Anti-patterns

### NEVER DO

**1. Queries with string formatting**
```rust
// ❌ WRONG - SQL injection
let query = format!("SELECT * FROM users WHERE username = '{}'", username);
sqlx::query(&query).fetch_one(&pool).await?;

// ✅ CORRECT - Parameterized query
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE username = ?",
    username
).fetch_one(&pool).await?;
```

**2. Ignore database errors**
```rust
// ❌ WRONG - Swallowing errors
let _ = repo.create(user).await;

// ✅ CORRECT - Propagating errors with context
repo.create(user).await
    .context("Failed to create user in database")?;
```

**3. Create multiple pools**
```rust
// ❌ WRONG - Multiple connections
let pool1 = Arc::new(pool.clone());
let user_repo1 = UserRepository::new(pool1);

let pool2 = Arc::new(pool.clone());  // NEW POOL!
let user_repo2 = UserRepository::new(pool2);

// ✅ CORRECT - Share the SAME pool
let pool = Arc::new(pool.clone());
let user_repo1 = UserRepository::new(pool.clone());
let user_repo2 = UserRepository::new(pool.clone());
```

**4. Queries without compile-time validation**
```rust
// ❌ WRONG - query! macro doesn't validate
sqlx::query("SELECT * FROM users")  // If users doesn't exist, only fails at runtime

// ✅ CORRECT - query_as! validates at compile-time
sqlx::query_as!(User, "SELECT * FROM users")  // If users doesn't exist, DOESN'T compile
```

**5. Not using Result/Option correctly**
```rust
// ❌ WRONG - unwrap() can panic
let user = repo.find_by_username("alice")
    .await?
    .unwrap();  // PANIC if doesn't exist

// ✅ CORRECT - Handle Option correctly
let user = repo.find_by_username("alice").await?;
if let Some(user) = user {
    // use user
} else {
    // user not found
}
```

## SQLx Query Types

### query_as!

Returns a specific struct:

```rust
sqlx::query_as!(
    User,
    "SELECT id, username, email, password_hash, created_at, updated_at
     FROM users
     WHERE username = ?",
    username
).fetch_one(&pool).await?
```

**Use when:**
- You know exactly which columns it returns
- Want complete type-safety
- SELECT query with structured result

### query!

Returns a tuple with results:

```rust
let (id, username, email): (String, String, String) = sqlx::query!(
    "SELECT id, username, email FROM users WHERE id = ?",
    user_id
).fetch_one(&pool).await?;
```

**Use when:**
- Need only some columns
- Custom query
- Aggregations (COUNT, SUM, etc)

### execute!

For INSERT, UPDATE, DELETE (no results):

```rust
sqlx::query!(
    "UPDATE loops SET status = ?, current_iteration = ?
     WHERE id = ?",
    "running",
    iteration_num,
    loop_id
)
.execute(&pool)
.await?;
```

## Migrations

### Migration Location

```
migrations/
├── 001_initial.sql
├── 002_add_files_table.sql
└── ...
```

### Migration Naming Convention

`{number}_{description}.sql`

- `001_initial.sql` - Initial schema
- `002_add_loop_container_id.sql` - Add field
- `003_add_task_priority_index.sql` - Index

### Running Migrations

```rust
// In Database::new(), migrations run automatically:
sqlx::migrate!("../migrations")
    .run(&pool)
    .await
    .context("Failed to run database migrations")?;
```

### Migration Best Practices

```sql
-- ✅ CORRECT - Idempotent
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    ...
);

-- ❌ WRONG - Fails if already exists
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    ...
);

-- ✅ CORRECT - Add column if not exists
-- SQLite doesn't support ALTER TABLE ... IF NOT EXISTS
-- Use migration with rollback or check schema
```

## Foreign Keys

### CASCADE Delete

```sql
-- Tasks deleted when Loop deleted
FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE

-- Iterations deleted when Task deleted
FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE

-- Files deleted when Iteration deleted
FOREIGN KEY (iteration_id) REFERENCES iterations(id) ON DELETE CASCADE
```

### SET NULL

```sql
-- parent_task_id NULL when parent deleted
FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE SET NULL
```

## Indexes

### Performance Critical Queries

```sql
-- Find user's loops
CREATE INDEX idx_loops_owner_id ON loops(owner_id);

-- Filter tasks by loop + status
CREATE INDEX idx_tasks_loop_status ON tasks(loop_id, status);

-- Order tasks by priority
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
```

## Dependencies

### Dependencies (Cargo.toml)

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono", "uuid"] }
anyhow = "1.0"
thiserror = "2.0"
ralph-models = { path = "../ralph-models" }
```

### Downstreams (who depends on this crate)

- `ralph-services` → Uses repositories for business logic
- `ralph-server` → Uses repositories in handlers (via services)

## Tests

### Integration Tests

```bash
# Run all tests
cargo test --package ralph-repositories

# Run with in-memory database (faster)
cargo test --package ralph-repositories

# Run specific test
cargo test --package ralph-repositories test_database_initialization
```

### Test Database

```rust
// Tests use memory database
#[tokio::test]
async fn test_database_initialization() -> Result<()> {
    let db = Database::new("sqlite::memory:").await?;
    // ...
}
```

## Pitfalls

### Common Confusions

**1. Compile-Time Validation**

```rust
// If you change a column name in migration,
// ALL SQLx queries using that column will NOT compile!

// This is a FEATURE, not a bug:
sqlx::query_as!(
    User,
    "SELECT id, username, email FROM users WHERE user_name = ?"  // user_name doesn't exist
    // ^^^ COMPILE-TIME ERROR!
)
```

**2. String vs &str in queries**

```rust
// ✅ CORRECT - Both String and &str work
let user_id: String = "uuid".to_string();
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)

// ✅ CORRECT - &str also works
let user_id = "uuid";
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)
```

**3. Row Ordering**

```rust
// query_as! requires columns to be in same order as struct
sqlx::query_as!(
    User,  // struct User has: id, username, email, ...
    "SELECT email, username, id FROM users"  // ❌ WRONG - different order!
)

// ✅ CORRECT - same order
sqlx::query_as!(
    User,
    "SELECT id, username, email FROM users"
)
```

**4. Nullable Columns**

```rust
// If column is NULLABLE in DB, use Option
struct LoopRow {
    pub description: Option<String>,  // ✓ Nullable
    pub name: String,               // ✓ NOT NULL
}
```

**5. DateTime Format**

```rust
// SQLx expects DateTime<Utc> for timestamp columns
// If you try String, will error:
sqlx::query!("INSERT INTO users (created_at) VALUES (?)", "2026-01-18")
// ❌ ERROR - expecting DateTime, not String

// ✅ CORRECT - Use DateTime<Utc>
sqlx::query!("INSERT INTO users (created_at) VALUES (?)", Utc::now())
```

## Downlinks (Additional Context)

**For better understanding:**
- `/AGENTS.md` - General architecture and project patterns
- `/ralph-models/AGENTS.md` - Models that these repositories persist
- `/ralph-services/AGENTS.md` - How these repositories are used in business logic
- `/migrations/AGENTS.md` - Database schema and migrations

---

**Last updated:** 2026-01-18
**Version:** 1.0
