# Ralph Server - HTTP Layer

## Propósito

Este crate fornece a camada HTTP do Ralph Loop Manager. É o ponto de entrada para todas as requisições HTTP, gerenciando handlers, middleware, templates e conexões WebSocket.

**O que esta área faz:**
- HTTP server baseado em Axum framework com runtime Tokio
- Handlers para autenticação (register, login, logout)
- Handlers CRUD para Loops e Tasks
- Controle de execução de loops (start, pause, resume, stop)
- Middleware de autenticação via session
- Middleware de proteção CSRF
- Middleware de rate limiting (token bucket algorithm)
- WebSocket streaming em tempo real para progresso de loops
- Templates Askama com HTMX + Tailwind CSS
- Validação de input (username, email, password, loop name)

**O que esta área NÃO faz:**
- Não contém lógica de negócio (isso é responsabilidade de `ralph-services`)
- Não acessa banco de dados diretamente (usa repositories via `ralph-repositories`)
- Não gerencia containers Docker (isso é responsabilidade de `ralph-services`)
- Não executa loops (isso é responsabilidade de `ralph-services::LoopExecutor`)

## Estrutura

```
ralph-server/
├── src/
│   ├── main.rs           # Entry point, inicialização de serviços
│   ├── lib.rs           # Re-export público
│   ├── router.rs         # Configuração de rotas Axum
│   ├── websocket.rs       # WebSocket handlers e BroadcastManager
│   ├── validation.rs      # Funções de validação de input
│   ├── handlers/
│   │   ├── mod.rs       # Re-export de handlers
│   │   ├── auth.rs      # Register, login, logout handlers
│   │   ├── health.rs    # Health check endpoint
│   │   ├── loops.rs     # Loop CRUD + control handlers
│   │   └── tasks.rs     # Task CRUD handlers
│   ├── middleware/
│   │   ├── mod.rs           # Re-export de middleware
│   │   ├── auth.rs          # Session-based authentication
│   │   ├── csrf.rs          # CSRF token protection
│   │   └── rate_limit.rs    # Token bucket rate limiting
│   └── templates/
│       ├── mod.rs               # Template structs (Askama)
│       ├── base.html            # Base template com navbar/footer
│       ├── auth/
│       │   ├── login.html        # Login form
│       │   └── register.html     # Register form
│       └── loops/
│           ├── index.html          # Loop list page
│           └── new.html            # Create loop form
```

## Invariantes Críticos

### AppState Clonabilidade

**SEMPRE** implemente `Clone` para `AppState`:

```rust
#[derive(Clone, Debug)]
pub struct AppState {
    pub auth_service: AuthService,
    pub session_store: SessionStore,
    pub csrf_store: CsrfTokenStore,
    pub loop_repository: LoopRepository,
    pub task_repository: TaskRepository,
    pub loop_executor: LoopExecutor,
    pub broadcast_manager: BroadcastManager,
}
```

**Por que isso é crítico:**
- Axum precisa clonar o state para cada request handler
- Handlers são `async fn`, então state precisa ser `Clone`
- Repositories já são `Arc<T>` internamente, então clone é barato

### Response Consistency

**TODOS** os handlers devem retornar tuplas consistentes:

```rust
// ✅ CERTO - Tupla (StatusCode, Json<Response>)
pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    // ...
    (StatusCode::CREATED, Json(response))
}

// ✅ CERTO - Para HTML templates
pub async fn list_loops_page(
    // ...
) -> (StatusCode, Html<String>) {
    // ...
    (StatusCode::OK, Html(html))
}

// ❌ ERRADO - Apenas Json (sem StatusCode)
pub async fn handler() -> Json<Response> {
    Json(response)
}
```

### HTTP Status Codes

Use status codes corretos para cada situação:

```rust
// Success
StatusCode::OK              // 200 - GET, PUT, PATCH
StatusCode::CREATED         // 201 - POST (criação)
StatusCode::NO_CONTENT       // 204 - DELETE

// Client Errors
StatusCode::BAD_REQUEST     // 400 - Validação falhou
StatusCode::UNAUTHORIZED     // 401 - Sem session, não autenticado
StatusCode::FORBIDDEN        // 403 - CSRF falhou
StatusCode::NOT_FOUND        // 404 - Resource não existe

// Server Errors
StatusCode::INTERNAL_SERVER_ERROR // 500 - Erro interno
StatusCode::TOO_MANY_REQUESTS    // 429 - Rate limit exceeded
```

### Ownership Checks

**SEMPRE** verifique ownership antes de permitir acesso:

