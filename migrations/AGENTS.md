# Database Migrations - Intent Layer

## Propósito

Este diretório contém todas as migrations SQL que definem e evoluem o schema do banco de dados para o Ralph Loop Manager. As migrations são executadas automaticamente via SQLx cada vez que a aplicação inicia.

**O que esta área faz:**
- Define o schema inicial do banco de dados (tabelas, colunas, tipos)
- Gerencia evolução do schema ao longo do tempo
- Estabelece foreign key constraints e regras de cascata
- Cria indexes para otimizar queries comuns
- Garante idempotência para execução segura múltipla

**O que esta área NÃO faz:**
- Não contém código Rust (somente SQL puro)
- Não valida regras de negócio complexas (apenas constraints de DB)
- Não roda queries de dados (apenas DDL - Data Definition Language)
- Não substitui testes de repository (aplica schema, não valida queries)

## Estrutura

```
migrations/
├── 001_users.sql              # Tabela de usuários e autenticação
├── 002_loops.sql              # Tabela de Ralph loops e configuração
├── 003_tasks.sql              # Tabela de tasks hierárquicas
└── 004_iterations_files.sql   # Tabelas de iterações e arquivos gerados
```

### Convenção de Nomenclatura

`{number}_{description}.sql`

- `number`: 3 dígitos zerados à esquerda (001, 002, 003...)
- `description`: snake_case descrevendo o propósito
- **Exemplo**: `005_add_loop_container_metrics.sql`

### Ordem de Execução

As migrations rodam em ordem numérica crescente a cada startup da aplicação:

```
001_users.sql
  → 002_loops.sql
      → 003_tasks.sql
          → 004_iterations_files.sql
```

## Schema do Banco de Dados

### Tabela: users

Armazena informações de autenticação e identificação de usuários.

```sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,                    -- UUID v4
    username TEXT UNIQUE NOT NULL,          -- Nome de usuário único
    email TEXT UNIQUE NOT NULL,             -- Email único
    password_hash TEXT NOT NULL,            -- Hash bcrypt (cost 12)
    created_at TEXT NOT NULL                -- ISO 8601 timestamp
);

CREATE INDEX idx_users_username ON users(username);
```

**Constraints:**
- `id`: PRIMARY KEY (UUID v4)
- `username`: UNIQUE - impede usuários duplicados
- `email`: UNIQUE - impede emails duplicados

**Indexes:**
- `idx_users_username`: Acelera lookups de login/username

---

### Tabela: loops

Armazena Ralph loops com configuração completa de execução.

```sql
CREATE TABLE IF NOT EXISTS loops (
    id TEXT PRIMARY KEY,                              -- UUID v4
    name TEXT NOT NULL,                               -- Nome descritivo
    description TEXT,                                 -- Opcional
    prd TEXT NOT NULL,                                -- Product Requirements Document (Markdown)
    owner_id TEXT NOT NULL,                           -- FK para users
    provider TEXT NOT NULL,                            -- "claude", "openai", "sourcegraph"
    model TEXT NOT NULL,                              -- Ex: "claude-3-opus"
    docker_image TEXT NOT NULL DEFAULT 'ralph-loop-manager:latest',
    cpu_limit INTEGER DEFAULT 1,                      -- Número de CPUs
    memory_limit INTEGER DEFAULT 1024,                 -- Memória em MB
    max_iterations INTEGER DEFAULT 100,                -- Limite de iterações
    iteration_timeout INTEGER DEFAULT 300,             -- Timeout em segundos
    iteration_delay INTEGER DEFAULT 0,                -- Delay entre iterações (ms)
    git_repo_url TEXT,                                -- URL do repositório Git
    git_branch_pattern TEXT DEFAULT 'ralph/{loop_id}/{timestamp}',
    status TEXT NOT NULL,                             -- "created", "running", "paused", "completed", "error"
    current_iteration INTEGER DEFAULT 0,              -- Iteração atual
    container_id TEXT,                                -- Docker container ID
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
);

CREATE INDEX idx_loops_owner_id ON loops(owner_id);
CREATE INDEX idx_loops_status ON loops(status);
```

