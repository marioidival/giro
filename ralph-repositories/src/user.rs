use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::{CreateUser, User};
use sqlx::{Pool, Sqlite};

/// Repository for User database operations
#[derive(Clone, Debug)]
pub struct UserRepository {
    pool: Pool<Sqlite>,
}

impl UserRepository {
    /// Create a new UserRepository instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::user::UserRepository;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: SqlitePool) {
    /// let user_repo = UserRepository::new(pool);
    /// # }
    /// ```
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new user in the database
    ///
    /// # Arguments
    /// * `create_user` - User creation data
    ///
    /// # Returns
    /// The created User with generated id and created_at timestamp
    ///
    /// # Errors
    /// Returns error if username or email already exists
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::user::UserRepository;
    /// # use ralph_models::CreateUser;
    /// # async fn example(repo: UserRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let create_user = CreateUser {
    ///     username: "alice".to_string(),
    ///     email: "alice@example.com".to_string(),
    ///     password: "hashed_password".to_string(),
    /// };
    /// let user = repo.create(create_user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, create_user: CreateUser) -> Result<User> {
        let user = User::new(
            create_user.username,
            create_user.email,
            create_user.password,
        );

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert user into database")?;

        Ok(user)
    }

    /// Find a user by username
    ///
    /// # Arguments
    /// * `username` - Username to search for
    ///
    /// # Returns
    /// Some(User) if found, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::user::UserRepository;
    /// # async fn example(repo: UserRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let user = repo.find_by_username("alice").await?;
    /// if let Some(user) = user {
    ///     println!("Found user: {}", user.username);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let result = sqlx::query_as::<_, (String, String, String, String, DateTime<Utc>)>(
            r#"
            SELECT id, username, email, password_hash, created_at
            FROM users
            WHERE username = ?
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query user by username")?;

        Ok(
            result.map(|(id, username, email, password_hash, created_at)| User {
                id,
                username,
                email,
                password_hash,
                created_at,
            }),
        )
    }

    /// Find a user by ID
    ///
    /// # Arguments
    /// * `id` - User ID to search for
    ///
    /// # Returns
    /// Some(User) if found, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::user::UserRepository;
    /// # async fn example(repo: UserRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let user = repo.find_by_id("user-id-123").await?;
    /// if let Some(user) = user {
    ///     println!("Found user: {}", user.username);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_id(&self, id: &str) -> Result<Option<User>> {
        let result = sqlx::query_as::<_, (String, String, String, String, DateTime<Utc>)>(
            r#"
            SELECT id, username, email, password_hash, created_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query user by id")?;

        Ok(
            result.map(|(id, username, email, password_hash, created_at)| User {
                id,
                username,
                email,
                password_hash,
                created_at,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: create user, find by username
    #[tokio::test]
    async fn test_create_and_find_by_username() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = UserRepository::new(db.pool().clone());

        let create_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };

        let created_user = repo.create(create_user).await?;

        let found_user = repo.find_by_username("testuser").await?;
        assert!(found_user.is_some());
        let user = found_user.unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hashed_password");
        assert_eq!(user.id, created_user.id);

        Ok(())
    }

    /// Integration test: create user, find by id
    #[tokio::test]
    async fn test_create_and_find_by_id() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = UserRepository::new(db.pool().clone());

        let create_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };

        let created_user = repo.create(create_user).await?;

        let found_user = repo.find_by_id(&created_user.id).await?;
        assert!(found_user.is_some());
        let user = found_user.unwrap();
        assert_eq!(user.id, created_user.id);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");

        Ok(())
    }

    /// Integration test: find non-existent user returns None
    #[tokio::test]
    async fn test_find_non_existent_user() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = UserRepository::new(db.pool().clone());

        let user = repo.find_by_username("nonexistent").await?;
        assert!(user.is_none());

        let user = repo.find_by_id("nonexistent-id").await?;
        assert!(user.is_none());

        Ok(())
    }

    /// Integration test: duplicate username returns error
    #[tokio::test]
    async fn test_duplicate_username_fails() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = UserRepository::new(db.pool().clone());

        let create_user = CreateUser {
            username: "testuser".to_string(),
            email: "test1@example.com".to_string(),
            password: "hashed_password".to_string(),
        };

        repo.create(create_user).await?;

        let duplicate_user = CreateUser {
            username: "testuser".to_string(),
            email: "test2@example.com".to_string(),
            password: "another_hash".to_string(),
        };

        let result = repo.create(duplicate_user).await;
        assert!(result.is_err());

        Ok(())
    }

    /// Integration test: duplicate email returns error
    #[tokio::test]
    async fn test_duplicate_email_fails() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = UserRepository::new(db.pool().clone());

        let create_user = CreateUser {
            username: "testuser1".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };

        repo.create(create_user).await?;

        let duplicate_user = CreateUser {
            username: "testuser2".to_string(),
            email: "test@example.com".to_string(),
            password: "another_hash".to_string(),
        };

        let result = repo.create(duplicate_user).await;
        assert!(result.is_err());

        Ok(())
    }
}
