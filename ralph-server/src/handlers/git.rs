//! Git credential HTTP handlers for Ralph Loop Manager server.
//!
//! This module provides handlers for listing, creating, and deleting Git credentials.
//! All handlers require authentication via auth middleware and check ownership
//! to ensure users can only access their own credentials.

use askama::Template;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use ralph_models::{CreateGitCredential, GitCredential};
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
use crate::middleware::csrf::CsrfToken;
use crate::templates::GitCredentialsListTemplate;

/// Request payload for creating a Git credential.
///
/// The token is provided as plaintext and will be encrypted before storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateGitCredentialRequest {
    /// Git provider name (e.g., "github", "gitlab", "bitbucket")
    pub provider: String,
    /// Personal access token (plaintext, will be encrypted)
    pub token: String,
    /// Optional username for Git operations
    pub username: Option<String>,
    /// Optional email for Git operations
    pub email: Option<String>,
}

/// Response structure for listing Git credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListGitCredentialsResponse {
    /// Whether listing was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The list of Git credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Vec<GitCredentialSummary>>,
}

/// A lightweight Git credential representation for list responses (without encrypted_token).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitCredentialSummary {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<GitCredential> for GitCredentialSummary {
    fn from(cred: GitCredential) -> Self {
        Self {
            id: cred.id,
            user_id: cred.user_id,
            provider: cred.provider,
            username: cred.username,
            email: cred.email,
            created_at: cred.created_at.to_rfc3339(),
            updated_at: cred.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for creating a Git credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateGitCredentialResponse {
    /// Whether creation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The ID of newly created credential (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

/// Response structure for deleting a Git credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteGitCredentialResponse {
    /// Whether deletion was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
}

/// Handles listing Git credentials requests for the authenticated user.
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Calls git_credentials_repo.list_by_user(user_id)
/// 3. Returns list of credentials without encrypted tokens
///
/// # Arguments
/// * `state` - The application state containing git credentials repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with credential list on success
/// * `500 Internal Server Error` for server errors
pub async fn list_git_credentials(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ListGitCredentialsResponse>) {
    match state
        .git_credentials_repository
        .list_by_user(&user_id)
        .await
    {
        Ok(credentials) => {
            let summaries: Vec<GitCredentialSummary> = credentials
                .into_iter()
                .map(GitCredentialSummary::from)
                .collect();

            (
                StatusCode::OK,
                Json(ListGitCredentialsResponse {
                    success: true,
                    message: format!("Retrieved {} credentials", summaries.len()),
                    credentials: Some(summaries),
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListGitCredentialsResponse {
                success: false,
                message: format!("Failed to retrieve credentials: {}", e),
                credentials: None,
            }),
        ),
    }
}

/// Handles creating a Git credential for the authenticated user.
///
/// This endpoint:
/// 1. Validates provider and token (required)
/// 2. Extracts user_id from auth middleware
/// 3. Creates CreateGitCredential from payload with user_id
/// 4. Calls git_credentials_repo.create() which encrypts the token
/// 5. Returns credential ID on success
///
/// # Arguments
/// * `payload` - The credential creation data (JSON)
/// * `state` - The application state containing git credentials repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `201 Created` with credential ID on success
/// * `400 Bad Request` for validation errors
/// * `500 Internal Server Error` for server errors
pub async fn create_git_credentials(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateGitCredentialRequest>,
) -> (StatusCode, Json<CreateGitCredentialResponse>) {
    // Validate provider
    if payload.provider.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreateGitCredentialResponse {
                success: false,
                message: "Provider is required".to_string(),
                credential_id: None,
            }),
        );
    }

    // Validate token
    if payload.token.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreateGitCredentialResponse {
                success: false,
                message: "Token is required".to_string(),
                credential_id: None,
            }),
        );
    }

    let create_credential = CreateGitCredential {
        user_id,
        provider: payload.provider.clone(),
        encrypted_token: payload.token, // Will be encrypted by the repository
        username: payload.username,
        email: payload.email,
    };

    match state
        .git_credentials_repository
        .create(create_credential)
        .await
    {
        Ok(credential) => (
            StatusCode::CREATED,
            Json(CreateGitCredentialResponse {
                success: true,
                message: "Git credential created successfully".to_string(),
                credential_id: Some(credential.id),
            }),
        ),
        Err(e) => {
            let error_msg = e.to_string();

            // Check for unique constraint violation
            if error_msg.contains("UNIQUE constraint failed")
                || error_msg.contains("already exists")
            {
                (
                    StatusCode::CONFLICT,
                    Json(CreateGitCredentialResponse {
                        success: false,
                        message: format!(
                            "A credential for provider '{}' already exists",
                            payload.provider
                        ),
                        credential_id: None,
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CreateGitCredentialResponse {
                        success: false,
                        message: format!("Failed to create credential: {}", e),
                        credential_id: None,
                    }),
                )
            }
        }
    }
}