```rust
// ❌ ERRADO - Sem verificação de ownership
pub async fn get_loop(..., Path(id): Path<String>) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?.unwrap();
    Ok(loop_)  // Qualquer usuário pode acessar loop de outro!
}

// ✅ CERTO - Com verificação de ownership
pub async fn get_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?;
    if loop_.owner_id != user_id {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GetLoopResponse {
                success: false,
                message: "You do not have permission to access this loop".to_string(),
                loop_: None,
            }),
        );
    }
    // continua com loop_
}
```

### CSRF Protection

**TODAS** as mutations (POST, PUT, DELETE, PATCH) precisam de CSRF token:

```rust
// Middleware CSRF gera token e adiciona em request.extensions()
Extension(csrf_token): Extension<CsrfToken>

// Template inclui token em forms
<input type="hidden" name="csrf_token" value="{{ csrf_token }}" />

// Cliente envia token em header
fetch("/api/loops", {
    method: "POST",
    headers: {
        "x-csrf-token": csrf_token,
    },
    body: JSON.stringify(data),
})
```

### Session Management

**SEMPRE** use SessionStore para gerenciar sessões:

```rust
// Criar sessão no login
let session_token = state.session_store.create_session(user.id.clone()).await;

// Validar sessão no middleware
if let Some(user_id) = session_store.validate_session(&session_id).await {
    request.extensions_mut().insert(user_id);
}

// Deletar sessão no logout
state.session_store.delete_session(session_id).await;
```

## Padrões de Uso

### Criar Novo Handler

```rust
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use ralph_models::{CreateLoop};
use serde::{Serialize};
use crate::handlers::auth::AppState;

// 1. Response struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoopResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_id: Option<String>,
}

// 2. Handler function
pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    // 3. Validação
    if payload.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreateLoopResponse {
                success: false,
                message: "Name is required".to_string(),
                loop_id: None,
            }),
        );
    }

    // 4. Atribuir ownership
    let mut create_data = payload;
    create_data.owner_id = user_id;

    // 5. Chamar service/repository
    match state.loop_repository.create(create_data).await {
        Ok(loop_) => (
            StatusCode::CREATED,
            Json(CreateLoopResponse {
                success: true,
                message: "Loop created successfully".to_string(),
                loop_id: Some(loop_.id),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateLoopResponse {
                success: false,
                message: format!("Failed to create loop: {}", e),
                loop_id: None,
            }),
        ),
    }
}
```

### Adicionar Nova Rota

No `router.rs`:

```rust
// 1. Importar handler
use crate::handlers::loops::{create_loop, delete_loop};

// 2. Adicionar rota em protected_routes()
fn protected_routes() -> Router<AppState> {
    Router::new()
        // ... outras rotas
        .route("/api/loops", post(create_loop))
        .route("/api/loops/{id}", delete(delete_loop))
        .route_layer(axum::middleware::from_fn(csrf_middleware))
        .route_layer(axum::middleware::from_fn(auth_middleware))
}
```

### Criar Nova Template

1. Definir struct em `templates/mod.rs`:
```rust
#[derive(Template)]
#[template(path = "loops/detail.html")]
pub struct LoopDetailTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loop_: LoopDetail,
}
```

2. Criar arquivo `templates/loops/detail.html`:
```html
{% extends "base.html" %}

{% block content %}
<div class="container mx-auto p-4">
    <h1 class="text-2xl font-bold mb-4">{{ loop_.name }}</h1>

    <p class="mb-4">{{ loop_.description }}</p>

    <div class="bg-gray-100 p-4 rounded">
        <h3 class="font-bold mb-2">PRD</h3>
        <pre class="whitespace-pre-wrap">{{ loop_.prd }}</pre>
    </div>
</div>
{% endblock %}
```

3. Usar template no handler:
```rust
pub async fn loop_detail(
    State(_state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let csrf_token = CsrfToken::generate().to_string();

    // ... buscar loop ...

    let template = LoopDetailTemplate {
        logged_in,
        csrf_token,
        loop_: LoopDetail::from(loop_),
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}
```

### Rate Limiting

Configurar via environment variable:

```bash
# .env
RATE_LIMIT=60  # requests per minute (default: 100)
```

Middleware automaticamente:
- Extrai IP de headers (`x-forwarded-for`, `x-real-ip`)
- Usa token bucket algorithm por IP
- Retorna 429 com `Retry-After` header quando limit excedido

### WebSocket Connection

Cliente JavaScript:

```javascript
// Connect to WebSocket for loop
const ws = new WebSocket(`ws://localhost:3000/ws/loops/${loopId}`);

ws.onmessage = (event) => {
    const message = JSON.parse(event.data);

    switch (message.type) {
        case "loop_status":
            console.log("Loop status changed:", message.data.status);
            break;
        case "iteration_complete":
            console.log("Iteration completed:", message.data.iteration_number);
            break;
        case "loop_error":
            console.error("Loop error:", message.data.error);
            break;
    }
};

