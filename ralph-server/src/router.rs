//! HTTP router for Ralph Loop Manager server.
//!
//! This module configures Axum router with all route handlers and middleware.

use crate::handlers::{
    auth::{AppState, login, logout, register},
    health_check,
    loops::{
        create_loop, delete_loop, get_loop, list_loops, list_loops_page, pause_loop, resume_loop,
        start_loop, stop_loop,
    },
    tasks::{create_task, delete_task, get_task, list_tasks},
};
use crate::middleware::{
    auth::auth_middleware,
    csrf::csrf_middleware,
    rate_limit::{create_rate_limiter_from_env, rate_limit_middleware},
};
use axum::http::{HeaderName, HeaderValue, Method};
use axum::{
    Extension, Router,
    routing::{get, post},
};
use std::env;
use tower_http::cors::CorsLayer;

/// Creates and configures the HTTP router for the Ralph Loop Manager.
///
/// This function sets up:
/// - Public routes (no authentication required)
/// - Protected routes (authentication required)
/// - CORS layer with configurable origins from environment variables
/// - Route handlers for health checks, authentication, loops, and tasks
///
/// # Arguments
/// * `state` - The application state containing shared services and stores
///
/// # Returns
/// * A configured `Router` instance ready to be served
///
/// # Environment Variables
/// * `CORS_ORIGINS` - Comma-separated list of allowed CORS origins (default: "http://localhost:3000")
///
/// # Example
/// ```ignore
/// let state = AppState::new(...);
/// let app = create_router(state);
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
/// axum::serve(listener, app).await?;
/// ```
pub fn create_router(state: AppState) -> Router {
    let cors_origins =
        env::var("CORS_ORIGINS").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let cors = build_cors_layer(&cors_origins);

    let rate_limiter = create_rate_limiter_from_env();

    Router::new()
        .route("/health", get(health_check))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .merge(protected_routes())
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
        .layer(Extension(state.csrf_store.clone()))
        .layer(Extension(state.session_store.clone()))
        .layer(cors)
        .with_state(state)
}

/// Creates and configures protected routes that require authentication.
///
/// All routes in this router require a valid session token in request headers.
/// The session token can be provided via:
/// - `session` header
/// - `authorization` header
///
/// # Returns
/// * A configured `Router` instance with authentication middleware applied
///
/// # Protected Routes
/// ## Loops
/// - `GET /loops` - Render loop list page (HTML)
/// - `GET /api/loops` - List all loops for the authenticated user
/// - `POST /api/loops` - Create a new loop
/// - `GET /api/loops/:id` - Get a specific loop
/// - `DELETE /api/loops/:id` - Delete a loop
/// - `POST /api/loops/:id/start` - Start a loop
/// - `POST /api/loops/:id/pause` - Pause a running loop
/// - `POST /api/loops/:id/resume` - Resume a paused loop
/// - `POST /api/loops/:id/stop` - Stop a loop
///
/// ## Tasks
/// - `POST /api/loops/:id/tasks` - Create a task for a loop
/// - `GET /api/loops/:id/tasks` - List tasks for a loop
/// - `GET /api/tasks/:id` - Get a specific task
/// - `DELETE /api/tasks/:id` - Delete a task
fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/loops", get(list_loops_page))
        .route("/api/loops", get(list_loops).post(create_loop))
        .route("/api/loops/{id}", get(get_loop).delete(delete_loop))
        .route("/api/loops/{id}/start", post(start_loop))
        .route("/api/loops/{id}/pause", post(pause_loop))
        .route("/api/loops/{id}/resume", post(resume_loop))
        .route("/api/loops/{id}/stop", post(stop_loop))
        .route("/api/loops/{id}/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/{id}", get(get_task).delete(delete_task))
        .route_layer(axum::middleware::from_fn(csrf_middleware))
        .route_layer(axum::middleware::from_fn(auth_middleware))
}

