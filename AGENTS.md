# Ralph Loop Manager - Intent Layer (Raiz)

## Propósito

Ralph Loop Manager é uma plataforma web para gerenciar, monitorar e orquestrar **Ralph loops** - workflows de desenvolvimento com IA que executam continuamente em containers Docker.

**O que esta área faz:**
- Plataforma completa para gerenciar loops de desenvolvimento com IA (Ralph)
- Execução isolada em containers Docker
- Monitoramento em tempo real via WebSocket
- Histórico completo de iterações e tasks
- Integração com Git (auto PRs/MRs)
- Suporte a múltiplos providers LLM (Claude, OpenAI, Sourcegraph Amp)

**O que esta área NÃO faz:**
- Não é um IDE ou editor de código
- Não executa código diretamente no host (sempre em containers)
- Não substitui sistemas de CI/CD completos (foca em desenvolvimento iterativo)

## Arquitetura Geral

### Stack Tecnológica
- **Backend:** Rust 1.85+ (Axum web framework, Tokio async runtime)
- **Database:** SQLite (com caminho para PostgreSQL)
- **Frontend:** HTMX + Tailwind CSS (templates Askama)
- **Containers:** Docker 27.x+ para isolamento de execução

### Workspace Structure (Cargo)

```
giro/
├── ralph-models/          # Data models (User, Loop, Task, Iteration, File)
├── ralph-repositories/     # Database repositories (SQLx)
├── ralph-agent/          # LLM agent com adapters para múltiplos providers
├── ralph-services/       # Business logic (Auth, Docker, Loop execution)
├── ralph-server/         # HTTP server, handlers, middleware, WebSocket, templates
├── migrations/           # Database schema migrations
└── templates/           # HTMX frontend templates
```

### Fluxo de Dados

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

## Padrões de Uso

### Como Rodar o Projeto

```bash
# 1. Configurar ambiente
cp .env.example .env
# Editar .env e adicionar API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY)

# 2. Build
cargo build --release

# 3. Rodar servidor
cargo run --bin ralph-server
# Server disponível em http://localhost:3000

# 4. Testar
cargo test
```

### Como Adicionar Nova Feature

1. **Model**: Se envolve dados, primeiro crie/edite em `ralph-models/`
2. **Repository**: Crie métodos em `ralph-repositories/` para database ops
3. **Service**: Adicione lógica de negócio em `ralph-services/`
4. **Handler**: Crie handler em `ralph-server/src/handlers/`
5. **Route**: Adicione rota em `ralph-server/src/router.rs`
6. **Template**: Se necessário, crie template em `ralph-server/templates/`

### Como Debugar

```rust
// Tracing está configurado no main.rs
info!("Mensagem informativa");
warn!("Aviso");
error!("Erro: {}", e);
debug!("Debug info (necessário RUST_LOG=debug)");
```

Variável de ambiente para debug:
```bash
RUST_LOG=debug cargo run
```

## Invariantes Críticos

### Database
- **Todas** as operações de database devem usar SQLx com queries type-safe
- **Nunca** construa queries com string formatting (SQL injection risk)
- **Sempre** use prepared queries via `query!()`, `query_as!()`, etc.
- Migrations devem ser idempotentes

### Async/Await
- Tokio runtime é obrigatório em todo código async
- Use `.await` corretamente - **nunca** bloqueio em contexto async
- Preferir `Arc<T>` para compartilhar estado entre tasks

### Docker
- **Todo** loop roda em container isolado
- Containers não devem ter acesso ao host filesystem (exceto volumes montados)
- Resource limits (CPU, memória) devem sempre ser aplicados
- Containers são removidos após completion

### Auth/Security
- Rotas protegidas usam middleware de autenticação
- CSRF tokens são obrigatórios em todas as mutations (POST/PUT/DELETE)
- Senhas são hasheadas com bcrypt (cost 12)
- API keys são encriptadas no banco

## Anti-padrões

