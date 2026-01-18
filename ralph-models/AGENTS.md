# Ralph Models - Intent Layer

## Propósito

Este crate contém todos os modelos de dados (data models) usados no Ralph Loop Manager. São estruturas puras, sem dependências externas, que definem a forma dos dados que circulam pela aplicação.

**O que esta área faz:**
- Define modelos de domínio (User, Loop, Task, Iteration, File)
- Fornece DTOs para criação/edição (CreateUser, CreateLoop, CreateTask)
- Implementa enums de status com conversão de/para string
- Validações básicas e testes unitários dos modelos

**O que esta área NÃO faz:**
- Não acessa banco de dados (isso é responsabilidade de `ralph-repositories`)
- Não contém lógica de negócio (isso é responsabilidade de `ralph-services`)
- Não lida com HTTP requests/responses (isso é responsabilidade de `ralph-server`)

## Estrutura de Modelos

### Modelos Principais

```
ralph-models/
├── user.rs        # User, CreateUser, LoginUser
├── loop_.rs       # Loop, CreateLoop, LoopStatus
├── task.rs        # Task, CreateTask, TaskStatus
├── iteration.rs   # Iteration, IterationStatus
├── file.rs        # File (artefatos gerados)
└── lib.rs        # Re-export de todos os modelos
```

### Hierarquia de Relacionamentos

```
User
  └── Loop (owner_id)
        ├─ Task (loop_id)
        │   └─ Iteration (task_id)
        │         └─ File (iteration_id)
        │
        └─ Iteration (loop_id, direto para logs globais)
```

## Invariantes Críticos

### Regras de Business

**User:**
- `id`: UUID v4 gerado automaticamente
- `username`: Deve ser único (validado em repository)
- `email`: Deve ser único (validado em repository)
- `password_hash`: Hash bcrypt (nunca armazene senha em texto claro)

**Loop:**
- `id`: UUID v4 gerado automaticamente
- `owner_id`: Deve referenciar um User existente (FK)
- `status`: Deve seguir máquina de estados (Created → Running → Paused → Completed/Error)
- `container_id`: None quando não está rodando
- `current_iteration`: Incrementado a cada iteração executada
- `prd`: Markdown que define o contexto do projeto

**Task:**
- `id`: UUID v4 gerado automaticamente
- `loop_id`: Deve referenciar um Loop existente (FK com CASCADE)
- `status`: Deve seguir máquina de estados (Pending → InProgress → Completed/Failed/Cancelled)
- `parent_task_id`: Para suporte a subtasks (CASCADE SET NULL)
- `created_by`: Deve ser 'user' ou 'llm'
- `priority`: Maior valor = mais prioritária

**Iteration:**
- `id`: UUID v4 gerado automaticamente
- `loop_id`: Deve referenciar um Loop existente (FK)
- `task_id`: Deve referenciar uma Task existente (FK)
- `iteration_number`: Sequencial dentro do loop
- `status`: Running → Completed/Error
- `started_at`: Sempre definido ao criar iteração
- `completed_at`: Definido quando status ≠ Running

### Defaults Obrigatórios

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
Created    → Estado inicial, container não existe
Running    → Container rodando, executando iterações
Paused      → Container pausado (docker pause)
Completed   → Loop terminou com sucesso
Error       → Loop terminou com erro
```

### TaskStatus

```
Pending     → Aguardando execução
InProgress  → Sendo executada agora
Completed   → Execução com sucesso
Failed      → Execução falhou
Cancelled   → Cancelada pelo usuário
```

### IterationStatus

```
Running     → Em execução
Completed   → Completada com sucesso
Error       → Erro durante execução
```

## Padrões de Uso

### Criar um Novo Loop

```rust
use ralph_models::{Loop, CreateLoop};

let create_loop = CreateLoop {
    name: "My First Loop".to_string(),
    description: Some("Test loop for demo".to_string()),
    prd: "Build a simple calculator".to_string(),
    owner_id: user_id.clone(),
    provider: "claude".to_string(),
    model: "claude-3-opus".to_string(),
    docker_image: None,  // usa default
    cpu_limit: None,    // usa default
    memory_limit: None, // usa default
    max_iterations: None,  // usa default
    iteration_timeout: None,  // usa default
    iteration_delay: None,  // usa default
    git_repo_url: None,
    git_branch_pattern: None,
};

let loop_ = Loop::new(create_loop);
// loop_.id é um UUID v4
// loop_.status é Created
```

### Criar uma Nova Task

```rust
use ralph_models::{Task, CreateTask};

let create_task = CreateTask {
    loop_id: loop_id.clone(),
    title: "Add addition function".to_string(),
    description: "Implement add(a, b) -> a + b".to_string(),
    priority: Some(5),  // alta prioridade
    parent_task_id: None,
    created_by: "user".to_string(),
};

let task = Task::new(create_task);
// task.id é um UUID v4
// task.status é Pending
```

### Criar uma Nova Iteration

```rust
use ralph_models::Iteration;

let iteration = Iteration::new(
    loop_id.clone(),
    task_id.clone(),
    1,  // iteration_number
);
// iteration.id é um UUID v4
// iteration.status é Running
// iteration.started_at é agora
```

### Serialização/Deserialização

Todos os modelos implementam `Serialize` e `Deserialize` do Serde:

```rust
use serde_json;