**Constraints:**
- `owner_id`: FK → users(id) - impede loops sem dono

**Defaults:**
- `docker_image`: "ralph-loop-manager:latest"
- `cpu_limit`: 1
- `memory_limit`: 1024 MB
- `max_iterations`: 100
- `iteration_timeout`: 300s (5 minutos)
- `iteration_delay`: 0ms
- `git_branch_pattern`: "ralph/{loop_id}/{timestamp}"

**Indexes:**
- `idx_loops_owner_id`: Queries do tipo "todos os loops do usuário X"
- `idx_loops_status`: Filtragem por status ("loops rodando")

---

### Tabela: tasks

Armazena tasks hierárquicas dentro de loops, com priorização e status.

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,                          -- UUID v4
    loop_id TEXT NOT NULL,                        -- FK para loops
    title TEXT NOT NULL,                          -- Título curto
    description TEXT NOT NULL,                    -- Descrição detalhada
    status TEXT NOT NULL,                         -- "pending", "in_progress", "completed", "failed", "cancelled"
    priority INTEGER DEFAULT 0,                   -- Prioridade (maior = mais importante)
    parent_task_id TEXT,                          -- FK para tasks (subtasks)
    created_by TEXT NOT NULL,                     -- "user" ou "llm"
    iteration_id TEXT,                            -- FK para iterations (quando criada durante iteração)
    started_at TEXT,                              -- Timestamp de início
    completed_at TEXT,                            -- Timestamp de conclusão
    error_message TEXT,                           -- Mensagem de erro se falhou
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
- `loop_id`: FK → loops(id) ON DELETE CASCADE - deleta tasks quando loop é deletado
- `parent_task_id`: FK → tasks(id) ON DELETE SET NULL - seta NULL quando parent é deletado

**Indexes:**
- `idx_tasks_loop_status`: Queries do tipo "tasks pendentes do loop X"
- `idx_tasks_status_priority`: Queries do tipo "próxima task a executar" (ordena por priority DESC)
- `idx_tasks_parent`: Queries do tipo "subtasks da task X"

---

### Tabela: iterations

Armazena cada execução de iteração do loop, com output e métricas.

```sql
CREATE TABLE IF NOT EXISTS iterations (
    id TEXT PRIMARY KEY,                      -- UUID v4
    loop_id TEXT NOT NULL,                    -- FK para loops
    task_id TEXT NOT NULL,                    -- FK para tasks
    iteration_number INTEGER NOT NULL,        -- Sequencial (1, 2, 3...)
    output TEXT,                              -- Output do LLM
    error TEXT,                               -- Mensagem de erro se falhou
    status TEXT NOT NULL,                     -- "running", "completed", "error"
    started_at TEXT NOT NULL,
    completed_at TEXT,
    tokens_used INTEGER,                      -- Tokens consumidos pela API
    FOREIGN KEY (loop_id) REFERENCES loops(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX idx_iterations_loop_number ON iterations(loop_id, iteration_number);
```

**Constraints:**
- `loop_id`: FK → loops(id) - tracking de qual loop
- `task_id`: FK → tasks(id) - tracking de qual task estava sendo executada

**Indexes:**
- `idx_iterations_loop_number`: Queries do tipo "todas as iterações do loop X em ordem"

---

### Tabela: files

Armazena arquivos criados/modificados durante iterações.

```sql
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,                      -- UUID v4
    iteration_id TEXT NOT NULL,               -- FK para iterations
    path TEXT NOT NULL,                       -- Caminho do arquivo
    content_hash TEXT,                        -- Hash do conteúdo (para deduplicação)
    size INTEGER,                             -- Tamanho em bytes
    file_type TEXT,                           -- Tipo do arquivo (ex: "rust", "sql", "md")
    created_at TEXT NOT NULL,
    FOREIGN KEY (iteration_id) REFERENCES iterations(id)
);

CREATE INDEX idx_files_iteration ON files(iteration_id);
```

