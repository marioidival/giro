//! Authentication HTTP handlers for the Ralph Loop Manager server.
//!
//! This module provides handlers for user registration, login, and logout operations.
//! All handlers use JSON request/response format and integrate with the AuthService
//! and SessionStore for authentication and session management.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use ralph_models::{CreateUser, LoginUser};
use ralph_repositories::{LoopRepository, TaskRepository};
use ralph_services::{AuthService, LoopExecutor};
use serde::{Deserialize, Serialize};

use crate::middleware::auth::SessionStore;
use crate::validation::{validate_email, validate_password, validate_username};

/// Application state containing shared services and stores.
///
/// This state is injected into handlers via the `State` extractor.
#[derive(Clone, Debug)]
pub struct AppState {
    /// Authentication service for user operations
    pub auth_service: AuthService,
    /// Session store for managing user sessions
    pub session_store: SessionStore,
    /// Loop repository for loop database operations
    pub loop_repository: LoopRepository,
    /// Task repository for task database operations
    pub task_repository: TaskRepository,
    /// Loop executor for controlling loop execution (start, pause, resume, stop)
    pub loop_executor: LoopExecutor,
}

impl AppState {
    /// Creates a new AppState with provided services.
    ///
    /// # Arguments
    /// * `auth_service` - The authentication service instance
    /// * `session_store` - The session store instance
    /// * `loop_repository` - The loop repository instance
    /// * `loop_executor` - The loop executor instance for controlling loop execution
    pub fn new(
        auth_service: AuthService,
        session_store: SessionStore,
        loop_repository: LoopRepository,
        task_repository: TaskRepository,
        loop_executor: LoopExecutor,
    ) -> Self {
        Self {
            auth_service,
            session_store,
            loop_repository,
            task_repository,
            loop_executor,
        }
    }
}

/// Response structure for user registration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegisterResponse {
    /// Whether the registration was successful
    pub success: bool,
    /// A message describing the result
    pub message: String,
    /// The ID of the newly created user (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Response structure for user login.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoginResponse {
    /// Whether the login was successful
    pub success: bool,
    /// A message describing the result
    pub message: String,
    /// The ID of the authenticated user (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// The session token for authentication (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

/// Response structure for user logout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogoutResponse {
    /// Whether the logout was successful
    pub success: bool,
    /// A message describing the result
    pub message: String,
}

/// Handles user registration requests.
///
/// This endpoint:
/// 1. Validates the username, email, and password
/// 2. Calls the AuthService to create the user
/// 3. Creates a session for the new user
/// 4. Returns the user ID and session token
///
/// # Arguments
/// * `state` - The application state containing auth service and session store
/// * `payload` - The user registration data (JSON)
///
/// # Returns
/// * `201 Created` with user details on success
/// * `400 Bad Request` for validation errors
/// * `409 Conflict` for duplicate usernames
/// * `500 Internal Server Error` for server errors
#[axum::debug_handler]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<RegisterResponse>) {
    // Validate username
    if let Err(e) = validate_username(&payload.username) {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: format!("Invalid username: {}", e.message.unwrap_or_default()),
                user_id: None,
            }),
        );
    }

    // Validate email
    if let Err(e) = validate_email(&payload.email) {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: format!("Invalid email: {}", e.message.unwrap_or_default()),
                user_id: None,
            }),
        );
    }

    // Validate password
    if let Err(e) = validate_password(&payload.password) {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: format!("Invalid password: {}", e.message.unwrap_or_default()),
                user_id: None,
            }),
        );
    }

    // Register user through auth service
    match state.auth_service.register(payload).await {
        Ok(user) => {
            // Create session for the new user
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

            // Map error to appropriate status code
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

/// Handles user login requests.
///
/// This endpoint:
/// 1. Validates the username and password
/// 2. Calls the AuthService to authenticate the user
/// 3. Creates a session for the authenticated user
/// 4. Returns the user ID and session token
///
/// # Arguments
/// * `state` - The application state containing auth service and session store
/// * `payload` - The user login data (JSON)
///
/// # Returns
/// * `200 OK` with user details and session token on success
/// * `400 Bad Request` for validation errors
/// * `401 Unauthorized` for invalid credentials
/// * `500 Internal Server Error` for server errors
#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginUser>,
) -> (StatusCode, Json<LoginResponse>) {
    // Validate username
    if let Err(e) = validate_username(&payload.username) {
        return (
            StatusCode::BAD_REQUEST,
            Json(LoginResponse {
                success: false,
                message: format!("Invalid username: {}", e.message.unwrap_or_default()),
                user_id: None,
                session_token: None,
            }),
        );
    }

    // Validate password
    if let Err(e) = validate_password(&payload.password) {
        return (
            StatusCode::BAD_REQUEST,
            Json(LoginResponse {
                success: false,
                message: format!("Invalid password: {}", e.message.unwrap_or_default()),
                user_id: None,
                session_token: None,
            }),
        );
    }

    // Authenticate user through auth service
    match state.auth_service.login(payload).await {
        Ok(user) => {
            // Create session for the authenticated user
            let session_token = state.session_store.create_session(user.id.clone()).await;

            (
                StatusCode::OK,
                Json(LoginResponse {
                    success: true,
                    message: "Login successful".to_string(),
                    user_id: Some(user.id),
                    session_token: Some(session_token),
                }),
            )
        }
        Err(e) => {
            let error_msg = e.to_string();

            if error_msg.contains("Invalid") {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(LoginResponse {
                        success: false,
                        message: error_msg,
                        user_id: None,
                        session_token: None,
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LoginResponse {
                        success: false,
                        message: "Login failed".to_string(),
                        user_id: None,
                        session_token: None,
                    }),
                )
            }
        }
    }
}

/// Handles user logout requests.
///
/// This endpoint removes the user's session from the session store.
/// The session ID is extracted from the request headers.
///
/// # Arguments
/// * `state` - The application state containing the session store
///
/// # Returns
/// * `200 OK` on successful logout
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Json<LogoutResponse> {
    // Extract session ID from headers
    let session_id = headers
        .get("session")
        .or_else(|| headers.get("authorization"))
        .and_then(|value| value.to_str().ok());

    if let Some(session_id) = session_id {
        state.session_store.delete_session(session_id).await;
        Json(LogoutResponse {
            success: true,
            message: "Logout successful".to_string(),
        })
    } else {
        Json(LogoutResponse {
            success: true,
            message: "No session to logout".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_response_serialization() {
        let response = RegisterResponse {
            success: true,
            message: "User created".to_string(),
            user_id: Some("user-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: RegisterResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_login_response_serialization() {
        let response = LoginResponse {
            success: true,
            message: "Login successful".to_string(),
            user_id: Some("user-456".to_string()),
            session_token: Some("session-token-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: LoginResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_logout_response_serialization() {
        let response = LogoutResponse {
            success: true,
            message: "Logout successful".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: LogoutResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_register_response_skips_null_user_id() {
        let response = RegisterResponse {
            success: false,
            message: "Error".to_string(),
            user_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("user_id"));
    }

    #[test]
    fn test_login_response_skips_null_fields() {
        let response = LoginResponse {
            success: false,
            message: "Error".to_string(),
            user_id: None,
            session_token: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("user_id"));
        assert!(!json.contains("session_token"));
    }
}
