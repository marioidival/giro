use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            email,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoginUser {
    pub username: String,
    pub password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new_generates_unique_id() {
        let user1 = User::new(
            "alice".to_string(),
            "alice@example.com".to_string(),
            "hash123".to_string(),
        );
        let user2 = User::new(
            "bob".to_string(),
            "bob@example.com".to_string(),
            "hash456".to_string(),
        );

        assert_ne!(user1.id, user2.id);
    }

    #[test]
    fn test_user_new_sets_all_fields() {
        let username = "testuser".to_string();
        let email = "test@example.com".to_string();
        let password_hash = "hashedpassword".to_string();

        let user = User::new(username.clone(), email.clone(), password_hash.clone());

        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, password_hash);
        assert!(Uuid::parse_str(&user.id).is_ok());
    }

    #[test]
    fn test_user_new_generates_valid_uuid() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hash123".to_string(),
        );

        let uuid_result = Uuid::parse_str(&user.id);
        assert!(uuid_result.is_ok());
    }

    #[test]
    fn test_user_created_at_is_set() {
        let before_creation = Utc::now();
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hash123".to_string(),
        );
        let after_creation = Utc::now();

        assert!(user.created_at >= before_creation);
        assert!(user.created_at <= after_creation);
    }

    #[test]
    fn test_create_user_serialization() {
        let create_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&create_user).unwrap();
        let deserialized: CreateUser = serde_json::from_str(&json).unwrap();

        assert_eq!(create_user, deserialized);
    }

    #[test]
    fn test_login_user_serialization() {
        let login_user = LoginUser {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&login_user).unwrap();
        let deserialized: LoginUser = serde_json::from_str(&json).unwrap();

        assert_eq!(login_user, deserialized);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hash123".to_string(),
        );

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.password_hash, deserialized.password_hash);
        assert_eq!(user.created_at, deserialized.created_at);
        assert_eq!(user.updated_at, deserialized.updated_at);
    }
}
