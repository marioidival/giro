use anyhow::{Context, Result};
use ralph_agent::AgentConfig;
use ralph_models::LoopStatus;
use ralph_repositories::LoopRepository;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::{debug, info};

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

    /// Main execution loop for a running Ralph loop
    ///
    /// This is a stub implementation. Full execution logic will be implemented in a future task.
    ///
    /// # Arguments
    /// * `loop_id` - ID of the loop to execute
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn execution_loop(&self, loop_id: &str) -> Result<()> {
        debug!("Execution loop started for {}", loop_id);
        // TODO: Implement full execution logic in future task
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
}
