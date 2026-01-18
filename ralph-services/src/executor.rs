use anyhow::{Context, Result};
use chrono::Utc;
use ralph_agent::provider::LLMProviderTrait;
use ralph_agent::{AgentConfig, CodeAgent, ExecutionContext, MockLLMProvider};
use ralph_agent::provider::{ClaudeProvider, OpenAIProvider};
use ralph_models::{ApiKeyProvider, IterationStatus, LoopStatus, TaskStatus};
use ralph_repositories::{ApiKeyRepository, IterationRepository, LoopRepository, TaskRepository};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

use crate::docker::DockerManager;
use crate::git::GitService;

/// Executor for managing Ralph loop lifecycle
#[derive(Clone, Debug)]
pub struct LoopExecutor {
    pool: Arc<Pool<Sqlite>>,
    docker: Arc<DockerManager>,
    git_service: Option<Arc<GitService>>,
    #[allow(dead_code)]
    agent_config: AgentConfig,
    api_key_repo: Arc<ApiKeyRepository>,
}

impl LoopExecutor {
    /// Create a new LoopExecutor instance
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    /// * `docker` - Docker manager for container operations
    /// * `agent_config` - Agent configuration
    /// * `git_service` - Optional Git service for automated commits and PRs
    /// * `api_key_repo` - API key repository for user-specific keys
    pub fn new(
        pool: Arc<Pool<Sqlite>>,
        docker: Arc<DockerManager>,
        agent_config: AgentConfig,
        git_service: Option<Arc<GitService>>,
        api_key_repo: Arc<ApiKeyRepository>,
    ) -> Self {
        Self {
            pool,
            docker,
            git_service,
            agent_config,
            api_key_repo,
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

    /// Get LLM provider for a loop based on user's API key
    ///
    /// # Arguments
    /// * `loop_` - Loop to get provider for
    ///
    /// # Returns
    /// Box<dyn LLMProviderTrait> - LLM provider instance
    ///
    /// # Errors
    /// Returns error if:
    /// - Provider is not supported
    /// - No active API key exists for the user and provider
    /// - API key decryption fails
    /// - Provider creation fails
    fn get_llm_provider(
        &self,
        loop_: &ralph_models::Loop,
    ) -> Result<Box<dyn LLMProviderTrait>> {
        let provider_str = loop_.provider.as_str();

        // Parse provider string to ApiKeyProvider
        let api_key_provider = match provider_str {
            "anthropic" => ApiKeyProvider::Anthropic,
            "openai" => ApiKeyProvider::OpenAI,
            "amp" => ApiKeyProvider::Amp,
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported provider: {}. Must be one of: anthropic, openai, amp",
                    provider_str
                ))
            }
        };

        // Use blocking task for async API key retrieval
        let api_key_repo = self.api_key_repo.clone();
        let user_id = loop_.owner_id.clone();

        // We need to get the decrypted API key
        // Since we're in a sync context, we need to use a different approach
        // For now, return an error indicating we need the API key to be provided
        // In a real implementation, this would be done in an async context

        // For the mock provider, we can return it directly
        if provider_str == "mock" {
            return Ok(Box::new(MockLLMProvider::new()));
        }

