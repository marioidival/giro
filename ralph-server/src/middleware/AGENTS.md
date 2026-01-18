# Middleware Layer

## Propósito

Este módulo fornece a camada de middleware HTTP para o Ralph Loop Manager. Responsável por proteger endpoints, gerenciar sessões, prevenir ataques CSRF e limitar taxa de requisições.

**O que esta área faz:**
- **Autenticação**: Validação de sessões e injeção de `user_id` em handlers
- **Proteção CSRF**: Geração e validação de tokens para prevenir ataques de cross-site request forgery
- **Rate Limiting**: Limite de requisições por IP usando algoritmo token bucket
- **Ordering e Stacking**: Ordem correta de aplicação de middlewares na pipeline HTTP

**O que esta área NÃO faz:**
- Não gerencia lógica de negócio de autenticação (isso é responsabilidade de `ralph-services::AuthService`)
- Não acessa banco de dados (usa storage em memória para sessions/tokens)
- Não implementa login/logout (isso é responsabilidade dos handlers)
- Não faz autorização baseada em roles (apenas verifica autenticação)

## Estrutura

```
ralph-server/src/middleware/
├── mod.rs           # Re-exports públicos
├── auth.rs          # SessionStore, auth_middleware, require_auth
├── csrf.rs          # CsrfTokenStore, CsrfToken, csrf_middleware
└── rate_limit.rs    # RateLimiter, TokenBucket, rate_limit_middleware
```

### Componentes

#### Auth Middleware (`auth.rs`)
- `SessionStore`: Storage em memória (`Arc<RwLock<HashMap>>`) mapeando `session_id` → `user_id`
- `auth_middleware`: Função de middleware que valida sessão e injeta `user_id` em extensions
- `require_auth`: Helper function para extrair `user_id` de request
- `extract_session_id`: Extrai session ID de headers (`session` ou `authorization`)

#### CSRF Middleware (`csrf.rs`)
- `CsrfTokenStore`: Storage em memória para tokens CSRF por sessão
- `CsrfToken`: Wrapper type-safe com implementações `Display`, `AsRef`, `From<String>`
- `csrf_middleware`: Gera/valida tokens e retorna 403 em falha
- `is_safe_method`: Verifica se método HTTP é seguro (GET, HEAD, OPTIONS, TRACE)
- `extract_csrf_token`: Extrai token de headers (`x-csrf-token` ou `csrf-token`)

#### Rate Limiting Middleware (`rate_limit.rs`)
- `RateLimiter`: Gerencia buckets de tokens por cliente IP
- `TokenBucket`: Implementação do algoritmo token bucket com refill automático
- `RateLimitConfig`: Configuração carregada de environment variables
- `rate_limit_middleware`: Verifica limites e retorna 429 com `Retry-After` header
- `extract_client_ip`: Extrai IP de headers (`x-forwarded-for` ou `x-real-ip`)

## Invariantes Críticos

### Ordem de Middleware

A ordem de aplicação dos middlewares é **CRÍTICA** e NUNCA deve ser alterada:

```rust
// ✅ CERTO - Ordem correta em router.rs
Router::new()
    .merge(protected_routes())
    .layer(axum::middleware::from_fn_with_state(
        rate_limiter,
        rate_limit_middleware,  // 1. Rate limiting (primeiro, outermost)
    ))
    .layer(Extension(state.csrf_store.clone()))  // 2. Injeção de stores
    .layer(Extension(state.session_store.clone()))
    .layer(cors)  // 3. CORS
    .with_state(state)

// Para rotas protegidas:
fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/loops", post(create_loop))
        .route_layer(axum::middleware::from_fn(csrf_middleware))  // 4. CSRF validation
        .route_layer(axum::middleware::from_fn(auth_middleware))  // 5. Auth (último, innermost)
}
```

**Por que essa ordem é crítica:**
1. **Rate limiting primeiro**: Protege contra abuso antes de qualquer processamento
2. **Stores injetados**: Auth e CSRF middlewares precisam das stores disponíveis
3. **Auth antes de CSRF**: Precisa saber quem é o usuário para validar token da sessão dele
4. **CSRF no protected routes**: Apenas rotas que modificam estado precisam de validação

