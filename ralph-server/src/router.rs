//! HTTP router for Ralph Loop Manager server.
//!
//! This module configures the Axum router with all route handlers and middleware.

use crate::handlers::{
    auth::{AppState, login, logout, register},
    health_check,
};
use axum::http::HeaderValue;
use axum::{
    Router,
    routing::{get, post},
};
use std::env;
use tower_http::cors::{Any, CorsLayer};

/// Creates and configures the HTTP router for the Ralph Loop Manager.
///
/// This function sets up:
/// - Public routes (no authentication required)
/// - CORS layer with configurable origins from environment variables
/// - Route handlers for health checks and authentication
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

    Router::new()
        .route("/health", get(health_check))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .layer(cors)
        .with_state(state)
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

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true)
}

#[cfg(test)]
mod tests {
    use super::*;

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