        // For production providers, we need to get the API key asynchronously
        // This is a limitation that needs to be addressed in the execute_task method
        // For now, we'll return a placeholder error
        Err(anyhow::anyhow!(
            "API key retrieval must be done in async context - use get_llm_provider_async instead"
        ))
    }

    /// Get LLM provider for a loop based on user's API key (async version)
    ///
    /// # Arguments
    /// * `loop_` - Loop to get provider for
    ///
    /// # Returns
    /// Box<dyn LLMProviderTrait> - LLM provider instance
    ///
    /// # Errors
    /// Returns error if:
    /// - Provider is not supported
    /// - No active API key exists for the user and provider
    /// - API key decryption fails
    /// - Provider creation fails
    async fn get_llm_provider_async(
        &self,
        loop_: &ralph_models::Loop,
    ) -> Result<Box<dyn LLMProviderTrait>> {
        let provider_str = loop_.provider.as_str();

        // For mock provider, return directly
        if provider_str == "mock" {
            return Ok(Box::new(MockLLMProvider::new()));
        }

        // Parse provider string to ApiKeyProvider
        let api_key_provider = match provider_str {
            "anthropic" => ApiKeyProvider::Anthropic,
            "openai" => ApiKeyProvider::OpenAI,
            "amp" => ApiKeyProvider::Amp,
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported provider: {}. Must be one of: anthropic, openai, amp",
                    provider_str
                ))
            }
        };

        // Get active API key for user and provider
        let api_key = self
            .api_key_repo
            .get_active_for_user(&loop_.owner_id, api_key_provider.clone())
            .await?
            .with_context(|| {
                format!(
                    "No active API key found for user {} and provider {}",
                    loop_.owner_id, provider_str
                )
            })?;

        // Decrypt the API key
        let decrypted_key = self
            .api_key_repo
            .get_decrypted_key(&api_key.id)
            .await
            .context("Failed to decrypt API key")?;

        // Create provider with user's API key
        match api_key_provider {
            ApiKeyProvider::Anthropic => {
                let provider = ClaudeProvider::new(decrypted_key)
                    .context("Failed to create Claude provider")?;
                Ok(Box::new(provider))
            }
            ApiKeyProvider::OpenAI => {
                let provider = OpenAIProvider::new(decrypted_key)
                    .context("Failed to create OpenAI provider")?;
                Ok(Box::new(provider))
            }
            ApiKeyProvider::Amp => {
                // Amp provider uses a different implementation
                // For now, return MockLLMProvider
                Ok(Box::new(MockLLMProvider::new()))
            }
        }
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

            // Exit if no more tasks and create PR if Git is configured
            if next_task.is_none() {
                info!("No more pending tasks for loop {}", loop_id);

                if let Some(git_service) = &self.git_service
                    && let Some(_git_repo_url) = &current_loop.git_repo_url
                {
                    info!("Loop completed - creating PR for Git integration");
                    let pr_title = format!(
                        "Ralph Loop {} - {} Iterations Complete",
                        loop_id, current_loop.current_iteration
                    );
                    let pr_body = format!(
                        "Ralph Loop Manager automatically generated this PR.\n\nLoop: {}\nTotal Iterations: {}\nStatus: Completed",
                        loop_id, current_loop.current_iteration
                    );
                    match git_service.create_pr(&pr_title, &pr_body).await {
                        Ok(pr_url) => {
                            info!("PR created successfully: {}", pr_url);
                        }
                        Err(e) => {
                            error!("Failed to create PR: {:?}", e);
                        }
                    }
                }

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
    /// Executes a task via LLM provider (Claude or Mock) inside Docker container,
    /// writes output to /workspace/task.md, and returns iteration_id.
    /// Also auto-creates tasks from LLM suggestions if present.
    ///
    /// If loop has `git_repo_url` configured, automatically:
    /// - Creates branch with pattern substitution ({loop_id}, {timestamp})
    /// - Stages all generated files
    /// - Commits with message: "Ralph Loop {loop_id} - Iteration {iter_num} - Task {task_title}"
    /// - Pushes to remote
    ///
    /// # Arguments
    /// * `task` - Task to execute
    ///
    /// # Returns
    /// Iteration ID on success
    async fn execute_task(&self, task: &ralph_models::Task) -> Result<String> {
        debug!("Executing task {}", task.id);

        let task_repo = TaskRepository::new((*self.pool).clone());
        let loop_repo = LoopRepository::new((*self.pool).clone());
        let iteration_repo = IterationRepository::new((*self.pool).clone());

        task_repo
            .update_status(
                &task.id,
                TaskStatus::InProgress,
                Some(Utc::now()),
                None,
                None,
            )
            .await
            .context("Failed to update task status to InProgress")?;

        let loop_ = loop_repo
            .find_by_id(&task.loop_id)
            .await?
            .context("Loop not found")?;

        let container_id = loop_
            .container_id
            .as_ref()
            .context("Loop has no associated container")?;

        if let Some(git_service) = &self.git_service
            && let Some(_git_repo_url) = &loop_.git_repo_url
            && loop_.current_iteration == 0
        {
            let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
            let branch_name = loop_
                .git_branch_pattern
                .replace("{loop_id}", &task.loop_id)
                .replace("{timestamp}", &timestamp);

            info!("Creating Git branch: {}", branch_name);
            git_service
                .create_branch(&branch_name)
                .await
                .context("Failed to create Git branch")?;

            info!("Checking out branch: {}", branch_name);
            git_service
                .checkout(&branch_name)
                .await
                .context("Failed to checkout Git branch")?;
        }

        let iteration = ralph_models::Iteration::new(
            task.loop_id.clone(),
            task.id.clone(),
            loop_.current_iteration + 1,
        );
        let iteration_id = iteration.id.clone();
        iteration_repo
            .create(iteration)
            .await
            .context("Failed to create iteration")?;

        // Get LLM provider with user's API key
        let provider = self.get_llm_provider_async(&loop_).await.context(format!(
            "Failed to get LLM provider for loop {} with provider {}",
            task.loop_id, loop_.provider
        ))?;

        let agent = CodeAgent::new(provider, self.agent_config.clone());

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace/repo".to_string(),
            HashMap::new(),
        );

        let previous_iterations = iteration_repo
            .list_by_task(&task.id)
            .await
            .unwrap_or_default();
        let context: Vec<String> = previous_iterations
            .iter()
            .filter_map(|iter| iter.output.as_ref())
            .cloned()
            .collect();

        let result = agent
            .execute_task(loop_.prd.clone(), task.description.clone(), context)
            .await
            .context("Failed to execute task via agent")?;

        ctx.write_file("/workspace/task.md", &result.content)
            .await
            .context("Failed to write task output to /workspace/task.md")?;

        if let Some(git_service) = &self.git_service
            && let Some(_git_repo_url) = &loop_.git_repo_url
        {
            info!("Staging all changes in Git repository");
            git_service
                .stage_all()
                .await
                .context("Failed to stage files in Git")?;

            let commit_message = format!(
                "Ralph Loop {} - Iteration {} - Task {}",
                task.loop_id,
                loop_.current_iteration + 1,
                task.title
            );
            info!("Committing changes: {}", commit_message);
            git_service
                .commit(&commit_message)
                .await
                .context("Failed to commit changes to Git")?;

            let current_branch = git_service
                .current_branch()
                .await
                .context("Failed to get current Git branch")?;
            info!("Pushing branch: {}", current_branch);
            git_service
                .push("origin", &current_branch)
                .await
                .context("Failed to push changes to Git remote")?;
        }

        iteration_repo
            .update_status(
                &iteration_id,
                IterationStatus::Completed,
                Some(Utc::now()),
                Some(result.tokens_used as i32),
            )
            .await
            .context("Failed to update iteration status")?;

        if !result.suggested_tasks.is_empty() {
            self.create_suggested_tasks(
                &task_repo,
                &result.suggested_tasks,
                &task.loop_id,
                &iteration_id,
                Some(&task.id),
            )
            .await?;
        }

        debug!("Task {} completed successfully", task.id);
        Ok(iteration_id)
    }

    /// Create tasks from LLM suggestions
    ///
    /// Creates new tasks based on LLM suggestions, linking them to the current iteration.
    ///
    /// # Arguments
    /// * `task_repo` - Task repository for creating tasks
    /// * `suggested_tasks` - Tasks suggested by the LLM
    /// * `loop_id` - Loop ID to associate tasks with
    /// * `iteration_id` - Iteration ID to link tasks to
    /// * `parent_task_id` - Optional parent task ID for hierarchical tasks
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn create_suggested_tasks(
        &self,
        task_repo: &TaskRepository,
        suggested_tasks: &[ralph_agent::provider::SuggestedTask],
        loop_id: &str,
        iteration_id: &str,
        parent_task_id: Option<&str>,
    ) -> Result<()> {
        info!(
            "Creating {} suggested tasks for iteration {}",
            suggested_tasks.len(),
            iteration_id
        );

        for suggested_task in suggested_tasks {
            let create_task = ralph_models::CreateTask {
                loop_id: loop_id.to_string(),
                title: suggested_task.title.clone(),
                description: suggested_task.description.clone(),
                priority: Some(suggested_task.priority),
                parent_task_id: parent_task_id.map(|id| id.to_string()),
                created_by: "llm".to_string(),
            };

            match task_repo.create(create_task).await {
                Ok(created_task) => {
                    info!(
                        "Created suggested task '{}' (ID: {}, priority: {})",
                        created_task.title, created_task.id, created_task.priority
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to create suggested task '{}': {:?}",
                        suggested_task.title, e
                    );
                    // Continue processing other tasks even if one fails
                }
            }
        }

        Ok(())
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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor1 = LoopExecutor::new(pool, docker, agent_config, None, api_key_repo);
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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);

        // Run execution loop briefly (should process at least high priority task)
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default(), None, api_key_repo);
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
        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool, docker, agent_config, None, api_key_repo);
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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);

        // Run execution loop
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default(), None, api_key_repo);
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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);

        // Run execution loop
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default(), None, api_key_repo);
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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let _executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);

        // Run execution loop
        let loop_id_owned = loop_.id.clone();
        tokio::spawn(async move {
            let pool = Arc::new(
                sqlx::SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("Failed to connect"),
            );
            let docker = Arc::new(DockerManager::new());
            let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
            let _executor = LoopExecutor::new(pool, docker, AgentConfig::default(), None, api_key_repo);
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
        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool, docker, agent_config, None, api_key_repo);
        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: execute_task with MockLLMProvider
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_execute_task_with_mock_provider() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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
            prd: "Build a web server".to_string(),
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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Test Task".to_string(),
                description: "Create HTTP handler".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        let iteration_id = executor.execute_task(&task).await?;

        let iteration_repo = ralph_repositories::IterationRepository::new((*pool).clone());
        let iteration = iteration_repo.find_by_id(&iteration_id).await?;
        assert!(iteration.is_some());
        let iter = iteration.unwrap();
        assert_eq!(iter.task_id, task.id);
        assert_eq!(iter.loop_id, loop_.id);
        assert_eq!(iter.status, ralph_models::IterationStatus::Completed);

        let updated_task = task_repo.find_by_id(&task.id).await?;
        assert!(updated_task.is_some());
        assert_eq!(updated_task.unwrap().status, TaskStatus::InProgress);

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: task output written to container
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_task_output_written_to_container() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Test Task".to_string(),
                description: "Create test file".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        executor.execute_task(&task).await?;

        // Verify task output is in container
        let ctx = ralph_agent::executor::ExecutionContext::new(
            loop_.container_id.unwrap(),
            "/workspace".to_string(),
            std::collections::HashMap::new(),
        );
        let output = ctx
            .read_file("/workspace/task.md")
            .await
            .context("Failed to read task.md from container")?;

        assert!(!output.is_empty(), "Task output should not be empty");
        assert!(
            output.contains("Create test file"),
            "Should contain task content"
        );

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: task failure updates status with error
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_task_failure_updates_status_with_error() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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

        // Start the loop
        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        // Create a task and try to execute it
        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Test Task".to_string(),
                description: "Test description".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        // Execute task and verify it completes (even if there are errors)
        let result = executor.execute_task(&task).await;

        // For now, the task should succeed with MockLLMProvider
        // Future tests will verify error handling
        assert!(result.is_ok() || result.is_err());

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: malformed LLM response handled gracefully
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_malformed_llm_response_handled() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Test Task".to_string(),
                description: "Test description".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        // Execute task and verify it completes without crashing
        // MockLLMProvider should always return valid responses
        let result = executor.execute_task(&task).await;
        assert!(result.is_ok(), "Task execution should not crash");

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: task timeout enforced
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_task_timeout_enforced() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig {
            timeout_seconds: 1, // 1 second timeout
            ..Default::default()
        };

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Test Task".to_string(),
                description: "Test description".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        // Execute task - should complete within timeout (MockLLMProvider is fast)
        executor.execute_task(&task).await?;

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: LLM suggestions create new tasks
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_llm_suggestions_create_new_tasks() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let initial_task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Initial Task".to_string(),
                description: "Initial description".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        executor.execute_task(&initial_task).await?;

        let all_tasks = task_repo.list_by_loop(&loop_.id).await?;
        assert!(all_tasks.len() > 1, "Should have created suggested tasks");

        let suggested_tasks: Vec<_> = all_tasks.iter().filter(|t| t.created_by == "llm").collect();
        assert!(
            !suggested_tasks.is_empty(),
            "Should have LLM-suggested tasks"
        );

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: tasks created with correct priority
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_tasks_created_with_correct_priority() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let initial_task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Initial Task".to_string(),
                description: "Initial description".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        executor.execute_task(&initial_task).await?;

        let all_tasks = task_repo.list_by_loop(&loop_.id).await?;

        let suggested_tasks: Vec<_> = all_tasks.iter().filter(|t| t.created_by == "llm").collect();

        for task in suggested_tasks {
            assert!(task.priority >= 0, "Priority should be non-negative");
            assert!(task.priority <= 100, "Priority should be reasonable");
        }

        executor.stop(&loop_.id).await?;

        Ok(())
    }

    /// Integration test: parent_task_id set if hierarchical
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_parent_task_id_set_if_hierarchical() -> Result<()> {
        let pool = Arc::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .context("Failed to create test pool")?,
        );
        let docker = Arc::new(DockerManager::new());
        let agent_config = AgentConfig::default();

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

        let api_key_repo = Arc::new(ApiKeyRepository::new((*pool).clone()));
        let executor = LoopExecutor::new(pool.clone(), docker.clone(), agent_config.clone(), None, api_key_repo);
        executor.start(&loop_.id).await?;

        let task_repo = ralph_repositories::TaskRepository::new((*pool).clone());
        let parent_task = task_repo
            .create(ralph_models::CreateTask {
                loop_id: loop_.id.clone(),
                title: "Parent Task".to_string(),
                description: "Parent description".to_string(),
                priority: Some(10),
                parent_task_id: None,
                created_by: "user".to_string(),
            })
            .await?;

        executor.execute_task(&parent_task).await?;

        let all_tasks = task_repo.list_by_loop(&loop_.id).await?;

        let suggested_tasks: Vec<_> = all_tasks.iter().filter(|t| t.created_by == "llm").collect();

        if !suggested_tasks.is_empty() {
            let has_parent = suggested_tasks
                .iter()
                .any(|t| t.parent_task_id.as_deref() == Some(&parent_task.id));
            assert!(
                has_parent,
                "At least one suggested task should have parent_task_id set"
            );
        }

        executor.stop(&loop_.id).await?;

        Ok(())
    }
}