/// Handles deleting a Git credential.
///
/// This endpoint:
/// 1. Calls git_credentials_repo.delete(id, user_id) which verifies ownership
/// 2. Returns 404 if credential not found or user doesn't own it
/// 3. Returns success on deletion
///
/// # Arguments
/// * `path` - Path parameters containing credential ID
/// * `state` - The application state containing git credentials repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful deletion
/// * `404 Not Found` if credential doesn't exist or user doesn't own it
/// * `500 Internal Server Error` for server errors
pub async fn delete_git_credentials(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(credential_id): Path<String>,
) -> (StatusCode, Json<DeleteGitCredentialResponse>) {
    match state
        .git_credentials_repository
        .delete(&credential_id, &user_id)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(DeleteGitCredentialResponse {
                success: true,
                message: "Git credential deleted successfully".to_string(),
            }),
        ),
        Err(e) => {
            let error_msg = e.to_string();

            if error_msg.contains("not found or unauthorized") {
                (
                    StatusCode::NOT_FOUND,
                    Json(DeleteGitCredentialResponse {
                        success: false,
                        message: "Git credential not found".to_string(),
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DeleteGitCredentialResponse {
                        success: false,
                        message: format!("Failed to delete credential: {}", e),
                    }),
                )
            }
        }
    }
}

/// Handles rendering the Git credentials management page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Generates CSRF token
/// 3. Retrieves all credentials for the user
/// 4. Renders the credentials management template
///
/// # Arguments
/// * `state` - The application state containing git credentials repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `500 Internal Server Error` for server errors
pub async fn git_credentials_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    // Get credentials for the user
    let credentials = match state
        .git_credentials_repository
        .list_by_user(&user_id)
        .await
    {
        Ok(creds) => creds.into_iter().map(GitCredentialSummary::from).collect(),
        Err(_e) => Vec::new(),
    };

    let csrf_token = CsrfToken::generate().to_string();

    let template = GitCredentialsListTemplate {
        logged_in,
        csrf_token,
        credentials,
        active_path: "/git/credentials".to_string(),
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_git_credential_request_serialization() {
        let request = CreateGitCredentialRequest {
            provider: "github".to_string(),
            token: "ghp_test_token".to_string(),
            username: Some("alice".to_string()),
            email: Some("alice@example.com".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateGitCredentialRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_create_git_credential_request_without_optional_fields() {
        let request = CreateGitCredentialRequest {
            provider: "gitlab".to_string(),
            token: "glpat_test_token".to_string(),
            username: None,
            email: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateGitCredentialRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_git_credential_summary_serialization() {
        let summary = GitCredentialSummary {
            id: "cred-123".to_string(),
            user_id: "user-456".to_string(),
            provider: "github".to_string(),
            username: Some("alice".to_string()),
            email: Some("alice@example.com".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: GitCredentialSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(summary, deserialized);
    }

    #[test]
    fn test_list_git_credentials_response_serialization() {
        let response = ListGitCredentialsResponse {
            success: true,
            message: "Retrieved 2 credentials".to_string(),
            credentials: Some(vec![]),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ListGitCredentialsResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_create_git_credential_response_serialization() {
        let response = CreateGitCredentialResponse {
            success: true,
            message: "Credential created".to_string(),
            credential_id: Some("cred-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CreateGitCredentialResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_delete_git_credential_response_serialization() {
        let response = DeleteGitCredentialResponse {
            success: true,
            message: "Credential deleted".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: DeleteGitCredentialResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_list_response_skips_null_credentials() {
        let response = ListGitCredentialsResponse {
            success: false,
            message: "Error".to_string(),
            credentials: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("credentials"));
    }

    #[test]
    fn test_create_response_skips_null_credential_id() {
        let response = CreateGitCredentialResponse {
            success: false,
            message: "Error".to_string(),
            credential_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("credential_id"));
    }
}
