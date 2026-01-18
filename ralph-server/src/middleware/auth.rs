//! Authentication middleware for ralph-server
//!
//! This module provides session-based authentication with in-memory session storage.
//! It includes middleware for protecting routes and extractors for retrieving authenticated user IDs.

use axum::{
    Extension,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory session storage mapping session IDs to user IDs
///
/// Uses `Arc<RwLock<HashMap>>` to provide thread-safe, concurrent access to sessions.
/// This is a simple implementation that will be replaced with tower-sessions in future sprints.
#[derive(Clone, Debug)]
pub struct SessionStore {
    pub sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl SessionStore {
    /// Creates a new empty session store
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new session for a user and returns the session ID
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user to create a session for
    ///
    /// # Returns
    /// A session ID (UUID v4 string)
    pub async fn create_session(&self, user_id: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), user_id);
        session_id
    }

    /// Validates a session and returns the associated user ID if valid
    ///
    /// # Arguments
    /// * `session_id` - The session ID to validate
    ///
    /// # Returns
    /// `Some(user_id)` if the session is valid, `None` otherwise
    pub async fn validate_session(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Deletes a session
    ///
    /// # Arguments
    /// * `session_id` - The session ID to delete
    ///
    /// # Returns
    /// `true` if the session existed and was deleted, `false` otherwise
    pub async fn delete_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).is_some()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the session ID from request headers
///
/// Looks for a "session" or "authorization" header. The value should be the session ID.
pub fn extract_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("session")
        .or_else(|| headers.get("authorization"))
        .and_then(|value| value.to_str().ok().map(|s| s.to_string()))
}

/// Middleware implementation for authentication
pub async fn auth_middleware(
    Extension(session_store): Extension<SessionStore>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session_id = extract_session_id(request.headers());

    if let Some(session_id) = session_id
        && let Some(user_id) = session_store.validate_session(&session_id).await
    {
        request.extensions_mut().insert(user_id);
        return Ok(next.run(request).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Helper function to retrieve user_id from request extensions
///
/// This is an alternative to the `Extension<String>` extractor for cases where
/// you prefer to call a function rather than use the extractor pattern.
///
/// # Arguments
/// * `request` - The incoming HTTP request
///
/// # Returns
/// `Some(user_id)` if authenticated, `None` otherwise
pub fn require_auth<B>(request: &Request<B>) -> Option<String> {
    request.extensions().get::<String>().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_store_create_and_validate() {
        let store = SessionStore::new();
        let user_id = "user-123".to_string();

        let session_id = store.create_session(user_id.clone()).await;
        assert!(!session_id.is_empty());

        let retrieved_user_id = store.validate_session(&session_id).await;
        assert_eq!(retrieved_user_id, Some(user_id));
    }

    #[tokio::test]
    async fn test_session_store_validate_invalid_session() {
        let store = SessionStore::new();

        let result = store.validate_session("invalid-session-id").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_session_store_delete_session() {
        let store = SessionStore::new();
        let user_id = "user-456".to_string();

        let session_id = store.create_session(user_id.clone()).await;

        let deleted = store.delete_session(&session_id).await;
        assert!(deleted);

        let result = store.validate_session(&session_id).await;
        assert_eq!(result, None);

        let deleted_again = store.delete_session(&session_id).await;
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_extract_session_id_from_session_header() {
        let mut headers = HeaderMap::new();
        headers.insert("session", "test-session-id".parse().unwrap());

        let session_id = extract_session_id(&headers);
        assert_eq!(session_id, Some("test-session-id".to_string()));
    }

    #[tokio::test]
    async fn test_extract_session_id_from_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "test-session-id".parse().unwrap());

        let session_id = extract_session_id(&headers);
        assert_eq!(session_id, Some("test-session-id".to_string()));
    }

    #[tokio::test]
    async fn test_extract_session_id_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert("session", "session-session-id".parse().unwrap());

        let session_id = extract_session_id(&headers);
        assert_eq!(session_id, Some("session-session-id".to_string()));

        headers.remove("session");
        headers.insert("authorization", "auth-session-id".parse().unwrap());

        let session_id = extract_session_id(&headers);
        assert_eq!(session_id, Some("auth-session-id".to_string()));
    }

    #[tokio::test]
    async fn test_extract_session_id_missing() {
        let headers = HeaderMap::new();

        let session_id = extract_session_id(&headers);
        assert_eq!(session_id, None);
    }

    #[tokio::test]
    async fn test_require_auth_function() {
        let request = Request::builder()
            .extension("user-789".to_string())
            .body(())
            .unwrap();

        let user_id = require_auth(&request);
        assert_eq!(user_id, Some("user-789".to_string()));
    }

    #[tokio::test]
    async fn test_require_auth_function_not_authenticated() {
        let request = Request::builder().body(()).unwrap();

        let user_id = require_auth(&request);
        assert_eq!(user_id, None);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    fn create_test_app(session_store: SessionStore) -> Router {
        Router::new()
            .route("/protected", get(protected_handler))
            .route("/public", get(public_handler))
            .layer(axum::middleware::from_fn_with_state(
                session_store.clone(),
                auth_middleware,
            ))
            .layer(Extension(session_store))
    }

    async fn protected_handler(Extension(user_id): Extension<String>) -> String {
        format!("User ID: {}", user_id)
    }

    async fn public_handler() -> &'static str {
        "Public endpoint"
    }

    #[tokio::test]
    async fn test_protected_route_without_session_returns_401() {
        let session_store = SessionStore::new();
        let app = create_test_app(session_store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_protected_route_with_valid_session_allows_access() {
        let session_store = SessionStore::new();
        let user_id = "user-test-123".to_string();
        let session_id = session_store.create_session(user_id.clone()).await;

        let app = create_test_app(session_store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("session", session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(body_str, format!("User ID: {}", user_id));
    }

    #[tokio::test]
    async fn test_protected_route_with_invalid_session_returns_401() {
        let session_store = SessionStore::new();
        let app = create_test_app(session_store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("session", "invalid-session-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_protected_route_with_authorization_header() {
        let session_store = SessionStore::new();
        let user_id = "user-test-456".to_string();
        let session_id = session_store.create_session(user_id.clone()).await;

        let app = create_test_app(session_store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("authorization", session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(body_str, format!("User ID: {}", user_id));
    }

    #[tokio::test]
    async fn test_deleted_session_returns_401() {
        let session_store = SessionStore::new();
        let user_id = "user-test-789".to_string();
        let session_id = session_store.create_session(user_id.clone()).await;

        assert!(session_store.delete_session(&session_id).await);

        let app = create_test_app(session_store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("session", session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_multiple_sessions_for_same_user() {
        let session_store = SessionStore::new();
        let user_id = "user-test-multi".to_string();

        let session_id_1 = session_store.create_session(user_id.clone()).await;
        let session_id_2 = session_store.create_session(user_id.clone()).await;

        assert_ne!(session_id_1, session_id_2);

        let app = create_test_app(session_store);

        for session_id in [session_id_1, session_id_2] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/protected")
                        .header("session", session_id)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
