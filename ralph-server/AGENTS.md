# Ralph Server - HTTP Layer

## Purpose

This crate provides the HTTP layer of the Ralph Loop Manager. It is the entry point for all HTTP requests, managing handlers, middleware, templates, and WebSocket connections.

**What this area does:**
- HTTP server based on Axum framework with Tokio runtime
- Handlers for authentication (register, login, logout)
- CRUD handlers for Loops and Tasks
- Loop execution control (start, pause, resume, stop)
- Session-based authentication middleware
- CSRF protection middleware
- Rate limiting middleware (token bucket algorithm)
- Real-time WebSocket streaming for loop progress
- Askama templates with HTMX + Tailwind CSS
- Input validation (username, email, password, loop name)

**What this area does NOT do:**
- Does not contain business logic (that's `ralph-services`' responsibility)
- Does not access database directly (uses repositories via `ralph-repositories`)
- Does not manage Docker containers (that's `ralph-services`' responsibility)
- Does not execute loops (that's `ralph-services::LoopExecutor`'s responsibility)

## Structure

```
ralph-server/
├── src/
│   ├── main.rs           # Entry point, service initialization
│   ├── lib.rs           # Public re-exports
│   ├── router.rs         # Axum route configuration
│   ├── websocket.rs       # WebSocket handlers and BroadcastManager
│   ├── validation.rs      # Input validation functions
│   ├── handlers/
│   │   ├── mod.rs       # Handler re-exports
│   │   ├── auth.rs      # Register, login, logout handlers
│   │   ├── health.rs    # Health check endpoint
│   │   ├── loops.rs     # Loop CRUD + control handlers
│   │   └── tasks.rs     # Task CRUD handlers
│   ├── middleware/
│   │   ├── mod.rs           # Middleware re-exports
│   │   ├── auth.rs          # Session-based authentication
│   │   ├── csrf.rs          # CSRF token protection
│   │   └── rate_limit.rs    # Token bucket rate limiting
│   └── templates/
│       ├── mod.rs               # Template structs (Askama)
│       ├── base.html            # Base template with navbar/footer
│       ├── auth/
│       │   ├── login.html        # Login form
│       │   └── register.html     # Register form
│       └── loops/
│           ├── index.html          # Loop list page
│           └── new.html            # Create loop form
```

## Critical Invariants

### AppState Clonability

**ALWAYS** implement `Clone` for `AppState`:

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

**Why this is critical:**
- Axum needs to clone state for each request handler
- Handlers are `async fn`, so state needs to be `Clone`
- Repositories are already `Arc<T>` internally, so clone is cheap

### Response Consistency

**ALL** handlers must return consistent tuples:

```rust
// ✅ CORRECT - Tuple (StatusCode, Json<Response>)
pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    // ...
    (StatusCode::CREATED, Json(response))
}

// ✅ CORRECT - For HTML templates
pub async fn list_loops_page(
    // ...
) -> (StatusCode, Html<String>) {
    // ...
    (StatusCode::OK, Html(html))
}

// ❌ WRONG - Just Json (without StatusCode)
pub async fn handler() -> Json<Response> {
    Json(response)
}
```

### HTTP Status Codes

Use correct status codes for each situation:

```rust
// Success
StatusCode::OK              // 200 - GET, PUT, PATCH
StatusCode::CREATED         // 201 - POST (creation)
StatusCode::NO_CONTENT       // 204 - DELETE

// Client Errors
StatusCode::BAD_REQUEST     // 400 - Validation failed
StatusCode::UNAUTHORIZED     // 401 - No session, not authenticated
StatusCode::FORBIDDEN        // 403 - CSRF failed
StatusCode::NOT_FOUND        // 404 - Resource doesn't exist

// Server Errors
StatusCode::INTERNAL_SERVER_ERROR // 500 - Internal error
StatusCode::TOO_MANY_REQUESTS    // 429 - Rate limit exceeded
```

### Ownership Checks

**ALWAYS** verify ownership before allowing access:

```rust
// ❌ WRONG - Without ownership check
pub async fn get_loop(..., Path(id): Path<String>) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?.unwrap();
    Ok(loop_)  // Any user can access another's loop!
}

// ✅ CORRECT - With ownership check
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
    // continues with loop_
}
```

### CSRF Protection

**ALL** mutations (POST, PUT, DELETE, PATCH) need CSRF token:

```rust
// CSRF middleware generates token and adds to request.extensions()
Extension(csrf_token): Extension<CsrfToken>

// Template includes token in forms
<input type="hidden" name="csrf_token" value="{{ csrf_token }}" />

// Client sends token in header
fetch("/api/loops", {
    method: "POST",
    headers: {
        "x-csrf-token": csrf_token,
    },
    body: JSON.stringify(data),
})
```

### Session Management

**ALWAYS** use SessionStore to manage sessions:

