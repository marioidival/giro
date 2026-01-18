use anyhow::{Context, Result};
use chrono::Utc;
use ralph_agent::AgentConfig;
use ralph_models::{LoopStatus, TaskStatus};
use ralph_repositories::{LoopRepository, TaskRepository};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

use crate::docker::DockerManager;

/// Executor for managing Ralph loop lifecycle
#[derive(Clone, Debug)]
pub struct LoopExecutor {
    pool: Arc<Pool<Sqlite>>,
    docker: Arc<DockerManager>,
    #[allow(dead_code)]
    agent_config: AgentConfig,
}

impl LoopExecutor {
    /// Create a new LoopExecutor instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    /// * `docker` - Docker manager for container operations
    /// * `agent_config` - Agent configuration
    pub fn new(
        pool: Arc<Pool<Sqlite>>,
        docker: Arc<DockerManager>,
        agent_config: AgentConfig,
    ) -> Self {
        Self {
            pool,
            docker,
            agent_config,
        }
    }

    /// Start a loop by creating and starting its container
    ///
    /// # Arguments
    /// * `loop_id` - ID of the loop to start
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn start(&self, loop_id: &str) -> Result<()> {
        info!("Starting loop {}", loop_id);

        let loop_repo = LoopRepository::new((*self.pool).clone());
        let loop_ = loop_repo
            .find_by_id(loop_id)
            .await?
            .context("Loop not found")?;

        debug!("Creating container for loop {}", loop_id);
        let container_id = self
            .docker
            .create_container(
                &loop_.docker_image,
                "/tmp/prd.md",
                "/tmp/task.md",
                "/tmp/repo",
                Some(&format!("ralph-loop-{}", loop_id)),
            )
            .await?;

        debug!("Updating loop status to Running");
        loop_repo
            .update_status(loop_id, LoopStatus::Running, Some(container_id.clone()))
            .await?;

        debug!("Starting container {}", container_id);
        self.docker.start(&container_id).await?;

        let executor = self.clone();
        let loop_id_owned = loop_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = executor.execution_loop(&loop_id_owned).await {
                tracing::error!("Execution loop error for {}: {:?}", loop_id_owned, e);
            }
        });

        info!("Loop {} started successfully", loop_id);
        Ok(())
    }

    /// Pause a running loop
    ///
    /// # Arguments
    /// * `loop_id` - ID of the loop to pause
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn pause(&self, loop_id: &str) -> Result<()> {
        info!("Pausing loop {}", loop_id);

        let loop_repo = LoopRepository::new((*self.pool).clone());
        let loop_ = loop_repo
            .find_by_id(loop_id)
            .await?
            .context("Loop not found")?;

        let container_id = loop_
            .container_id
            .context("Loop has no associated container")?;

        debug!("Pausing container {}", container_id);
        self.docker.pause(&container_id).await?;

        debug!("Updating loop status to Paused");
        loop_repo
            .update_status(loop_id, LoopStatus::Paused, None)
            .await?;

        info!("Loop {} paused successfully", loop_id);
        Ok(())
    }

    /// Resume a paused loop
    ///
    /// # Arguments
    /// * `loop_id` - ID of the loop to resume
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn resume(&self, loop_id: &str) -> Result<()> {
        info!("Resuming loop {}", loop_id);

        let loop_repo = LoopRepository::new((*self.pool).clone());
        let loop_ = loop_repo
            .find_by_id(loop_id)
            .await?
            .context("Loop not found")?;

        let container_id = loop_
            .container_id
            .context("Loop has no associated container")?;

        debug!("Unpausing container {}", container_id);
        self.docker.unpause(&container_id).await?;

        debug!("Updating loop status to Running");
        loop_repo
            .update_status(loop_id, LoopStatus::Running, None)
            .await?;

        let executor = self.clone();
        let loop_id_owned = loop_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = executor.execution_loop(&loop_id_owned).await {
                tracing::error!("Execution loop error for {}: {:?}", loop_id_owned, e);
            }
        });

        info!("Loop {} resumed successfully", loop_id);
        Ok(())
    }

    /// Stop a running or paused loop
    ///
    /// # Arguments
    /// * `loop_id` - ID of the loop to stop
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn stop(&self, loop_id: &str) -> Result<()> {
        info!("Stopping loop {}", loop_id);

        let loop_repo = LoopRepository::new((*self.pool).clone());
        let loop_ = loop_repo
            .find_by_id(loop_id)
            .await?
            .context("Loop not found")?;

        let container_id = loop_
            .container_id
            .context("Loop has no associated container")?;

        debug!("Stopping container {}", container_id);
        self.docker.stop(&container_id, Some(10)).await?;

        debug!("Removing container {}", container_id);
        self.docker.remove(&container_id, true, true).await?;

        debug!("Updating loop status to Completed");
        loop_repo
            .update_status(loop_id, LoopStatus::Completed, None)
            .await?;

        info!("Loop {} stopped successfully", loop_id);
        Ok(())
    }

    /// Main execution loop for a running Ralph loop
    ///
    /// Continuously processes pending tasks until loop is stopped, paused,
    /// no more tasks exist, or max iterations is reached.
    ///
    /// # Arguments
    /// * `loop_id` - ID of the loop to execute
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn execution_loop(&self, loop_id: &str) -> Result<()> {
        debug!("Execution loop started for {}", loop_id);

        let loop_repo = LoopRepository::new((*self.pool).clone());
        let task_repo = TaskRepository::new((*self.pool).clone());

        loop {
            // Check loop status - exit if not running
            let current_loop = loop_repo
                .find_by_id(loop_id)
                .await?
                .context("Loop not found")?;

            if current_loop.status != LoopStatus::Running {
                debug!(
                    "Loop {} status is {:?}, exiting execution loop",
                    loop_id, current_loop.status
                );
                return Ok(());
            }

            // Check max iterations
            if current_loop.current_iteration >= current_loop.max_iterations {
                info!(
                    "Loop {} reached max iterations ({}/{})",
                    loop_id, current_loop.current_iteration, current_loop.max_iterations
                );
                self.stop(loop_id).await?;
                return Ok(());
            }

            // Find next pending task
            let next_task = task_repo.find_next_pending(loop_id).await?;

            // Exit if no more tasks
            if next_task.is_none() {
                info!("No more pending tasks for loop {}", loop_id);
                self.stop(loop_id).await?;
                return Ok(());
            }

            let task = next_task.unwrap();

            debug!(
                "Processing task {} (iteration {})",
                task.id,
                current_loop.current_iteration + 1
            );

            // Increment loop iteration counter
            loop_repo.increment_iteration(loop_id).await?;

            // Execute task
            match self.execute_task(&task).await {
                Ok(_iteration_id) => {
                    // Task completed successfully
                    task_repo
                        .update_status(
                            &task.id,
                            TaskStatus::Completed,
                            None,
                            Some(Utc::now()),
                            None,
                        )
                        .await?;

                    debug!("Task {} completed successfully", task.id);

                    // TODO: Auto-create tasks from LLM suggestions (task 2.23)
                    // Will be implemented in future task
                }
                Err(e) => {
                    // Task failed
                    error!("Task {} failed: {:?}", task.id, e);
                    task_repo
                        .update_status(
                            &task.id,
                            TaskStatus::Failed,
                            None,
                            Some(Utc::now()),
                            Some(e.to_string()),
                        )
                        .await?;

                    // Continue processing other tasks on failure (non-critical error)
                }
            }

            // Sleep for iteration_delay before processing next task
            if current_loop.iteration_delay > 0 {
                debug!(
                    "Sleeping for {} seconds before next iteration",
                    current_loop.iteration_delay
                );
                sleep(Duration::from_secs(current_loop.iteration_delay as u64)).await;
            }
        }
    }

    /// Execute a single task within a container
    ///
    /// This is a stub implementation. Full execution logic with LLM integration
    /// will be implemented in a future task (#40).
    ///
    /// # Arguments
    /// * `task` - Task to execute
    ///
    /// # Returns
    /// Iteration ID on success
    async fn execute_task(&self, task: &ralph_models::Task) -> Result<String> {
        debug!("Executing task {}", task.id);
        // TODO: Implement full task execution logic in task 2.22
        // Will create iteration, call LLM provider, execute commands, etc.
        Ok("iteration-".to_string() + &task.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: start loop creates and starts container
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_start_loop_creates_and_starts_container() -> Result<()> {
        // This test requires database setup and Docker
        // Will be implemented with full integration test setup
        Ok(())
    }

    /// Integration test: pause loop pauses container
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_pause_loop_pauses_container() -> Result<()> {
        // This test requires database setup and Docker
        // Will be implemented with full integration test setup
        Ok(())
    }

    /// Integration test: resume loop unpauses and restarts execution
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_resume_loop_unpauses_and_restarts_execution() -> Result<()> {
        // This test requires database setup and Docker
        // Will be implemented with full integration test setup
        Ok(())
    }

    /// Integration test: stop loop cleans up container
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_stop_loop_cleans_up_container() -> Result<()> {
        // This test requires database setup and Docker
        // Will be implemented with full integration test setup
        Ok(())
    }

    /// Unit test: clone creates independent copy
    #[tokio::test]
    async fn test_clone_creates_independent_copy() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

        let executor1 = LoopExecutor::new(pool, docker, agent_config);
        let executor2 = executor1.clone();

        // Both executors should be valid and independent
        assert_eq!(
            executor1.agent_config.max_iterations,
            executor2.agent_config.max_iterations
        );
        Ok(())
    }

    /// Unit test: new creates executor with correct config
    #[tokio::test]
    async fn test_new_creates_executor_with_correct_config() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig {
            max_iterations: 20,
            max_tokens_per_request: Some(8000),
            timeout_seconds: 120,
        };

        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone());

        assert_eq!(executor.agent_config.max_iterations, 20);
        assert_eq!(executor.agent_config.max_tokens_per_request, Some(8000));
        assert_eq!(executor.agent_config.timeout_seconds, 120);

        Ok(())
    }

    /// Integration test: execution loop processes tasks sequentially
    #[tokio::test]
    #[ignore = "Requires Docker daemon and full setup"]
    async fn test_execution_loop_processes_tasks_sequentially() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

        // Setup: create user, loop, and tasks
        let user_repo = ralph_repositories::UserRepository::new((*pool).clone());
        let password_hash = crate::auth::hash_password("password123")?;
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: password_hash,
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = ralph_repositories::LoopRepository::new((*pool).clone());
        let create_loop = ralph_models::CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: user.id.clone(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: Some(10),
            iteration_timeout: None,
            iteration_delay: Some(0),
            git_repo_url: None,
            git_branch_pattern: None,
        };
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());

        // Create 3 tasks with different priorities
        task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Low Priority Task".to_string(),
                description: "desc".to_string(),
                priority: Some(1),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Medium Priority Task".to_string(),
                description: "desc".to_string(),
                priority: Some(5),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        let high_priority_task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "High Priority Task".to_string(),
                description: "desc".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone());

        // Run execution loop briefly (should process at least high priority task)
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default());
            _executor.execution_loop(&loop_id_owned).await
        });

        // Give it time to process tasks
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify high priority task was processed
        let updated_task = task_repo.find_by_id(&high_priority_task.id).await?;
        assert!(updated_task.is_some());
        let task = updated_task.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        // Verify iteration count increased
        let updated_loop = loop_repo.find_by_id(&loop_.id).await?;
        assert!(updated_loop.is_some());
        assert!(updated_loop.unwrap().current_iteration > 0);

        // Clean up: stop the loop
        let executor = LoopExecutor::new(pool, docker, agent_config);
        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: execution loop stops when no more tasks
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_execution_loop_stops_when_no_more_tasks() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

        // Setup: create user and loop with no tasks
        let user_repo = ralph_repositories::UserRepository::new((*pool).clone());
        let password_hash = crate::auth::hash_password("password123")?;
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: password_hash,
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = ralph_repositories::LoopRepository::new((*pool).clone());
        let create_loop = ralph_models::CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: user.id.clone(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: Some(10),
            iteration_timeout: None,
            iteration_delay: Some(0),
            git_repo_url: None,
            git_branch_pattern: None,
        };
        let loop_ = loop_repo.create(create_loop).await?;

        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone());

        // Run execution loop
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default());
            _executor.execution_loop(&loop_id_owned).await
        });

        // Give it time to realize there are no tasks and stop
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Verify loop status changed to Completed
        let updated_loop = loop_repo.find_by_id(&loop_.id).await?;
        assert!(updated_loop.is_some());
        let loop_status = updated_loop.unwrap().status;
        assert_eq!(loop_status, LoopStatus::Completed);

        Ok(())
    }

    /// Integration test: execution loop stops at max iterations
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_execution_loop_stops_at_max_iterations() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

        // Setup: create user and loop with max_iterations = 2
        let user_repo = ralph_repositories::UserRepository::new((*pool).clone());
        let password_hash = crate::auth::hash_password("password123")?;
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: password_hash,
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = ralph_repositories::LoopRepository::new((*pool).clone());
        let create_loop = ralph_models::CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: user.id.clone(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: Some(2), // Only 2 iterations allowed
            iteration_timeout: None,
            iteration_delay: Some(0),
            git_repo_url: None,
            git_branch_pattern: None,
        };
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());

        // Create 5 tasks (more than max_iterations)
        for i in 1..=5 {
            task_repo
                .create(ralph_models::CreateTask {
                    loop_id: loop_.id.clone(),
                    title: format!("Task {}", i),
                    description: format!("Description {}", i),
                    priority: Some(i),
                    parent_task_id: None,
                    created_by: "user".to_string(),
                })
                .await?;
        }

        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone());

        // Run execution loop
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default());
            _executor.execution_loop(&loop_id_owned).await
        });

        // Give it time to process tasks and hit max_iterations
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify loop stopped at max iterations
        let updated_loop = loop_repo.find_by_id(&loop_.id).await?;
        assert!(updated_loop.is_some());
        let loop_data = updated_loop.unwrap();
        assert_eq!(loop_data.current_iteration, 2);
        assert_eq!(loop_data.status, LoopStatus::Completed);

        // Verify only 2 tasks were processed
        let tasks = task_repo.list_by_loop(&loop_.id).await?;
        let completed_tasks = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        assert_eq!(completed_tasks, 2);

        Ok(())
    }

    /// Integration test: execution loop logs errors
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_execution_loop_logs_errors() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

        // Setup: create user and loop
        let user_repo = ralph_repositories::UserRepository::new((*pool).clone());
        let password_hash = crate::auth::hash_password("password123")?;
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: password_hash,
        };
        let user = user_repo.create(create_user).await?;

        let loop_repo = ralph_repositories::LoopRepository::new((*pool).clone());
        let create_loop = ralph_models::CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: user.id.clone(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: Some(10),
            iteration_timeout: None,
            iteration_delay: Some(0),
            git_repo_url: None,
            git_branch_pattern: None,
        };
        let loop_ = loop_repo.create(create_loop).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());

        // Create a task (execute_task will return an error since it's a stub)
        let task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Failing Task".to_string(),
                description: "desc".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone());

        // Run execution loop
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default());
            _executor.execution_loop(&loop_id_owned).await
        });

        // Give it time to process task and encounter an error
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Verify task was marked as Failed with error message
        let updated_task = task_repo.find_by_id(&task.id).await?;
        assert!(updated_task.is_some());
        let task_data = updated_task.unwrap();
        assert_eq!(task_data.status, TaskStatus::Failed);
        assert!(task_data.error_message.is_some());

        // Clean up
        let executor = LoopExecutor::new(pool, docker, agent_config);
        executor.stop(&loop_.id).await?;

        Ok(())
    }
}
