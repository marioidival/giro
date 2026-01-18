# Middleware Layer

## Purpose

This module provides the HTTP middleware layer for the Ralph Loop Manager. Responsible for protecting endpoints, managing sessions, preventing CSRF attacks, and limiting request rates.

**What this area does:**
- **Authentication**: Session validation and `user_id` injection in handlers
- **CSRF Protection**: Token generation and validation to prevent cross-site request forgery attacks
- **Rate Limiting**: Request limit per IP using token bucket algorithm
- **Ordering and Stacking**: Correct application order of middlewares in HTTP pipeline

**What this area does NOT do:**
- Does not manage authentication business logic (this is responsibility of `ralph-services::AuthService`)
- Does not access database (uses in-memory storage for sessions/tokens)
- Does not implement login/logout (this is responsibility of handlers)
- Does not do role-based authorization (only checks authentication)

## Structure

```
ralph-server/src/middleware/
├── mod.rs           # Public re-exports
├── auth.rs          # SessionStore, auth_middleware, require_auth
├── csrf.rs          # CsrfTokenStore, CsrfToken, csrf_middleware
└── rate_limit.rs    # RateLimiter, TokenBucket, rate_limit_middleware
```

### Components

#### Auth Middleware (`auth.rs`)
- `SessionStore`: In-memory storage (`Arc<RwLock<HashMap>>`) mapping `session_id` → `user_id`
- `auth_middleware`: Middleware function that validates session and injects `user_id` in extensions
- `require_auth`: Helper function to extract `user_id` from request
- `extract_session_id`: Extracts session ID from headers (`session` or `authorization`)

#### CSRF Middleware (`csrf.rs`)
- `CsrfTokenStore`: In-memory storage for CSRF tokens per session
- `CsrfToken`: Type-safe wrapper with `Display`, `AsRef`, `From<String>` implementations
- `csrf_middleware`: Generates/validates tokens and returns 403 on failure
- `is_safe_method`: Checks if HTTP method is safe (GET, HEAD, OPTIONS, TRACE)
- `extract_csrf_token`: Extracts token from headers (`x-csrf-token` or `csrf-token`)

#### Rate Limiting Middleware (`rate_limit.rs`)
- `RateLimiter`: Manages token buckets per client IP
- `TokenBucket`: Token bucket algorithm implementation with automatic refill
- `RateLimitConfig`: Configuration loaded from environment variables
- `rate_limit_middleware`: Checks limits and returns 429 with `Retry-After` header
- `extract_client_ip`: Extracts IP from headers (`x-forwarded-for` or `x-real-ip`)

## Critical Invariants

### Middleware Order

The middleware application order is **CRITICAL** and should NEVER be altered:

```rust
// ✅ CORRECT - Correct order in router.rs
Router::new()
    .merge(protected_routes())
    .layer(axum::middleware::from_fn_with_state(
        rate_limiter,
        rate_limit_middleware,  // 1. Rate limiting (first, outermost)
    ))
    .layer(Extension(state.csrf_store.clone()))  // 2. Injection of stores
    .layer(Extension(state.session_store.clone()))
    .layer(cors)  // 3. CORS
    .with_state(state)

// For protected routes:
fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/api/loops", post(create_loop))
        .route_layer(axum::middleware::from_fn(csrf_middleware))  // 4. CSRF validation
        .route_layer(axum::middleware::from_fn(auth_middleware))  // 5. Auth (last, innermost)
}
```

**Why this order is critical:**
1. **Rate limiting first**: Protects against abuse before any processing
2. **Stores injected**: Auth and CSRF middlewares need stores available
3. **Auth before CSRF**: Need to know which user to validate their session's token
4. **CSRF on protected routes**: Only routes that modify state need validation

### SessionStore Thread-Safety

**ALL** `SessionStore` methods must be `async` and use `RwLock`:

```rust
// ✅ CORRECT - Thread-safe
impl SessionStore {
    pub async fn validate_session(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;  // Read lock
        sessions.get(session_id).cloned()
    }

    pub async fn create_session(&self, user_id: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;  // Write lock
        sessions.insert(session_id.clone(), user_id);
        session_id
    }
}
```

### CSRF Token per Session

**EACH** session must have **EXACTLY ONE** CSRF token:

```rust
// ❌ WRONG - Multiple tokens per session
csrf_store.store(&session_id, "token-1").await;
csrf_store.store(&session_id, "token-2").await;  // Overwrites token-1

// ✅ CORRECT - One token per session, reused
if let Some(existing) = csrf_store.get(&session_id).await {
    // Use existing token
} else {
    // Generate new token and store
    let token = CsrfToken::generate();
    csrf_store.store(&session_id, token.as_str()).await;
}
```

### Safe Methods vs Unsafe Methods

**SAFE METHODS** (DO NOT require CSRF): `GET`, `HEAD`, `OPTIONS`, `TRACE`
**UNSAFE METHODS** (REQUIRE CSRF): `POST`, `PUT`, `DELETE`, `PATCH`

```rust
// ✅ CORRECT - CSRF middleware implements this logic
fn is_safe_method(method: &axum::http::Method) -> bool {
    matches!(
        *method,
        axum::http::Method::GET
            | axum::http::Method::HEAD
            | axum::http::Method::OPTIONS
            | axum::http::Method::TRACE
    )
}

// In middleware:
if is_safe_method(request.method()) {
    return Ok(next.run(request).await);  // Passes without validation
}
// Validate token for unsafe methods
```

### Token Bucket Refill

**Token refill MUST be continuous and based on elapsed time:**

```rust
// ✅ CORRECT - Continuous refill
impl TokenBucket {
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed();
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate) as u32;

        self.tokens = std::cmp::min(self.capacity, self.tokens + tokens_to_add);
        self.last_refill = Instant::now();
    }
}

// ❌ WRONG - Fixed refill (loses requests between refills)
fn refill_fixed(&mut self) {
    // Refill happens only when explicitly called
    self.tokens = self.capacity;  // Loses tokens that should be added gradually
}
```

### IP Extraction Order

**IP extraction order is PREDEFINED:**

```rust
// ✅ CORRECT - Correct order
fn extract_client_ip(headers: &HeaderMap) -> String {
    // 1. Check x-forwarded-for (proxy/CDN)
    if let Some(forwarded_for) = headers.get("x-forwarded-for")
        && let Ok(forwarded_str) = forwarded_for.to_str()
        && let Some(client_ip) = forwarded_str.split(',').next()
    {
        return client_ip.trim().to_string();
    }

    // 2. Check x-real-ip (nginx, apache)
    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(real_ip_str) = real_ip.to_str()
    {
        return real_ip_str.to_string();
    }

    // 3. Fallback para "unknown"
    "unknown".to_string()
}
```

**Why this order:**
- `x-forwarded-for` is the standard header for proxies/CDNs
- First IP in list is always the original client IP
- `x-real-ip` is fallback for other reverse proxies
- "unknown" avoids blocking all requests if IP is not identified

## Usage Patterns

### Adding Middleware to Router

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
            rate_limit_middleware,  // Applied to ALL routes
        ))
        .layer(Extension(state.csrf_store.clone()))
        .layer(Extension(state.session_store.clone()))
        .layer(cors)
        .with_state(state)
}
```

### Creating Protected Routes

```rust
fn protected_routes() -> Router<AppState> {
    Router::new()
        // Protected routes (require auth + CSRF)
        .route("/api/loops", get(list_loops).post(create_loop))
        .route("/api/loops/{id}", get(get_loop).delete(delete_loop))
        .route("/api/loops/{id}/start", post(start_loop))
        .route("/api/loops/{id}/tasks", get(list_tasks).post(create_task))
        // MIDDLEWARE ORDER CRITICAL (inverting = doesn't work)
        .route_layer(axum::middleware::from_fn(csrf_middleware))  // CSRF first
        .route_layer(axum::middleware::from_fn(auth_middleware))   // Auth after
}
```

### Using Auth in Handlers

```rust
use axum::{Extension, Json};
use crate::handlers::auth::AppState;

pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,  // Injected by auth_middleware
    Json(payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    // user_id is automatically available
    // If not authenticated, request won't reach here (returns 401 in middleware)

    // Assign loop ownership to authenticated user
    let mut create_data = payload;
    create_data.owner_id = user_id;

    // ... rest of logic
}
```

### Using CSRF Token in Handlers

```rust
use axum::Extension;
use crate::middleware::csrf::CsrfToken;

