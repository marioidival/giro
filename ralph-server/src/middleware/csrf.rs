//! CSRF (Cross-Site Request Forgery) protection middleware for ralph-server
//!
//! This module provides CSRF protection using token-based validation.
//! It generates cryptographically secure tokens, stores them in sessions,
//! and validates them on state-changing HTTP requests (POST, PUT, DELETE, PATCH).

use axum::{
    Extension,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use rand::Rng;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

const CSRF_TOKEN_LENGTH: usize = 32;
const CSRF_HEADER_NAME: &str = "x-csrf-token";

/// CSRF token storage mapping session IDs to CSRF tokens
#[derive(Clone, Debug)]
pub struct CsrfTokenStore {
    tokens: Arc<RwLock<HashMap<String, String>>>,
}

impl CsrfTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Stores a CSRF token for a session
    pub async fn store(&self, session_id: &str, token: &str) {
        let mut tokens = self.tokens.write().await;
        tokens.insert(session_id.to_string(), token.to_string());
    }

    /// Gets the CSRF token for a session
    pub async fn get(&self, session_id: &str) -> Option<String> {
        let tokens = self.tokens.read().await;
        tokens.get(session_id).cloned()
    }

    /// Validates a CSRF token for a session
    pub async fn validate(&self, session_id: &str, token: &str) -> bool {
        let tokens = self.tokens.read().await;
        tokens.get(session_id).is_some_and(|stored| stored == token)
    }

    /// Deletes the CSRF token for a session
    pub async fn delete(&self, session_id: &str) -> bool {
        let mut tokens = self.tokens.write().await;
        tokens.remove(session_id).is_some()
    }
}

impl Default for CsrfTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF token wrapper for type safety and display purposes
#[derive(Clone, Debug, PartialEq)]
pub struct CsrfToken {
    token: String,
}

impl CsrfToken {
    /// Creates a new CSRF token from a string
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// Generates a new cryptographically secure random CSRF token
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let token: String = (0..CSRF_TOKEN_LENGTH)
            .map(|_| {
                const CHARSET: &[u8] =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        Self { token }
    }

    /// Returns the token as a string reference
    pub fn as_str(&self) -> &str {
        &self.token
    }
}

impl fmt::Display for CsrfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.token)
    }
}

impl AsRef<str> for CsrfToken {
    fn as_ref(&self) -> &str {
        &self.token
    }
}

impl From<String> for CsrfToken {
    fn from(token: String) -> Self {
        Self { token }
    }
}

/// Extension trait to add CSRF token to request extensions
///
/// This allows handlers to easily access the CSRF token for including
/// it in forms or API responses.
pub trait CsrfExt {
    /// Adds a CSRF token to the request extensions
    fn with_csrf_token(self, token: CsrfToken) -> Self;
}

impl<B> CsrfExt for Request<B> {
    fn with_csrf_token(mut self, token: CsrfToken) -> Self {
        self.extensions_mut().insert(token);
        self
    }
}

/// Checks if the HTTP method is safe (read-only)
///
/// Safe methods: GET, HEAD, OPTIONS, TRACE
/// These methods should not modify server state and thus don't require CSRF protection.
fn is_safe_method(method: &axum::http::Method) -> bool {
    matches!(
        *method,
        axum::http::Method::GET
            | axum::http::Method::HEAD
            | axum::http::Method::OPTIONS
            | axum::http::Method::TRACE
    )
}

/// Extracts CSRF token from request
///
/// Checks in order:
/// 1. `x-csrf-token` header
/// 2. `csrf-token` header
/// 3. Form body (if present)
fn extract_csrf_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(CSRF_HEADER_NAME)
        .or_else(|| headers.get("csrf-token"))
        .and_then(|value| value.to_str().ok().map(|s| s.to_string()))
}

