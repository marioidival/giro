use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::{CreateGitCredential, GitCredential};
use sqlx::{Pool, Sqlite};

use super::crypto::{decrypt_token, encrypt_token};

/// Repository for Git credential database operations
#[derive(Clone, Debug)]
pub struct GitCredentialsRepository {
    pool: Pool<Sqlite>,
}

impl GitCredentialsRepository {
    /// Create a new GitCredentialsRepository instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::git_credential::GitCredentialsRepository;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: SqlitePool) {
    /// let repo = GitCredentialsRepository::new(pool);
    /// # }
    /// ```
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new Git credential in database
    ///
    /// # Arguments
    /// * `create_credential` - Git credential creation data (token will be encrypted)
    ///
    /// # Returns
    /// The created GitCredential with generated id and timestamps
    ///
    /// # Errors
    /// Returns error if encryption fails or database insertion fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::git_credential::GitCredentialsRepository;
    /// # use ralph_models::CreateGitCredential;
    /// # async fn example(repo: GitCredentialsRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let create_credential = CreateGitCredential {
    ///     user_id: "user-123".to_string(),
    ///     provider: "github".to_string(),
    ///     encrypted_token: "ghp_my_secret_token".to_string(),  // Will be encrypted
    ///     username: Some("alice".to_string()),
    ///     email: Some("alice@example.com".to_string()),
    /// };
    /// let credential = repo.create(create_credential).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, create_credential: CreateGitCredential) -> Result<GitCredential> {
        // Encrypt the token before storing
        let encrypted_token = encrypt_token(&create_credential.encrypted_token)
            .context("Failed to encrypt Git token")?;

        let credential = GitCredential::new(
            create_credential.user_id,
            create_credential.provider,
            encrypted_token,
            create_credential.username,
            create_credential.email,
        );

        sqlx::query!(
            r#"
            INSERT INTO git_credentials (id, user_id, provider, encrypted_token, username, email, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            credential.id,
            credential.user_id,
            credential.provider,
            credential.encrypted_token,
            credential.username,
            credential.email,
            credential.created_at,
            credential.updated_at,
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert Git credential into database")?;

        Ok(credential)
    }

    /// List all Git credentials for a user
    ///
    /// # Arguments
    /// * `user_id` - User ID to list credentials for
    ///
    /// # Returns
    /// Vector of all Git credentials for the user
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::git_credential::GitCredentialsRepository;
    /// # async fn example(repo: GitCredentialsRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let credentials = repo.list_by_user("user-123").await?;
    /// println!("Found {} credentials", credentials.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<GitCredential>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, user_id, provider, encrypted_token, username, email, created_at, updated_at
            FROM git_credentials
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query Git credentials by user")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    user_id,
                    provider,
                    encrypted_token,
                    username,
                    email,
                    created_at,
                    updated_at,
                )| {
                    GitCredential {
                        id,
                        user_id,
                        provider,
                        encrypted_token,
                        username,
                        email,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }

    /// Find a Git credential by user ID and provider
    ///
    /// # Arguments
    /// * `user_id` - User ID to search for
    /// * `provider` - Provider name (e.g., "github", "gitlab", "bitbucket")
    ///
    /// # Returns
    /// Some(GitCredential) if found, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::git_credential::GitCredentialsRepository;
    /// # async fn example(repo: GitCredentialsRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let credential = repo.find_by_user_and_provider("user-123", "github").await?;
    /// if let Some(cred) = credential {
    ///     println!("Found GitHub credential for user");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_user_and_provider(
        &self,
        user_id: &str,
        provider: &str,
    ) -> Result<Option<GitCredential>> {
        let result = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, user_id, provider, encrypted_token, username, email, created_at, updated_at
            FROM git_credentials
            WHERE user_id = ? AND provider = ?
            "#,
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query Git credential by user and provider")?;

        Ok(result.map(
            |(id, user_id, provider, encrypted_token, username, email, created_at, updated_at)| {
                GitCredential {
                    id,
                    user_id,
                    provider,
                    encrypted_token,
                    username,
                    email,
                    created_at,
                    updated_at,
                }
            },
        ))
    }

