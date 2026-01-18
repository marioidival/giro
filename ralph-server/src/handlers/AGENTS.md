# Ralph Server - HTTP Handlers

## Propósito

Este módulo contém todos os HTTP handlers do Ralph Loop Manager. É a ponte entre as requisições HTTP e a lógica da aplicação, coordenando validação, autenticação, repositories e serviços.

**O que esta área faz:**
- Processa requisições HTTP e retorna respostas apropriadas
- Valida input de usuários (username, email, passwords, nomes de loops)
- Executa verificações de ownership para segurança
- Coordena chamadas a repositories e services
- Renderiza templates HTML para HTMX
- Controla execução de loops (start, pause, resume, stop)
- Gerencia health checks do sistema

**O que esta área NÃO faz:**
- NÃO contém lógica de negócio (delegado a `ralph-services`)
- NÃO acessa banco de dados diretamente (usa repositories via `ralph-repositories`)
- NÃO gerencia containers Docker (delegado a `ralph-services`)
- NÃO valida dados de lógica complexa (delegado a repositories/services)

## Estrutura

```
handlers/
├── mod.rs       # Re-export público de todos os handlers
├── auth.rs      # Authentication handlers (register, login, logout)
├── health.rs    # Health check endpoint
├── loops.rs     # Loop CRUD + control handlers (start, pause, resume, stop)
└── tasks.rs     # Task CRUD handlers
```

### Arquivos por Responsabilidade

**auth.rs:**
- `register()` - Criação de novo usuário
- `login()` - Autenticação de usuário
- `logout()` - Encerramento de sessão
- Response structs: `RegisterResponse`, `LoginResponse`, `LogoutResponse`

**health.rs:**
- `health_check()` - Verifica status do sistema e banco de dados
- Response struct: `HealthResponse`

**loops.rs:**
- CRUD: `create_loop()`, `list_loops()`, `get_loop()`, `delete_loop()`
- HTML pages: `list_loops_page()`, `new_loop_form()`
- Control: `start_loop()`, `pause_loop()`, `resume_loop()`, `stop_loop()`
- Response structs: `CreateLoopResponse`, `ListLoopsResponse`, `GetLoopResponse`, `DeleteLoopResponse`, `LoopControlResponse`
- DTOs: `LoopSummary`, `LoopDetail` (sem e com PRD)

**tasks.rs:**
- CRUD: `create_task()`, `list_tasks()`, `get_task()`, `delete_task()`
- Response structs: `CreateTaskResponse`, `ListTasksResponse`, `GetTaskResponse`, `DeleteTaskResponse`
- DTOs: `TaskSummary`, `TaskDetail` (sem e com description/error_message)

## Invariantes Críticos

### Handler Signature Padrão

**TODOS** os handlers devem seguir a mesma assinatura:

```rust
pub async fn handler_name(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    // outros extractores
) -> (StatusCode, Json<ResponseType>) {
    // ...
}

// Para HTML templates (HTMX)
pub async fn page_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    // ...
}
```

**Por que isso é crítico:**
- Consistência em toda a codebase
- Facilita manutenção e refatoração
- Permite composição de middleware

### Response Struct Padrão

**TODAS** as response structs devem seguir este padrão:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseType {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
}
```

**Regras:**
- `success` - Indica se operação foi bem-sucedida
- `message` - Mensagem descritiva do resultado
- Campos opcionais devem usar `#[serde(skip_serializing_if = "Option::is_none")]`
- Derive `PartialEq` para facilitar testes

### Validação Inline

**SEMPRE** valide input antes de chamar services/repositories:

```rust
// ❌ ERRADO - Validação em service
pub async fn create_loop(State(state): State<AppState>, Json(payload): Json<CreateLoop>) {
    // Validação fica aqui, no handler
    if payload.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(...));
    }
    state.loop_repository.create(payload).await?;
}

// ❌ ERRADO - Sem validação
pub async fn create_loop(State(state): State<AppState>, Json(payload): Json<CreateLoop>) {
    state.loop_repository.create(payload).await?;
}

// ✅ CERTO - Validação no handler
pub async fn create_loop(State(state): State<AppState>, Json(payload): Json<CreateLoop>) {
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
    state.loop_repository.create(payload).await?;
}
```

### Ownership Check em Handlers de Recursos

**SEMPRE** verifique ownership antes de retornar recursos:

```rust
// ❌ ERRADO - Sem verificação
pub async fn get_loop(Path(id): Path<String>) {
    let loop_ = state.loop_repository.find_by_id(&id).await?.unwrap();
    (StatusCode::OK, Json(loop_))
}

// ✅ CERTO - Com verificação
pub async fn get_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (StatusCode::UNAUTHORIZED, Json(...));
            }
            (StatusCode::OK, Json(GetLoopResponse {
                success: true,
                loop_: Some(LoopDetail::from(loop_)),
                message: "Loop retrieved".to_string(),
            }))
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(...)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(...)),
    }
}
```

### Match Pattern para Repository Results

**SEMPRE** use pattern matching completo:

```rust
// ❌ ERRADO - Usando unwrap()
let loop_ = state.loop_repository.find_by_id(&id).await.unwrap();

// ❌ ERRADO - Pattern incompleto
let loop_ = state.loop_repository.find_by_id(&id).await.unwrap_or(...);

// ✅ CERTO - Pattern completo
match state.loop_repository.find_by_id(&id).await {
    Ok(Some(loop_)) => {
        // tratamento para loop encontrado
    }
    Ok(None) => {
        // tratamento para loop não encontrado
        (StatusCode::NOT_FOUND, Json(...))
    }
    Err(e) => {
        // tratamento para erro
        (StatusCode::INTERNAL_SERVER_ERROR, Json(...))
    }
}
```

### Query Parameters com Defaults

**SEMPRE** defina funções de default para query params:

```rust
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_page")]
    pub page: usize,

    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_page() -> usize { 1 }
fn default_limit() -> usize { 10 }
```

## Padrões de Uso

### Handler Auth (Register/Login)

```rust
use axum::{Json, extract::State, http::StatusCode};
use ralph_models::CreateUser;
use crate::handlers::auth::AppState;
use crate::validation::{validate_username, validate_email, validate_password};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<RegisterResponse>) {
    // 1. Validar username
    if let Err(e) = validate_username(&payload.username) {
        return (StatusCode::BAD_REQUEST, Json(RegisterResponse {
            success: false,
            message: format!("Invalid username: {}", e.message.unwrap_or_default()),
            user_id: None,
        }));
    }

    // 2. Validar email
    if let Err(e) = validate_email(&payload.email) {
        return (StatusCode::BAD_REQUEST, Json(RegisterResponse {
            success: false,
            message: format!("Invalid email: {}", e.message.unwrap_or_default()),
            user_id: None,
        }));
    }

    // 3. Validar password
    if let Err(e) = validate_password(&payload.password) {
        return (StatusCode::BAD_REQUEST, Json(RegisterResponse {
            success: false,
            message: format!("Invalid password: {}", e.message.unwrap_or_default()),
            user_id: None,
        }));
    }

    // 4. Chamar auth service
    match state.auth_service.register(payload).await {
        Ok(user) => {
            // 5. Criar sessão
            let _session_token = state.session_store.create_session(user.id.clone()).await;

            (
                StatusCode::CREATED,
                Json(RegisterResponse {
                    success: true,
                    message: "User registered successfully".to_string(),
                    user_id: Some(user.id),
                }),
            )
        }
        Err(e) => {
            let error_msg = e.to_string();

            // 6. Mapear erro para status code apropriado
            if error_msg.contains("already exists") {
                (
                    StatusCode::CONFLICT,
                    Json(RegisterResponse {
                        success: false,
                        message: error_msg,
                        user_id: None,
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(RegisterResponse {
                        success: false,
                        message: "Registration failed".to_string(),
                        user_id: None,
                    }),
                )
            }
        }
    }
}
```

### Handler CRUD (Create)

