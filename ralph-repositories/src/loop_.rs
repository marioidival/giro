use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::{CreateLoop, Loop, LoopStatus};
use sqlx::{FromRow, Pool, Sqlite};
use std::str::FromStr;

#[derive(Debug, FromRow)]
struct LoopRow {
    id: String,
    name: String,
    description: Option<String>,
    prd: String,
    owner_id: String,
    provider: String,
    model: String,
    docker_image: String,
    cpu_limit: i32,
    memory_limit: i32,
    max_iterations: i32,
    iteration_timeout: i32,
    iteration_delay: i32,
    git_repo_url: Option<String>,
    git_branch_pattern: String,
    status: String,
    current_iteration: i32,
    container_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LoopRow> for Loop {
    fn from(row: LoopRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            prd: row.prd,
            owner_id: row.owner_id,
            provider: row.provider,
            model: row.model,
            docker_image: row.docker_image,
            cpu_limit: row.cpu_limit,
            memory_limit: row.memory_limit,
            max_iterations: row.max_iterations,
            iteration_timeout: row.iteration_timeout,
            iteration_delay: row.iteration_delay,
            status: LoopStatus::from_str(&row.status).unwrap_or(LoopStatus::Error),
            current_iteration: row.current_iteration,
            container_id: row.container_id,
            git_repo_url: row.git_repo_url,
            git_branch_pattern: row.git_branch_pattern,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct LoopSummaryRow {
    id: String,
    name: String,
    description: Option<String>,
    owner_id: String,
    provider: String,
    model: String,
    docker_image: String,
    cpu_limit: i32,
    memory_limit: i32,
    max_iterations: i32,
    iteration_timeout: i32,
    iteration_delay: i32,
    git_repo_url: Option<String>,
    git_branch_pattern: String,
    status: String,
    current_iteration: i32,
    container_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LoopSummaryRow> for Loop {
    fn from(row: LoopSummaryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            prd: String::new(),
            owner_id: row.owner_id,
            provider: row.provider,
            model: row.model,
            docker_image: row.docker_image,
            cpu_limit: row.cpu_limit,
            memory_limit: row.memory_limit,
            max_iterations: row.max_iterations,
            iteration_timeout: row.iteration_timeout,
            iteration_delay: row.iteration_delay,
            status: LoopStatus::from_str(&row.status).unwrap_or(LoopStatus::Error),
            current_iteration: row.current_iteration,
            container_id: row.container_id,
            git_repo_url: row.git_repo_url,
            git_branch_pattern: row.git_branch_pattern,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Repository for Loop database operations
#[derive(Clone, Debug)]
pub struct LoopRepository {
    pool: Pool<Sqlite>,
}

impl LoopRepository {
    /// Create a new LoopRepository instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: SqlitePool) {
    /// let loop_repo = LoopRepository::new(pool);
    /// # }
    /// ```
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new loop in database
    ///
    /// # Arguments
    /// * `create_loop` - Loop creation data
    ///
    /// # Returns
    /// The created Loop with generated id and timestamps
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # use ralph_models::CreateLoop;
    /// # async fn example(repo: LoopRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let create_loop = CreateLoop {
    ///     name: "Test Loop".to_string(),
    ///     description: None,
    ///     prd: "Test PRD".to_string(),
    ///     owner_id: "user123".to_string(),
    ///     provider: "claude".to_string(),
    ///     model: "claude-3-opus".to_string(),
    ///     docker_image: None,
    ///     cpu_limit: None,
    ///     memory_limit: None,
    ///     max_iterations: None,
    ///     iteration_timeout: None,
    ///     iteration_delay: None,
    ///     git_repo_url: None,
    ///     git_branch_pattern: None,
    /// };
    /// let loop_ = repo.create(create_loop).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, create_loop: CreateLoop) -> Result<Loop> {
        let loop_ = Loop::new(create_loop);

        sqlx::query(
            r#"
            INSERT INTO loops (
                id, name, description, prd, owner_id, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations,
                iteration_timeout, iteration_delay, git_repo_url, git_branch_pattern,
                status, current_iteration, container_id, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&loop_.id)
        .bind(&loop_.name)
        .bind(&loop_.description)
        .bind(&loop_.prd)
        .bind(&loop_.owner_id)
        .bind(&loop_.provider)
        .bind(&loop_.model)
        .bind(&loop_.docker_image)
        .bind(loop_.cpu_limit)
        .bind(loop_.memory_limit)
        .bind(loop_.max_iterations)
        .bind(loop_.iteration_timeout)
        .bind(loop_.iteration_delay)
        .bind(&loop_.git_repo_url)
        .bind(&loop_.git_branch_pattern)
        .bind(loop_.status.to_string())
        .bind(loop_.current_iteration)
        .bind(&loop_.container_id)
        .bind(loop_.created_at)
        .bind(loop_.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert loop into database")?;

        Ok(loop_)
    }

    /// Find a loop by ID with full PRD
    ///
    /// # Arguments
    /// * `id` - Loop ID to search for
    ///
    /// # Returns
    /// Some(Loop) if found with PRD, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # async fn example(repo: LoopRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let loop_ = repo.find_by_id("loop-id-123").await?;
    /// if let Some(loop_) = loop_ {
    ///     println!("Found loop: {} with PRD: {}", loop_.name, loop_.prd);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Loop>> {
        let result = sqlx::query_as::<_, LoopRow>(
            r#"
            SELECT
                id, name, description, prd, owner_id, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations,
                iteration_timeout, iteration_delay, git_repo_url, git_branch_pattern,
                status, current_iteration, container_id, created_at, updated_at
            FROM loops
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query loop by id")?;

        Ok(result.map(|row| row.into()))
    }

    /// List all loops for a given owner without PRD (summary)
    ///
    /// # Arguments
    /// * `owner_id` - Owner user ID to filter by
    ///
    /// # Returns
    /// Vector of Loops without PRD (summary)
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # async fn example(repo: LoopRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let loops = repo.list_by_owner("user123").await?;
    /// for loop_ in loops {
    ///     println!("Loop: {} (PRD omitted for list query)", loop_.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_by_owner(&self, owner_id: &str) -> Result<Vec<Loop>> {
        let results = sqlx::query_as::<_, LoopSummaryRow>(
            r#"
            SELECT
                id, name, description, owner_id, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations,
                iteration_timeout, iteration_delay, git_repo_url, git_branch_pattern,
                status, current_iteration, container_id, created_at, updated_at
            FROM loops
            WHERE owner_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query loops by owner")?;

        Ok(results.into_iter().map(|row| row.into()).collect())
    }

    /// Update loop status and container ID
    ///
    /// # Arguments
    /// * `id` - Loop ID to update
    /// * `status` - New status
    /// * `container_id` - Optional new container ID
    ///
    /// # Returns
    /// Number of rows affected (1 if updated, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # use ralph_models::LoopStatus;
    /// # async fn example(repo: LoopRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.update_status("loop-id-123", LoopStatus::Running, Some("container-456".to_string())).await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_status(
        &self,
        id: &str,
        status: LoopStatus,
        container_id: Option<String>,
    ) -> Result<u64> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE loops
            SET status = ?, container_id = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(&container_id)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update loop status")?;

        Ok(result.rows_affected())
    }

    /// Increment the current iteration counter for a loop
    ///
    /// # Arguments
    /// * `id` - Loop ID to increment iteration counter for
    ///
    /// # Returns
    /// Number of rows affected (1 if updated, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # async fn example(repo: LoopRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.increment_iteration("loop-id-123").await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn increment_iteration(&self, id: &str) -> Result<u64> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE loops
            SET current_iteration = current_iteration + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to increment loop iteration")?;

        Ok(result.rows_affected())
    }

    /// Delete a loop by ID
    ///
    /// # Arguments
    /// * `id` - Loop ID to delete
    ///
    /// # Returns
    /// Number of rows affected (1 if deleted, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # async fn example(repo: LoopRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.delete("loop-id-123").await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(r#"DELETE FROM loops WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete loop")?;

        Ok(result.rows_affected())
    }

    /// Get a reference to the underlying SqlitePool
    ///
    /// # Returns
    /// Reference to the SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::loop_::LoopRepository;
    /// # async fn example(repo: LoopRepository) {
    /// let pool = repo.pool();
    /// // Use pool for queries
    /// # }
    /// ```
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: create loop, find by id (with prd)
    #[tokio::test]
    async fn test_create_and_find_by_id() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        // Create loop
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: Some("Test description".to_string()),
            prd: "Test PRD content".to_string(),
            owner_id: user.id.clone(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
            git_repo_url: None,
            git_branch_pattern: None,
        };

        let created_loop = repo.create(create_loop).await?;

        let found_loop = repo.find_by_id(&created_loop.id).await?;
        assert!(found_loop.is_some());
        let loop_ = found_loop.unwrap();
        assert_eq!(loop_.id, created_loop.id);
        assert_eq!(loop_.name, "Test Loop");
        assert_eq!(loop_.description, Some("Test description".to_string()));
        assert_eq!(loop_.prd, "Test PRD content");
        assert_eq!(loop_.owner_id, user.id);
        assert_eq!(loop_.status, LoopStatus::Created);

        Ok(())
    }

    /// Integration test: create loops, list by owner (without prd)
    #[tokio::test]
    async fn test_create_and_list_by_owner() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        for i in 1..=3 {
            let create_loop = CreateLoop {
                name: format!("Loop {}", i),
                description: Some(format!("Description {}", i)),
                prd: format!("PRD content {}", i),
                owner_id: user.id.clone(),
                provider: "claude".to_string(),
                model: "claude-3-opus".to_string(),
                docker_image: None,
                cpu_limit: None,
                memory_limit: None,
                max_iterations: None,
                iteration_timeout: None,
                iteration_delay: None,
                git_repo_url: None,
                git_branch_pattern: None,
            };
            repo.create(create_loop).await?;
        }

        let loops = repo.list_by_owner(&user.id).await?;
        assert_eq!(loops.len(), 3);

        for loop_ in loops {
            assert!(loop_.prd.is_empty(), "PRD should be empty in list queries");
            assert_eq!(loop_.owner_id, user.id);
        }

        Ok(())
    }

    /// Integration test: update status
    #[tokio::test]
    async fn test_update_status() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: user.id.clone(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
            git_repo_url: None,
            git_branch_pattern: None,
        };

        let loop_ = repo.create(create_loop).await?;
        assert_eq!(loop_.status, LoopStatus::Created);

        let rows = repo
            .update_status(
                &loop_.id,
                LoopStatus::Running,
                Some("container-123".to_string()),
            )
            .await?;
        assert_eq!(rows, 1);

        let updated_loop = repo.find_by_id(&loop_.id).await?.unwrap();
        assert_eq!(updated_loop.status, LoopStatus::Running);
        assert_eq!(updated_loop.container_id, Some("container-123".to_string()));
        assert!(updated_loop.updated_at > loop_.updated_at);

        Ok(())
    }

