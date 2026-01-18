use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::{CreateTask, Task, TaskStatus};
use sqlx::{FromRow, Pool, Sqlite};
use std::str::FromStr;

#[derive(Debug, FromRow)]
struct TaskRow {
    id: String,
    loop_id: String,
    title: String,
    description: String,
    status: String,
    priority: i32,
    parent_task_id: Option<String>,
    created_by: String,
    iteration_id: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TaskRow> for Task {
    fn from(row: TaskRow) -> Self {
        Self {
            id: row.id,
            loop_id: row.loop_id,
            title: row.title,
            description: row.description,
            status: TaskStatus::from_str(&row.status).unwrap_or(TaskStatus::Failed),
            priority: row.priority,
            parent_task_id: row.parent_task_id,
            created_by: row.created_by,
            iteration_id: row.iteration_id,
            started_at: row.started_at,
            completed_at: row.completed_at,
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct TaskSummaryRow {
    id: String,
    loop_id: String,
    title: String,
    status: String,
    priority: i32,
    parent_task_id: Option<String>,
    created_by: String,
    iteration_id: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TaskSummaryRow> for Task {
    fn from(row: TaskSummaryRow) -> Self {
        Self {
            id: row.id,
            loop_id: row.loop_id,
            title: row.title,
            description: String::new(),
            status: TaskStatus::from_str(&row.status).unwrap_or(TaskStatus::Failed),
            priority: row.priority,
            parent_task_id: row.parent_task_id,
            created_by: row.created_by,
            iteration_id: row.iteration_id,
            started_at: row.started_at,
            completed_at: row.completed_at,
            error_message: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Repository for Task database operations
#[derive(Clone, Debug)]
pub struct TaskRepository {
    pool: Pool<Sqlite>,
}

impl TaskRepository {
    /// Create a new TaskRepository instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: SqlitePool) {
    /// let task_repo = TaskRepository::new(pool);
    /// # }
    /// ```
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new task in database
    ///
    /// # Arguments
    /// * `create_task` - Task creation data
    ///
    /// # Returns
    /// The created Task with generated id and timestamps
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # use ralph_models::CreateTask;
    /// # async fn example(repo: TaskRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let create_task = CreateTask {
    ///     loop_id: "loop123".to_string(),
    ///     title: "Test Task".to_string(),
    ///     description: "Test description".to_string(),
    ///     priority: None,
    ///     parent_task_id: None,
    ///     created_by: "user".to_string(),
    /// };
    /// let task = repo.create(create_task).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, create_task: CreateTask) -> Result<Task> {
        let task = Task::new(create_task);

        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, loop_id, title, description, status, priority,
                parent_task_id, created_by, iteration_id, started_at, completed_at,
                error_message, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.loop_id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(&task.status.to_string())
        .bind(task.priority)
        .bind(&task.parent_task_id)
        .bind(&task.created_by)
        .bind(&task.iteration_id)
        .bind(&task.started_at)
        .bind(&task.completed_at)
        .bind(&task.error_message)
        .bind(task.created_at)
        .bind(task.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert task into database")?;

        Ok(task)
    }

    /// Find a task by ID with full description and error_message
    ///
    /// # Arguments
    /// * `id` - Task ID to search for
    ///
    /// # Returns
    /// Some(Task) if found, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # async fn example(repo: TaskRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let task = repo.find_by_id("task-id-123").await?;
    /// if let Some(task) = task {
    ///     println!("Found task: {} with description: {}", task.title, task.description);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Task>> {
        let result = sqlx::query_as::<_, TaskRow>(
            r#"
            SELECT
                id, loop_id, title, description, status, priority,
                parent_task_id, created_by, iteration_id, started_at, completed_at,
                error_message, created_at, updated_at
            FROM tasks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query task by id")?;

        Ok(result.map(|row| row.into()))
    }

    /// List all tasks for a given loop without description and error_message (summary)
    ///
    /// # Arguments
    /// * `loop_id` - Loop ID to filter by
    ///
    /// # Returns
    /// Vector of Tasks without description and error_message (summary)
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # async fn example(repo: TaskRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let tasks = repo.list_by_loop("loop123").await?;
    /// for task in tasks {
    ///     println!("Task: {} (description omitted)", task.title);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_by_loop(&self, loop_id: &str) -> Result<Vec<Task>> {
        let results = sqlx::query_as::<_, TaskSummaryRow>(
            r#"
            SELECT
                id, loop_id, title, status, priority,
                parent_task_id, created_by, iteration_id, started_at, completed_at,
                created_at, updated_at
            FROM tasks
            WHERE loop_id = ?
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(loop_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query tasks by loop")?;

        Ok(results.into_iter().map(|row| row.into()).collect())
    }

    /// Find next pending task for a loop, ordered by priority DESC, created_at ASC
    ///
    /// # Arguments
    /// * `loop_id` - Loop ID to search in
    ///
    /// # Returns
    /// Some(Task) if a pending task exists, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # async fn example(repo: TaskRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(task) = repo.find_next_pending("loop123").await? {
    ///     println!("Next task: {} with priority {}", task.title, task.priority);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_next_pending(&self, loop_id: &str) -> Result<Option<Task>> {
        let result = sqlx::query_as::<_, TaskSummaryRow>(
            r#"
            SELECT
                id, loop_id, title, status, priority,
                parent_task_id, created_by, iteration_id, started_at, completed_at,
                created_at, updated_at
            FROM tasks
            WHERE loop_id = ? AND status = 'pending'
            ORDER BY priority DESC, created_at ASC
            LIMIT 1
            "#,
        )
        .bind(loop_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query next pending task")?;

        Ok(result.map(|row| row.into()))
    }

    /// Update task status with optional timestamps and error_message
    ///
    /// # Arguments
    /// * `id` - Task ID to update
    /// * `status` - New status
    /// * `started_at` - Optional started_at timestamp
    /// * `completed_at` - Optional completed_at timestamp
    /// * `error_message` - Optional error_message for failed tasks
    ///
    /// # Returns
    /// Number of rows affected (1 if updated, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # use ralph_models::TaskStatus;
    /// # async fn example(repo: TaskRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.update_status(
    ///     "task-id-123",
    ///     TaskStatus::Completed,
    ///     None,
    ///     Some(chrono::Utc::now()),
    ///     None
    /// ).await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_status(
        &self,
        id: &str,
        status: TaskStatus,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        error_message: Option<String>,
    ) -> Result<u64> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET status = ?, started_at = ?, completed_at = ?, error_message = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&status.to_string())
        .bind(&started_at)
        .bind(&completed_at)
        .bind(&error_message)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update task status")?;

        Ok(result.rows_affected())
    }

    /// Delete a task by ID
    ///
    /// # Arguments
    /// * `id` - Task ID to delete
    ///
    /// # Returns
    /// Number of rows affected (1 if deleted, 0 if not found)
    ///
    /// # Errors
    /// Returns error if database operation fails
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::task::TaskRepository;
    /// # async fn example(repo: TaskRepository) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = repo.delete("task-id-123").await?;
    /// assert_eq!(rows, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(r#"DELETE FROM tasks WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete task")?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: create task, find by id (full)
    #[tokio::test]
    async fn test_create_and_find_by_id() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = TaskRepository::new(db.pool().clone());

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

        let create_task = CreateTask {
            loop_id: loop_.id.clone(),
            title: "Test Task".to_string(),
            description: "Test description with details".to_string(),
            priority: Some(5),
            parent_task_id: None,
            created_by: "user".to_string(),
        };

        let created_task = repo.create(create_task).await?;

        let found_task = repo.find_by_id(&created_task.id).await?;
        assert!(found_task.is_some());
        let task = found_task.unwrap();
        assert_eq!(task.id, created_task.id);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "Test description with details");
        assert_eq!(task.loop_id, loop_.id);
        assert_eq!(task.priority, 5);
        assert_eq!(task.status, TaskStatus::Pending);

        Ok(())
    }

    /// Integration test: create tasks, list by loop (summary)
    #[tokio::test]
    async fn test_create_and_list_by_loop() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = TaskRepository::new(db.pool().clone());

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

        for i in 1..=3 {
            let create_task = CreateTask {
                loop_id: loop_.id.clone(),
                title: format!("Task {}", i),
                description: format!("Description {}", i),
                priority: Some(i),
                parent_task_id: None,
                created_by: "user".to_string(),
            };
            repo.create(create_task).await?;
        }

        let tasks = repo.list_by_loop(&loop_.id).await?;
        assert_eq!(tasks.len(), 3);

        assert_eq!(tasks[0].title, "Task 3");
        assert_eq!(tasks[0].priority, 3);
        assert_eq!(tasks[1].title, "Task 2");
        assert_eq!(tasks[1].priority, 2);
        assert_eq!(tasks[2].title, "Task 1");
        assert_eq!(tasks[2].priority, 1);

        for task in tasks {
            assert!(
                task.description.is_empty(),
                "Description should be empty in list queries"
            );
            assert!(
                task.error_message.is_none(),
                "Error message should be None in list queries"
            );
        }

        Ok(())
    }

    /// Integration test: find next pending (ordered)
    #[tokio::test]
    async fn test_find_next_pending_ordered() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = TaskRepository::new(db.pool().clone());

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

        let task1 = repo
            .create(CreateTask {
                loop_id: loop_.id.clone(),
                title: "Low Priority".to_string(),
                description: "desc".to_string(),
                priority: Some(1),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        let task2 = repo
            .create(CreateTask {
                loop_id: loop_.id.clone(),
                title: "High Priority".to_string(),
                description: "desc".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        let next_task = repo.find_next_pending(&loop_.id).await?;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, task2.id);

        repo.update_status(
            &task2.id,
            TaskStatus::Completed,
            None,
            Some(Utc::now()),
            None,
        )
        .await?;

        let next_task = repo.find_next_pending(&loop_.id).await?;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, task1.id);

        Ok(())
    }

    /// Integration test: update status with timestamps
    #[tokio::test]
    async fn test_update_status_with_timestamps() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = TaskRepository::new(db.pool().clone());

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

        let create_task = CreateTask {
            loop_id: loop_.id.clone(),
            title: "Test Task".to_string(),
            description: "desc".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };

        let task = repo.create(create_task).await?;
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());

        let now = Utc::now();
        let rows = repo
            .update_status(&task.id, TaskStatus::InProgress, Some(now), None, None)
            .await?;
        assert_eq!(rows, 1);

        let updated_task = repo.find_by_id(&task.id).await?.unwrap();
        assert_eq!(updated_task.status, TaskStatus::InProgress);
        assert_eq!(updated_task.started_at, Some(now));
        assert!(updated_task.completed_at.is_none());

        let completed_at = Utc::now();
        let rows = repo
            .update_status(
                &task.id,
                TaskStatus::Completed,
                None,
                Some(completed_at),
                None,
            )
            .await?;
        assert_eq!(rows, 1);

        let updated_task = repo.find_by_id(&task.id).await?.unwrap();
        assert_eq!(updated_task.status, TaskStatus::Completed);
        assert_eq!(updated_task.completed_at, Some(completed_at));

        let error_msg = "Task failed due to timeout".to_string();
        repo.update_status(
            &task.id,
            TaskStatus::Failed,
            None,
            None,
            Some(error_msg.clone()),
        )
        .await?;

        let updated_task = repo.find_by_id(&task.id).await?.unwrap();
        assert_eq!(updated_task.status, TaskStatus::Failed);
        assert_eq!(updated_task.error_message, Some(error_msg));

        Ok(())
    }

    /// Integration test: find next pending when no pending tasks
    #[tokio::test]
    async fn test_find_next_pending_none() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = TaskRepository::new(db.pool().clone());

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

        let next_task = repo.find_next_pending(&loop_.id).await?;
        assert!(next_task.is_none());

        Ok(())
    }
}
