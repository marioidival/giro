# Ralph Repositories - Intent Layer

## Propósito

Este crate fornece a camada de acesso a banco de dados para o Ralph Loop Manager. Implementa o padrão Repository para abstrair operações de database usando SQLx com queries type-safe.

**O que esta área faz:**
- Gerencia conexão com banco de dados (SQLite pool)
- Executa migrations automaticamente ao iniciar
- Fornece repositories para cada modelo (User, Loop, Task, Iteration, File)
- Implementa queries type-safe com SQLx
- Valida constraints de unicidade e foreign keys

**O que esta área NÃO faz:**
- Não contém lógica de negócio (isso é responsabilidade de `ralph-services`)
- Não valida regras complexas (somente constraints de DB)
- Não lida com HTTP requests/responses

## Estrutura

```
ralph-repositories/
├── database.rs        # Database, connection pool, migrations
├── user.rs           # UserRepository
├── loop_.rs          # LoopRepository
├── task.rs           # TaskRepository
├── iteration.rs      # IterationRepository
├── file.rs           # FileRepository
└── lib.rs           # Re-export de todos os repositories
```

## Invariantes Críticos

### SQLx Type-Safe Queries

**SEMPRE** use queries SQLx type-safe (`query!`, `query_as!`, etc.):

```rust
// ✅ CERTO - Query type-safe (validado em compile-time)
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE username = ?",
    username
).fetch_one(&pool).await?

// ❌ ERRADO - Query insegura (SQL injection risk)
sqlx::query(&format!(
    "SELECT * FROM users WHERE username = '{}'",
    username
))
```

**Por que isso é crítico:**
- SQLx valida queries contra o schema em compile-time
- Se a tabela mudar, o código NÃO compila
- Previne SQL injection
- Garante tipos corretos

### Database Pragmas (SQLite)

```rust
foreign_keys=ON          // FK constraints ativadas
journal_mode=WAL         // Write-Ahead Logging para concorrência
synchronous=NORMAL        // Balance entre segurança e performance
cache_size=64MB          // Cache maior para performance
auto_vacuum=INCREMENTAL  // VACUUM automático
```

### Connection Pool

```rust
// Repository deve ter clone barato (Arc<T>)
#[derive(Clone)]
pub struct UserRepository {
    pool: Pool<Sqlite>,  // Clone compartilha o pool
}

// Nunca criar múltiplos pools:
// ❌ let pool1 = Database::new(&url).await?.pool();
// ❌ let pool2 = Database::new(&url).await?.pool();

// ✅ Criar UMA vez e compartilhar via Arc:
let pool = Arc::new(Database::new(&url).await?.pool());
let user_repo = UserRepository::new(pool.clone());
let loop_repo = LoopRepository::new(pool.clone());
```

## Padrões de Uso

### Inicializar Database

```rust
use ralph_repositories::Database;

// Cria pool, aplica migrations, verifica pragmas
let db = Database::new("sqlite:ralph.db").await?;

// Obter pool para repositories
let pool = db.pool();
```

### Criar Repository

```rust
use ralph_repositories::UserRepository;

let user_repo = UserRepository::new(pool.clone());
// UserRepository implementou Clone para compartilhar pool
```

### Query Patterns

#### SELECT Single

```rust
// Buscar por username (retorna Option)
let user = user_repo
    .find_by_username("alice")
    .await?;

if let Some(user) = user {
    println!("Found user: {}", user.username);
}

// Buscar por ID (retorna Result com NotFound se não existe)
let user = user_repo
    .find_by_id(&user_id)
    .await?;
```

#### SELECT Multiple

```rust
// Listar loops de um usuário
let loops = loop_repo
    .find_by_owner(&user_id)
    .await?;

// Listar tasks com filtro de status
let pending_tasks = task_repo
    .find_by_loop_and_status(&loop_id, TaskStatus::Pending)
    .await?;
```

#### INSERT

```rust
// Criar novo usuário
let create_user = CreateUser {
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
    password: hashed_password,
};

let user = user_repo.create(create_user).await?;
// user.id é UUID v4 gerado
```