**Constraints:**
- `iteration_id`: FK → iterations(id) - tracking de qual iteração criou o arquivo

**Indexes:**
- `idx_files_iteration`: Queries do tipo "arquivos da iteração X"

## Invariantes Críticos

### Idempotência

**TODAS** as migrations devem ser idempotentes - rodar múltiplas vezes sem erro:

```sql
-- ✅ CERTO - Idempotent
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    ...
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- ❌ ERRADO - Falha se já existe
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    ...
);

CREATE INDEX idx_users_username ON users(username);
```

### Foreign Keys

**TODOS** os relacionamentos devem ter FK constraints definidas:

```sql
-- ✅ CERTO - FK com apropriado ON DELETE
FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE

-- ❌ ERRADO - Sem FK constraint (risco de orphaned records)
loop_id TEXT NOT NULL
```

### Cascade Rules

**SEMPRE** definir comportamento de cascata apropriado:

- **CASCADE DELETE**: Quando parent é deletado, children são deletados
  - `tasks.loop_id` → `loops.id` (se deletar loop, deletar tasks)
  - `iterations.loop_id` → `loops.id` (se deletar loop, deletar iterations)
  - `files.iteration_id` → `iterations.id` (se deletar iteração, deletar files)

- **SET NULL**: Quando parent é deletado, children têm FK setada para NULL
  - `tasks.parent_task_id` → `tasks.id` (se deletar task parent, subtasks continuam)

- **NO ACTION**: Default - não deleta children se parent é deletado (falha constraint)
  - `loops.owner_id` → `users.id` (não deletar loop se usuário deletado)

### Text Timestamps

**SEMPRE** usar TEXT para timestamps (ISO 8601):

```sql
-- ✅ CERTO - Text timestamp
created_at TEXT NOT NULL  -- "2026-01-18T14:30:00Z"

-- ❌ ERRADO - SQLite não tem tipo DATETIME nativo
created_at DATETIME NOT NULL
```

### Indexes Performance-Critical

**SEMPRE** criar indexes para queries frequentes:

```sql
-- Queries que filtram por status
SELECT * FROM loops WHERE status = 'running';
-- ↓ Necessário index
CREATE INDEX idx_loops_status ON loops(status);

-- Queries que ordenam por priority
SELECT * FROM tasks WHERE status = 'pending' ORDER BY priority DESC LIMIT 1;
-- ↓ Necessário composite index
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
```

## Padrões de Uso

### Como Criar Nova Migration

1. **Identifique a versão próxima** (ex: 005 se a última é 004)
2. **Crie arquivo** `005_{descrição}.sql` no diretório `migrations/`
3. **Escreva SQL idempotente** com `CREATE TABLE IF NOT EXISTS`
4. **Adicione indexes** para queries que serão impactadas
5. **Defina FK constraints** apropriadas
6. **Teste** rodando a aplicação (migrations rodam automaticamente)

**Exemplo: Adicionar tabela de métricas de container**

```sql
-- 005_container_metrics.sql
-- Adicionar tabela para rastrear métricas de CPU/memória de containers

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

### Como Adicionar Coluna

```sql
-- 006_add_loop_auto_commit_flag.sql
-- Adicionar flag para habilitar auto-commit de PRs

-- SQLite não suporta ALTER TABLE ... IF NOT EXISTS
-- Use verificação ou crie nova migration que falhe se já existe
ALTER TABLE loops ADD COLUMN auto_commit INTEGER DEFAULT 0;
```

### Como Adicionar Índice

```sql
-- 007_add_loop_created_at_index.sql
-- Adicionar index para ordenar loops por data de criação

CREATE INDEX IF NOT EXISTS idx_loops_created_at ON loops(created_at DESC);
```

### Como Executar Migrations Manualmente

```bash
# Migrations rodam automaticamente no startup
cargo run --bin ralph-server

# Output no log:
# INFO ralph_repositories::database: Applying migrations
# INFO ralph_repositories::database: Database pragmas verified successfully
# INFO ralph_server: Database initialized and migrations applied
```

### Como Verificar Schema Atual

```bash
# Conecte ao SQLite database
sqlite3 ralph.db