pub async fn new_loop_form(
    Extension(csrf_token): Extension<CsrfToken>,  // Injected by csrf_middleware
) -> (StatusCode, Html<String>) {
    let token = csrf_token.as_str();  // Converts to &str

    // Template receives the token
    let template = NewLoopTemplate {
        csrf_token: token.to_string(),
        // ... other fields
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Error: {}", e))),
    }
}
```

### Sending CSRF Token from Frontend

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
        "x-csrf-token": csrfToken,  // Token obtained from meta tag or cookie
    },
    body: JSON.stringify({
        name: "My Loop",
        description: "Description",
    }),
});
```

### Configuring Rate Limiting

```bash
# .env
RATE_LIMIT=60  # requests per minute (default: 100)
```

```rust
// Rate limiting is automatic when applied to router
// Returns 429 with Retry-After header when limit exceeded

// Client can implement automatic retry:
fetch("/api/loops")
    .then(response => {
        if (response.status === 429) {
            const retryAfter = response.headers.get('Retry-After');
            setTimeout(() => retryRequest(), retryAfter * 1000);
        }
    });
```

### Managing Sessions

```rust
// In login handler
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<LoginResponse>) {
    match state.auth_service.authenticate(&payload.username, &payload.password).await {
        Ok(user) => {
            // Create session
            let session_id = state.session_store.create_session(user.id.clone()).await;

            // Generate CSRF token for the session
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

// In logout handler
pub async fn logout(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<LogoutResponse>) {
    // Extract session_id from headers
    if let Some(session_id) = extract_session_id(request.headers()) {
        // Delete session
        state.session_store.delete_session(&session_id).await;
        // Delete CSRF token
        state.csrf_store.delete(&session_id).await;
    }

    (StatusCode::OK, Json(LogoutResponse { success: true }))
}
```

## Anti-patterns

### NEVER DO

**1. Inverting middleware order**
```rust
// ❌ WRONG - CSRF before Auth (session_id not available)
.route_layer(axum::middleware::from_fn(csrf_middleware))
.route_layer(axum::middleware::from_fn(auth_middleware))

// ✅ CORRECT - Auth first, CSRF after
.route_layer(axum::middleware::from_fn(csrf_middleware))
.route_layer(axum::middleware::from_fn(auth_middleware))
```

**2. CSRF in safe methods**
```rust
// ❌ WRONG - Validating CSRF in GET
if let Some(provided) = extract_csrf_token(request.headers()) {
    if !csrf_store.validate(&session_id, &provided).await {
        return Err(StatusCode::FORBIDDEN);
    }
}

// ✅ CORRECT - Only validates unsafe methods
if !is_safe_method(request.method()) {
    if let Some(provided) = extract_csrf_token(request.headers()) {
        if !csrf_store.validate(&session_id, &provided).await {
            return Err(StatusCode::FORBIDDEN);
        }
    }
}
```

**3. Blocking requests without identified IP**
```rust
// ❌ WRONG - Blocks all without IP headers
if extract_client_ip(headers) == "unknown" {
    return Err(StatusCode::FORBIDDEN);
}

// ✅ CORRECT - Use "unknown" as shared bucket
let client_ip = extract_client_ip(headers);  // Returns "unknown" if not found
// Rate limiting continues working (all "unknown" share bucket)
```

**4. Sync locks in async context**
```rust
// ❌ WRONG - Mutex can block thread
use std::sync::Mutex;
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, String>>,  // Mutex blocks thread
}

// ✅ CORRECT - RwLock allows multiple reads
use tokio::sync::RwLock;
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, String>>,  // RwLock is async-friendly
}
```

**5. Ignoring Retry-After header**
```rust
// ❌ WRONG - Returns 429 without Retry-After
Err(StatusCode::TOO_MANY_REQUESTS)

// ✅ CORRECT - Includes Retry-After header
let mut response = (StatusCode::TOO_MANY_REQUESTS, ()).into_response();
response.headers_mut().insert(
    header::RETRY_AFTER,
    retry_after.to_string().parse().unwrap(),
);
Ok(response)
```