```rust
pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(mut payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    // 1. Validar
    if let Err(e) = validate_loop_name(&payload.name) {
        return (StatusCode::BAD_REQUEST, Json(CreateLoopResponse {
            success: false,
            message: format!("Invalid loop name: {}", e.message.unwrap_or_default()),
            loop_id: None,
        }));
    }

    // 2. Atribuir ownership
    payload.owner_id = user_id;

    // 3. Chamar repository
    match state.loop_repository.create(payload).await {
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

### Handler com Paginação (List)

```rust
pub async fn list_loops(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<ListLoopsQuery>,
) -> (StatusCode, Json<ListLoopsResponse>) {
    match state.loop_repository.list_by_owner(&user_id).await {
        Ok(mut all_loops) => {
            let total = all_loops.len();

            // 1. Calcular pagination
            let start = if query.page > 0 {
                (query.page - 1) * query.limit
            } else {
                0
            };

            let end = std::cmp::min(start + query.limit, total);

            // 2. Extrair slice paginado
            let loops: Vec<LoopSummary> = if start < total {
                all_loops.drain(start..end).map(LoopSummary::from).collect()
            } else {
                Vec::new()
            };

            (
                StatusCode::OK,
                Json(ListLoopsResponse {
                    success: true,
                    loops,
                    page: query.page,
                    limit: query.limit,
                    total,
                }),
            )
        }
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListLoopsResponse {
                success: false,
                loops: Vec::new(),
                page: query.page,
                limit: query.limit,
                total: 0,
            }),
        ),
    }
}
```

### Handler com Ownership Check (Get)

```rust
pub async fn get_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<GetLoopResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            // 1. Verificar ownership
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

            // 2. Retornar com DTO
            (
                StatusCode::OK,
                Json(GetLoopResponse {
                    success: true,
                    message: "Loop retrieved successfully".to_string(),
                    loop_: Some(LoopDetail::from(loop_)),
                }),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(GetLoopResponse {
                success: false,
                message: "Loop not found".to_string(),
                loop_: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetLoopResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                loop_: None,
            }),
        ),
    }
}
```

### Handler de Controle (Loop Control)

```rust
pub async fn start_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<LoopControlResponse>) {
    // 1. Verificar ownership
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(LoopControlResponse {
                        success: false,
                        message: "You do not have permission to control this loop".to_string(),
                        status: loop_.status.to_string(),
                    }),
                );
            }

            // 2. Chamar executor
            match state.loop_executor.start(&id).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(LoopControlResponse {
                        success: true,
                        message: "Loop started successfully".to_string(),
                        status: "running".to_string(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LoopControlResponse {
                        success: false,
                        message: format!("Failed to start loop: {}", e),
                        status: loop_.status.to_string(),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(LoopControlResponse {
                success: false,
                message: "Loop not found".to_string(),
                status: "unknown".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoopControlResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                status: "unknown".to_string(),
            }),
        ),
    }
}
```

### Handler de HTML Template (HTMX)

```rust
use askama::Template;
use axum::response::Html;

#[derive(Template)]
#[template(path = "loops/index.html")]
struct LoopListTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loops: Vec<LoopSummary>,
    pub page: u32,
    pub total_pages: u32,
    pub has_prev: bool,
    pub has_next: bool,
}

pub async fn list_loops_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<ListLoopsQuery>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let csrf_token = CsrfToken::generate().to_string();

    // 1. Buscar dados
    match state.loop_repository.list_by_owner(&user_id).await {
        Ok(mut all_loops) => {
            let total = all_loops.len();
            let total_pages = if total == 0 { 1 } else { total.div_ceil(query.limit) };

            let start = if query.page > 0 {
                (query.page - 1) * query.limit
            } else {
                0
            };

            let end = std::cmp::min(start + query.limit, total);

            let loops: Vec<LoopSummary> = if start < total {
                all_loops.drain(start..end).map(LoopSummary::from).collect()
            } else {
                Vec::new()
            };

            let page = query.page as u32;

            // 2. Criar template
            let template = LoopListTemplate {
                logged_in,
                csrf_token,
                loops,
                page,
                total_pages: total_pages as u32,
                has_prev: page > 1,
                has_next: page < total_pages as u32,
            };

            // 3. Renderizar
            match template.render() {
                Ok(html) => (StatusCode::OK, Html(html)),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("Failed to render template: {}", e)),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to retrieve loops: {}", e)),
        ),
    }
}
```

### DTO Pattern (From Implementations)

```rust
// DTO leve para listas (sem campos pesados)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub status: String,
    pub current_iteration: i32,
    pub created_at: String,
    pub updated_at: String,
    // ... outros campos EXCETO prd
}

impl From<ralph_models::Loop> for LoopSummary {
    fn from(loop_: ralph_models::Loop) -> Self {
        Self {
            id: loop_.id,
            name: loop_.name,
            description: loop_.description,
            owner_id: loop_.owner_id,
            status: loop_.status.to_string(),
            current_iteration: loop_.current_iteration,
            created_at: loop_.created_at.to_rfc3339(),
            updated_at: loop_.updated_at.to_rfc3339(),
        }
    }
}