#### UPDATE

```rust
// Atualizar loop (status, current_iteration, container_id)
let updated_loop = loop_repo
    .update_status(&loop_id, LoopStatus::Running, Some(container_id), 0)
    .await?;
```

#### DELETE

```rust
// Deletar loop (CASCADE deleta tasks, iterations, files)
loop_repo.delete(&loop_id).await?;

// Deletar task
task_repo.delete(&task_id).await?;
```

### Transaction Support (quando necessário)

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

Quando o DB retorna tipos diferentes do model:

```rust
// DB retorna status como String, model usa LoopStatus enum
struct LoopRow {
    status: String,  // "running", "paused", etc
    // ... outros campos
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

## Anti-padrões

### NUNCA FAZER

**1. Queries com string formatting**
```rust
// ❌ ERRADO - SQL injection
let query = format!("SELECT * FROM users WHERE username = '{}'", username);
sqlx::query(&query).fetch_one(&pool).await?;

// ✅ CERTO - Parameterized query
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE username = ?",
    username
).fetch_one(&pool).await?;
```

**2. Ignorar erros de banco**
```rust
// ❌ ERRADO - Swallowing errors
let _ = repo.create(user).await;

// ✅ CERTO - Propagando erros com context
repo.create(user).await
    .context("Failed to create user in database")?;
```

**3. Criar múltiplos pools**
```rust
// ❌ ERRADO - Multiple connections
let pool1 = Arc::new(pool.clone());
let user_repo1 = UserRepository::new(pool1);

let pool2 = Arc::new(pool.clone());  // NOVO POOL!
let user_repo2 = UserRepository::new(pool2);

// ✅ CERTO - Compartilhar o MESMO pool
let pool = Arc::new(pool.clone());
let user_repo1 = UserRepository::new(pool.clone());
let user_repo2 = UserRepository::new(pool.clone());
```

**4. Queries sem validation em compile-time**
```rust
// ❌ ERRADO - query! macro não valida
sqlx::query("SELECT * FROM users")  // Se users não existe, só falha em runtime

// ✅ CERTO - query_as! valida em compile-time
sqlx::query_as!(User, "SELECT * FROM users")  // Se users não existe, NÃO compila
```

**5. Não usar Result/Option corretamente**
```rust
// ❌ ERRADO - unwrap() pode panic
let user = repo.find_by_username("alice")
    .await?
    .unwrap();  // PANIC se não existir

// ✅ CERTO - Tratar Option corretamente
let user = repo.find_by_username("alice").await?;
if let Some(user) = user {
    // usar user
} else {
    // usuário não encontrado
}
```

## SQLx Query Types

### query_as!

Retorna uma struct específica:

```rust
sqlx::query_as!(
    User,
    "SELECT id, username, email, password_hash, created_at, updated_at
     FROM users
     WHERE username = ?",
    username
).fetch_one(&pool).await?
```

**Use quando:**
- Você sabe exatamente quais colunas retorna
- Quer type-safety completo
- Query SELECT com resultado estruturado

### query!

Retorna uma tupla com os resultados:

```rust
let (id, username, email): (String, String, String) = sqlx::query!(
    "SELECT id, username, email FROM users WHERE id = ?",
    user_id
).fetch_one(&pool).await?;
```

**Use quando:**
- Precisa de apenas algumas colunas
- Query customizada
- Aggregations (COUNT, SUM, etc)

### execute!

Para INSERT, UPDATE, DELETE (sem resultados):

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

- `001_initial.sql` - Schema inicial
- `002_add_loop_container_id.sql` - Adicionar campo
- `003_add_task_priority_index.sql` - Índice

### Running Migrations

```rust
// No Database::new(), migrations rodam automaticamente:
sqlx::migrate!("../migrations")
    .run(&pool)
    .await
    .context("Failed to run database migrations")?;
```

### Migration Best Practices

```sql
-- ✅ CERTO - Idempotent
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    ...
);