**6. Non-cryptographic token generation**
```rust
// ❌ WRONG - Predictable
let token = format!("csrf-{}", std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());

// ✅ CORRECT - Cryptographically secure
let mut rng = rand::thread_rng();
let token: String = (0..32)
    .map(|_| {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        CHARSET[rng.gen_range(0..CHARSET.len())] as char
    })
    .collect();
```

**7. Manually verifying user_id**
```rust
// ❌ WRONG - Duplicating auth middleware logic
pub async fn handler(request: Request) -> ... {
    if let Some(session_id) = extract_session_id(request.headers()) {
        if let Some(user_id) = session_store.validate_session(&session_id).await {
            // Use user_id...
        }
    }
}

// ✅ CORRECT - Use Extension injected by middleware
pub async fn handler(Extension(user_id): Extension<String>) -> ... {
    // user_id is already validated by auth_middleware
    // If not authenticated, request won't reach here
}
```

## Specific Patterns

### Request Flow

```
Client Request
    ↓
┌─────────────────────────────────────┐
│  Router Middleware (outermost)     │
├─────────────────────────────────────┤
│ 1. Rate Limiting                  │ ← First (IP level)
│    - Extracts IP from headers         │
│    - Checks token bucket            │
│    - Returns 429 if limited      │
└──────────────┬────────────────────┘
               │ Passes
               ▼
┌───────────────────────────────────────────────┐
│ 2. Extension Injections           │
│    - SessionStore                 │
│    - CsrfTokenStore               │
└──────────────┬────────────────────┘
               │ Passes
               ▼
┌───────────────────────────────────────────────┐
│ 3. CORS                          │
│    - Checks origins                 │
│    - Adds CORS headers             │
└──────────────┬────────────────────┘
               │ Passes
               ▼
┌─────────────────────────────────────────────┐
│ 4. Auth Middleware (protected)   │ ← For protected routes
│    - Extracts session_id from headers │
│    - Validates with SessionStore      │
│    - Injects user_id in extensions │
│    - Returns 401 if invalid       │
└──────────────┬────────────────────┘
               │ Passes
               ▼
┌─────────────────────────────────────────────┐
│ 5. CSRF Middleware (protected)   │ ← For protected routes
│    - Checks HTTP method             │
│    - If safe: passes            │
│    - If unsafe: validates token   │
│    - Injects CSRF token           │
│    - Returns 403 if invalid      │
└──────────────┬────────────────────┘
               │ Passes
               ▼
┌─────────────────────────────────────────────┐
│  Handler                         │ ← Final processing
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
2. AuthService.authenticate() validates credentials
   ↓
3. SessionStore.create_session(user_id)
   - Generates session_id (UUID v4)
   - Stores: session_id → user_id
   ↓
4. CsrfToken.generate()
   - Generates cryptographically secure token
   ↓
5. CsrfTokenStore.store(session_id, csrf_token)
   - Stores: session_id → csrf_token
   ↓
6. Returns to client: { session_id, csrf_token }
   ↓
7. Client sends session_id in headers ("session" or "authorization")
   ↓
8. Client sends csrf_token in headers ("x-csrf-token")
   ↓
9. Request arrives at server
   ↓
10. Auth middleware validates session_id → user_id
    ↓
11. CSRF middleware validates csrf_token → stored
    ↓
12. Handler processes with user_id available
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

... (consuming tokens)

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

## Dependencies

### Dependencies (Cargo.toml)

```toml
[dependencies]
# Web framework
axum.workspace = true

# Async runtime
tokio.workspace = true

# Utilities
rand = "0.8"                    # For CSRF token generation
uuid.workspace = true           # For session_id generation