### SessionStore Thread-Safety

**TODOS** os métodos de `SessionStore` devem ser `async` e usar `RwLock`:

```rust
// ✅ CERTO - Thread-safe
impl SessionStore {
    pub async fn validate_session(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;  // Lock de leitura
        sessions.get(session_id).cloned()
    }

    pub async fn create_session(&self, user_id: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;  // Lock de escrita
        sessions.insert(session_id.clone(), user_id);
        session_id
    }
}
```

### CSRF Token per Session

**CADA** sessão deve ter **EXATAMENTE UM** token CSRF:

```rust
// ❌ ERRADO - Múltiplos tokens por sessão
csrf_store.store(&session_id, "token-1").await;
csrf_store.store(&session_id, "token-2").await;  // Sobrescreve token-1

// ✅ CERTO - Um token por sessão, reutilizado
if let Some(existing) = csrf_store.get(&session_id).await {
    // Usa token existente
} else {
    // Gera novo token e armazena
    let token = CsrfToken::generate();
    csrf_store.store(&session_id, token.as_str()).await;
}
```

### Safe Methods vs Unsafe Methods

**MÉTODOS SEGUROS** (NÃO requerem CSRF): `GET`, `HEAD`, `OPTIONS`, `TRACE`
**MÉTODOS INSEGUROS** (REQUEREM CSRF): `POST`, `PUT`, `DELETE`, `PATCH`

```rust
// ✅ CERTO - CSRF middleware implementa essa lógica
fn is_safe_method(method: &axum::http::Method) -> bool {
    matches!(
        *method,
        axum::http::Method::GET
            | axum::http::Method::HEAD
            | axum::http::Method::OPTIONS
            | axum::http::Method::TRACE
    )
}

// No middleware:
if is_safe_method(request.method()) {
    return Ok(next.run(request).await);  // Passa sem validação
}
// Valida token para métodos inseguros
```

### Token Bucket Refill

**O refill de tokens DEVE ser contínuo e baseado no tempo decorrido**:

```rust
// ✅ CERTO - Refill contínuo
impl TokenBucket {
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed();
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate) as u32;

        self.tokens = std::cmp::min(self.capacity, self.tokens + tokens_to_add);
        self.last_refill = Instant::now();
    }
}

// ❌ ERRADO - Refill fixo (perde requests entre refills)
fn refill_fixed(&mut self) {
    // Refill acontece apenas quando chamado explicitamente
    self.tokens = self.capacity;  // Perde tokens que deveriam ser adicionados gradualmente
}
```

### IP Extraction Order

**A ordem de extração de IP é PREDEFINIDA**:

```rust
// ✅ CERTO - Ordem correta
fn extract_client_ip(headers: &HeaderMap) -> String {
    // 1. Verifica x-forwarded-for (proxy/CDN)
    if let Some(forwarded_for) = headers.get("x-forwarded-for")
        && let Ok(forwarded_str) = forwarded_for.to_str()
        && let Some(client_ip) = forwarded_str.split(',').next()
    {
        return client_ip.trim().to_string();
    }

    // 2. Verifica x-real-ip (nginx, apache)
    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(real_ip_str) = real_ip.to_str()
    {
        return real_ip_str.to_string();
    }

    // 3. Fallback para "unknown"
    "unknown".to_string()
}
```

**Por que essa ordem:**
- `x-forwarded-for` é o header padrão para proxies/CDNs
- Primeiro IP na lista é sempre o IP original do cliente
- `x-real-ip` é fallback para outros reverse proxies
- "unknown" evita bloquear todas as requisições se IP não for identificado

## Padrões de Uso

### Como Adicionar Middleware no Router

```rust
use crate::middleware::{
    auth::auth_middleware,
    csrf::csrf_middleware,
    rate_limit::{create_rate_limiter_from_env, rate_limit_middleware},
};

pub fn create_router(state: AppState) -> Router {
    let rate_limiter = create_rate_limiter_from_env();

    Router::new()
        .merge(protected_routes())
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,  // Aplicado em TODAS as rotas
        ))
        .layer(Extension(state.csrf_store.clone()))
        .layer(Extension(state.session_store.clone()))
        .layer(cors)
        .with_state(state)
}
```

