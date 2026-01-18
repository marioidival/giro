use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::Iteration;
use sqlx::{FromRow, Pool, Sqlite};
use std::str::FromStr;

#[derive(Debug, FromRow)]
struct IterationRow {
    id: String,
    loop_id: String,
    task_id: String,
    iteration_number: i64,
    output: Option<String>,
    error: Option<String>,
    status: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    tokens_used: Option<i64>,
}

impl From<IterationRow> for Iteration {
    fn from(row: IterationRow) -> Self {
        Self {
            id: row.id,
            loop_id: row.loop_id,
            task_id: row.task_id,
            iteration_number: row.iteration_number as i32,
            output: row.output,
            error: row.error,
            status: ralph_models::IterationStatus::from_str(&row.status)
                .unwrap_or(ralph_models::IterationStatus::Error),
            started_at: row.started_at,
            completed_at: row.completed_at,
            tokens_used: row.tokens_used.map(|t| t as i32),
        }
    }
}

/// Repository for Iteration database operations
#[derive(Clone, Debug)]
pub struct IterationRepository {
    pool: Pool<Sqlite>,
}

impl IterationRepository {
    /// Create a new IterationRepository instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: SqlitePool) {
    /// let iteration_repo = IterationRepository::new(pool);
    /// # }
    /// ```
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new iteration in database
    ///
    /// # Arguments
    /// * `iteration` - Iteration to create
    ///
    /// # Returns
    /// The created Iteration
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # use ralph_models::Iteration;
    /// # async fn example(repo: IterationRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let iteration = Iteration::new("loop_id".to_string(), "task_id".to_string(), 1);
    /// let created = repo.create(iteration).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, iteration: Iteration) -> Result<Iteration> {
        sqlx::query(
            r#"
            INSERT INTO iterations (
                id, loop_id, task_id, iteration_number, output, error,
                status, started_at, completed_at, tokens_used
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&iteration.id)
        .bind(&iteration.loop_id)
        .bind(&iteration.task_id)
        .bind(iteration.iteration_number)
        .bind(&iteration.output)
        .bind(&iteration.error)
        .bind(iteration.status.to_string())
        .bind(iteration.started_at)
        .bind(iteration.completed_at)
        .bind(iteration.tokens_used.map(|t| t as i64))
        .execute(&self.pool)
        .await
        .context("Failed to insert iteration into database")?;

        Ok(iteration)
    }

    /// Find an iteration by ID
    ///
    /// # Arguments
    /// * `id` - Iteration ID to search for
    ///
    /// # Returns
    /// Some(Iteration) if found, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # async fn example(repo: IterationRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let iteration = repo.find_by_id("iteration-id-123").await?;
    /// if let Some(iteration) = iteration {
    ///     println!("Found iteration: {}", iteration.iteration_number);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Iteration>> {
        let result = sqlx::query_as::<_, IterationRow>(
            r#"
            SELECT
                id, loop_id, task_id, iteration_number, output, error,
                status, started_at, completed_at, tokens_used
            FROM iterations
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query iteration by id")?;

        Ok(result.map(|row| row.into()))
    }

    /// List all iterations for a given loop
    ///
    /// # Arguments
    /// * `loop_id` - Loop ID to filter by
    ///
    /// # Returns
    /// Vector of Iterations ordered by iteration_number
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # async fn example(repo: IterationRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let iterations = repo.list_by_loop("loop123").await?;
    /// for iteration in iterations {
    ///     println!("Iteration {}: {:?}", iteration.iteration_number, iteration.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_by_loop(&self, loop_id: &str) -> Result<Vec<Iteration>> {
        let results = sqlx::query_as::<_, IterationRow>(
            r#"
            SELECT
                id, loop_id, task_id, iteration_number, output, error,
                status, started_at, completed_at, tokens_used
            FROM iterations
            WHERE loop_id = ?
            ORDER BY iteration_number ASC
            "#,
        )
        .bind(loop_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query iterations by loop")?;

        Ok(results.into_iter().map(|row| row.into()).collect())
    }

    /// List all iterations for a given task
    ///
    /// # Arguments
    /// * `task_id` - Task ID to filter by
    ///
    /// # Returns
    /// Vector of Iterations ordered by iteration_number
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # async fn example(repo: IterationRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let iterations = repo.list_by_task("task123").await?;
    /// for iteration in iterations {
    ///     println!("Iteration {}: {:?}", iteration.iteration_number, iteration.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<Iteration>> {
        let results = sqlx::query_as::<_, IterationRow>(
            r#"
            SELECT
                id, loop_id, task_id, iteration_number, output, error,
                status, started_at, completed_at, tokens_used
            FROM iterations
            WHERE task_id = ?
            ORDER BY iteration_number ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query iterations by task")?;

        Ok(results.into_iter().map(|row| row.into()).collect())
    }

