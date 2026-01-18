# Ralph Services - Intent Layer

## Propósito

Este crate contém a lógica de negócio central do Ralph Loop Manager. Implementa serviços que coordenam operações complexas entre repositories, LLM agents e containers Docker.

**O que esta área faz:**
- **AuthService**: Gerencia autenticação de usuários (registro, login, hashing de senha)
- **DockerManager**: Gerencia ciclo de vida de containers Docker (criação, start, pause, stop, remove)
- **LoopExecutor**: Orquestra execução de loops Ralph (task processing, state management, LLM integration)
- Coordena integração entre `ralph-repositories`, `ralph-agent` e Docker
- Aplica regras de negócio e validações

**O que esta área NÃO faz:**
- Não acessa banco de dados diretamente (usa `ralph-repositories`)
- Não lida com HTTP requests/responses (isso é responsabilidade de `ralph-server`)
- Não implementa modelos de dados (isso é responsabilidade de `ralph-models`)
- Não contém templates ou HTML (isso é responsabilidade de `ralph-server`)

## Estrutura

```
ralph-services/
├── src/
│   ├── lib.rs       # Re-export público dos serviços
│   ├── auth.rs      # AuthService (login, registro, password hashing)
│   ├── docker.rs    # DockerManager (container lifecycle)
│   └── executor.rs  # LoopExecutor (orchestration de loops)
└── Cargo.toml      # Dependencies (bcrypt, bollard, tokio, etc)
```

## Invariantes Críticos

### AuthService

**Password Security:**
- **TODAS** as senhas devem ser hasheadas com bcrypt (cost 12 - DEFAULT_COST)
- **NUNCA** armazene senhas em texto claro
- Password validation: mínimo 8 caracteres
- Username validation: 3-50 caracteres, apenas alfanuméricos

**Username Uniqueness:**
- Username deve ser único antes de criar usuário
- Verificar via `user_repo.find_by_username()` antes de `user_repo.create()`

**Login Security:**
- **SEMPRE** use mesma mensagem de erro para "username inválido" e "senha inválida"
- Previne enumeração de usuários (user enumeration attack)

### DockerManager

**Resource Limits Obrigatórios:**
```rust
cpu_quota = 1_000_000_000i64     // 1 CPU core
cpu_period = 1_000_000i64        // 1ms period
memory = 1_073_741_824i64        // 1GB (1024^3)
memory_swap = memory               // No swap (segurança)
```

**Volume Mounts Obrigatórios:**
```rust
binds = vec![
    format!("{}:/workspace/prd.md:ro", prd_path),    // Read-only
    format!("{}:/workspace/task.md:ro", task_path),   // Read-only
    format!("{}:/workspace/repo", repo_path),          // Read-write
]
```

**Container Naming:**
- Containers Ralph devem ter prefixo `ralph-loop-{loop_id}`
- Facilita cleanup e identificação

**Cleanup Pattern:**
- Orphaned containers (prefixo `ralph-`, estado `exited` ou `dead`) devem ser removidos automaticamente
- Running containers **NUNCA** são removidos durante cleanup

### LoopExecutor

**State Transitions:**
```
Loop: Created → Running → Paused → Completed/Error
Task: Pending → InProgress → Completed/Failed
Iteration: Running → Completed/Error
```

**Execution Loop:**
- Loop **NUNCA** deve ser blocking em handler HTTP
- Use `tokio::spawn()` para execução assíncrona
- Loop continua até: no more tasks, max_iterations, ou manual stop

**Task Processing:**
- Tasks são processados em ordem de priority (maior = mais prioritária)
- LLM pode sugerir novas tasks (criadas automaticamente)
- Task failure **NÃO** para o loop (continua com próxima task)

**Error Handling:**
- Erros em tasks não críticas são logados e loop continua
- Apenas erros fatais (sem tasks, max_iterations) param o loop
- Container é limpo ao parar (stop + remove + volumes)

## Padrões de Uso

### AuthService - Registro

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
// user.password_hash é bcrypt hash (NÃO é a senha original!)
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
// User autenticado com sucesso
```

### DockerManager - Criar Container

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

// Pause (mantém container rodando mas paused)
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
// Remove orphaned containers automaticamente
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
// Container criado, started, e execution_loop rodando em background
```

### LoopExecutor - Pause/Resume/Stop

```rust
// Pause (pausa container)
executor.pause(&loop_id).await?;

// Resume (unpause + restart execution loop)
executor.resume(&loop_id).await?;

// Stop (stop container + remove + update status to Completed)
executor.stop(&loop_id).await?;
```

### LoopExecutor - Task Processing Flow

```rust
// Execution loop (rodando em background via tokio::spawn)
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

## Anti-padrões

### NUNCA FAZER

**1. Armazenar senha em texto claro**
```rust
// ❌ ERRADO - PERIGO DE SEGURANÇA
User {
    password: "password123".to_string(),
}