### Como Criar Rotas Protegidas

```rust
fn protected_routes() -> Router<AppState> {
    Router::new()
        // Rotas protegidas (requerem auth + CSRF)
        .route("/api/loops", get(list_loops).post(create_loop))
        .route("/api/loops/{id}", get(get_loop).delete(delete_loop))
        .route("/api/loops/{id}/start", post(start_loop))
        .route("/api/loops/{id}/tasks", get(list_tasks).post(create_task))
        // MIDDLEWARE ORDEM CRÍTICA (inverter = não funciona)
        .route_layer(axum::middleware::from_fn(csrf_middleware))  // CSRF primeiro
        .route_layer(axum::middleware::from_fn(auth_middleware))   // Auth depois
}
```

### Como Usar Auth em Handlers

```rust
use axum::{Extension, Json};
use crate::handlers::auth::AppState;

pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,  // Injetado pelo auth_middleware
    Json(payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    // user_id está disponível automaticamente
    // Se não autenticado, request não chega aqui (retorna 401 no middleware)

    // Atribuir ownership do loop ao usuário autenticado
    let mut create_data = payload;
    create_data.owner_id = user_id;

    // ... restante da lógica
}
```

### Como Usar CSRF Token em Handlers

```rust
use axum::Extension;
use crate::middleware::csrf::CsrfToken;

pub async fn new_loop_form(
    Extension(csrf_token): Extension<CsrfToken>,  // Injetado pelo csrf_middleware
) -> (StatusCode, Html<String>) {
    let token = csrf_token.as_str();  // Converte para &str

    // Template recebe o token
    let template = NewLoopTemplate {
        csrf_token: token.to_string(),
        // ... outros campos
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Error: {}", e))),
    }
}
```

### Como Enviar CSRF Token do Frontend

**Form HTML:**
```html
<form hx-post="/api/loops" hx-target="#result">
    <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
    <input type="text" name="name" placeholder="Loop name" />
    <textarea name="description" placeholder="Description"></textarea>
    <button type="submit">Create Loop</button>
</form>
```

**JavaScript Fetch:**
```javascript
fetch("/api/loops", {
    method: "POST",
    headers: {
        "Content-Type": "application/json",
        "x-csrf-token": csrfToken,  // Token obtido de meta tag ou cookie
    },
    body: JSON.stringify({
        name: "My Loop",
        description: "Description",
    }),
});
```

### Como Configurar Rate Limiting

```bash
# .env
RATE_LIMIT=60  # requests por minuto (default: 100)
```

```rust
// Rate limiting é automático quando aplicado no router
// Retorno 429 com header Retry-After quando limit excedido

// Cliente pode implementar retry automático:
fetch("/api/loops")
    .then(response => {
        if (response.status === 429) {
            const retryAfter = response.headers.get('Retry-After');
            setTimeout(() => retryRequest(), retryAfter * 1000);
        }
    });
```

### Como Gerenciar Sessões

```rust
// No login handler
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<LoginResponse>) {
    match state.auth_service.authenticate(&payload.username, &payload.password).await {
        Ok(user) => {
            // Criar sessão
            let session_id = state.session_store.create_session(user.id.clone()).await;

            // Gerar token CSRF para a sessão
            let csrf_token = CsrfToken::generate();
            state.csrf_store.store(&session_id, csrf_token.as_str()).await;

            (StatusCode::OK, Json(LoginResponse {
                success: true,
                user_id: user.id,
                session_token: session_id,
                csrf_token: csrf_token.to_string(),
            }))
        }
        Err(e) => (StatusCode::UNAUTHORIZED, Json(...)),
    }
}

// No logout handler
pub async fn logout(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<LogoutResponse>) {
    // Extrair session_id de headers
    if let Some(session_id) = extract_session_id(request.headers()) {
        // Deletar sessão
        state.session_store.delete_session(&session_id).await;
        // Deletar token CSRF
        state.csrf_store.delete(&session_id).await;
    }

    (StatusCode::OK, Json(LogoutResponse { success: true }))
}
```

## Anti-padrões

### NUNCA FAZER