    /// Integration test: delete loop
    #[tokio::test]
    async fn test_delete_loop() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: user.id.clone(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
            git_repo_url: None,
            git_branch_pattern: None,
        };

        let loop_ = repo.create(create_loop).await?;

        let found = repo.find_by_id(&loop_.id).await?;
        assert!(found.is_some());

        let rows = repo.delete(&loop_.id).await?;
        assert_eq!(rows, 1);

        let found = repo.find_by_id(&loop_.id).await?;
        assert!(found.is_none());

        Ok(())
    }

    /// Integration test: update non-existent loop
    #[tokio::test]
    async fn test_update_non_existent_loop() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let rows = repo
            .update_status("non-existent-id", LoopStatus::Running, None)
            .await?;
        assert_eq!(rows, 0);

        Ok(())
    }

    /// Integration test: delete non-existent loop
    #[tokio::test]
    async fn test_delete_non_existent_loop() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let rows = repo.delete("non-existent-id").await?;
        assert_eq!(rows, 0);

        Ok(())
    }

    /// Integration test: list by owner with no loops
    #[tokio::test]
    async fn test_list_by_owner_no_loops() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopRepository::new(db.pool().clone());

        let loops = repo.list_by_owner("non-existent-user").await?;
        assert!(loops.is_empty());

        Ok(())
    }
}