### NUNCA FAZER

**Database:**
```rust
// ❌ ERRADO - SQL injection
let query = format!("SELECT * FROM loops WHERE id = '{}'", id);

// ✅ CERTO - Type-safe query
let loop = sqlx::query_as!(
    Loop,
    "SELECT * FROM loops WHERE id = ?",
    id
).fetch_one(&pool).await?;
```

**Async/Await:**
```rust
// ❌ ERRADO - Bloqueando contexto async
std::thread::sleep(Duration::from_secs(1));

// ✅ CERTO - Async sleep
tokio::time::sleep(Duration::from_secs(1)).await;
```

**Error Handling:**
```rust
// ❌ ERRADO - Ignorando erros
let _ = some_operation();

// ✅ CERTO - Propagando erros
some_operation().await?;
```

**Docker:**
```rust
// ❌ ERRADO - Container sem resource limits
docker.create_container(...);

// ✅ CERTO - Com limits
docker.create_container()
    .with_host_config(HostConfig {
        memory: Some(1024 * 1024 * 1024), // 1GB
        cpu_quota: Some(100000),           // 1 CPU
        ..
    })
```

**Frontend (HTMX):**
```rust
// ❌ ERRADO - Retornando JSON quando HTML é esperado
Json(data)

// ✅ CERTO - Retornando HTML para HTMX
Html(template)
```

## Dependências

### Dependencies Internas
- `ralph-models` → Nenhuma (models puros)
- `ralph-repositories` → `ralph-models`
- `ralph-agent` → `ralph-models`
- `ralph-services` → `ralph-models`, `ralph-repositories`, `ralph-agent`
- `ralph-server` → `ralph-models`, `ralph-repositories`, `ralph-services`

### Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/ralph-models/AGENTS.md` - Modelos de dados e validação
- `/ralph-repositories/AGENTS.md` - Database operations e SQLx patterns
- `/ralph-agent/AGENTS.md` - LLM integration e provider adapters
- `/ralph-services/AGENTS.md` - Business logic e Docker integration
- `/ralph-server/AGENTS.md` - HTTP handlers, middleware, WebSocket
- `/migrations/AGENTS.md` - Database schema e migrations

**Documentação:**
- `README.md` - Setup e instruções de uso
- `PRD-ralph-loop-management.md` - Requisitos detalhados do produto
- `2026-01-17-ralph-loop-manager-sprint-breakdown.md` - Roadmap de implementação

## Armadilhas

### Confusões Comuns

**1. Database Connection Pool**
```rust
// ❌ ERRADO - Criando múltiplos pools
let pool1 = Database::new(&url).await?.pool();
let pool2 = Database::new(&url).await?.pool();

// ✅ CERTO - Compartilhando pool via Arc
let pool = Arc::new(Database::new(&url).await?.pool());
```

**2. Handler State**
```rust
// ❌ ERRADO - Clone desnecessário
async fn handler(State(repo): State<LoopRepository>) {
    let repo2 = repo.clone(); // desnecessário
}

// ✅ CERTO - Use State direto ou .clone() quando necessário
async fn handler(State(repo): State<Arc<LoopRepository>>) {
    // repo já é Arc, pode ser usado direto
}
```

**3. WebSocket Connection Management**
```rust
// ❌ ERRADO - Sem cleanup
let ws = WebSocketUpgrade::new(...);
ws.on_upgrade(move |socket| async move {
    // handle socket
});

// ✅ CERTO - Com cleanup
let ws = WebSocketUpgrade::new(...);
ws.on_upgrade(move |socket| {
    let broadcast = broadcast.clone();
    async move {
        let mut socket = socket;
        let mut rx = broadcast.subscribe();
        // handle socket
        // cleanup automático quando socket fecha
    }
});
```