**1. Inverter ordem de middleware**
```rust
// ❌ ERRADO - CSRF antes de Auth (session_id não disponível)
.route_layer(axum::middleware::from_fn(csrf_middleware))
.route_layer(axum::middleware::from_fn(auth_middleware))

// ✅ CERTO - Auth primeiro, CSRF depois
.route_layer(axum::middleware::from_fn(csrf_middleware))
.route_layer(axum::middleware::from_fn(auth_middleware))
```

**2. CSRF em métodos seguros**
```rust
// ❌ ERRADO - Validando CSRF em GET
if let Some(provided) = extract_csrf_token(request.headers()) {
    if !csrf_store.validate(&session_id, &provided).await {
        return Err(StatusCode::FORBIDDEN);
    }
}

// ✅ CERTO - Apenas valida métodos inseguros
if !is_safe_method(request.method()) {
    if let Some(provided) = extract_csrf_token(request.headers()) {
        if !csrf_store.validate(&session_id, &provided).await {
            return Err(StatusCode::FORBIDDEN);
        }
    }
}
```

**3. Bloquear requisições sem IP identificado**
```rust
// ❌ ERRADO - Bloqueia todos sem headers de IP
if extract_client_ip(headers) == "unknown" {
    return Err(StatusCode::FORBIDDEN);
}

// ✅ CERTO - Usa "unknown" como bucket compartilhado
let client_ip = extract_client_ip(headers);  // Retorna "unknown" se não encontrado
// Rate limiting continua funcionando (todos os "unknown" compartilham bucket)
```

**4. Sync locks em async context**
```rust
// ❌ ERRADO - Mutex pode bloquear thread
use std::sync::Mutex;
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, String>>>,  // Mutex bloqueia thread
}

// ✅ CERTO - RwLock permite múltiplas leituras
use tokio::sync::RwLock;
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, String>>>,  // RwLock é async-friendly
}
```

**5. Ignorar Retry-After header**
```rust
// ❌ ERRADO - Retorna 429 sem Retry-After
Err(StatusCode::TOO_MANY_REQUESTS)

// ✅ CERTO - Inclui Retry-After header
let mut response = (StatusCode::TOO_MANY_REQUESTS, ()).into_response();
response.headers_mut().insert(
    header::RETRY_AFTER,
    retry_after.to_string().parse().unwrap(),
);
Ok(response)
```

**6. Token generation não-cryptográfico**
```rust
// ❌ ERRADO - Previsível
let token = format!("csrf-{}", std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());

// ✅ CERTO - Criptograficamente seguro
let mut rng = rand::thread_rng();
let token: String = (0..32)
    .map(|_| {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        CHARSET[rng.gen_range(0..CHARSET.len())] as char
    })
    .collect();
```

**7. Verificar user_id manualmente**
```rust
// ❌ ERRADO - Duplicando lógica do auth middleware
pub async fn handler(request: Request) -> ... {
    if let Some(session_id) = extract_session_id(request.headers()) {
        if let Some(user_id) = session_store.validate_session(&session_id).await {
            // Usar user_id...
        }
    }
}

// ✅ CERTO - Usar Extension injetado pelo middleware
pub async fn handler(Extension(user_id): Extension<String>) -> ... {
    // user_id já está validado pelo auth_middleware
    // Se não autenticado, request nem chega aqui
}
```

## Padrões Específicos

### Request Flow

```
Client Request
    ↓
┌─────────────────────────────────────┐
│  Router Middleware (outermost)     │
├─────────────────────────────────────┤
│ 1. Rate Limiting                  │ ← Primeiro (IP level)
│    - Extrai IP de headers         │
│    - Verifica bucket de tokens     │
│    - Retorna 429 se limitado      │
└──────────────┬────────────────────┘
               │ Passa
               ▼
┌─────────────────────────────────────┐
│ 2. Extension Injections           │
│    - SessionStore                 │
│    - CsrfTokenStore               │
└──────────────┬────────────────────┘
               │ Passa
               ▼
┌─────────────────────────────────────┐
│ 3. CORS                          │
│    - Verifica origins             │
│    - Adiciona headers CORS        │
└──────────────┬────────────────────┘
               │ Passa
               ▼
┌─────────────────────────────────────┐
│ 4. Auth Middleware (protected)   │ ← Para rotas protegidas
│    - Extrai session_id de headers │
│    - Valida com SessionStore      │
│    - Injeta user_id em extensions │
│    - Retorna 401 se inválido       │
└──────────────┬────────────────────┘
               │ Passa
               ▼
┌─────────────────────────────────────┐
│ 5. CSRF Middleware (protected)   │ ← Para rotas protegidas
│    - Verifica método HTTP         │
│    - Se seguro: passa            │
│    - Se inseguro: valida token   │
│    - Injeta CSRF token           │
│    - Retorna 403 se inválido      │
└──────────────┬────────────────────┘
               │ Passa
               ▼
┌─────────────────────────────────────┐
│  Handler                         │ ← Processamento final
│    - Extension(user_id)           │
│    - Extension(csrf_token)        │
│    - State(state)                │
│    - Json(payload)               │
└─────────────────────────────────────┘
```