ws.onerror = (error) => {
    console.error("WebSocket error:", error);
};
```

## Anti-padrões

### NUNCA FAZER

**1. Ignorar CSRF protection**
```rust
// ❌ ERRADO - POST sem CSRF
.route("/api/loops", post(create_loop))

// ✅ CERTO - Com middleware CSRF
.route("/api/loops", post(create_loop))
.route_layer(axum::middleware::from_fn(csrf_middleware))
```

**2. Retornar JSON quando HTML é esperado (HTMX)**
```rust
// ❌ ERRADO - HTMX espera HTML
pub async fn list_loops_page(...) -> Json<ListLoopsResponse> {
    Json(response)
}

// ✅ CERTO - HTMX precisa de HTML
pub async fn list_loops_page(...) -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html(html))
}
```

**3. Não verificar ownership**
```rust
// ❌ ERRADO - Qualquer user pode acessar
pub async fn get_loop(..., Path(id): Path<String>) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?.unwrap();
    Ok(loop_)
}

// ✅ CERTO - Verificar ownership
pub async fn get_loop(...) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?;
    if loop_.owner_id != user_id {
        return (StatusCode::UNAUTHORIZED, ...);
    }
    Ok(loop_)
}
```

**4. Sempre retornar StatusCode + Response**
```rust
// ❌ ERRADO - Apenas response sem status
pub async fn handler() -> Json<Response> {
    Json(response)
}

// ✅ CERTO - Sempre com StatusCode
pub async fn handler() -> (StatusCode, Json<Response>) {
    (StatusCode::OK, Json(response))
}
```

**5. Swallow errors de template**
```rust
// ❌ ERRADO - Ignorando erro
let html = template.render().unwrap();

// ✅ CERTO - Propagando erro
match template.render() {
    Ok(html) => (StatusCode::OK, Html(html)),
    Err(e) => (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!("Failed to render template: {}", e)),
    ),
}
```

**6. Usar unwrap() sem tratamento**
```rust
// ❌ ERRADO - Pode panic
let loop_ = state.loop_repository.find_by_id(&id).await.unwrap();

// ✅ CERTO - Propagando erro com status apropriado
match state.loop_repository.find_by_id(&id).await {
    Ok(Some(loop_)) => Ok(loop_),
    Ok(None) => (
        StatusCode::NOT_FOUND,
        Json(GetLoopResponse { ... }),
    ),
    Err(e) => (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GetLoopResponse { ... }),
    ),
}
```

## Padrões Específicos do ralph-server

### Request Flow (Auth)

```
1. Client POST /api/auth/register
   ↓
2. Handler valida (username, email, password)
   ↓
3. auth_service.register() (hash de password)
   ↓
4. UserRepository.create() (DB)
   ↓
5. SessionStore.create_session()
   ↓
6. Retorna 201 Created com { success, user_id, session_token }
```

### Request Flow (Loop Control)

```
1. Client POST /api/loops/{id}/start
   ↓
2. Auth middleware valida session
   ↓
3. CSRF middleware valida token
   ↓
4. Handler verifica ownership (loop.owner_id == user_id)
   ↓
5. LoopExecutor.start(loop_id)
   ↓
6. DockerManager cria container
   ↓
7. BroadcastManager.broadcast_loop_status("running")
   ↓
8. Clientes WebSocket recebem update
```

### WebSocket Message Flow

```
1. Client conecta: ws://localhost:3000/ws/loops/{loop_id}
   ↓
2. BroadcastManager.add_client(loop_id, sender)
   ↓
3. Handler envia mensagem de boas-vindas (WsMessage::Ack)
   ↓
4. LoopExecutor executa iteração
   ↓
5. BroadcastManager.broadcast_loop_status(loop_id, "running")
   ↓
6. Todos os clientes conectados recebem WsMessage::LoopStatus
   ↓
7. BroadcastManager.broadcast_iteration_complete(loop_id, iter_num, task_id)
   ↓
8. Todos os clientes recebem WsMessage::IterationComplete
```

## Dependências

### Dependencies (Cargo.toml)

```toml
[dependencies]
# Workspace crates
ralph-models = { path = "../ralph-models" }
ralph-repositories = { path = "../ralph-repositories" }
ralph-services = { path = "../ralph-services" }
ralph-agent = { path = "../ralph-agent" }