-- ❌ ERRADO - Falha se já existe
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    ...
);

-- ✅ CERTO - Adicionar coluna se não existe
-- SQLite não suporta ALTER TABLE ... IF NOT EXISTS
-- Use migration com rollback ou verifique schema
```

## Foreign Keys

### CASCADE Delete

```sql
-- Tasks deletados quando Loop deletado
FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE

-- Iterations deletados quando Task deletado
FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE

-- Files deletados quando Iteration deletado
FOREIGN KEY (iteration_id) REFERENCES iterations(id) ON DELETE CASCADE
```

### SET NULL

```sql
-- parent_task_id NULL quando parent deletado
FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE SET NULL
```

## Indexes

### Performance Critical Queries

```sql
-- Buscar loops de usuário
CREATE INDEX idx_loops_owner_id ON loops(owner_id);

-- Filtrar tasks por loop + status
CREATE INDEX idx_tasks_loop_status ON tasks(loop_id, status);

-- Ordenar tasks por priority
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
```

## Dependências

### Dependencies (Cargo.toml)

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono", "uuid"] }
anyhow = "1.0"
thiserror = "2.0"
ralph-models = { path = "../ralph-models" }
```

### Downstreams (quem depende deste crate)

- `ralph-services` → Usa repositories para lógica de negócio
- `ralph-server` → Usa repositories em handlers (via services)

## Testes

### Integration Tests

```bash
# Rodar todos os testes
cargo test --package ralph-repositories

# Rodar com database em memória (mais rápido)
cargo test --package ralph-repositories

# Rodar um teste específico
cargo test --package ralph-repositories test_database_initialization
```

### Test Database

```rust
// Tests usam memory database
#[tokio::test]
async fn test_database_initialization() -> Result<()> {
    let db = Database::new("sqlite::memory:").await?;
    // ...
}
```

## Armadilhas

### Confusões Comuns

**1. Compile-Time Validation**

```rust
// Se você mudar o nome de uma coluna na migration,
// TODAS as queries SQLx que usam essa coluna NÃO vão compilar!

// Isso é uma FEATURE, não um bug:
sqlx::query_as!(
    User,
    "SELECT id, username, email FROM users WHERE user_name = ?"  // user_name NÃO existe
    // ^^^ ERRO EM COMPILE-TIME!
)
```

**2. String vs &str em queries**

```rust
// ✅ CERTO - Tanto String quanto &str funcionam
let user_id: String = "uuid".to_string();
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)

// ✅ CERTO - &str também funciona
let user_id = "uuid";
sqlx::query!("SELECT * FROM users WHERE id = ?", user_id)
```

**3. Row Ordering**

```rust
// query_as! exige que as colunas estejam na mesma ordem da struct
sqlx::query_as!(
    User,  // struct User tem: id, username, email, ...
    "SELECT email, username, id FROM users"  // ❌ ERRADO - ordem diferente!
)

// ✅ CERTO - mesma ordem
sqlx::query_as!(
    User,
    "SELECT id, username, email FROM users"
)
```

**4. Nullable Columns**

```rust
// Se coluna é NULLABLE no DB, use Option
struct LoopRow {
    pub description: Option<String>,  // ✓ Nullable
    pub name: String,               // ✓ NOT NULL
}
```

**5. DateTime Format**

```rust
// SQLx espera DateTime<Utc> para colunas timestamp
// Se você tentar String, vai dar erro:
sqlx::query!("INSERT INTO users (created_at) VALUES (?)", "2026-01-18")
// ❌ ERRO - esperando DateTime, não String

// ✅ CERTO - Use DateTime<Utc>
sqlx::query!("INSERT INTO users (created_at) VALUES (?)", Utc::now())
```

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral e padrões do projeto
- `/ralph-models/AGENTS.md` - Modelos que estes repositories persistem
- `/ralph-services/AGENTS.md` - Como estes repositories são usados na lógica de negócio
- `/migrations/AGENTS.md` - Database schema e migrations

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