# Liste todas as tabelas
.tables

# Veja schema de uma tabela
.schema users

# Veja índices
.indexes

# Saia
.quit
```

## Anti-padrões

### NUNCA FAZER

**1. Não-idempotent DDL**
```sql
-- ❌ ERRADO - Falha na segunda execução
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL
);

-- ✅ CERTO - Idempotent
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL
);
```

**2. Sem Foreign Keys**
```sql
-- ❌ ERRADO - Orphaned records possíveis
CREATE TABLE tasks (
    loop_id TEXT NOT NULL,  -- Sem FK!
    title TEXT NOT NULL
);

-- ✅ CERTO - FK constraint
CREATE TABLE tasks (
    loop_id TEXT NOT NULL,
    title TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE
);
```

**3. CASCADE Inapropriado**
```sql
-- ❌ ERRADO - Deletar loops quando usuário deletado
-- Isso perde dados importantes!
FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE

-- ✅ CERTO - Impedir deleção se loop existe
FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE RESTRICT
-- Ou usar NO ACTION (default)
FOREIGN KEY (owner_id) REFERENCES users(id)
```

**4. Missing Indexes**
```sql
-- ❌ ERRADO - Query lenta: WHERE status = 'running'
CREATE TABLE loops (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL  -- Sem index!
);

-- ✅ CERTO - Index para performance
CREATE TABLE loops (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL
);
CREATE INDEX idx_loops_status ON loops(status);
```

**5. Integer para Timestamps**
```sql
-- ❌ ERRADO - Unix timestamp (não legível, timezone issues)
created_at INTEGER NOT NULL

-- ✅ CERTO - ISO 8601 string (legível, timezone included)
created_at TEXT NOT NULL  -- "2026-01-18T14:30:00Z"
```

**6. Modificar Migration Existente**

```sql
-- ❌ ERRADO - Nunca modifique migration que já rodou em produção
-- Se 001_users.sql já foi aplicada, não edite o arquivo!
-- Isso quebra a integridade de migrations.

-- ✅ CERTO - Crie nova migration para a mudança
-- 008_add_user_last_login.sql
ALTER TABLE users ADD COLUMN last_login TEXT;
```

## SQLx Migration System

### Como Funciona

```rust
// Em ralph-repositories/src/database.rs
sqlx::migrate!("../migrations")
    .run(&pool)
    .await
    .context("Failed to run database migrations")?;
```

**O que SQLx faz:**
1. Lê todos os arquivos `.sql` do diretório `migrations/`
2. Cria tabela `_sqlx_migrations` para tracking
3. Checa quais migrations já foram aplicadas
4. Executa migrations pendentes em ordem numérica
5. Grava sucesso na tabela `_sqlx_migrations`

### Tabela de Tracking

```sql
-- Criada automaticamente pelo SQLx
CREATE TABLE _sqlx_migrations (
    version INTEGER PRIMARY KEY,
    description TEXT,
    installed_on TEXT NOT NULL,
    checksum TEXT
);
```

**Conteúdo exemplo:**
| version | description | installed_on | checksum |
|---------|-------------|--------------|----------|
| 1 | create_users_table | 2026-01-18T14:00:00Z | abc123... |
| 2 | create_loops_table | 2026-01-18T14:00:01Z | def456... |
| 3 | create_tasks_table | 2026-01-18T14:00:02Z | ghi789... |
| 4 | create_iterations_files_tables | 2026-01-18T14:00:03Z | jkl012... |

### Rollbacks

**SQLx não suporta rollback automático.**

Para reverter schema, crie migration de revert:

```sql
-- 009_revert_container_metrics.sql
-- Reverter migration 005 (remover tabela)

DROP TABLE IF EXISTS container_metrics;
DROP INDEX IF EXISTS idx_container_metrics_loop;
DROP INDEX IF EXISTS idx_container_metrics_recorded;
```

## Dependências

### Dependencies (Cargo.toml)

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate", "chrono"] }
```