```rust
// Create session on login
let session_token = state.session_store.create_session(user.id.clone()).await;

// Validate session in middleware
if let Some(user_id) = session_store.validate_session(&session_id).await {
    request.extensions_mut().insert(user_id);
}

// Delete session on logout
state.session_store.delete_session(session_id).await;
```

## Usage Patterns

### Create New Handler

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
    // 3. Validation
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

    // 4. Assign ownership
    let mut create_data = payload;
    create_data.owner_id = user_id;

    // 5. Call service/repository
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

### Add New Route

In `router.rs`:

```rust
// 1. Import handler
use crate::handlers::loops::{create_loop, delete_loop};

// 2. Add route in protected_routes()
fn protected_routes() -> Router<AppState> {
    Router::new()
        // ... other routes
        .route("/api/loops", post(create_loop))
        .route("/api/loops/{id}", delete(delete_loop))
        .route_layer(axum::middleware::from_fn(csrf_middleware))
        .route_layer(axum::middleware::from_fn(auth_middleware))
}
```

### Create New Template

1. Define struct in `templates/mod.rs`:
```rust
#[derive(Template)]
#[template(path = "loops/detail.html")]
pub struct LoopDetailTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loop_: LoopDetail,
}
```

2. Create file `templates/loops/detail.html`:
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

3. Use template in handler:
```rust
pub async fn loop_detail(
    State(_state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let csrf_token = CsrfToken::generate().to_string();

    // ... fetch loop ...

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

Configure via environment variable:

```bash
# .env
RATE_LIMIT=60  # requests per minute (default: 100)
```

Middleware automatically:
- Extracts IP from headers (`x-forwarded-for`, `x-real-ip`)
- Uses token bucket algorithm per IP
- Returns 429 with `Retry-After` header when limit exceeded

### WebSocket Connection

JavaScript client:

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

## Anti-patterns

### NEVER DO

**1. Ignore CSRF protection**
```rust
// ❌ WRONG - POST without CSRF
.route("/api/loops", post(create_loop))

// ✅ CORRECT - With CSRF middleware
.route("/api/loops", post(create_loop))
.route_layer(axum::middleware::from_fn(csrf_middleware))
```

**2. Return JSON when HTML is expected (HTMX)**
```rust
// ❌ WRONG - HTMX expects HTML
pub async fn list_loops_page(...) -> Json<ListLoopsResponse> {
    Json(response)
}

// ✅ CORRECT - HTMX needs HTML
pub async fn list_loops_page(...) -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html(html))
}
```

**3. Not verify ownership**
```rust
// ❌ WRONG - Any user can access
pub async fn get_loop(..., Path(id): Path<String>) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?.unwrap();
    Ok(loop_)
}

// ✅ CORRECT - Verify ownership
pub async fn get_loop(...) -> ... {
    let loop_ = state.loop_repository.find_by_id(&id).await?;
    if loop_.owner_id != user_id {
        return (StatusCode::UNAUTHORIZED, ...);
    }
    Ok(loop_)
}
```

**4. Always return StatusCode + Response**
```rust
// ❌ WRONG - Just response without status
pub async fn handler() -> Json<Response> {
    Json(response)
}

// ✅ CORRECT - Always with StatusCode
pub async fn handler() -> (StatusCode, Json<Response>) {
    (StatusCode::OK, Json(response))
}
```

**5. Swallow template errors**
```rust
// ❌ WRONG - Ignoring error
let html = template.render().unwrap();

// ✅ CORRECT - Propagating error
match template.render() {
    Ok(html) => (StatusCode::OK, Html(html)),
    Err(e) => (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!("Failed to render template: {}", e)),
    ),
}
```

**6. Use unwrap() without handling**
```rust
// ❌ WRONG - Can panic
let loop_ = state.loop_repository.find_by_id(&id).await.unwrap();

// ✅ CORRECT - Propagating error with appropriate status
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

## ralph-server Specific Patterns

### Request Flow (Auth)

```
1. Client POST /api/auth/register
   ↓
2. Handler validates (username, email, password)
   ↓
3. auth_service.register() (password hash)
   ↓
4. UserRepository.create() (DB)
   ↓
5. SessionStore.create_session()
   ↓
6. Returns 201 Created with { success, user_id, session_token }
```

### Request Flow (Loop Control)

```
1. Client POST /api/loops/{id}/start
   ↓
2. Auth middleware validates session
   ↓
3. CSRF middleware validates token
   ↓
4. Handler verifies ownership (loop.owner_id == user_id)
   ↓
5. LoopExecutor.start(loop_id)
   ↓
6. DockerManager creates container
   ↓
7. BroadcastManager.broadcast_loop_status("running")
   ↓
8. WebSocket clients receive update
```

### WebSocket Message Flow