// DTO completo para detalhes (com campos pesados)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub prd: String,  // Campo pesado incluído
    pub owner_id: String,
    pub status: String,
    pub current_iteration: i32,
    pub created_at: String,
    pub updated_at: String,
    // ... outros campos
}

impl From<ralph_models::Loop> for LoopDetail {
    fn from(loop_: ralph_models::Loop) -> Self {
        Self {
            id: loop_.id,
            name: loop_.name,
            description: loop_.description,
            prd: loop_.prd,  // PRD incluído
            owner_id: loop_.owner_id,
            status: loop_.status.to_string(),
            current_iteration: loop_.current_iteration,
            created_at: loop_.created_at.to_rfc3339(),
            updated_at: loop_.updated_at.to_rfc3339(),
        }
    }
}
```

### Handler com Nested Ownership Check (Task)

```rust
pub async fn create_task(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(loop_id): Path<String>,
    Json(payload): Json<CreateTask>,
) -> (StatusCode, Json<CreateTaskResponse>) {
    // 1. Validar título
    if payload.title.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(CreateTaskResponse {
            success: false,
            message: "Title is required".to_string(),
            task_id: None,
        }));
    }

    // 2. Verificar ownership do loop (nested resource)
    match state.loop_repository.find_by_id(&loop_id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(CreateTaskResponse {
                        success: false,
                        message: "You do not have permission to add tasks to this loop".to_string(),
                        task_id: None,
                    }),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(CreateTaskResponse {
                    success: false,
                    message: "Loop not found".to_string(),
                    task_id: None,
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateTaskResponse {
                    success: false,
                    message: format!("Failed to verify loop: {}", e),
                    task_id: None,
                }),
            );
        }
    }

    // 3. Criar task com loop_id
    let create_task = CreateTask {
        loop_id: loop_id.clone(),
        title: payload.title,
        description: payload.description,
        priority: payload.priority,
        parent_task_id: payload.parent_task_id,
        created_by: "user".to_string(),
    };

    match state.task_repository.create(create_task).await {
        Ok(task) => (
            StatusCode::CREATED,
            Json(CreateTaskResponse {
                success: true,
                message: "Task created successfully".to_string(),
                task_id: Some(task.id),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateTaskResponse {
                success: false,
                message: format!("Failed to create task: {}", e),
                task_id: None,
            }),
        ),
    }
}
```

## Anti-padrões

### NUNCA FAZER

**1. Validação no Repository/Service**
```rust
// ❌ ERRADO - Repository não deve validar
impl LoopRepository {
    pub async fn create(&self, loop_: CreateLoop) -> Result<Loop> {
        if loop_.name.trim().is_empty() {
            return Err(Error::Validation("Name is required".to_string()));
        }
        // ...
    }
}

// ✅ CERTO - Handler valida, repository persiste
pub async fn create_loop(
    State(state): State<AppState>,
    Json(payload): Json<CreateLoop>,
) {
    if payload.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(...));
    }
    state.loop_repository.create(payload).await?;
}
```

**2. Ignorar Ownership Check**
```rust
// ❌ ERRADO - Qualquer usuário pode acessar
pub async fn get_loop(Path(id): Path<String>) {
    let loop_ = state.loop_repository.find_by_id(&id).await?.unwrap();
    (StatusCode::OK, Json(loop_))
}

// ✅ CERTO - Verificar ownership
pub async fn get_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (StatusCode::UNAUTHORIZED, Json(...));
            }
            (StatusCode::OK, Json(...))
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(...)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(...)),
    }
}
```

**3. Usar unwrap() sem tratamento**
```rust
// ❌ ERRADO - Pode panic
pub async fn get_loop(Path(id): Path<String>) {
    let loop_ = state.loop_repository.find_by_id(&id).await.unwrap();
    (StatusCode::OK, Json(loop_))
}

// ✅ CERTO - Pattern matching completo
pub async fn get_loop(Path(id): Path<String>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => (StatusCode::OK, Json(loop_)),
        Ok(None) => (StatusCode::NOT_FOUND, Json(...)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(...)),
    }
}
```

**4. Retornar Response sem StatusCode**
```rust
// ❌ ERRADO - Sem StatusCode
pub async fn handler() -> Json<Response> {
    Json(response)
}