    /// Update iteration status with optional completed_at and tokens_used
    ///
    /// # Arguments
    /// * `id` - Iteration ID to update
    /// * `status` - New status
    /// * `completed_at` - Optional completed_at timestamp
    /// * `tokens_used` - Optional tokens used count
    ///
    /// # Returns
    /// Number of rows affected (1 if updated, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # use ralph_models::IterationStatus;
    /// # async fn example(repo: IterationRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.update_status(
    ///     "iteration-id-123",
    ///     IterationStatus::Completed,
    ///     Some(chrono::Utc::now()),
    ///     Some(1500)
    /// ).await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_status(
        &self,
        id: &str,
        status: ralph_models::IterationStatus,
        completed_at: Option<DateTime<Utc>>,
        tokens_used: Option<i32>,
    ) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE iterations
            SET status = ?, completed_at = ?, tokens_used = ?
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(completed_at)
        .bind(tokens_used.map(|t| t as i64))
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update iteration status")?;

        Ok(result.rows_affected())
    }

    /// Delete an iteration by ID
    ///
    /// # Arguments
    /// * `id` - Iteration ID to delete
    ///
    /// # Returns
    /// Number of rows affected (1 if deleted, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::iteration::IterationRepository;
    /// # async fn example(repo: IterationRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.delete("iteration-id-123").await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(r#"DELETE FROM iterations WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete iteration")?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: create iteration, find by id
    #[tokio::test]
    async fn test_create_and_find_by_id() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = IterationRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = crate::loop_::LoopRepository::new(db.pool().clone());
        let create_loop = ralph_models::CreateLoop {
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
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = crate::task::TaskRepository::new(db.pool().clone());
        let create_task = ralph_models::CreateTask {
            loop_id: loop_.id.clone(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };
        let task = task_repo.create(create_task).await?;

        let iteration = ralph_models::Iteration {
            id: uuid::Uuid::new_v4().to_string(),
            loop_id: loop_.id.clone(),
            task_id: task.id.clone(),
            iteration_number: 1,
            output: Some("Test output".to_string()),
            error: None,
            status: ralph_models::IterationStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            tokens_used: None,
        };

        let created_iteration = repo.create(iteration.clone()).await?;

        let found_iteration = repo.find_by_id(&created_iteration.id).await?;
        assert!(found_iteration.is_some());
        let iter = found_iteration.unwrap();
        assert_eq!(iter.id, created_iteration.id);
        assert_eq!(iter.iteration_number, 1);
        assert_eq!(iter.loop_id, loop_.id);
        assert_eq!(iter.task_id, task.id);
        assert_eq!(iter.output, Some("Test output".to_string()));
        assert_eq!(iter.status, ralph_models::IterationStatus::Running);

        Ok(())
    }

    /// Integration test: list iterations by loop
    #[tokio::test]
    async fn test_list_by_loop() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = IterationRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = crate::loop_::LoopRepository::new(db.pool().clone());
        let create_loop = ralph_models::CreateLoop {
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
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = crate::task::TaskRepository::new(db.pool().clone());
        let create_task = ralph_models::CreateTask {
            loop_id: loop_.id.clone(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };
        let task = task_repo.create(create_task).await?;

        for i in 1..=3 {
            let iteration = ralph_models::Iteration {
                id: uuid::Uuid::new_v4().to_string(),
                loop_id: loop_.id.clone(),
                task_id: task.id.clone(),
                iteration_number: i,
                output: Some(format!("Output {}", i)),
                error: None,
                status: ralph_models::IterationStatus::Completed,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                tokens_used: Some(i * 1000),
            };
            repo.create(iteration).await?;
        }

        let iterations = repo.list_by_loop(&loop_.id).await?;
        assert_eq!(iterations.len(), 3);
        assert_eq!(iterations[0].iteration_number, 1);
        assert_eq!(iterations[1].iteration_number, 2);
        assert_eq!(iterations[2].iteration_number, 3);

        Ok(())
    }

    /// Integration test: list iterations by task
    #[tokio::test]
    async fn test_list_by_task() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = IterationRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = crate::loop_::LoopRepository::new(db.pool().clone());
        let create_loop = ralph_models::CreateLoop {
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
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = crate::task::TaskRepository::new(db.pool().clone());
        let create_task = ralph_models::CreateTask {
            loop_id: loop_.id.clone(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };
        let task = task_repo.create(create_task).await?;

        for i in 1..=2 {
            let iteration = ralph_models::Iteration {
                id: uuid::Uuid::new_v4().to_string(),
                loop_id: loop_.id.clone(),
                task_id: task.id.clone(),
                iteration_number: i,
                output: Some(format!("Output {}", i)),
                error: None,
                status: ralph_models::IterationStatus::Completed,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                tokens_used: Some(i * 1000),
            };
            repo.create(iteration).await?;
        }

        let iterations = repo.list_by_task(&task.id).await?;
        assert_eq!(iterations.len(), 2);
        assert_eq!(iterations[0].iteration_number, 1);
        assert_eq!(iterations[1].iteration_number, 2);

        Ok(())
    }

    /// Integration test: update status
    #[tokio::test]
    async fn test_update_status() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = IterationRepository::new(db.pool().clone());

        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = crate::loop_::LoopRepository::new(db.pool().clone());
        let create_loop = ralph_models::CreateLoop {
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
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = crate::task::TaskRepository::new(db.pool().clone());
        let create_task = ralph_models::CreateTask {
            loop_id: loop_.id.clone(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };
        let task = task_repo.create(create_task).await?;

        let iteration = ralph_models::Iteration {
            id: uuid::Uuid::new_v4().to_string(),
            loop_id: loop_.id.clone(),
            task_id: task.id.clone(),
            iteration_number: 1,
            output: Some("Test output".to_string()),
            error: None,
            status: ralph_models::IterationStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            tokens_used: None,
        };

        let created_iteration = repo.create(iteration).await?;
        assert_eq!(
            created_iteration.status,
            ralph_models::IterationStatus::Running
        );
        assert!(created_iteration.completed_at.is_none());
        assert!(created_iteration.tokens_used.is_none());

        let completed_at = Utc::now();
        let rows = repo
            .update_status(
                &created_iteration.id,
                ralph_models::IterationStatus::Completed,
                Some(completed_at),
                Some(1500),
            )
            .await?;
        assert_eq!(rows, 1);

        let updated_iteration = repo.find_by_id(&created_iteration.id).await?.unwrap();
        assert_eq!(
            updated_iteration.status,
            ralph_models::IterationStatus::Completed
        );
        assert_eq!(updated_iteration.completed_at, Some(completed_at));
        assert_eq!(updated_iteration.tokens_used, Some(1500));

        Ok(())
    }
}
