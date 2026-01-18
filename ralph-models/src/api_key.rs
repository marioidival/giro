use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// LLM provider enum for API keys
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ApiKeyProvider {
    /// Anthropic Claude API
    Anthropic,
    /// OpenAI API (GPT-4, etc)
    OpenAI,
    /// Sourcegraph Amp
    Amp,
}

impl ApiKeyProvider {
    /// Convert provider to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyProvider::Anthropic => "anthropic",
            ApiKeyProvider::OpenAI => "openai",
            ApiKeyProvider::Amp => "amp",
        }
    }
}

impl FromStr for ApiKeyProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(ApiKeyProvider::Anthropic),
            "openai" => Ok(ApiKeyProvider::OpenAI),
            "amp" => Ok(ApiKeyProvider::Amp),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

/// API key model for storing user-specific LLM provider credentials
///
/// API keys are encrypted before storage in the database using AES-256-GCM.
/// Each user can have one API key per provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiKey {
    /// Unique identifier for the API key
    pub id: String,
    /// User ID who owns this API key
    pub user_id: String,
    /// LLM provider (anthropic, openai, amp)
    pub provider: ApiKeyProvider,
    /// Encrypted API key (AES-256-GCM encrypted)
    pub encrypted_key: String,
    /// Whether this API key is active
    pub is_active: bool,
    /// Timestamp when this API key was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when this API key was last updated
    pub updated_at: DateTime<Utc>,
}

impl ApiKey {
    /// Create a new API key
    ///
    /// # Arguments
    /// * `user_id` - User ID who owns this API key
    /// * `provider` - LLM provider
    /// * `encrypted_key` - Encrypted API key
    /// * `is_active` - Whether the key is active
    ///
    /// # Returns
    /// A new ApiKey instance with generated ID and timestamps
    pub fn new(
        user_id: String,
        provider: ApiKeyProvider,
        encrypted_key: String,
        is_active: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            provider,
            encrypted_key,
            is_active,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Request payload for creating a new API key
///
/// This struct is used for validation when creating a new API key.
/// The key field contains the plaintext API key which will be encrypted
/// before storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateApiKey {
    /// User ID who will own this API key
    pub user_id: String,
    /// LLM provider
    pub provider: ApiKeyProvider,
    /// Plaintext API key (will be encrypted before storage)
    pub key: String,
    /// Whether the key should be active
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_provider_as_str() {
        assert_eq!(ApiKeyProvider::Anthropic.as_str(), "anthropic");
        assert_eq!(ApiKeyProvider::OpenAI.as_str(), "openai");
        assert_eq!(ApiKeyProvider::Amp.as_str(), "amp");
    }

    #[test]
    fn test_api_key_provider_from_str() {
        assert_eq!(
            "anthropic".parse::<ApiKeyProvider>().unwrap(),
            ApiKeyProvider::Anthropic
        );
        assert_eq!(
            "ANTHROPIC".parse::<ApiKeyProvider>().unwrap(),
            ApiKeyProvider::Anthropic
        );
        assert_eq!(
            "openai".parse::<ApiKeyProvider>().unwrap(),
            ApiKeyProvider::OpenAI
        );
        assert_eq!(
            "amp".parse::<ApiKeyProvider>().unwrap(),
            ApiKeyProvider::Amp
        );
        assert!("unknown".parse::<ApiKeyProvider>().is_err());
    }

    #[test]
    fn test_api_key_new_generates_unique_id() {
        let key1 = ApiKey::new(
            "user1".to_string(),
            ApiKeyProvider::Anthropic,
            "encrypted1".to_string(),
            true,
        );
        let key2 = ApiKey::new(
            "user1".to_string(),
            ApiKeyProvider::OpenAI,
            "encrypted2".to_string(),
            true,
        );

        assert_ne!(key1.id, key2.id);
    }

    #[test]
    fn test_api_key_new_sets_all_fields() {
        let user_id = "user123".to_string();
        let provider = ApiKeyProvider::Anthropic;
        let encrypted_key = "encrypted_key_abc123".to_string();
        let is_active = true;

        let key = ApiKey::new(
            user_id.clone(),
            provider.clone(),
            encrypted_key.clone(),
            is_active,
        );

        assert_eq!(key.user_id, user_id);
        assert_eq!(key.provider, provider);
        assert_eq!(key.encrypted_key, encrypted_key);
        assert_eq!(key.is_active, is_active);
        assert!(Uuid::parse_str(&key.id).is_ok());
    }

    #[test]
    fn test_api_key_new_generates_valid_uuid() {
        let key = ApiKey::new(
            "user123".to_string(),
            ApiKeyProvider::OpenAI,
            "encrypted_key".to_string(),
            true,
        );

        let uuid_result = Uuid::parse_str(&key.id);
        assert!(uuid_result.is_ok());
    }

    #[test]
    fn test_api_key_created_at_is_set() {
        let before_creation = Utc::now();
        let key = ApiKey::new(
            "user123".to_string(),
            ApiKeyProvider::Amp,
            "encrypted_key".to_string(),
            true,
        );
        let after_creation = Utc::now();

        assert!(key.created_at >= before_creation);
        assert!(key.created_at <= after_creation);
    }

    #[test]
    fn test_api_key_updated_at_equals_created_at() {
        let key = ApiKey::new(
            "user123".to_string(),
            ApiKeyProvider::Anthropic,
            "encrypted_key".to_string(),
            true,
        );

        assert_eq!(key.created_at, key.updated_at);
    }

    #[test]
    fn test_api_key_inactive() {
        let key = ApiKey::new(
            "user123".to_string(),
            ApiKeyProvider::OpenAI,
            "encrypted_key".to_string(),
            false,
        );

        assert!(!key.is_active);
    }

    #[test]
    fn test_api_key_serialization() {
        let key = ApiKey::new(
            "user123".to_string(),
            ApiKeyProvider::Anthropic,
            "encrypted_key".to_string(),
            true,
        );

        let json = serde_json::to_string(&key).unwrap();
        let deserialized: ApiKey = serde_json::from_str(&json).unwrap();

        assert_eq!(key.id, deserialized.id);
        assert_eq!(key.user_id, deserialized.user_id);
        assert_eq!(key.provider, deserialized.provider);
        assert_eq!(key.encrypted_key, deserialized.encrypted_key);
        assert_eq!(key.is_active, deserialized.is_active);
        assert_eq!(key.created_at, deserialized.created_at);
        assert_eq!(key.updated_at, deserialized.updated_at);
    }

    #[test]
    fn test_api_key_provider_serialization() {
        let provider = ApiKeyProvider::OpenAI;

        let json = serde_json::to_string(&provider).unwrap();
        let deserialized: ApiKeyProvider = serde_json::from_str(&json).unwrap();

        assert_eq!(provider, deserialized);
    }

    #[test]
    fn test_create_api_key_serialization() {
        let create_key = CreateApiKey {
            user_id: "user123".to_string(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-test-key".to_string(),
            is_active: true,
        };

        let json = serde_json::to_string(&create_key).unwrap();
        let deserialized: CreateApiKey = serde_json::from_str(&json).unwrap();

        assert_eq!(create_key, deserialized);
    }

    #[test]
    fn test_create_api_key_inactive() {
        let create_key = CreateApiKey {
            user_id: "user123".to_string(),
            provider: ApiKeyProvider::OpenAI,
            key: "sk-test-key".to_string(),
            is_active: false,
        };

        assert!(!create_key.is_active);
    }
}