// ✅ CERTO - Com StatusCode
pub async fn handler() -> (StatusCode, Json<Response>) {
    (StatusCode::OK, Json(response))
}
```

**5. Retornar JSON para HTMX**
```rust
// ❌ ERRADO - HTMX espera HTML
pub async fn list_loops_page() -> Json<ListLoopsResponse> {
    Json(response)
}

// ✅ CERTO - HTMX precisa de HTML
pub async fn list_loops_page() -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html(html))
}
```

**6. Ignorar erros de template**
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

**7. Mudar mensagem genérica em vez de detalhar erro**
```rust
// ❌ ERRADO - Perdendo informações úteis
Err(e) => (
    StatusCode::INTERNAL_SERVER_ERROR,
    Json(Response {
        success: false,
        message: "An error occurred".to_string(),
    }),
)

// ✅ CERTO - Incluindo detalhes do erro
Err(e) => (
    StatusCode::INTERNAL_SERVER_ERROR,
    Json(Response {
        success: false,
        message: format!("Failed to create loop: {}", e),
    }),
)
```

**8. Criar handler sem teste de serialização**
```rust
// ❌ ERRADO - Sem teste
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response { ... }

// ✅ CERTO - Com teste
#[cfg(test)]
mod tests {
    #[test]
    fn test_response_serialization() {
        let response = Response {
            success: true,
            message: "Success".to_string(),
            loop_id: Some("id-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }
}
```

## Padrões Específicos dos Handlers

### Request Flow (Auth)

```
1. Client POST /api/auth/register
   ↓
2. Handler valida input (username, email, password)
   ↓
3. auth_service.register() - hash de password, cria User
   ↓
4. UserRepository.create() - persiste no banco
   ↓
5. session_store.create_session() - cria sessão
   ↓
6. Retorna 201 Created com { success, user_id, session_token }
```

### Request Flow (Loop CRUD)

```
1. Client POST /api/loops
   ↓
2. Auth middleware injeta user_id em extensions
   ↓
3. CSRF middleware valida token
   ↓
4. Handler valida loop name
   ↓
5. Handler atribui owner_id = user_id
   ↓
6. loop_repository.create() - persiste loop
   ↓
7. Retorna 201 Created com { success, loop_id }
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
5. loop_executor.start(loop_id)
   ↓
6. DockerManager cria container
   ↓
7. BroadcastManager.broadcast_loop_status("running")
   ↓
8. Clientes WebSocket recebem update
   ↓
9. Retorna 200 OK com { success, status: "running" }
```

### Request Flow (Task CRUD - Nested Resource)

```
1. Client POST /api/loops/{loop_id}/tasks
   ↓
2. Handler valida task title
   ↓
3. Handler verifica loop ownership (via loop_repository.find_by_id)
   ↓
4. Handler cria Task com loop_id
   ↓
5. task_repository.create()
   ↓
6. Retorna 201 Created com { success, task_id }
```

### Request Flow (HTML Page)

```
1. Client GET /loops?page=1&limit=10
   ↓
2. Handler gera CSRF token
   ↓
3. Handler verifica logged_in (!user_id.is_empty())
   ↓
4. Handler busca loops (loop_repository.list_by_owner)
   ↓
5. Handler calcula pagination (start, end, total_pages)
   ↓
6. Handler cria Template struct
   ↓
7. Handler renderiza template (template.render())
   ↓
8. Retorna 200 OK com HTML
```

## Dependências

### Imports Comuns

```rust
// Axum
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};

// Models
use ralph_models::{CreateUser, CreateUser, CreateLoop, CreateTask};

// State
use crate::handlers::auth::AppState;

// Middleware
use crate::middleware::csrf::CsrfToken;

// Templates
use askama::Template;
use crate::templates::{LoopListTemplate, LoopFormTemplate};

// Validation
use crate::validation::{
    validate_username,
    validate_email,
    validate_password,
    validate_loop_name,
};

// Serde
use serde::{Serialize, Deserialize};
```

### Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/ralph-server/AGENTS.md` - HTTP layer completo, middleware, templates
- `/ralph-models/AGENTS.md` - Models usados nos handlers (User, Loop, Task)
- `/ralph-repositories/AGENTS.md` - Repositories chamados pelos handlers
- `/ralph-services/AGENTS.md` - Services chamados pelos handlers (AuthService, LoopExecutor)
- `/ralph-server/src/middleware/` - Middleware que injeta dados nos handlers

## Armadilhas

### Confusões Comuns

**1. State vs Extension**

```rust
// State: Application state (serviços, repositories)
State(state): State<AppState>  // Injetado via router

// Extension: Request-specific data (injetado por middleware)
Extension(user_id): Extension<String>  // user_id injetado por auth middleware
Extension(csrf_token): Extension<CsrfToken>  // token injetado por CSRF middleware
```

**2. Path vs Query**

```rust
// Path: Extrai da URL path /api/loops/{id}
Path(id): Path<String>  // id vem de /api/loops/123

// Query: Extrai query params ?page=1&limit=10
Query(params): Query<ListQuery>  // params.page, params.limit
```

**3. Json vs Html**

```rust
// Json: Para APIs REST
-> (StatusCode, Json<Response>)  // Cliente espera JSON

// Html: Para HTMX e páginas web
-> (StatusCode, Html<String>)  // Cliente espera HTML
```

**4. LoopSummary vs LoopDetail**

```rust
// LoopSummary: Para listas (sem campos pesados)
// Não inclui PRD (pode ser muito grande)
// Usado em list_loops()

// LoopDetail: Para detalhes (com campos pesados)
// Inclui PRD completo
// Usado em get_loop()
```

**5. CreatedBy em Tasks**

```rust
// created_by é sempre "user" para tasks criadas via HTTP
// Tasks criadas pelo LLM têm created_by = "system"
let create_task = CreateTask {
    created_by: "user".to_string(),  // Manual/HTTP
    // ...
};
```

### Comportamentos Inesperados

**1. Session Management é automático no login**

```rust
// auth_service.register() NÃO cria sessão
// Handler deve criar explicitamente:
match state.auth_service.register(payload).await {
    Ok(user) => {
        let _session_token = state.session_store.create_session(user.id.clone()).await;
        // ...
    }
}
```

**2. Logout não recebe session_id no payload**

```rust
// Session ID vem do header, não do body
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) {
    let session_id = headers
        .get("session")
        .or_else(|| headers.get("authorization"))
        .and_then(|value| value.to_str().ok());

    if let Some(session_id) = session_id {
        state.session_store.delete_session(session_id).await;
    }
}
```

**3. Health check usa loop_repository.pool()**

```rust
// health_check precisa de acesso ao pool
async fn check_database_connection(repository: &LoopRepository) -> Result<(), sqlx::Error> {
    query("SELECT 1").fetch_one(repository.pool()).await?;
    Ok(())
}
```

**4. Pagination usa drain() para performance**

```rust
// drain() consome elementos e retorna ownership
let loops: Vec<LoopSummary> = if start < total {
    all_loops.drain(start..end).map(LoopSummary::from).collect()
} else {
    Vec::new()
};

// NOTA: all_loops fica vazia após drain()
```

**5. Loop executor retorna sucesso mesmo se falhar**

```rust
// loop_executor.start() pode retornar Ok() mesmo se container falhou depois
// É responsabilidade do LoopExecutor atualizar status do loop
match state.loop_executor.start(&id).await {
    Ok(_) => {
        // Executor iniciou com sucesso (pode falhar depois)
        (StatusCode::OK, Json(...))
    }
    Err(e) => {
        // Erro imediato (ex: loop não existe, Docker indisponível)
        (StatusCode::INTERNAL_SERVER_ERROR, Json(...))
    }
}
```

## Checklist Antes de Commitar

- [ ] Handler valida input antes de chamar repository/service
- [ ] Handler verifica ownership antes de retornar recursos
- [ ] Handler usa pattern matching completo (Ok(Some), Ok(None), Err)
- [ ] Response struct segue padrão (success, message, optional fields)
- [ ] Response usa `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] Handler retorna `(StatusCode, Json<Response>)` ou `(StatusCode, Html<String>)`
- [ ] Handler usa `State<AppState>` e `Extension<String>` corretamente
- [ ] Query params têm funções de default
- [ ] DTOs implementam `From<ralph_models::Model>` trait
- [ ] Template handlers tratam erros de renderização
- [ ] Handlers de auth criam sessão após registro/login
- [ ] Handlers de nested resources verificam ownership do parent
- [ ] Response structs têm testes de serialização
- [ ] Não há `unwrap()` sem tratamento
- [ ] Erros são formatados com detalhes (não mensagens genéricas)

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
