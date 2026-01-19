//! User profile HTTP handlers for Ralph Loop Manager server.
//!
//! This module provides handlers for user profile page and settings.
//! All handlers require authentication via auth middleware.

use askama::Template;
use axum::{Extension, extract::State, http::StatusCode, response::Html};
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
use crate::middleware::csrf::CsrfToken;
use crate::templates::{ProfileTemplate, SettingsTemplate};

/// Response structure for profile operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangePasswordResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
}

/// Request payload for changing password.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangePasswordRequest {
    /// Current password
    pub current_password: String,
    /// New password
    pub new_password: String,
}

/// Handles rendering the user profile page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Generates CSRF token
/// 3. Retrieves user information
/// 4. Renders the profile template
///
/// # Arguments
/// * `state` - The application state containing user repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `500 Internal Server Error` for server errors
pub async fn profile_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let active_path = "/profile".to_string();

    // Get user information
    let user_display = match state.auth_service.get_user_by_id(&user_id).await {
        Ok(Some(user)) => UserDisplay::from(user),
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Html("User not found".to_string()));
        }
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("Failed to retrieve user".to_string()),
            );
        }
    };

    let csrf_token = CsrfToken::generate().to_string();

    let template = ProfileTemplate {
        logged_in,
        csrf_token,
        user: user_display,
        active_path,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

/// Handles rendering the settings page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Generates CSRF token
/// 3. Renders the settings template with links to various settings pages
///
/// # Arguments
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `500 Internal Server Error` for server errors
pub async fn settings_page(Extension(user_id): Extension<String>) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let active_path = "/settings".to_string();
    let csrf_token = CsrfToken::generate().to_string();

    let template = SettingsTemplate {
        logged_in,
        csrf_token,
        active_path,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

/// Display representation for user profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDisplay {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

impl From<ralph_models::User> for UserDisplay {
    fn from(user: ralph_models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_models::User;

    #[test]
    fn test_user_display_from_user() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        let display = UserDisplay::from(user.clone());

        assert_eq!(display.id, user.id);
        assert_eq!(display.username, user.username);
        assert_eq!(display.email, user.email);
        assert_eq!(display.created_at, user.created_at.to_rfc3339());
    }

    #[test]
    fn test_change_password_response_serialization() {
        let response = ChangePasswordResponse {
            success: true,
            message: "Password changed".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ChangePasswordResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_change_password_request_serialization() {
        let request = ChangePasswordRequest {
            current_password: "old_password".to_string(),
            new_password: "new_password".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ChangePasswordRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }
}
