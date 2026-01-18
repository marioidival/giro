use anyhow::{Context, Result, bail};
use bcrypt::{DEFAULT_COST, hash, verify};
use ralph_models::{CreateUser, LoginUser, User};
use ralph_repositories::UserRepository;

#[derive(Clone, Debug)]
pub struct AuthService {
    user_repo: UserRepository,
}

impl AuthService {
    pub fn new(user_repo: UserRepository) -> Self {
        Self { user_repo }
    }

    pub async fn register(&self, create_user: CreateUser) -> Result<User> {
        if create_user.password.is_empty() {
            bail!("Password cannot be empty");
        }
        if create_user.password.len() < 8 {
            bail!("Password must be at least 8 characters");
        }

        let username = &create_user.username;
        if username.len() < 3 || username.len() > 50 {
            bail!("Username must be between 3 and 50 characters");
        }
        if !username.chars().all(|c| c.is_alphanumeric()) {
            bail!("Username must contain only alphanumeric characters");
        }

        if self
            .user_repo
            .find_by_username(username)
            .await
            .context("Failed to check username existence")?
            .is_some()
        {
            bail!("Username already exists");
        }

        let password_hash =
            hash_password(&create_user.password).context("Failed to hash password")?;

        let user = self
            .user_repo
            .create(CreateUser {
                username: create_user.username,
                email: create_user.email,
                password: password_hash,
            })
            .await
            .context("Failed to create user")?;

        Ok(user)
    }

    pub async fn login(&self, login_user: LoginUser) -> Result<User> {
        let user = self
            .user_repo
            .find_by_username(&login_user.username)
            .await
            .context("Failed to query user")?
            .ok_or_else(|| anyhow::anyhow!("Invalid username or password"))?;

        let is_valid = verify_password(&login_user.password, &user.password_hash)
            .context("Failed to verify password")?;

        if !is_valid {
            bail!("Invalid username or password");
        }

        Ok(user)
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
        self.user_repo
            .find_by_id(id)
            .await
            .context("Failed to query user by id")
    }
}

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hashed: &str) -> Result<bool> {
    let is_valid = verify(password, hashed)?;
    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_same_password() {
        let password = "test_password_123";
        let hashed = hash_password(password).expect("Failed to hash password");

        assert_ne!(hashed, password);
        let is_valid = verify_password(password, &hashed).expect("Failed to verify password");
        assert!(is_valid);
    }

    #[test]
    fn test_verify_fails_with_wrong_password() {
        let password = "test_password_123";
        let wrong_password = "wrong_password_456";
        let hashed = hash_password(password).expect("Failed to hash password");

        let is_valid = verify_password(wrong_password, &hashed).expect("Failed to verify password");
        assert!(!is_valid);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use ralph_repositories::{Database, UserRepository};

    #[tokio::test]
    async fn test_register_new_user() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let create_user = CreateUser {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "secure_password".to_string(),
        };

        let user = auth_service.register(create_user).await?;

        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert_ne!(user.password_hash, "secure_password");

        Ok(())
    }

    #[tokio::test]
    async fn test_register_duplicate_user_fails() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let create_user = CreateUser {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "secure_password".to_string(),
        };

        auth_service.register(create_user.clone()).await?;

        let result = auth_service.register(create_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_with_empty_password_fails() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let create_user = CreateUser {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "".to_string(),
        };

        let result = auth_service.register(create_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_with_short_password_fails() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let create_user = CreateUser {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "short".to_string(),
        };

        let result = auth_service.register(create_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("8 characters"));

        Ok(())
    }

    #[tokio::test]
    async fn test_login_with_valid_credentials() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let create_user = CreateUser {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "secure_password".to_string(),
        };

        auth_service.register(create_user).await?;

        let login_user = LoginUser {
            username: "alice".to_string(),
            password: "secure_password".to_string(),
        };

        let user = auth_service.login(login_user).await?;

        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");

        Ok(())
    }

    #[tokio::test]
    async fn test_login_with_invalid_password_fails() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let create_user = CreateUser {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "secure_password".to_string(),
        };

        auth_service.register(create_user).await?;

        let login_user = LoginUser {
            username: "alice".to_string(),
            password: "wrong_password".to_string(),
        };

        let result = auth_service.login(login_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));

        Ok(())
    }

    #[tokio::test]
    async fn test_login_with_nonexistent_user_fails() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        let user_repo = UserRepository::new(db.pool().clone());
        let auth_service = AuthService::new(user_repo);

        let login_user = LoginUser {
            username: "nonexistent".to_string(),
            password: "password".to_string(),
        };

        let result = auth_service.login(login_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));

        Ok(())
    }
}
