//! API key HTTP handlers for Ralph Loop Manager server.
//!
//! This module provides handlers for listing, creating, and deactivating API keys.
//! All handlers require authentication via auth middleware and check ownership
//! to ensure users can only access their own API keys.

use askama::Template;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use ralph_models::{ApiKeyProvider, CreateApiKey};
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
use crate::middleware::csrf::CsrfToken;
use crate::templates::ApiKeysListTemplate;

/// Response structure for listing API keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListApiKeysResponse {
    /// Whether listing was successful
    pub success: bool,
    /// The list of API keys (without exposing the actual keys)
    pub keys: Vec<ApiKeySummary>,
}

/// A lightweight API key representation for list responses.
/// The actual API key is never exposed in responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiKeySummary {
    pub id: String,
    pub provider: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ralph_models::ApiKey> for ApiKeySummary {
    fn from(key: ralph_models::ApiKey) -> Self {
        Self {
            id: key.id,
            provider: key.provider.as_str().to_string(),
            is_active: key.is_active,
            created_at: key.created_at.to_rfc3339(),
            updated_at: key.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for creating an API key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateApiKeyResponse {
    /// Whether creation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The ID of newly created API key (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

/// Request payload for creating a new API key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateApiKeyRequest {
    /// LLM provider
    pub provider: String,
    /// Plaintext API key
    pub key: String,
}

/// Response structure for deactivating an API key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeactivateApiKeyResponse {
    /// Whether deactivation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
}

/// Handles listing API keys for the authenticated user.
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Lists all API keys for the user
/// 3. Returns key summaries without exposing the actual keys
///
/// # Arguments
/// * `state` - The application state containing API key repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with list of API key summaries on success
/// * `500 Internal Server Error` for server errors
pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ListApiKeysResponse>) {
    match state.api_key_repository.list_by_user(&user_id).await {
        Ok(keys) => {
            let summaries: Vec<ApiKeySummary> = keys.into_iter().map(ApiKeySummary::from).collect();
            (
                StatusCode::OK,
                Json(ListApiKeysResponse {
                    success: true,
                    keys: summaries,
                }),
            )
        }
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListApiKeysResponse {
                success: false,
                keys: Vec::new(),
            }),
        ),
    }
}

/// Handles creating a new API key.
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Validates the provider
/// 3. Creates the API key with encryption
/// 4. Returns the key ID on success
///
/// # Arguments
/// * `payload` - The API key creation data (JSON)
/// * `state` - The application state containing API key repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `201 Created` with key ID on success
/// * `400 Bad Request` for validation errors
/// * `500 Internal Server Error` for server errors
pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> (StatusCode, Json<CreateApiKeyResponse>) {
    // Parse provider from string
    let provider = match payload.provider.to_lowercase().as_str() {
        "anthropic" => ApiKeyProvider::Anthropic,
        "openai" => ApiKeyProvider::OpenAI,
        "amp" => ApiKeyProvider::Amp,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(CreateApiKeyResponse {
                    success: false,
                    message: format!("Invalid provider: {}", payload.provider),
                    key_id: None,
                }),
            );
        }
    };

    // Validate key is not empty
    if payload.key.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreateApiKeyResponse {
                success: false,
                message: "API key cannot be empty".to_string(),
                key_id: None,
            }),
        );
    }

    let create_key = CreateApiKey {
        user_id,
        provider,
        key: payload.key,
        is_active: true,
    };

    match state.api_key_repository.create(create_key).await {
        Ok(key) => (
            StatusCode::CREATED,
            Json(CreateApiKeyResponse {
                success: true,
                message: "API key created successfully".to_string(),
                key_id: Some(key.id),
            }),
        ),
        Err(e) => {
            let error_msg = e.to_string();

            // Map unique constraint error to conflict
            if error_msg.contains("UNIQUE constraint failed") {
                (
                    StatusCode::CONFLICT,
                    Json(CreateApiKeyResponse {
                        success: false,
                        message: format!(
                            "API key for {} provider already exists",
                            payload.provider
                        ),
                        key_id: None,
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CreateApiKeyResponse {
                        success: false,
                        message: "Failed to create API key".to_string(),
                        key_id: None,
                    }),
                )
            }
        }
    }
}