**4. Container Lifecycle**
```rust
// ❌ ERRADO - Container sem cleanup
let container = docker.create_container(...).await?;
docker.start_container(&id).await?;
// Se der erro, container fica rodando

// ✅ CERTO - Com cleanup pattern
let container = docker.create_container(...).await?;
docker.start_container(&id).await?;
let guard = ContainerGuard::new(docker.clone(), id);
// Quando guard sai, container é removido automaticamente
```

### Comportamentos Inesperados

**1. SQLx Compile-Time Checks**
```rust
// Queries SQLx são validadas em tempo de compilação
// Se o schema mudar, o código NÃO compila
let loop = sqlx::query_as!(
    Loop,
    "SELECT * FROM loops WHERE id = ?",  // Se coluna não existir, erro em compile-time
    id
).fetch_one(&pool).await?;
```

**2. Axum State Cloning**
```rust
// AppState deve implementar Clone
#[derive(Clone)]
struct AppState {
    repo: Arc<LoopRepository>,  // Clone barato (apenas o Arc)
}
```

**3. HTMX Request Detection**
```rust
// Para saber se request veio do HTMX:
let is_htmx = req
    .headers()
    .get("HX-Request")
    .is_some();
```

**4. Async Drop não existe em Rust**
```rust
// ❌ Isso não funciona
struct ContainerGuard {
    docker: Docker,
    id: String,
}
impl Drop for ContainerGuard {
    fn drop(&mut self) {
        async {  // NÃO pode ser async!
            self.docker.remove_container(&self.id).await?;
        }
    }
}

// ✅ Use background task ou explicit cleanup
tokio::spawn(async move {
    docker.remove_container(&id).await.ok();
});
```

## Configurações

### Environment Variables (Obrigatórias)

```bash
# Database
DATABASE_URL=sqlite:ralph.db

# Server
SERVER_ADDR=0.0.0.0:3000
CORS_ORIGINS=http://localhost:3000

# LLM Providers (pelo menos um obrigatório)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...

# Rate Limiting
RATE_LIMIT_PER_MINUTE=60
```

### Environment Variables (Opcionais)

```bash
# Docker
DOCKER_HOST=unix:///var/run/docker.sock

# Logging
RUST_LOG=info  # ou debug, warn, error

# Session
SESSION_EXPIRY_HOURS=24

# CORS
CORS_ORIGINS=http://localhost:3000,https://example.com
```

## Padrões Específicos de Ralph

### Loop Execution Flow

```
1. Loop criado (status: created)
   ↓
2. Container Docker criado com volumes:
   - /workspace/prd.md (PRD do projeto)
   - /workspace/task.md (Task atual)
   - /workspace/repo/ (Repositório Git)
   ↓
3. Container inicia com entrypoint script
   ↓
4. Loop Ralph começa: while :; do cat PROMPT.md | npx amp ; done
   ↓
5. A cada iteração:
   - Busca próxima task (status: pending)
   - Marca como in_progress
   - Envia PRD + Task para LLM
   - LLM executa (gera código, etc)
   - Task marcada como completed/failed
   - LLM pode criar novas tasks
   ↓
6. Repete até sem tasks ou max_iterations
   ↓
7. Container para e é removido
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

- **Cada loop tem canal dedicado**
- **BroadcastManager gerencia múltiplos canais**
- **Formato de mensagem:** JSON com `{type: "output", data: "..."}`
- **Clients se inscrevem para receber updates em tempo real**

## Checklist Antes de Commitar

- [ ] `cargo fmt` (formatting)
- [ ] `cargo clippy -- -D warnings` (linting)
- [ ] `cargo test` (tests passando)
- [ ] Queries SQLx são type-safe (usando macros)
- [ ] Não há `unwrap()` desnecessários
- [ ] Erros são tratados com `?` ou `anyhow::Context`
- [ ] Novos models têm migrations correspondentes
- [ ] Handlers protegidos usam auth middleware
- [ ] CSRF tokens incluídos em forms
- [ ] Docker containers têm resource limits
- [ ] Não há hardcoded secrets ou URLs

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