// ✅ CERTO - Hash bcrypt
let password_hash = hash_password("password123")?;
User {
    password_hash,
}
```

**2. Criar container sem resource limits**
```rust
// ❌ ERRADO - Container sem limits pode consumir todo o host
let config = Config {
    image: Some("alpine:latest".to_string()),
    host_config: None,  // NÃO!
    ..
};

// ✅ CERTO - Com limits obrigatórios
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

**3. Bloquear handler HTTP com execution_loop**
```rust
// ❌ ERRADO - Handler fica bloqueado infinitamente
async fn handler(State(executor): State<LoopExecutor>) {
    executor.execution_loop(&loop_id).await?;  // Bloqueia!
    Ok(())
}

// ✅ CERTO - Spawn em background
async fn handler(State(executor): State<LoopExecutor>) {
    tokio::spawn(async move {
        let _ = executor.execution_loop(&loop_id).await;
    });
    Ok(())  // Handler retorna imediatamente
}
```

**4. Não fazer cleanup de containers**
```rust
// ❌ ERRADO - Container fica rodando se der erro
let container_id = docker.create_container(...).await?;
docker.start(&container_id).await?;
// Se der erro, container fica rodando forever!

// ✅ CERTO - Cleanup pattern (manual ou via DockerManager)
let container_id = docker.create_container(...).await?;
docker.start(&container_id).await?;

// Em caso de erro, limpa:
if let Err(e) = some_operation().await {
    docker.stop(&container_id, Some(10)).await.ok();
    docker.remove(&container_id, true, true).await.ok();
    return Err(e);
}
```

**5. Reveal username existence em login**
```rust
// ❌ ERRADO - User enumeration attack
pub async fn login(&self, username: &str, password: &str) -> Result<User> {
    let user = self.user_repo.find_by_username(username).await?;

    if user.is_none() {
        bail!("Username not found");  // ❌ Vazou que username não existe
    }

    let user = user.unwrap();
    if !verify_password(password, &user.password_hash)? {
        bail!("Invalid password");  // ❌ Mensagem diferente
    }

    Ok(user)
}

// ✅ CERTO - Mesma mensagem para ambos os casos
pub async fn login(&self, login_user: LoginUser) -> Result<User> {
    let user = self
        .user_repo
        .find_by_username(&login_user.username)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Invalid username or password"))?;

    let is_valid = verify_password(&login_user.password, &user.password_hash)?;

    if !is_valid {
        bail!("Invalid username or password");  // ✅ Mesma mensagem
    }

    Ok(user)
}
```

**6. Ignorar erros de task execution**
```rust
// ❌ ERRADO - Silently swallowing errors
match self.execute_task(&task).await {
    Ok(_) => { /* success */ }
    Err(e) => { /* ignora erro */ }
}

// ✅ CERTO - Logar e atualizar status
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

**7. Criar múltiplos Docker clients**
```rust
// ❌ ERRADO - Multiple connections
let manager1 = DockerManager::new();
let manager2 = DockerManager::new();  // NOVO CLIENTE!

// ✅ CERTO - Clone singleton (mesma conexão)
let manager = DockerManager::new();
let manager_clone = manager.clone();  // Clone barato (Docker é Arc)
```

**8. Volume mounts sem :ro para arquivos read-only**
```rust
// ❌ ERRADO - PRD pode ser modificado no container
let binds = vec![
    format!("{}:/workspace/prd.md", prd_path),  // RW por padrão
];

// ✅ CERTO - Read-only para arquivos que não devem mudar
let binds = vec![
    format!("{}:/workspace/prd.md:ro", prd_path),  // Read-only
    format!("{}:/workspace/task.md:ro", task_path),
    format!("{}:/workspace/repo", repo_path),  // RW (pode escrever)
];
```

## Dependências

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
- Password hashing com adaptive cost factor
- DEFAULT_COST = 12 (balance entre security e performance)
- Verificação segura contra timing attacks

**bollard (0.18)**
- Rust Docker client (async)
- Abstração para Docker API
- Platform-aware connections (Unix named socket, Windows named pipe)

**once_cell (1.20)**
- Singleton pattern para Docker client
- `OnceCell<Docker>` global em `docker.rs`

**tokio**
- Async runtime para toda a aplicação
- `tokio::spawn()` para background tasks
- `tokio::time::sleep()` para delays

## Downstreams (quem depende deste crate)

- `ralph-server` → Usa AuthService, DockerManager, LoopExecutor em handlers

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral e padrões do projeto
- `/ralph-models/AGENTS.md` - Modelos que estes services usam
- `/ralph-repositories/AGENTS.md` - Como estes services persistem dados
- `/ralph-agent/AGENTS.md` - LLM providers e agentes (quando existir)

## Armadilhas

### Confusões Comuns

**1. Docker client singleton vs multiple instances**
```rust
// DockerManager usa singleton global (DOCKER OnceCell)
// Mas DockerManager::new() pode ser chamado múltiplas vezes
let manager1 = DockerManager::new();  // Usa DOCKER.get_or_init()
let manager2 = DockerManager::new();  // Usa MESMO DOCKER instance