/// Handles deactivating an API key.
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Deactivates the API key with ownership check
/// 3. Returns success on deactivation
///
/// # Arguments
/// * `path` - Path parameters containing API key ID
/// * `state` - The application state containing API key repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful deactivation
/// * `404 Not Found` if key doesn't exist or user doesn't own it
/// * `500 Internal Server Error` for server errors
pub async fn deactivate_api_key(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<DeactivateApiKeyResponse>) {
    match state.api_key_repository.deactivate(&id, &user_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(DeactivateApiKeyResponse {
                success: true,
                message: "API key deactivated successfully".to_string(),
            }),
        ),
        Err(e) => {
            let error_msg = e.to_string();

            if error_msg.contains("not found or unauthorized") {
                (
                    StatusCode::NOT_FOUND,
                    Json(DeactivateApiKeyResponse {
                        success: false,
                        message: error_msg,
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DeactivateApiKeyResponse {
                        success: false,
                        message: "Failed to deactivate API key".to_string(),
                    }),
                )
            }
        }
    }
}

/// Handles rendering the API keys management page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Generates CSRF token
/// 3. Retrieves all API keys for the user
/// 4. Renders the API keys management template
///
/// # Arguments
/// * `state` - The application state containing API key repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `500 Internal Server Error` for server errors
pub async fn api_keys_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    // Get API keys for the user
    let keys = match state.api_key_repository.list_by_user(&user_id).await {
        Ok(k) => k.into_iter().map(ApiKeySummary::from).collect(),
        Err(_e) => Vec::new(),
    };

    let csrf_token = CsrfToken::generate().to_string();

    let template = ApiKeysListTemplate {
        logged_in,
        csrf_token,
        keys,
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
    fn test_api_key_summary_serialization() {
        let summary = ApiKeySummary {
            id: "key-123".to_string(),
            provider: "anthropic".to_string(),
            is_active: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: ApiKeySummary = serde_json::from_str(&json).unwrap();

        assert_eq!(summary, deserialized);
        // Ensure actual key is not in the JSON
        assert!(!json.contains("\"key\":"));
    }

    #[test]
    fn test_list_api_keys_response_serialization() {
        let response = ListApiKeysResponse {
            success: true,
            keys: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ListApiKeysResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_create_api_key_response_serialization() {
        let response = CreateApiKeyResponse {
            success: true,
            message: "Key created".to_string(),
            key_id: Some("key-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CreateApiKeyResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_deactivate_api_key_response_serialization() {
        let response = DeactivateApiKeyResponse {
            success: true,
            message: "Key deactivated".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: DeactivateApiKeyResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_create_api_key_request_serialization() {
        let request = CreateApiKeyRequest {
            provider: "anthropic".to_string(),
            key: "sk-ant-test-key".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateApiKeyRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_api_key_summary_from_api_key() {
        let api_key = ralph_models::ApiKey::new(
            "user-123".to_string(),
            ApiKeyProvider::OpenAI,
            "encrypted_key".to_string(),
            true,
        );

        let summary = ApiKeySummary::from(api_key.clone());

        assert_eq!(summary.id, api_key.id);
        assert_eq!(summary.provider, "openai");
        assert_eq!(summary.is_active, api_key.is_active);
    }

    #[test]
    fn test_create_api_key_request_empty_key_is_invalid() {
        let request = CreateApiKeyRequest {
            provider: "anthropic".to_string(),
            key: "   ".to_string(),
        };

        assert!(request.key.trim().is_empty());
    }

    #[test]
    fn test_provider_parsing_case_insensitive() {
        let providers = vec![
            ("anthropic", ApiKeyProvider::Anthropic),
            ("ANTHROPIC", ApiKeyProvider::Anthropic),
            ("Anthropic", ApiKeyProvider::Anthropic),
            ("openai", ApiKeyProvider::OpenAI),
            ("OPENAI", ApiKeyProvider::OpenAI),
            ("OpenAI", ApiKeyProvider::OpenAI),
            ("amp", ApiKeyProvider::Amp),
            ("AMP", ApiKeyProvider::Amp),
            ("Amp", ApiKeyProvider::Amp),
        ];

        for (input, expected) in providers {
            let provider = match input.to_lowercase().as_str() {
                "anthropic" => ApiKeyProvider::Anthropic,
                "openai" => ApiKeyProvider::OpenAI,
                "amp" => ApiKeyProvider::Amp,
                _ => panic!("Invalid provider: {}", input),
            };
            assert_eq!(provider, expected, "Failed for input: {}", input);
        }
    }
}