# Web framework
axum = { version = "0.8", features = ["ws", "macros"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }
futures-util = "0.3"

# Async runtime
tokio.workspace = true

# Database
sqlx.workspace = true

# Utilities
anyhow.workspace = true
dotenvy = "0.15"
validator = { version = "0.20", features = ["derive"] }
uuid.workspace = true
chrono.workspace = true
rand = "0.8"

# Logging
tracing.workspace = true
tracing-subscriber.workspace = true

# Templates
askama = "0.15"

# Serialization
serde.workspace = true
serde_json.workspace = true
```

### Downstreams (quem depende deste crate)

Nenhum - Este é o crate de entrada (application binary)

### Upstreams (quem este crate depende)

- `ralph-models` - Usado em handlers para DTOs (CreateUser, CreateLoop, CreateTask)
- `ralph-repositories` - Repositories (UserRepository, LoopRepository, TaskRepository)
- `ralph-services` - AuthService, DockerManager, LoopExecutor
- `ralph-agent` - AgentConfig

## Armadilhas

### Confusões Comuns

**1. Extensões de Request vs Extractores**

```rust
// Extension: usado para passar dados entre middleware/handlers
Extension(user_id): Extension<String>  // user_id injetado pelo auth middleware

// Extractor: usado para extrair dados da request
Path(id): Path<String>              // id do path /api/loops/{id}
Query(params): Query<ListQuery>     // query params ?page=1&limit=10
State(state): State<AppState>        // application state
Json(payload): Json<CreateLoop>    // body JSON
```

**2. HTTP Methods vs Rota Protection**

```rust
// Safe methods (GET, HEAD, OPTIONS, TRACE) não precisam de CSRF
.route("/api/loops", get(list_loops))  // CSRF não checado

// Unsafe methods (POST, PUT, DELETE, PATCH) PRECISAM de CSRF
.route("/api/loops", post(create_loop))  // CSRF checado no middleware
.route_layer(axum::middleware::from_fn(csrf_middleware))
```

**3. WebSocket vs HTTP**

```rust
// WebSocket é uma rota separada que faz upgrade de HTTP
.route("/ws/loops/{id}", get(websocket_handler))  // WebSocketUpgrade extractor

// HTTP são rotas normais
.route("/api/loops/{id}", get(get_loop))  // Json/Html response
```

**4. Rate Limiting vs Auth**

```rust
// Rate limiting é por IP (antes de auth)
.layer(axum::middleware::from_fn_with_state(rate_limiter, rate_limit_middleware))

// Auth é por session (depois de rate limiting)
.route_layer(axum::middleware::from_fn(auth_middleware))
```

**5. CsrfToken vs CsrfTokenStore**

```rust
// CsrfToken: wrapper type-safe para token string
let token = CsrfToken::generate();  // gera novo token
let token_str = token.as_str();  // retorna &str

// CsrfTokenStore: gerencia tokens de múltiplas sessões
state.csrf_store.store(&session_id, token.as_str()).await;  // armazena
let stored = state.csrf_store.get(&session_id).await;  // recupera
let valid = state.csrf_store.validate(&session_id, token.as_str()).await;  // valida
```

### Comportamentos Inesperados

**1. Auth Middleware injeta user_id em extensions**

```rust
// Após auth middleware, user_id está disponível em todos os handlers
pub async fn handler(Extension(user_id): Extension<String>) {
    // user_id é String contendo o ID do usuário autenticado
    // Se não autenticado, user_id é string vazia ""
}

// NOTA: Para rotas não protegidas, user_id pode estar ausente
// Use .unwrap_or(String::new()) para safety
```

**2. Template errors em runtime**

```rust
// Askama compila templates em compile-time
// Se template tem erro sintático, NÃO compila

// Erros em runtime (ex: missing variable) returnam Err
match template.render() {
    Ok(html) => (StatusCode::OK, Html(html)),
    Err(e) => {
        // e contém detalhes do erro (missing var, etc)
        (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Template error: {}", e)))
    },
}
```

**3. WebSocket cleanup automático**

```rust
// Quando cliente desconecta, socket é dropado
// Background task é abortada automaticamente

// Cleanup em remove_client() usa Arc::ptr_eq
state.broadcast_manager.remove_client(&loop_id, &sender_arc).await;

// NOTA: Se o mesmo client se reconectar, cria nova Arc
// Clientes antigos não são removidos automaticamente
```

**4. Rate limiting refill automático**

```rust
// Token bucket refills a cada request
// Rate: requests_per_minute / 60 (tokens por segundo)

// Exemplo: 60 requests/minuto = 1 token/segundo
bucket.refill()  // adiciona tokens baseado no tempo decorrido

// Se esgotou, retorna Err(retry_after)
// retry_after = ceil(1.0 / refill_rate) segundos até próximo token
```

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral e padrões do projeto
- `/ralph-models/AGENTS.md` - Modelos usados em handlers
- `/ralph-repositories/AGENTS.md` - Repositories usados em handlers
- `/ralph-services/AGENTS.md` - Services usados em handlers (AuthService, LoopExecutor)
- `/ralph-agent/AGENTS.md` - AgentConfig usado em AppState

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