/// Builds a CORS layer from a comma-separated list of origins.
///
/// # Arguments
/// * `origins_str` - Comma-separated list of allowed origins
///
/// # Returns
/// * A configured `CorsLayer`
fn build_cors_layer(origins_str: &str) -> CorsLayer {
    let origins: Vec<HeaderValue> = origins_str
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<HeaderValue>()
                .unwrap_or_else(|_| HeaderValue::from_str("http://localhost:3000").unwrap())
        })
        .collect();

    let allowed_methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];

    let allowed_headers: Vec<HeaderName> = vec![
        HeaderName::from_static("content-type"),
        HeaderName::from_static("authorization"),
        HeaderName::from_static("session"),
    ];

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
        .allow_credentials(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_cors_layer_with_single_origin() {
        let origins = "http://localhost:3000";
        let cors = build_cors_layer(origins);
        let _ = cors;
    }

    #[test]
    fn test_cors_layer_with_multiple_origins() {
        let origins = "http://localhost:3000,http://localhost:8080,https://example.com";
        let cors = build_cors_layer(origins);
        let _ = cors;
    }

    #[test]
    fn test_cors_layer_with_whitespace() {
        let origins = "http://localhost:3000 , http://localhost:8080 , https://example.com";
        let cors = build_cors_layer(origins);
        let _ = cors;
    }

    #[test]
    fn test_parse_origins_from_env() {
        unsafe {
            env::set_var(
                "CORS_ORIGINS",
                "http://localhost:3000,http://localhost:8080",
            );
        }

        let cors_origins = env::var("CORS_ORIGINS").unwrap();
        let origins: Vec<&str> = cors_origins.split(',').map(|s| s.trim()).collect();

        assert_eq!(origins.len(), 2);
        assert!(origins.contains(&"http://localhost:3000"));
        assert!(origins.contains(&"http://localhost:8080"));
    }

    #[test]
    fn test_default_cors_origin() {
        let cors_origins =
            env::var("CORS_ORIGINS").unwrap_or_else(|_| "http://localhost:3000".to_string());

        assert_eq!(cors_origins, "http://localhost:3000");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::middleware::auth::SessionStore;
    use crate::middleware::csrf::CsrfTokenStore;
    use axum::{
        body::Body,
        body::to_bytes,
        http::{Method, Request, StatusCode},
    };
    use ralph_repositories::{LoopRepository, TaskRepository};
    use ralph_services::{AuthService, DockerManager, LoopExecutor};
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn create_test_state() -> AppState {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS loops (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                prd TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                docker_image TEXT NOT NULL,
                cpu_limit INTEGER NOT NULL,
                memory_limit INTEGER NOT NULL,
                max_iterations INTEGER NOT NULL,
                iteration_timeout INTEGER NOT NULL,
                iteration_delay INTEGER NOT NULL,
                git_repo_url TEXT,
                git_branch_pattern TEXT NOT NULL,
                status TEXT NOT NULL,
                current_iteration INTEGER NOT NULL DEFAULT 0,
                container_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                loop_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority INTEGER NOT NULL,
                parent_task_id TEXT,
                created_by TEXT NOT NULL,
                iteration_id TEXT,
                started_at TEXT,
                completed_at TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let user_repo = ralph_repositories::UserRepository::new(pool.clone());
        let auth_service = AuthService::new(user_repo);
        let session_store = SessionStore::new();
        let csrf_store = CsrfTokenStore::new();
        let loop_repository = LoopRepository::new(pool.clone());
        let task_repository = TaskRepository::new(pool.clone());

        let docker = Arc::new(DockerManager::new());
        let agent_config = ralph_agent::agent::AgentConfig::default();
        let loop_executor = LoopExecutor::new(Arc::new(pool), docker, agent_config);

        let broadcast_manager = crate::websocket::BroadcastManager::new();

        AppState::new(
            auth_service,
            session_store.clone(),
            csrf_store,
            loop_repository,
            task_repository,
            loop_executor,
            broadcast_manager,
        )
    }

    #[tokio::test]
    async fn test_protected_loops_routes_require_auth() {
        let state = create_test_state().await;
        let app = create_router(state);

        let test_routes = vec![
            ("/loops", Method::GET),
            ("/api/loops", Method::GET),
            ("/api/loops", Method::POST),
            ("/api/loops/test-id", Method::GET),
            ("/api/loops/test-id", Method::DELETE),
            ("/api/loops/test-id/start", Method::POST),
            ("/api/loops/test-id/pause", Method::POST),
            ("/api/loops/test-id/resume", Method::POST),
            ("/api/loops/test-id/stop", Method::POST),
        ];

        for (route, method) in test_routes {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(route)
                        .method(method)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Route {} should return 401 without auth",
                route
            );
        }
    }

    #[tokio::test]
    async fn test_protected_tasks_routes_require_auth() {
        let state = create_test_state().await;
        let app = create_router(state);

        let test_routes = vec![
            ("/api/loops/test-id/tasks", Method::GET),
            ("/api/loops/test-id/tasks", Method::POST),
            ("/api/tasks/test-id", Method::GET),
            ("/api/tasks/test-id", Method::DELETE),
        ];

        for (route, method) in test_routes {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(route)
                        .method(method)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Route {} should return 401 without auth",
                route
            );
        }
    }

    #[tokio::test]
    async fn test_protected_routes_with_valid_session() {
        let state = create_test_state().await;
        let app = create_router(state.clone());

        let user_id = "test-user-id".to_string();
        let session_id = state.session_store.create_session(user_id.clone()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/loops")
                    .method(Method::GET)
                    .header("session", session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(body_str.contains("\"success\":true"));
    }

    #[tokio::test]
    async fn test_public_routes_work_without_auth() {
        let state = create_test_state().await;
        let app = create_router(state);

        let test_routes = vec![
            ("/health", Method::GET),
            ("/api/auth/register", Method::POST),
            ("/api/auth/login", Method::POST),
        ];

        for (route, method) in test_routes {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(route)
                        .method(method)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Public route {} should not return 401",
                route
            );
        }
    }

    #[tokio::test]
    async fn test_all_loop_routes_with_valid_session() {
        let state = create_test_state().await;
        let app = create_router(state.clone());

        let user_id = "test-user-id".to_string();
        let session_id = state.session_store.create_session(user_id).await;

        let test_cases = vec![
            ("/loops", Method::GET),
            ("/api/loops", Method::GET),
            ("/api/loops", Method::POST),
            ("/api/loops/test-id", Method::GET),
            ("/api/loops/test-id", Method::DELETE),
            ("/api/loops/test-id/start", Method::POST),
            ("/api/loops/test-id/pause", Method::POST),
            ("/api/loops/test-id/resume", Method::POST),
            ("/api/loops/test-id/stop", Method::POST),
        ];

        for (route, method) in test_cases {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(route)
                        .method(method)
                        .header("session", &session_id)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Route {} with valid session should not return 401",
                route
            );
        }
    }

    #[tokio::test]
    async fn test_all_task_routes_with_valid_session() {
        let state = create_test_state().await;
        let app = create_router(state.clone());

        let user_id = "test-user-id".to_string();
        let session_id = state.session_store.create_session(user_id).await;

        let test_cases = vec![
            ("/api/loops/test-id/tasks", Method::GET),
            ("/api/loops/test-id/tasks", Method::POST),
            ("/api/tasks/test-id", Method::GET),
            ("/api/tasks/test-id", Method::DELETE),
        ];

        for (route, method) in test_cases {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(route)
                        .method(method)
                        .header("session", &session_id)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Route {} with valid session should not return 401",
                route
            );
        }
    }

    #[tokio::test]
    async fn test_invalid_session_returns_401() {
        let state = create_test_state().await;
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/loops")
                    .method(Method::GET)
                    .header("session", "invalid-session-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