### Session Lifecycle

```
1. Login
   ↓
2. AuthService.authenticate() valida credentials
   ↓
3. SessionStore.create_session(user_id)
   - Gera session_id (UUID v4)
   - Armazena: session_id → user_id
   ↓
4. CsrfToken.generate()
   - Gera token criptograficamente seguro
   ↓
5. CsrfTokenStore.store(session_id, csrf_token)
   - Armazena: session_id → csrf_token
   ↓
6. Retorna para cliente: { session_id, csrf_token }
   ↓
7. Cliente envia session_id em headers ("session" ou "authorization")
   ↓
8. Cliente envia csrf_token em headers ("x-csrf-token")
   ↓
9. Request chega ao servidor
   ↓
10. Auth middleware valida session_id → user_id
    ↓
11. CSRF middleware valida csrf_token → armazenado
    ↓
12. Handler processa com user_id disponível
   ↓
13. Logout
    ↓
14. SessionStore.delete_session(session_id)
    ↓
15. CsrfTokenStore.delete(session_id)
```

### Token Bucket Refill

```
Initial state:
- Capacity: 100 tokens
- Current tokens: 100
- Refill rate: 1.67 tokens/second (100/min / 60)

Request 1: Consume 1 token
- Tokens: 99
- Time: T + 0s

Request 2: Consume 1 token
- Tokens: 98
- Time: T + 0.5s

... (consumindo tokens)

Request 100: Consume 1 token
- Tokens: 0
- Time: T + 30s

Request 101: Rate limited!
- Wait for refill
- Time elapsed: 30s
- Tokens to add: 30s * 1.67 = 50 tokens
- New tokens: 50
- Request succeeds
- Tokens: 49
```

## Dependências

### Dependencies (Cargo.toml)

```toml
[dependencies]
# Web framework
axum.workspace = true

# Async runtime
tokio.workspace = true

# Utilities
rand = "0.8"                    # Para CSRF token generation
uuid.workspace = true           # Para session_id generation

# Logging
tracing.workspace = true
```

### Dependencies Internas

Nenhuma - Middleware é um módulo self-contained que depende apenas de:
- `axum` para extractors e tipos de middleware
- `tokio::sync` para primitivos de concorrência
- `std` e `rand` para utilities

### Downstreams (quem usa este módulo)

- `ralph-server/router.rs` - Configura middleware no router
- `ralph-server/handlers/*.rs` - Usa extractors (Extension, SessionStore, CsrfToken)

## Armadilhas

### Confusões Comuns

**1. Extension vs State vs Extractors**

```rust
// Extension: dados injetados por middleware
Extension(user_id): Extension<String>      // user_id do auth_middleware
Extension(csrf_token): Extension<CsrfToken> // csrf_token do csrf_middleware

// State: application state compartilhada
State(state): State<AppState>  // AppState configurado no router

// Extractors: dados extraídos da request
Path(id): Path<String>         // /api/loops/{id}
Query(params): Query<ListQuery> // ?page=1&limit=10
Json(payload): Json<CreateLoop> // Body JSON
```

**2. SessionStore vs CsrfTokenStore**

```rust
// SessionStore: mapeia session_id → user_id
let user_id = session_store.validate_session(&session_id).await;

// CsrfTokenStore: mapeia session_id → csrf_token
let token = csrf_store.get(&session_id).await;
let valid = csrf_store.validate(&session_id, &provided_token).await;

// Ambos são independentes mas relacionados pelo session_id
```