```
1. Client connects: ws://localhost:3000/ws/loops/{loop_id}
   ↓
2. BroadcastManager.add_client(loop_id, sender)
   ↓
3. Handler sends welcome message (WsMessage::Ack)
   ↓
4. LoopExecutor executes iteration
   ↓
5. BroadcastManager.broadcast_loop_status(loop_id, "running")
   ↓
6. All connected clients receive WsMessage::LoopStatus
   ↓
7. BroadcastManager.broadcast_iteration_complete(loop_id, iter_num, task_id)
   ↓
8. All clients receive WsMessage::IterationComplete
```

## Dependencies

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

### Downstreams (who depends on this crate)

None - This is the entry crate (application binary)

### Upstreams (who this crate depends on)

- `ralph-models` - Used in handlers for DTOs (CreateUser, CreateLoop, CreateTask)
- `ralph-repositories` - Repositories (UserRepository, LoopRepository, TaskRepository)
- `ralph-services` - AuthService, DockerManager, LoopExecutor
- `ralph-agent` - AgentConfig

## Pitfalls

### Common Confusions

**1. Request Extensions vs Extractors**

```rust
// Extension: used to pass data between middleware/handlers
Extension(user_id): Extension<String>  // user_id injected by auth middleware

// Extractor: used to extract data from request
Path(id): Path<String>              // id from path /api/loops/{id}
Query(params): Query<ListQuery>     // query params ?page=1&limit=10
State(state): State<AppState>        // application state
Json(payload): Json<CreateLoop>    // JSON body
```

**2. HTTP Methods vs Route Protection**

```rust
// Safe methods (GET, HEAD, OPTIONS, TRACE) don't need CSRF
.route("/api/loops", get(list_loops))  // CSRF not checked

// Unsafe methods (POST, PUT, DELETE, PATCH) REQUIRE CSRF
.route("/api/loops", post(create_loop))  // CSRF checked in middleware
.route_layer(axum::middleware::from_fn(csrf_middleware))
```

**3. WebSocket vs HTTP**

```rust
// WebSocket is a separate route that upgrades from HTTP
.route("/ws/loops/{id}", get(websocket_handler))  // WebSocketUpgrade extractor

// HTTP are normal routes
.route("/api/loops/{id}", get(get_loop))  // Json/Html response
```

**4. Rate Limiting vs Auth**

```rust
// Rate limiting is per IP (before auth)
.layer(axum::middleware::from_fn_with_state(rate_limiter, rate_limit_middleware))

// Auth is per session (after rate limiting)
.route_layer(axum::middleware::from_fn(auth_middleware))
```

**5. CsrfToken vs CsrfTokenStore**

```rust
// CsrfToken: type-safe wrapper for token string
let token = CsrfToken::generate();  // generates new token
let token_str = token.as_str();  // returns &str

// CsrfTokenStore: manages tokens for multiple sessions
state.csrf_store.store(&session_id, token.as_str()).await;  // store
let stored = state.csrf_store.get(&session_id).await;  // retrieve
let valid = state.csrf_store.validate(&session_id, token.as_str()).await;  // validate
```

### Unexpected Behaviors

**1. Auth middleware injects user_id in extensions**

```rust
// After auth middleware, user_id is available in all handlers
pub async fn handler(Extension(user_id): Extension<String>) {
    // user_id is String containing authenticated user's ID
    // If not authenticated, user_id is empty string ""
}

// NOTE: For unprotected routes, user_id may be absent
// Use .unwrap_or(String::new()) for safety
```

**2. Template errors at runtime**

```rust
// Askama compiles templates at compile-time
// If template has syntax error, it DOESN'T compile

// Errors at runtime (e.g., missing variable) return Err
match template.render() {
    Ok(html) => (StatusCode::OK, Html(html)),
    Err(e) => {
        // e contains error details (missing var, etc)
        (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Template error: {}", e)))
    },
}
```

**3. Automatic WebSocket cleanup**

```rust
// When client disconnects, socket is dropped
// Background task is automatically aborted

// Cleanup in remove_client() uses Arc::ptr_eq
state.broadcast_manager.remove_client(&loop_id, &sender_arc).await;

// NOTE: If same client reconnects, creates new Arc
// Old clients are not automatically removed
```

**4. Automatic rate limiting refill**

```rust
// Token bucket refills on each request
// Rate: requests_per_minute / 60 (tokens per second)

// Example: 60 requests/minute = 1 token/second
bucket.refill()  // adds tokens based on elapsed time

// If exhausted, returns Err(retry_after)
// retry_after = ceil(1.0 / refill_rate) seconds until next token
```

## Downlinks (Additional Context)

**For better understanding:**
- `/AGENTS.md` - General architecture and project patterns
- `/ralph-models/AGENTS.md` - Models used in handlers
- `/ralph-repositories/AGENTS.md` - Repositories used in handlers
- `/ralph-services/AGENTS.md` - Services used in handlers (AuthService, LoopExecutor)
- `/ralph-agent/AGENTS.md` - AgentConfig used in AppState

---

**Last updated:** 2026-01-18
**Version:** 1.0