// Clonar DockerManager é barato (Arc<Docker>)
let manager_clone = manager1.clone();  // Share mesma conexão Docker
```

**2. LoopExecutor::start() vs execution_loop()**
```rust
// start() é público - cria container, inicia, e spawns execution_loop
executor.start(&loop_id).await?;

// execution_loop() é privado - rodado em background via tokio::spawn
// NÃO chame execution_loop diretamente em handlers!
```

**3. Task status updates**
```rust
// execution_loop atualiza status de task automaticamente:
// - Pending → InProgress (antes de executar)
// - InProgress → Completed/Failed (após execução)

// NÃO atualize manualmente em handler HTTP!
// Use executor.start() e deixe o loop gerenciar
```

**4. Container cleanup timing**
```rust
// Container é removido APENAS em:
// 1. executor.stop() → stop + remove
// 2. docker_manager.cleanup_orphaned_containers() → containers exited/dead

// Container NÃO é removido em:
// - executor.pause() → apenas pause (continua existindo)
// - executor.resume() → apenas unpause
// - Se der erro durante create_container (container nem existe)
```

**5. bcrypt cost factor**
```rust
// DEFAULT_COST = 12 (no bcrypt crate)
// Este valor é usado automaticamente em hash_password()

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;  // Cost 12
    Ok(hashed)
}

// NÃO mude DEFAULT_COST sem bom motivo:
// - Cost 10: 2x mais rápido, 4x menos seguro
// - Cost 12: balance atual (padrão da indústria)
// - Cost 14: 4x mais lento, 16x mais seguro
```

### Comportamentos Inesperados

**1. Loop continua mesmo após task failure**
```rust
// execution_loop NÃO para se uma task falha!
match self.execute_task(&task).await {
    Ok(_) => { /* task.completed */ }
    Err(e) => {
        task_repo.update_status(&task.id, TaskStatus::Failed, ...).await?;
        // Continua com próxima task!
    }
}

// Apenas para se:
// - Não há mais tasks
// - max_iterations atingido
// - Loop status mudou (manual stop)
// - Erro fatal (mas isso é raro)
```

**2. Execution loop é infinito até conditions**
```rust
async fn execution_loop(&self, loop_id: &str) -> Result<()> {
    loop {  // Loop infinito!
        // Check conditions
        if loop_.status != LoopStatus::Running {
            return Ok(());  // Sai aqui
        }

        // Process task
        // Sleep

        // Repete...
    }
}
// Sem return no loop = loop infinito (não é bug, é design!)
```

**3. tokio::spawn error handling**
```rust
// tokio::spawn() NÃO propaga erros
tokio::spawn(async move {
    if let Err(e) = executor.execution_loop(&loop_id_owned).await {
        tracing::error!("Execution loop error: {:?}", e);
        // Erro é logado, mas NÃO propagado!
    }
});

// Handler continua normalmente mesmo se execution_loop falhar
// Use logs ou eventos para monitorar loop health
```

**4. Docker container name collision**
```rust
// Se container com mesmo nome já existe, create_container() retorna erro
let container_id = docker_manager
    .create_container(
        "alpine:latest",
        "/tmp/prd.md",
        "/tmp/task.md",
        "/tmp/repo",
        Some("ralph-loop-123"),  // Se existir → Erro!
    )
    .await?;

// Solução: Stop/remove antes ou usar nome único
// LoopExecutor usa formato: ralph-loop-{loop_id} (loop_id é UUID, então único)
```

**5. Password hash verification não é case-sensitive**
```rust
// bcrypt verification NÃO é case-sensitive para o password original
let hash1 = hash_password("Password123")?;
let hash2 = hash_password("password123")?;

// Hashes são DIFERENTES (bcrypt usa salt aleatório)
assert_ne!(hash1, hash2);

// Mas ambos verificam corretamente
assert!(verify_password("Password123", &hash1)?);  // True
assert!(verify_password("password123", &hash1)?);  // False
assert!(verify_password("password123", &hash2)?);  // True
```

## Testes

### Unit Tests

```bash
# Testes de hash/verify de senha
cargo test --package ralph-services test_hash_and_verify

# Testes de singleton Docker
cargo test --package ralph-services test_docker_singleton
```

### Integration Tests

```bash
# Todos os testes (alguns requerem Docker daemon)
cargo test --package ralph-services

# Pular testes que requerem Docker
cargo test --package ralph-services -- --ignore

# Testes específicos
cargo test --package ralph-services test_register_new_user
cargo test --package ralph-services test_create_container_with_volumes
```

### Testes que requerem Docker Daemon

A maioria dos testes de integração em `docker.rs` e `executor.rs` requerem Docker rodando:

```bash
# Start Docker antes de rodar testes
docker info  # Verifica se Docker está rodando

# Rodar testes
cargo test --package ralph-services

# Se Docker não está rodando, testes são ignorados (#[ignore])
# Use --ignored para ver quais testes foram pulados
cargo test --package ralph-services -- --list --ignored
```

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
