use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitCredential {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub encrypted_token: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GitCredential {
    pub fn new(
        user_id: String,
        provider: String,
        encrypted_token: String,
        username: Option<String>,
        email: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            provider,
            encrypted_token,
            username,
            email,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateGitCredential {
    pub user_id: String,
    pub provider: String,
    pub encrypted_token: String,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_credential_new_generates_unique_id() {
        let cred1 = GitCredential::new(
            "user1".to_string(),
            "github".to_string(),
            "encrypted1".to_string(),
            Some("alice".to_string()),
            Some("alice@example.com".to_string()),
        );
        let cred2 = GitCredential::new(
            "user1".to_string(),
            "github".to_string(),
            "encrypted2".to_string(),
            Some("alice".to_string()),
            Some("alice@example.com".to_string()),
        );

        assert_ne!(cred1.id, cred2.id);
    }

    #[test]
    fn test_git_credential_new_sets_all_fields() {
        let user_id = "user123".to_string();
        let provider = "github".to_string();
        let encrypted_token = "encrypted_token_abc123".to_string();
        let username = Some("testuser".to_string());
        let email = Some("test@example.com".to_string());

        let cred = GitCredential::new(
            user_id.clone(),
            provider.clone(),
            encrypted_token.clone(),
            username.clone(),
            email.clone(),
        );

        assert_eq!(cred.user_id, user_id);
        assert_eq!(cred.provider, provider);
        assert_eq!(cred.encrypted_token, encrypted_token);
        assert_eq!(cred.username, username);
        assert_eq!(cred.email, email);
        assert!(Uuid::parse_str(&cred.id).is_ok());
    }

    #[test]
    fn test_git_credential_new_generates_valid_uuid() {
        let cred = GitCredential::new(
            "user123".to_string(),
            "github".to_string(),
            "encrypted_token".to_string(),
            Some("testuser".to_string()),
            Some("test@example.com".to_string()),
        );

        let uuid_result = Uuid::parse_str(&cred.id);
        assert!(uuid_result.is_ok());
    }

    #[test]
    fn test_git_credential_created_at_is_set() {
        let before_creation = Utc::now();
        let cred = GitCredential::new(
            "user123".to_string(),
            "github".to_string(),
            "encrypted_token".to_string(),
            Some("testuser".to_string()),
            Some("test@example.com".to_string()),
        );
        let after_creation = Utc::now();

        assert!(cred.created_at >= before_creation);
        assert!(cred.created_at <= after_creation);
    }

    #[test]
    fn test_git_credential_updated_at_equals_created_at() {
        let cred = GitCredential::new(
            "user123".to_string(),
            "github".to_string(),
            "encrypted_token".to_string(),
            Some("testuser".to_string()),
            Some("test@example.com".to_string()),
        );

        assert_eq!(cred.created_at, cred.updated_at);
    }

    #[test]
    fn test_git_credential_without_username_email() {
        let cred = GitCredential::new(
            "user123".to_string(),
            "github".to_string(),
            "encrypted_token".to_string(),
            None,
            None,
        );

        assert!(cred.username.is_none());
        assert!(cred.email.is_none());
    }

    #[test]
    fn test_create_git_credential_serialization() {
        let create_cred = CreateGitCredential {
            user_id: "user123".to_string(),
            provider: "github".to_string(),
            encrypted_token: "encrypted_token".to_string(),
            username: Some("testuser".to_string()),
            email: Some("test@example.com".to_string()),
        };

        let json = serde_json::to_string(&create_cred).unwrap();
        let deserialized: CreateGitCredential = serde_json::from_str(&json).unwrap();

        assert_eq!(create_cred, deserialized);
    }

    #[test]
    fn test_create_git_credential_without_optional_fields() {
        let create_cred = CreateGitCredential {
            user_id: "user123".to_string(),
            provider: "github".to_string(),
            encrypted_token: "encrypted_token".to_string(),
            username: None,
            email: None,
        };

        let json = serde_json::to_string(&create_cred).unwrap();
        let deserialized: CreateGitCredential = serde_json::from_str(&json).unwrap();

        assert_eq!(create_cred, deserialized);
    }

    #[test]
    fn test_git_credential_serialization() {
        let cred = GitCredential::new(
            "user123".to_string(),
            "github".to_string(),
            "encrypted_token".to_string(),
            Some("testuser".to_string()),
            Some("test@example.com".to_string()),
        );

        let json = serde_json::to_string(&cred).unwrap();
        let deserialized: GitCredential = serde_json::from_str(&json).unwrap();

        assert_eq!(cred.id, deserialized.id);
        assert_eq!(cred.user_id, deserialized.user_id);
        assert_eq!(cred.provider, deserialized.provider);
        assert_eq!(cred.encrypted_token, deserialized.encrypted_token);
        assert_eq!(cred.username, deserialized.username);
        assert_eq!(cred.email, deserialized.email);
        assert_eq!(cred.created_at, deserialized.created_at);
        assert_eq!(cred.updated_at, deserialized.updated_at);
    }
}