# Logging
tracing.workspace = true
```

### Internal Dependencies

None - Middleware is a self-contained module that depends only on:
- `axum` for extractors and middleware types
- `tokio::sync` for concurrency primitives
- `std` and `rand` for utilities

### Downstreams (who uses this module)

- `ralph-server/router.rs` - Configures middleware in router
- `ralph-server/handlers/*.rs` - Uses extractors (Extension, SessionStore, CsrfToken)

## Pitfalls

### Common Confusions

**1. Extension vs State vs Extractors**

```rust
// Extension: data injected by middleware
Extension(user_id): Extension<String>      // user_id from auth_middleware
Extension(csrf_token): Extension<CsrfToken> // csrf_token from csrf_middleware

// State: application state shared
State(state): State<AppState>  // AppState configured in router

// Extractors: data extracted from request
Path(id): Path<String>         // /api/loops/{id}
Query(params): Query<ListQuery> // ?page=1&limit=10
Json(payload): Json<CreateLoop> // Body JSON
```

**2. SessionStore vs CsrfTokenStore**

```rust
// SessionStore: maps session_id → user_id
let user_id = session_store.validate_session(&session_id).await;

// CsrfTokenStore: maps session_id → csrf_token
let token = csrf_store.get(&session_id).await;
let valid = csrf_store.validate(&session_id, &provided_token).await;

// Both are independent but related by session_id
```

**3. Mutex vs RwLock**

```rust
// Mutex: only one "borrow" at a time (read OR write)
use std::sync::Mutex;
let data = Arc<Mutex<HashMap<String, String>>>;

let guard = data.lock().unwrap();  // Blocks for read OR write
// Even concurrent reads wait for other reads

// RwLock: multiple reads, exclusive write
use tokio::sync::RwLock;
let data = Arc<RwLock<HashMap<String, String>>>;

let guard = data.read().await;    // Blocks only for writes
let mut guard = data.write().await; // Blocks for reads AND writes
```

**4. Safe methods don't require CSRF**

```rust
// ❌ WRONG - GET needs CSRF
let response = fetch("/api/loops", {
    method: "GET",
    headers: { "x-csrf-token": token }
});

// ✅ CORRECT - GET doesn't need CSRF
let response = fetch("/api/loops", {
    method: "GET"  // CSRF middleware ignores safe method
});

// Only unsafe methods need:
let response = fetch("/api/loops", {
    method: "POST",
    headers: { "x-csrf-token": token }  // Required
});
```

**5. Rate limiting is per IP, not per user**

```rust
// Rate limiting is based on client IP
// Same authenticated user with multiple IPs = multiple buckets

// Example: user with different IP (VPN, network change)
// Creates new token bucket, rate limit starts from zero

// For rate limiting per user, you would need to store user_id
// in the bucket, but this is not currently implemented
```

### Unexpected Behaviors

**1. CSRF token persists during session**

```rust
// CSRF token is generated ONCE per session and reused
// Does not change with each request

// This is correct: CSRF token should persist
// Client should store token and send in all mutations

// ❌ Do not regenerate token on each request
// This would break form submissions and AJAX requests
```

**2. Rate limiting refill is continuous**

```rust
// Tokens are added continuously, not in fixed intervals
// Example: 60 requests/min = 1 token/second

// After 30 seconds without requests:
// - Tokens recovered: 30
// - If capacity was 100 and we had 0, now we have 30

// This is different from "fixed window" where tokens are reset
// Token bucket allows bursts (consume all at once, then wait)
```

**3. IP extraction fallback to "unknown"**

```rust
// If request doesn't have x-forwarded-for or x-real-ip:
// IP is "unknown"

// All requests with "unknown" share SAME bucket
// This can limit unduly if many clients don't have headers

// In production, configure reverse proxy (nginx, cloudflare, etc)
// to always add x-forwarded-for or x-real-ip
```

**4. SessionStore doesn't expire sessions automatically**

```rust
// Sessions do NOT expire automatically
// Persist until explicitly deleted in logout

// This is a known limitation of the current implementation
// Future: implement TTL/expiry with background cleanup task

// For logout:
session_store.delete_session(&session_id).await;
csrf_store.delete(&session_id).await;
```

**5. Middleware order is outer-to-inner**

```rust
// Router layers are applied from outside to inside
// First layer applied = executed first (outermost)
// Last layer applied = executed last (innermost)

Router::new()
    .layer(middleware_A)  // Executes FIRST
    .layer(middleware_B)  // Executes AFTER A
    .layer(middleware_C)  // Executes AFTER B
    .route_layer(middleware_D)  // Executes AFTER C
    .route_layer(middleware_E)  // Executes AFTER D (last, before handler)

// Execution order: A → B → C → D → E → Handler
```

## Downlinks (Additional Context)

**To understand better:**
- `/AGENTS.md` - General project architecture
- `/ralph-server/AGENTS.md` - Middleware usage in handlers and router
- `/ralph-services/AGENTS.md` - AuthService for credential authentication

---

**Last updated:** 2026-01-18
**Version:** 1.0