/// CSRF protection middleware implementation
///
/// This middleware:
/// 1. Generates and stores a CSRF token for the session if not present
/// 2. Validates CSRF token on state-changing requests (POST, PUT, DELETE, PATCH)
/// 3. Returns 403 Forbidden on validation failure
/// 4. Passes through for safe HTTP methods (GET, HEAD, OPTIONS)
/// 5. Adds CSRF token to request extensions for handlers to use
///
/// # Arguments
/// * `csrf_store` - The CSRF token store for managing tokens
/// * `request` - The incoming HTTP request
/// * `next` - The next middleware/handler in the chain
///
/// # Returns
/// * `Ok(Response)` on successful validation or safe methods
/// * `Err(StatusCode::FORBIDDEN)` on CSRF validation failure
pub async fn csrf_middleware(
    Extension(csrf_store): Extension<CsrfTokenStore>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session_id = super::auth::extract_session_id(request.headers());

    if session_id.is_none() {
        return Ok(next.run(request).await);
    }

    let session_id = session_id.unwrap();

    let token = if let Some(existing_token) = csrf_store.get(&session_id).await {
        CsrfToken::new(existing_token)
    } else {
        let new_token = CsrfToken::generate();
        csrf_store.store(&session_id, new_token.as_str()).await;
        new_token
    };

    request.extensions_mut().insert(token.clone());

    if is_safe_method(request.method()) {
        return Ok(next.run(request).await);
    }

    let provided_token = extract_csrf_token(request.headers());

    if let Some(provided) = provided_token
        && csrf_store.validate(&session_id, &provided).await
    {
        tracing::debug!("CSRF token validated successfully");
        return Ok(next.run(request).await);
    }

    tracing::warn!("CSRF token validation failed for session {}", session_id);
    Err(StatusCode::FORBIDDEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let token1 = CsrfToken::generate();
        let token2 = CsrfToken::generate();

        assert_eq!(token1.as_str().len(), CSRF_TOKEN_LENGTH);
        assert_eq!(token2.as_str().len(), CSRF_TOKEN_LENGTH);
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_csrf_token_from_string() {
        let token = CsrfToken::new("test-token".to_string());
        assert_eq!(token.as_str(), "test-token");
    }

    #[test]
    fn test_csrf_token_display() {
        let token = CsrfToken::new("test-token".to_string());
        assert_eq!(format!("{}", token), "test-token");
    }

    #[test]
    fn test_csrf_token_as_ref() {
        let token = CsrfToken::new("test-token".to_string());
        assert_eq!(token.as_ref(), "test-token");
    }

    #[test]
    fn test_is_safe_method() {
        use axum::http::Method;

        assert!(is_safe_method(&Method::GET));
        assert!(is_safe_method(&Method::HEAD));
        assert!(is_safe_method(&Method::OPTIONS));
        assert!(is_safe_method(&Method::TRACE));
        assert!(!is_safe_method(&Method::POST));
        assert!(!is_safe_method(&Method::PUT));
        assert!(!is_safe_method(&Method::DELETE));
        assert!(!is_safe_method(&Method::PATCH));
    }

    #[test]
    fn test_extract_csrf_token_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert(CSRF_HEADER_NAME, "test-token".parse().unwrap());

        let token = extract_csrf_token(&headers);
        assert_eq!(token, Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_csrf_token_from_alternative_header() {
        let mut headers = HeaderMap::new();
        headers.insert("csrf-token", "test-token".parse().unwrap());

        let token = extract_csrf_token(&headers);
        assert_eq!(token, Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_csrf_token_missing() {
        let headers = HeaderMap::new();

        let token = extract_csrf_token(&headers);
        assert_eq!(token, None);
    }

    #[test]
    fn test_csrf_ext_with_token() {
        let request = Request::builder().uri("/test").body(()).unwrap();

        let token = CsrfToken::new("test-token".to_string());
        let request = request.with_csrf_token(token.clone());

        let extracted_token = request.extensions().get::<CsrfToken>();
        assert_eq!(extracted_token, Some(&token));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode},
        routing::{get, post},
    };
    use tower::ServiceExt;

    fn create_test_app(csrf_store: CsrfTokenStore) -> Router {
        Router::new()
            .route("/public", get(public_handler))
            .route(
                "/protected",
                post(protected_handler)
                    .put(protected_handler)
                    .delete(protected_handler),
            )
            .layer(axum::middleware::from_fn_with_state(
                csrf_store.clone(),
                csrf_middleware,
            ))
            .layer(Extension(csrf_store))
    }

    async fn public_handler() -> &'static str {
        "Public endpoint"
    }

    async fn protected_handler() -> &'static str {
        "Protected endpoint"
    }

    #[tokio::test]
    async fn test_safe_methods_without_csrf_token() {
        let csrf_store = CsrfTokenStore::new();
        let app = create_test_app(csrf_store);

        let methods = vec![Method::GET, Method::HEAD];

        for method in methods {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/public")
                        .method(&method)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Method {:?} should work without CSRF token",
                method
            );
        }
    }

    #[tokio::test]
    async fn test_post_without_csrf_token_returns_403() {
        let csrf_store = CsrfTokenStore::new();
        let app = create_test_app(csrf_store.clone());

        let session_id = "test-session-no-csrf".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::POST)
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_post_with_valid_csrf_token_succeeds() {
        let csrf_store = CsrfTokenStore::new();
        let app = create_test_app(csrf_store.clone());

        let session_id = "test-session-123".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::POST)
                    .header("session", &session_id)
                    .header(CSRF_HEADER_NAME, csrf_token.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_put_method_requires_csrf() {
        let csrf_store = CsrfTokenStore::new();
        let app = create_test_app(csrf_store.clone());

        let session_id = "test-put-session".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::PUT)
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::PUT)
                    .header("session", &session_id)
                    .header(CSRF_HEADER_NAME, csrf_token.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_delete_method_requires_csrf() {
        let csrf_store = CsrfTokenStore::new();
        let app = create_test_app(csrf_store.clone());

        let session_id = "test-delete-session".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::DELETE)
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::DELETE)
                    .header("session", &session_id)
                    .header(CSRF_HEADER_NAME, csrf_token.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_token_generation_and_storage() {
        let csrf_store = CsrfTokenStore::new();

        let session_id = "test-session-789".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        let retrieved_token = csrf_store.get(&session_id).await;
        assert_eq!(retrieved_token, Some(format!("{}", csrf_token)));
    }

    #[tokio::test]
    async fn test_csrf_token_validation() {
        let csrf_store = CsrfTokenStore::new();

        let session_id = "test-session-validate".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        assert!(csrf_store.validate(&session_id, csrf_token.as_str()).await);
        assert!(!csrf_store.validate(&session_id, "invalid").await);
    }

    #[tokio::test]
    async fn test_csrf_token_deletion() {
        let csrf_store = CsrfTokenStore::new();

        let session_id = "test-session-delete".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        assert!(csrf_store.get(&session_id).await.is_some());

        csrf_store.delete(&session_id).await;

        assert!(csrf_store.get(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_multiple_sessions_have_separate_tokens() {
        let csrf_store = CsrfTokenStore::new();

        let session_id_1 = "session-1".to_string();
        let session_id_2 = "session-2".to_string();

        let token_1 = CsrfToken::generate();
        let token_2 = CsrfToken::generate();

        csrf_store.store(&session_id_1, token_1.as_str()).await;
        csrf_store.store(&session_id_2, token_2.as_str()).await;

        let retrieved_1 = csrf_store.get(&session_id_1).await;
        let retrieved_2 = csrf_store.get(&session_id_2).await;

        assert_eq!(retrieved_1, Some(format!("{}", token_1)));
        assert_eq!(retrieved_2, Some(format!("{}", token_2)));
        assert_ne!(token_1, token_2);
    }

    #[tokio::test]
    async fn test_post_with_invalid_csrf_token_returns_403() {
        let csrf_store = CsrfTokenStore::new();
        let app = create_test_app(csrf_store.clone());

        let session_id = "test-session-456".to_string();
        let csrf_token = CsrfToken::generate();

        csrf_store.store(&session_id, csrf_token.as_str()).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::DELETE)
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .method(Method::DELETE)
                    .header("session", &session_id)
                    .header(CSRF_HEADER_NAME, csrf_token.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