**3. Mutex vs RwLock**

```rust
// Mutex: apenas um "borrow" por vez (leitura OU escrita)
use std::sync::Mutex;
let data = Arc<Mutex<HashMap<String, String>>>;

let guard = data.lock().unwrap();  // Bloqueia para leitura OU escrita
// Mesmo leituras concorrentes esperam por outras leituras

// RwLock: múltiplas leituras, escrita exclusiva
use tokio::sync::RwLock;
let data = Arc<RwLock<HashMap<String, String>>>;

let guard = data.read().await;    // Bloqueia apenas para escritas
let mut guard = data.write().await; // Bloqueia para leituras E escritas
```

**4. Safe methods não requerem CSRF**

```rust
// ❌ ERRADO - GET precisa de CSRF
let response = fetch("/api/loops", {
    method: "GET",
    headers: { "x-csrf-token": token }
});

// ✅ CERTO - GET não precisa de CSRF
let response = fetch("/api/loops", {
    method: "GET"  // CSRF middleware ignora método seguro
});

// Apenas métodos inseguros precisam:
let response = fetch("/api/loops", {
    method: "POST",
    headers: { "x-csrf-token": token }  // Obrigatório
});
```

**5. Rate limiting é por IP, não por usuário**

```rust
// Rate limiting é baseado em IP do cliente
// Mesmo usuário autenticado com múltiplos IPs = múltiplos buckets

// Exemplo: usuário com IP diferente (VPN, mudança de rede)
// Cria novo bucket de tokens, rate limit começa do zero

// Para rate limiting por usuário, seria necessário armazenar user_id
// no bucket, mas isso não é implementado atualmente
```

### Comportamentos Inesperados

**1. CSRF token persiste durante sessão**

```rust
// CSRF token é gerado UMA vez por sessão e reutilizado
// Não muda a cada request

// Isso é correto: CSRF token deve persistir
// Cliente deve guardar token e enviar em todas as mutations

// ❌ Não regenerar token a cada request
// Isso quebraria form submissions e requisições AJAX
```

**2. Rate limiting refill é contínuo**

```rust
// Tokens são adicionados continuamente, não em intervalos fixos
// Exemplo: 60 requests/min = 1 token/segundo

// Após 30 segundos sem requests:
// - Tokens recuperados: 30
// - Se capacity era 100 e tinhamos 0, agora temos 30

// Isso é diferente de "fixed window" onde tokens são resetados
// Token bucket permite bursts (consumir todos de uma vez, depois esperar)
```

**3. IP extraction fallback para "unknown"**

```rust
// Se request não tem x-forwarded-for nem x-real-ip:
// IP é "unknown"

// Todas as requisições com "unknown" compartilham MESMO bucket
// Isso pode limitar indevidamente se muitos clientes não têm headers

// Em produção, configure reverse proxy (nginx, cloudflare, etc)
// para sempre adicionar x-forwarded-for ou x-real-ip
```

**4. SessionStore não expira sessões automaticamente**

```rust
// Sessões NÃO expiram automaticamente
// Persistem até serem explicitamente deletadas em logout

// Isso é uma limitação conhecida da implementação atual
// Futuro: implementar TTL/expiry com background cleanup task

// Para logout:
session_store.delete_session(&session_id).await;
csrf_store.delete(&session_id).await;
```

**5. Middleware order é outer-to-inner**

```rust
// Router layers são aplicados de fora para dentro
// Primeiro layer aplicado = executado primeiro (outermost)
// Último layer aplicado = executado por último (innermost)

Router::new()
    .layer(middleware_A)  // Executa PRIMEIRO
    .layer(middleware_B)  // Executa DEPOIS de A
    .layer(middleware_C)  // Executa DEPOIS de B
    .route_layer(middleware_D)  // Executa DEPOIS de C
    .route_layer(middleware_E)  // Executa DEPOIS de D (último, antes do handler)

// Order de execução: A → B → C → D → E → Handler
```

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral do projeto
- `/ralph-server/AGENTS.md` - Uso de middleware em handlers e router
- `/ralph-services/AGENTS.md` - AuthService para autenticação de credenciais

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