    /// Delete a Git credential by ID with ownership check
    ///
    /// # Arguments
    /// * `id` - Credential ID to delete
    /// * `user_id` - User ID to verify ownership
    ///
    /// # Returns
    /// Ok(()) if successful
    ///
    /// # Errors
    /// Returns error if credential doesn't exist or doesn't belong to user
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::git_credential::GitCredentialsRepository;
    /// # async fn example(repo: GitCredentialsRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// repo.delete("cred-123", "user-456").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM git_credentials
            WHERE id = ? AND user_id = ?
            "#,
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete Git credential")?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "Git credential not found or unauthorized: id={}, user_id={}",
                id,
                user_id
            );
        }

        Ok(())
    }

    /// Get decrypted token from a Git credential
    ///
    /// # Arguments
    /// * `id` - Credential ID to get token from
    ///
    /// # Returns
    /// Decrypted plaintext token
    ///
    /// # Errors
    /// Returns error if credential doesn't exist or decryption fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::git_credential::GitCredentialsRepository;
    /// # async fn example(repo: GitCredentialsRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = repo.get_decrypted_token("cred-123").await?;
    /// println!("Decrypted token: {}", token);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_decrypted_token(&self, id: &str) -> Result<String> {
        let encrypted_token = sqlx::query!(
            r#"
            SELECT encrypted_token
            FROM git_credentials
            WHERE id = ?
            "#,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map(|row| row.encrypted_token)
        .context("Failed to query Git credential token")?;

        decrypt_token(&encrypted_token).context("Failed to decrypt Git token")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_user(db: &crate::database::Database) -> Result<ralph_models::User> {
        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        user_repo
            .create(ralph_models::CreateUser {
                username: "alice".to_string(),
                email: "alice@example.com".to_string(),
                password: "hashed_password".to_string(),
            })
            .await
    }

    /// Integration test: create and find Git credential
    #[tokio::test]
    async fn test_create_and_find_by_user_and_provider() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let create_credential = CreateGitCredential {
            user_id: user.id.clone(),
            provider: "github".to_string(),
            encrypted_token: "ghp_test_token_123456".to_string(),
            username: Some("alice".to_string()),
            email: Some("alice@example.com".to_string()),
        };

        let created = repo.create(create_credential.clone()).await?;

        let found = repo
            .find_by_user_and_provider(&created.user_id, &created.provider)
            .await?;

        assert!(found.is_some());
        let credential = found.unwrap();
        assert_eq!(credential.id, created.id);
        assert_eq!(credential.user_id, created.user_id);
        assert_eq!(credential.provider, created.provider);
        assert_eq!(credential.username, created.username);
        assert_eq!(credential.email, created.email);
        assert_eq!(credential.created_at, created.created_at);
        assert_eq!(credential.updated_at, created.updated_at);

        Ok(())
    }

    /// Integration test: list all credentials for user
    #[tokio::test]
    async fn test_list_by_user() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        // Create multiple credentials for same user
        repo.create(CreateGitCredential {
            user_id: user.id.clone(),
            provider: "github".to_string(),
            encrypted_token: "ghp_token_1".to_string(),
            username: Some("alice".to_string()),
            email: None,
        })
        .await?;

        repo.create(CreateGitCredential {
            user_id: user.id.clone(),
            provider: "gitlab".to_string(),
            encrypted_token: "glpat_token_2".to_string(),
            username: Some("alice".to_string()),
            email: Some("alice@example.com".to_string()),
        })
        .await?;

        // Create credential for different user
        repo.create(CreateGitCredential {
            user_id: "user-456".to_string(),
            provider: "github".to_string(),
            encrypted_token: "ghp_token_3".to_string(),
            username: None,
            email: None,
        })
        .await?;

        let credentials = repo.list_by_user(&user.id).await?;
        assert_eq!(credentials.len(), 2);
        assert!(credentials.iter().all(|c| c.user_id == user.id));

        Ok(())
    }

    /// Integration test: delete credential
    #[tokio::test]
    async fn test_delete_credential() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let credential = repo
            .create(CreateGitCredential {
                user_id: "user-123".to_string(),
                provider: "github".to_string(),
                encrypted_token: "ghp_test_token".to_string(),
                username: None,
                email: None,
            })
            .await?;

        repo.delete(&credential.id, "user-123").await?;

        let found = repo.find_by_user_and_provider("user-123", "github").await?;

        assert!(found.is_none());

        Ok(())
    }

    /// Integration test: delete credential with wrong user fails
    #[tokio::test]
    async fn test_delete_credential_unauthorized_fails() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let credential = repo
            .create(CreateGitCredential {
                user_id: "user-123".to_string(),
                provider: "github".to_string(),
                encrypted_token: "ghp_test_token".to_string(),
                username: None,
                email: None,
            })
            .await?;

        let result = repo.delete(&credential.id, "user-456").await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found or unauthorized")
        );

        Ok(())
    }

    /// Integration test: get decrypted token
    #[tokio::test]
    async fn test_get_decrypted_token() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let original_token = "ghp_test_token_1234567890abcdef".to_string();
        let credential = repo
            .create(CreateGitCredential {
                user_id: "user-123".to_string(),
                provider: "github".to_string(),
                encrypted_token: original_token.clone(),
                username: None,
                email: None,
            })
            .await?;

        let decrypted = repo.get_decrypted_token(&credential.id).await?;
        assert_eq!(original_token, decrypted);

        Ok(())
    }

    /// Integration test: non-existent credential returns None
    #[tokio::test]
    async fn test_find_non_existent_credential() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let found = repo.find_by_user_and_provider("user-999", "github").await?;

        assert!(found.is_none());

        Ok(())
    }

    /// Integration test: unique constraint on user_id + provider
    #[tokio::test]
    async fn test_unique_constraint_on_user_and_provider() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        repo.create(CreateGitCredential {
            user_id: "user-123".to_string(),
            provider: "github".to_string(),
            encrypted_token: "ghp_token_1".to_string(),
            username: None,
            email: None,
        })
        .await?;

        let duplicate = CreateGitCredential {
            user_id: "user-123".to_string(),
            provider: "github".to_string(),
            encrypted_token: "ghp_token_2".to_string(),
            username: None,
            email: None,
        };

        let result = repo.create(duplicate).await;
        assert!(result.is_err());

        Ok(())
    }

    /// Integration test: same provider different user works
    #[tokio::test]
    async fn test_same_provider_different_user() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        repo.create(CreateGitCredential {
            user_id: "user-123".to_string(),
            provider: "github".to_string(),
            encrypted_token: "ghp_token_1".to_string(),
            username: None,
            email: None,
        })
        .await?;

        repo.create(CreateGitCredential {
            user_id: "user-456".to_string(),
            provider: "github".to_string(),
            encrypted_token: "ghp_token_2".to_string(),
            username: None,
            email: None,
        })
        .await?;

        let user1_creds = repo.list_by_user("user-123").await?;
        let user2_creds = repo.list_by_user("user-456").await?;

        assert_eq!(user1_creds.len(), 1);
        assert_eq!(user2_creds.len(), 1);

        Ok(())
    }

    /// Integration test: credentials ordered by created_at DESC
    #[tokio::test]
    async fn test_credentials_ordered_by_created_at() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = GitCredentialsRepository::new(db.pool().clone());

        let user_id = "user-123".to_string();

        let cred1 = repo
            .create(CreateGitCredential {
                user_id: user_id.clone(),
                provider: "github".to_string(),
                encrypted_token: "ghp_token_1".to_string(),
                username: None,
                email: None,
            })
            .await?;

        // Wait a tiny bit to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cred2 = repo
            .create(CreateGitCredential {
                user_id: user_id.clone(),
                provider: "gitlab".to_string(),
                encrypted_token: "glpat_token_2".to_string(),
                username: None,
                email: None,
            })
            .await?;

        let credentials = repo.list_by_user(&user_id).await?;
        assert_eq!(credentials.len(), 2);

        // First credential should be newer (later created_at)
        assert_eq!(credentials[0].id, cred2.id);
        assert_eq!(credentials[1].id, cred1.id);

        Ok(())
    }
}
