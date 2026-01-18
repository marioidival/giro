use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::{ApiKey, ApiKeyProvider, CreateApiKey};
use sqlx::{Pool, Sqlite};

use super::crypto::{decrypt_token, encrypt_token};

/// Repository for API key database operations
#[derive(Clone, Debug)]
pub struct ApiKeyRepository {
    pool: Pool<Sqlite>,
}

impl ApiKeyRepository {
    /// Create a new ApiKeyRepository instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: SqlitePool) {
    /// let repo = ApiKeyRepository::new(pool);
    /// # }
    /// ```
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new API key in database
    ///
    /// # Arguments
    /// * `create_key` - API key creation data (key will be encrypted)
    ///
    /// # Returns
    /// The created ApiKey with generated id and timestamps
    ///
    /// # Errors
    /// Returns error if encryption fails or database insertion fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # use ralph_models::{CreateApiKey, ApiKeyProvider};
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let create_key = CreateApiKey {
    ///     user_id: "user-123".to_string(),
    ///     provider: ApiKeyProvider::Anthropic,
    ///     key: "sk-ant-api03-1234567890".to_string(),  // Will be encrypted
    ///     is_active: true,
    /// };
    /// let key = repo.create(create_key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, create_key: CreateApiKey) -> Result<ApiKey> {
        // Encrypt the API key before storing
        let encrypted_key = encrypt_token(&create_key.key)
            .context("Failed to encrypt API key")?;

        let key = ApiKey::new(
            create_key.user_id,
            create_key.provider.clone(),
            encrypted_key,
            create_key.is_active,
        );

        let provider_str = key.provider.as_str();
        sqlx::query!(
            r#"
            INSERT INTO api_keys (id, user_id, provider, encrypted_key, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            key.id,
            key.user_id,
            provider_str,
            key.encrypted_key,
            key.is_active,
            key.created_at,
            key.updated_at,
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert API key into database")?;

        Ok(key)
    }

    /// List all API keys for a user (with encrypted keys)
    ///
    /// # Arguments
    /// * `user_id` - User ID to list keys for
    ///
    /// # Returns
    /// Vector of all API keys for the user (keys remain encrypted)
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let keys = repo.list_by_user("user-123").await?;
    /// println!("Found {} API keys", keys.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                bool,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, user_id, provider, encrypted_key, is_active, created_at, updated_at
            FROM api_keys
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query API keys by user")?;

        Ok(rows
            .into_iter()
            .map(
                |(id, user_id, provider, encrypted_key, is_active, created_at, updated_at)| {
                    ApiKey {
                        id,
                        user_id,
                        provider: provider.parse().unwrap_or(ApiKeyProvider::Anthropic),
                        encrypted_key,
                        is_active,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }

    /// Find an active API key by user ID and provider
    ///
    /// # Arguments
    /// * `user_id` - User ID to search for
    /// * `provider` - LLM provider (anthropic, openai, amp)
    ///
    /// # Returns
    /// Some(ApiKey) if found and active, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # use ralph_models::ApiKeyProvider;
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let key = repo.get_active_for_user("user-123", ApiKeyProvider::Anthropic).await?;
    /// if let Some(k) = key {
    ///     println!("Found active Anthropic API key for user");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_active_for_user(
        &self,
        user_id: &str,
        provider: ApiKeyProvider,
    ) -> Result<Option<ApiKey>> {
        let result = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                bool,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, user_id, provider, encrypted_key, is_active, created_at, updated_at
            FROM api_keys
            WHERE user_id = ? AND provider = ? AND is_active = 1
            "#,
        )
        .bind(user_id)
        .bind(provider.as_str())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query active API key by user and provider")?;

        Ok(result.map(
            |(id, user_id, provider, encrypted_key, is_active, created_at, updated_at)| {
                ApiKey {
                    id,
                    user_id,
                    provider: provider.parse().unwrap_or(ApiKeyProvider::Anthropic),
                    encrypted_key,
                    is_active,
                    created_at,
                    updated_at,
                }
            },
        ))
    }

    /// Deactivate an API key by ID with ownership check
    ///
    /// # Arguments
    /// * `id` - API key ID to deactivate
    /// * `user_id` - User ID to verify ownership
    ///
    /// # Returns
    /// Ok(()) if successful
    ///
    /// # Errors
    /// Returns error if key doesn't exist or doesn't belong to user
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// repo.deactivate("key-123", "user-456").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn deactivate(&self, id: &str, user_id: &str) -> Result<()> {
        let now = Utc::now();
        let result = sqlx::query!(
            r#"
            UPDATE api_keys
            SET is_active = 0, updated_at = ?
            WHERE id = ? AND user_id = ?
            "#,
            now,
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .context("Failed to deactivate API key")?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "API key not found or unauthorized: id={}, user_id={}",
                id,
                user_id
            );
        }

        Ok(())
    }

    /// Get decrypted API key by ID
    ///
    /// # Arguments
    /// * `id` - API key ID to get decrypted key from
    ///
    /// # Returns
    /// Decrypted plaintext API key
    ///
    /// # Errors
    /// Returns error if key doesn't exist or decryption fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let key = repo.get_decrypted_key("key-123").await?;
    /// println!("Decrypted API key: {}", key);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_decrypted_key(&self, id: &str) -> Result<String> {
        let encrypted_key = sqlx::query!(
            r#"
            SELECT encrypted_key
            FROM api_keys
            WHERE id = ?
            "#,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map(|row| row.encrypted_key)
        .context("Failed to query API key")?;

        decrypt_token(&encrypted_key).context("Failed to decrypt API key")
    }

    /// Delete an API key by ID with ownership check
    ///
    /// # Arguments
    /// * `id` - API key ID to delete
    /// * `user_id` - User ID to verify ownership
    ///
    /// # Returns
    /// Ok(()) if successful
    ///
    /// # Errors
    /// Returns error if key doesn't exist or doesn't belong to user
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// repo.delete("key-123", "user-456").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM api_keys
            WHERE id = ? AND user_id = ?
            "#,
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete API key")?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "API key not found or unauthorized: id={}, user_id={}",
                id,
                user_id
            );
        }

        Ok(())
    }

    /// Reactivate a deactivated API key by ID with ownership check
    ///
    /// # Arguments
    /// * `id` - API key ID to reactivate
    /// * `user_id` - User ID to verify ownership
    ///
    /// # Returns
    /// Ok(()) if successful
    ///
    /// # Errors
    /// Returns error if key doesn't exist or doesn't belong to user
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::api_key::ApiKeyRepository;
    /// # async fn example(repo: ApiKeyRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// repo.reactivate("key-123", "user-456").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reactivate(&self, id: &str, user_id: &str) -> Result<()> {
        let now = Utc::now();
        let result = sqlx::query!(
            r#"
            UPDATE api_keys
            SET is_active = 1, updated_at = ?
            WHERE id = ? AND user_id = ?
            "#,
            now,
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .context("Failed to reactivate API key")?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "API key not found or unauthorized: id={}, user_id={}",
                id,
                user_id
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    async fn create_test_user(db: &crate::database::Database) -> Result<ralph_models::User> {
        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let suffix = Uuid::new_v4();
        user_repo
            .create(ralph_models::CreateUser {
                username: format!("alice_{}", suffix),
                email: format!("alice_{}@example.com", suffix),
                password: "hashed_password".to_string(),
            })
            .await
    }

    /// Integration test: create and find active API key
    #[tokio::test]
    async fn test_create_and_get_active_for_user() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let create_key = CreateApiKey {
            user_id: user.id.clone(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-api03-1234567890".to_string(),
            is_active: true,
        };

        let created = repo.create(create_key.clone()).await?;

        let found = repo
            .get_active_for_user(&created.user_id, created.provider.clone())
            .await?;

        assert!(found.is_some());
        let key = found.unwrap();
        assert_eq!(key.id, created.id);
        assert_eq!(key.user_id, created.user_id);
        assert_eq!(key.provider, created.provider);
        assert_eq!(key.is_active, created.is_active);
        assert_eq!(key.created_at, created.created_at);
        assert_eq!(key.updated_at, created.updated_at);

        Ok(())
    }

    /// Integration test: list all API keys for user
    #[tokio::test]
    async fn test_list_by_user() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;
        let user2 = create_test_user(&db).await?;

        // Create multiple keys for same user
        repo.create(CreateApiKey {
            user_id: user.id.clone(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-key-1".to_string(),
            is_active: true,
        })
        .await?;

        repo.create(CreateApiKey {
            user_id: user.id.clone(),
            provider: ApiKeyProvider::OpenAI,
            key: "sk-openai-key-2".to_string(),
            is_active: true,
        })
        .await?;

        // Create key for different user
        repo.create(CreateApiKey {
            user_id: user2.id.clone(),
            provider: ApiKeyProvider::Amp,
            key: "amp-key-3".to_string(),
            is_active: true,
        })
        .await?;

        let keys = repo.list_by_user(&user.id).await?;
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|k| k.user_id == user.id));

        Ok(())
    }

    /// Integration test: deactivate API key
    #[tokio::test]
    async fn test_deactivate_key() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let key = repo
            .create(CreateApiKey {
                user_id: user.id.clone(),
                provider: ApiKeyProvider::Anthropic,
                key: "sk-ant-test-key".to_string(),
                is_active: true,
            })
            .await?;

        repo.deactivate(&key.id, &user.id).await?;

        let found = repo.get_active_for_user(&user.id, ApiKeyProvider::Anthropic).await?;

        assert!(found.is_none());

        Ok(())
    }

    /// Integration test: deactivate with wrong user fails
    #[tokio::test]
    async fn test_deactivate_key_unauthorized_fails() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user1 = create_test_user(&db).await?;
        let user2 = create_test_user(&db).await?;

        let key = repo
            .create(CreateApiKey {
                user_id: user1.id.clone(),
                provider: ApiKeyProvider::Anthropic,
                key: "sk-ant-test-key".to_string(),
                is_active: true,
            })
            .await?;

        let result = repo.deactivate(&key.id, &user2.id).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found or unauthorized")
        );

        Ok(())
    }

    /// Integration test: get decrypted key
    #[tokio::test]
    async fn test_get_decrypted_key() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let original_key = "sk-ant-api03-1234567890abcdef".to_string();
        let key = repo
            .create(CreateApiKey {
                user_id: user.id.clone(),
                provider: ApiKeyProvider::Anthropic,
                key: original_key.clone(),
                is_active: true,
            })
            .await?;

        let decrypted = repo.get_decrypted_key(&key.id).await?;
        assert_eq!(original_key, decrypted);

        Ok(())
    }

    /// Integration test: delete API key
    #[tokio::test]
    async fn test_delete_key() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let key = repo
            .create(CreateApiKey {
                user_id: user.id.clone(),
                provider: ApiKeyProvider::Anthropic,
                key: "sk-ant-test-key".to_string(),
                is_active: true,
            })
            .await?;

        repo.delete(&key.id, &user.id).await?;

        let found = repo.get_active_for_user(&user.id, ApiKeyProvider::Anthropic).await?;

        assert!(found.is_none());

        Ok(())
    }

    /// Integration test: reactivate deactivated key
    #[tokio::test]
    async fn test_reactivate_key() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let key = repo
            .create(CreateApiKey {
                user_id: user.id.clone(),
                provider: ApiKeyProvider::Anthropic,
                key: "sk-ant-test-key".to_string(),
                is_active: true,
            })
            .await?;

        // Deactivate the key
        repo.deactivate(&key.id, &user.id).await?;

        let found = repo
            .get_active_for_user(&user.id, ApiKeyProvider::Anthropic)
            .await?;
        assert!(found.is_none());

        // Reactivate the key
        repo.reactivate(&key.id, &user.id).await?;

        let found = repo
            .get_active_for_user(&user.id, ApiKeyProvider::Anthropic)
            .await?;
        assert!(found.is_some());

        Ok(())
    }

    /// Integration test: non-existent key returns None
    #[tokio::test]
    async fn test_find_non_existent_key() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let found = repo
            .get_active_for_user("user-999", ApiKeyProvider::Anthropic)
            .await?;

        assert!(found.is_none());

        Ok(())
    }

    /// Integration test: unique constraint on user_id + provider
    #[tokio::test]
    async fn test_unique_constraint_on_user_and_provider() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        repo.create(CreateApiKey {
            user_id: user.id.clone(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-key-1".to_string(),
            is_active: true,
        })
        .await?;

        let duplicate = CreateApiKey {
            user_id: user.id.clone(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-key-2".to_string(),
            is_active: true,
        };

        let result = repo.create(duplicate).await;
        assert!(result.is_err());

        Ok(())
    }

    /// Integration test: same provider different user works
    #[tokio::test]
    async fn test_same_provider_different_user() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user1 = create_test_user(&db).await?;
        let user2 = create_test_user(&db).await?;

        repo.create(CreateApiKey {
            user_id: user1.id.clone(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-key-1".to_string(),
            is_active: true,
        })
        .await?;

        repo.create(CreateApiKey {
            user_id: user2.id.clone(),
            provider: ApiKeyProvider::Anthropic,
            key: "sk-ant-key-2".to_string(),
            is_active: true,
        })
        .await?;

        let user1_keys = repo.list_by_user(&user1.id).await?;
        let user2_keys = repo.list_by_user(&user2.id).await?;

        assert_eq!(user1_keys.len(), 1);
        assert_eq!(user2_keys.len(), 1);

        Ok(())
    }

    /// Integration test: keys ordered by created_at DESC
    #[tokio::test]
    async fn test_keys_ordered_by_created_at() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = ApiKeyRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let key1 = repo
            .create(CreateApiKey {
                user_id: user.id.clone(),
                provider: ApiKeyProvider::Anthropic,
                key: "sk-ant-key-1".to_string(),
                is_active: true,
            })
            .await?;

        // Wait a tiny bit to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let key2 = repo
            .create(CreateApiKey {
                user_id: user.id.clone(),
                provider: ApiKeyProvider::OpenAI,
                key: "sk-openai-key-2".to_string(),
                is_active: true,
            })
            .await?;

        let keys = repo.list_by_user(&user.id).await?;
        assert_eq!(keys.len(), 2);

        // First key should be newer (later created_at)
        assert_eq!(keys[0].id, key2.id);
        assert_eq!(keys[1].id, key1.id);

        Ok(())
    }
}