**Features relevantes:**
- `migrate`: Habilita `sqlx::migrate!()` macro
- `sqlite`: Suporte para SQLite
- `runtime-tokio`: Runtime async Tokio

### Downstreams (quem depende das migrations)

- `ralph-repositories` → Executa migrations via SQLx
- `ralph-server` → Inicia database (que roda migrations)
- `ralph-services` → Usa models que dependem do schema

## Armadilhas

### Confusões Comuns

**1. Ordem de Migrations Importa**

```sql
-- ❌ ERRADO - Migration 002 tenta criar FK para tabela que não existe ainda
-- 002_tasks.sql
FOREIGN KEY (loop_id) REFERENCES loops(id)  -- loops foi criado em 003!

-- ✅ CERTO - loops existe antes de tasks
-- 002_loops.sql
CREATE TABLE loops (...);

-- 003_tasks.sql
CREATE TABLE tasks (
    loop_id TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id)  -- loops já existe
);
```

**2. SQLite ALTER TABLE Limitations**

```sql
-- SQLite não suporta:
ALTER TABLE users DROP COLUMN password_hash;  -- ❌ NÃO suportado

-- Workaround: criar nova tabela e migrar dados
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

**3. Migration Naming e Execução**

```sql
-- ❌ ERRADO - Migration 002 executará antes de 010
-- 002_add_important_feature.sql
-- 010_add_urgent_bugfix.sql

-- ✅ CERTO - Use números sequenciais
-- 005_add_important_feature.sql
-- 006_add_urgent_bugfix.sql
```

**4. Type Mismatch entre Schema e Rust**

```sql
-- Schema define:
CREATE TABLE loops (
    status TEXT NOT NULL  -- "running", "paused", etc
);

-- Mas o código Rust espera LoopStatus enum:
// Isso causará erro em compile-time com SQLx!
let status: LoopStatus = sqlx::query_as!(Loop, "SELECT status FROM loops WHERE id = ?", id)
    .fetch_one(&pool)
    .await?
    .status;  // ❌ ERRO: status é String, não LoopStatus

// ✅ CERTO - Mapear String → Enum no repository
impl From<LoopRow> for Loop {
    fn from(row: LoopRow) -> Self {
        Self {
            status: LoopStatus::from_str(&row.status).unwrap_or(LoopStatus::Error),
            // ...
        }
    }
}
```

**5. NULL vs String Vazia**

```sql
-- ❌ ERRADO - Ambiguidade
description TEXT  -- Pode ser NULL ou "" ?

-- ✅ CERTO - Seja explícito
description TEXT NOT NULL DEFAULT ''  -- Nunca NULL, pode ser ""
-- OU
description TEXT  -- Pode ser NULL, mas não ""
```

## Pragmas SQLite

Os seguintes pragmas são configurados automaticamente em `ralph-repositories/src/database.rs`:

```rust
.pragma("foreign_keys", "1")           // FK constraints ativadas
.pragma("journal_mode", "WAL")         // Write-Ahead Logging (melhor concorrência)
.pragma("synchronous", "NORMAL")       // Balance segurança/performance
.pragma("cache_size", "-65536")        // 64MB cache
.pragma("auto_vacuum", "INCREMENTAL")  // VACUUM automático
```

**Por que importam:**
- `foreign_keys=ON`: Sem isso, FK constraints são ignoradas!
- `journal_mode=WAL`: Permite leitura e escrita simultâneas
- `synchronous=NORMAL`: Mais rápido que FULL, ainda seguro
- `cache_size=64MB`: Reduz I/O de disco
- `auto_vacuum=INCREMENTAL`: Libera espaço automaticamente

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral do projeto
- `/ralph-repositories/AGENTS.md` - Como migrations são executadas e usadas
- `/ralph-models/AGENTS.md` - Models que correspondem às tabelas do schema
- `/ralph-services/AGENTS.md` - Lógica de negócio que depende do schema

**Arquivos relacionados:**
- `ralph-repositories/src/database.rs` - Execução de migrations
- `ralph-server/src/main.rs` - Inicialização do database

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