let json = serde_json::to_string(&loop_)?;
let deserialized: Loop = serde_json::from_str(&json)?;
```

### Conversão de Status

```rust
use ralph_models::LoopStatus;

// Enum → String
let status_string = format!("{}", LoopStatus::Running);  // "running"

// String → Enum
let status = LoopStatus::from_str("running")?;
```

## Anti-padrões

### NUNCA FAZER

**1. Modificar modelos após criação**
```rust
// ❌ ERRADO - Violar imutabilidade
let mut loop_ = Loop::new(create_loop);
loop_.id = "custom-id".to_string();  // NÃO

// ✅ CERTO - Criar novo loop com CreateLoop atualizado
let create_loop = CreateLoop { ... };
let loop_ = Loop::new(create_loop);
```

**2. Armazenar senha em texto claro**
```rust
// ❌ ERRADO
User {
    password: "password123".to_string(),  // PERIGO
}

// ✅ CERTO
User {
    password_hash: bcrypt::hash("password123", 12)?,
}
```

**3. Validações complexas em models**
```rust
// ❌ ERRADO - Models devem ser simples
impl User {
    pub fn validate(&self) -> Result<(), Error> {
        // Validações complexas
        if self.username.len() < 3 { ... }
        if !self.email.contains('@') { ... }
    }
}

// ✅ CERTO - Validações em repository ou service
impl UserRepository {
    pub async fn create(&self, create_user: CreateUser) -> Result<User> {
        // Validar antes de persistir
        validate_username(&create_user.username)?;
        validate_email(&create_user.email)?;
        // ...
    }
}
```

**4. Ignorar timestamps**
```rust
// ❌ ERRADO - Overriding timestamps
let mut user = User::new(...);
user.created_at = Utc::now() - Duration::days(1);  // NÃO

// ✅ CERTO - Deixar modelos definirem timestamps
let user = User::new(...);  // created_at é Utc::now() automaticamente
```

**5. Não definir defaults corretamente**
```rust
// ❌ ERRADO - Opcional sem default
impl Loop {
    pub fn new(create_loop: CreateLoop) -> Self {
        Self {
            cpu_limit: create_loop.cpu_limit,  // pode ser None!
        }
    }
}

// ✅ CERTO - Fornecer defaults
impl Loop {
    pub fn new(create_loop: CreateLoop) -> Self {
        Self {
            cpu_limit: create_loop.cpu_limit.unwrap_or(1),
        }
    }
}
```

## Dependências

### Dependencies (Cargo.toml)

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }  # Serialize/Deserialize
serde_json = "1.0"  # JSON serialization
chrono = { version = "0.4", features = ["serde"] }  # DateTime<Utc>
uuid = { version = "1.11", features = ["serde"] }  # UUID generation
```

### Downstreams (quem depende deste crate)

- `ralph-repositories` → Usa os modelos para queries de database
- `ralph-agent` → Usa os modelos para interação com LLM
- `ralph-services` → Usa os modelos para lógica de negócio
- `ralph-server` → Usa os modelos em handlers e templates

## Testes

Cada modelo tem testes unitários abrangentes:

```bash
# Rodar todos os testes
cargo test --package ralph-models

# Rodar testes de um modelo específico
cargo test --package ralph-models user
cargo test --package ralph-models loop
cargo test --package ralph-models task
cargo test --package ralph-models iteration
```

### Coverage Esperado

- **User**: Geração de UUID, serialização, campos preenchidos
- **Loop**: Defaults, custom values, status enum, timestamps
- **Task**: Defaults, priority, parent_task, status enum
- **Iteration**: Creation, status enum, timestamps

## Armadilhas

### Confusões Comuns

**1. UUID como String vs Uuid type**
```rust
// Os modelos usam String para IDs para facilitar serialização
pub struct Loop {
    pub id: String,  // String contendo UUID
}

// Para validar se é UUID válido:
use uuid::Uuid;
Uuid::parse_str(&loop_.id).is_ok()
```

**2. Option vs Default**
```rust
// CreateLoop usa Option para parâmetros opcionais
pub struct CreateLoop {
    pub cpu_limit: Option<i32>,
    pub memory_limit: Option<i32>,
}

// Loop usa valores concretos (com defaults aplicados)
pub struct Loop {
    pub cpu_limit: i32,        // sempre definido
    pub memory_limit: i32,     // sempre definido
}
```

**3. Status Enums não são strings**
```rust
// ❌ ERRADO - Comparar com string
if loop_.status == "running" { ... }

// ✅ CERTO - Comparar com enum
if loop_.status == LoopStatus::Running { ... }

// Ou converter quando necessário
let status_str = loop_.status.to_string();  // "running"
```

**4. Timestamps são UTC**
```rust
// Todos os timestamps são DateTime<Utc>
pub struct Loop {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Para converter para local:
use chrono::Local;
let local_time = loop_.created_at.with_timezone(&Local);
```

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral e padrões do projeto
- `/ralph-repositories/AGENTS.md` - Como estes modelos são persistidos no banco
- `/ralph-services/AGENTS.md` - Como estes modelos são usados na lógica de negócio
- `/ralph-server/AGENTS.md` - Como estes modelos fluem através da API HTTP

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
