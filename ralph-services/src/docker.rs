use anyhow::{Context, Result};
use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::models::{ContainerSummary, HostConfig};
use once_cell::sync::OnceCell;
use tracing::{debug, info, warn};

/// Global singleton Docker client instance
pub static DOCKER: OnceCell<Docker> = OnceCell::new();

/// Get or initialize the global Docker client
pub fn docker() -> &'static Docker {
    DOCKER.get_or_init(|| {
        debug!("Initializing Docker client");
        #[cfg(unix)]
        let client =
            Docker::connect_with_unix_defaults().expect("Failed to connect to Docker daemon");
        #[cfg(windows)]
        let client =
            Docker::connect_with_named_pipe_defaults().expect("Failed to connect to Docker daemon");

        info!("Docker client initialized");
        client
    })
}

/// DockerManager for managing container lifecycle
#[derive(Clone, Debug)]
pub struct DockerManager {
    client: Docker,
}

impl Default for DockerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerManager {
    /// Create a new DockerManager instance
    pub fn new() -> Self {
        Self {
            client: docker().clone(),
        }
    }

    /// Create a new container with the specified configuration
    ///
    /// # Arguments
    /// * `image` - Docker image to use (e.g., "ubuntu:latest")
    /// * `prd_path` - Path to PRD file (will be mounted read-only at /workspace/prd.md)
    /// * `task_path` - Path to task file (will be mounted read-only at /workspace/task.md)
    /// * `repo_path` - Path to repository (will be mounted at /workspace/repo)
    /// * `name` - Optional container name
    ///
    /// # Returns
    /// Container ID of the created container
    pub async fn create_container(
        &self,
        image: &str,
        prd_path: &str,
        task_path: &str,
        repo_path: &str,
        name: Option<&str>,
    ) -> Result<String> {
        info!(
            "Creating container from image {} with mounts: prd={}, task={}, repo={}",
            image, prd_path, task_path, repo_path
        );

        let binds = vec![
            format!("{}:/workspace/prd.md:ro", prd_path),
            format!("{}:/workspace/task.md:ro", task_path),
            format!("{}:/workspace/repo", repo_path),
        ];

        let cpu_quota = 1_000_000_000i64;
        let cpu_period = 1_000_000i64;
        let memory_limit: i64 = 1_073_741_824;

        let host_config = HostConfig {
            binds: Some(binds),
            cpu_quota: Some(cpu_quota),
            cpu_period: Some(cpu_period),
            memory: Some(memory_limit),
            memory_swap: Some(memory_limit),
            ..Default::default()
        };

        let config = Config {
            image: Some(image.to_string()),
            host_config: Some(host_config),
            cmd: Some(vec![
                "sh".to_string(),
                "-c".to_string(),
                "sleep infinity".to_string(),
            ]),
            ..Default::default()
        };

        let mut options: Option<CreateContainerOptions<String>> = None;
        if let Some(container_name) = name {
            options = Some(CreateContainerOptions {
                name: container_name.to_string(),
                ..Default::default()
            });
        }

        let result = match options {
            Some(opts) => self.client.create_container(Some(opts), config).await,
            None => {
                self.client
                    .create_container::<String, String>(None, config)
                    .await
            }
        };

        let response = result.context("Failed to create Docker container")?;

        let container_id = response.id;
        debug!("Container created with ID: {}", container_id);

        Ok(container_id)
    }

    /// Start a container
    ///
    /// # Arguments
    /// * `container_id` - ID of the container to start
    pub async fn start(&self, container_id: &str) -> Result<()> {
        info!("Starting container {}", container_id);
        self.client
            .start_container::<String>(container_id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;
        debug!("Container {} started", container_id);
        Ok(())
    }

    /// Pause a running container
    ///
    /// # Arguments
    /// * `container_id` - ID of the container to pause
    pub async fn pause(&self, container_id: &str) -> Result<()> {
        info!("Pausing container {}", container_id);
        self.client
            .pause_container(container_id)
            .await
            .context("Failed to pause container")?;
        debug!("Container {} paused", container_id);
        Ok(())
    }

    /// Unpause a paused container
    ///
    /// # Arguments
    /// * `container_id` - ID of the container to unpause
    pub async fn unpause(&self, container_id: &str) -> Result<()> {
        info!("Unpausing container {}", container_id);
        self.client
            .unpause_container(container_id)
            .await
            .context("Failed to unpause container")?;
        debug!("Container {} unpaused", container_id);
        Ok(())
    }

    /// Stop a running container
    ///
    /// # Arguments
    /// * `container_id` - ID of the container to stop
    /// * `timeout_seconds` - Optional timeout in seconds before force killing
    pub async fn stop(&self, container_id: &str, timeout_seconds: Option<i32>) -> Result<()> {
        info!(
            "Stopping container {} with timeout {:?}",
            container_id, timeout_seconds
        );

        let options = timeout_seconds.map(|t| StopContainerOptions { t: t.into() });

        self.client
            .stop_container(container_id, options)
            .await
            .context("Failed to stop container")?;
        debug!("Container {} stopped", container_id);
        Ok(())
    }

    /// Remove a container
    ///
    /// # Arguments
    /// * `container_id` - ID of the container to remove
    /// * `force` - Force removal even if running
    /// * `volumes` - Remove associated volumes
    pub async fn remove(&self, container_id: &str, force: bool, volumes: bool) -> Result<()> {
        info!(
            "Removing container {} (force={}, volumes={})",
            container_id, force, volumes
        );
        let options = Some(RemoveContainerOptions {
            force,
            v: volumes,
            ..Default::default()
        });

        self.client
            .remove_container(container_id, options)
            .await
            .context("Failed to remove container")?;
        debug!("Container {} removed", container_id);
        Ok(())
    }

    /// Inspect a container to get its state
    pub async fn inspect_container(
        &self,
        container_id: &str,
    ) -> Result<bollard::models::ContainerInspectResponse> {
        debug!("Inspecting container {}", container_id);
        self.client
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .context("Failed to inspect container")
    }

    /// List containers
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
        debug!("Listing containers (all={})", all);
        let options = ListContainersOptions::<String> {
            all,
            ..Default::default()
        };

        self.client
            .list_containers(Some(options))
            .await
            .context("Failed to list containers")
    }

    /// Clean up orphaned containers on startup
    ///
    /// Removes containers with "ralph-" prefix that are in stopped or exited state.
    /// Running containers are not affected.
    ///
    /// # Returns
    /// Number of containers cleaned up
    pub async fn cleanup_orphaned_containers(&self) -> Result<usize> {
        info!("Starting orphaned container cleanup");

        let containers = self.list_containers(true).await?;
        let mut cleaned_count = 0;

        for container in containers {
            let container_id = container.id.as_deref().unwrap_or("");
            let default_name = String::new();
            let container_name = container
                .names
                .as_ref()
                .and_then(|names| names.first())
                .unwrap_or(&default_name);

            if !container_name.starts_with("/ralph-") {
                continue;
            }

            let is_stopped_or_exited = container
                .state
                .as_deref()
                .map(|state| state == "exited" || state == "dead")
                .unwrap_or(false);

            if is_stopped_or_exited {
                info!(
                    "Cleaning up orphaned container {} ({})",
                    container_name, container_id
                );
                if let Err(e) = self.remove(container_id, true, true).await {
                    warn!("Failed to remove container {}: {:?}", container_id, e);
                } else {
                    cleaned_count += 1;
                    info!("Successfully cleaned up container {}", container_id);
                }
            } else {
                debug!(
                    "Skipping running container {} ({})",
                    container_name, container_id
                );
            }
        }

        info!("Cleanup complete: {} containers removed", cleaned_count);
        Ok(cleaned_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_singleton() {
        let docker1 = docker();
        let docker2 = docker();

        assert!(
            std::ptr::eq(docker1, docker2),
            "Docker client should be singleton"
        );
    }

    #[test]
    fn test_docker_manager_creation() {
        let manager = DockerManager::new();
        assert_eq!(
            std::mem::size_of_val(&manager),
            std::mem::size_of::<DockerManager>()
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn docker_available() -> bool {
        #[cfg(unix)]
        {
            Docker::connect_with_unix_defaults().is_ok()
        }
        #[cfg(windows)]
        {
            Docker::connect_with_named_pipe_defaults().is_ok()
        }
    }

    async fn setup_test_files() -> Result<(TempDir, PathBuf, PathBuf, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path();

        let prd_path = base_path.join("test_prd.md");
        fs::write(&prd_path, "# Test PRD\n\nThis is a test PRD file.")?;

        let task_path = base_path.join("test_task.md");
        fs::write(&task_path, "# Test Task\n\nThis is a test task file.")?;

        let repo_path = base_path.join("test_repo");
        fs::create_dir_all(&repo_path)?;
        fs::write(repo_path.join("README.md"), "# Test Repository")?;

        Ok((temp_dir, prd_path, task_path, repo_path))
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_create_container_with_volumes() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_container_volumes"),
            )
            .await?;

        assert!(!container_id.is_empty(), "Container ID should not be empty");

        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_verify_volume_mounts() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_container_verify_volumes"),
            )
            .await?;

        let inspect = manager.inspect_container(&container_id).await?;

        let binds = inspect
            .host_config
            .and_then(|hc| hc.binds)
            .ok_or_else(|| anyhow::anyhow!("No binds found in container config"))?;

        assert!(!binds.is_empty(), "Should have volume bindings");

        let binds_str = binds.join(" ");
        assert!(
            binds_str.contains("/workspace/prd.md:ro"),
            "PRD should be mounted read-only"
        );
        assert!(
            binds_str.contains("/workspace/task.md:ro"),
            "Task should be mounted read-only"
        );
        assert!(
            binds_str.contains("/workspace/repo"),
            "Repo should be mounted"
        );

        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_enforce_resource_limits() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_container_limits"),
            )
            .await?;

        let inspect = manager.inspect_container(&container_id).await?;

        let host_config = inspect
            .host_config
            .ok_or_else(|| anyhow::anyhow!("No host config"))?;

        let cpu_quota = host_config
            .cpu_quota
            .ok_or_else(|| anyhow::anyhow!("No CPU quota set"))?;
        let cpu_period = host_config
            .cpu_period
            .ok_or_else(|| anyhow::anyhow!("No CPU period set"))?;

        assert_eq!(cpu_quota, 1_000_000_000, "CPU quota should be 1 core");
        assert_eq!(cpu_period, 1_000_000, "CPU period should be 1ms");

        let memory = host_config
            .memory
            .ok_or_else(|| anyhow::anyhow!("No memory limit set"))?;

        assert_eq!(memory, 1_073_741_824, "Memory limit should be 1GB");

        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_start_and_stop_container() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_container_start_stop"),
            )
            .await?;

        manager.start(&container_id).await?;

        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect
            .state
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(running, "Container should be running after start");

        manager.stop(&container_id, Some(10)).await?;

        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect.state.and_then(|s| s.running).unwrap_or(false);
        assert!(!running, "Container should not be running after stop");

        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_pause_and_unpause_container() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_container_pause"),
            )
            .await?;

        manager.start(&container_id).await?;

        manager.pause(&container_id).await?;

        let inspect = manager.inspect_container(&container_id).await?;
        let paused = inspect
            .state
            .as_ref()
            .and_then(|s| s.paused)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(
            paused && running,
            "Container should be both paused and running"
        );

        manager.unpause(&container_id).await?;

        let inspect = manager.inspect_container(&container_id).await?;
        let paused = inspect
            .state
            .as_ref()
            .and_then(|s| s.paused)
            .unwrap_or(false);
        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(
            !paused && running,
            "Container should be running but not paused"
        );

        manager.stop(&container_id, Some(10)).await?;
        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_container_state_transitions() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_container_state"),
            )
            .await?;

        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect.state.and_then(|s| s.running).unwrap_or(false);
        assert!(!running, "Container should not be running initially");

        manager.start(&container_id).await?;
        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(running, "Container should be running after start");

        manager.pause(&container_id).await?;
        let inspect = manager.inspect_container(&container_id).await?;
        let paused = inspect
            .state
            .as_ref()
            .and_then(|s| s.paused)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(
            paused && running,
            "Container should be both paused and running"
        );

        manager.unpause(&container_id).await?;
        let inspect = manager.inspect_container(&container_id).await?;
        let paused = inspect
            .state
            .as_ref()
            .and_then(|s| s.paused)
            .unwrap_or(false);
        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(
            !paused && running,
            "Container should be running but not paused"
        );

        manager.stop(&container_id, Some(10)).await?;
        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .unwrap_or(false);
        assert!(!running, "Container should not be running after stop");

        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_orphaned_container_cleanup() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("test_orphaned_container"),
            )
            .await?;

        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect.state.and_then(|s| s.running).unwrap_or(false);
        assert!(!running, "Container should not be running");

        manager.remove(&container_id, false, true).await?;

        let result = manager.inspect_container(&container_id).await;
        assert!(result.is_err(), "Container should not exist after removal");

        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_cleanup_orphaned_containers_removes_stopped() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("ralph-orphaned-test"),
            )
            .await?;

        manager.start(&container_id).await?;
        manager.stop(&container_id, Some(10)).await?;

        let cleaned_count = manager.cleanup_orphaned_containers().await?;
        assert_eq!(cleaned_count, 1, "Should clean up one orphaned container");

        let result = manager.inspect_container(&container_id).await;
        assert!(result.is_err(), "Orphaned container should be removed");

        drop(temp_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_cleanup_orphaned_containers_skips_running() -> Result<()> {
        if !docker_available() {
            warn!("Docker not available, skipping test");
            return Ok(());
        }

        let manager = DockerManager::new();
        let (temp_dir, prd_path, task_path, repo_path) = setup_test_files().await?;

        let container_id = manager
            .create_container(
                "alpine:latest",
                prd_path.to_str().unwrap(),
                task_path.to_str().unwrap(),
                repo_path.to_str().unwrap(),
                Some("ralph-running-test"),
            )
            .await?;

        manager.start(&container_id).await?;

        let cleaned_count = manager.cleanup_orphaned_containers().await?;
        assert_eq!(cleaned_count, 0, "Should not clean up running containers");

        let inspect = manager.inspect_container(&container_id).await?;
        let running = inspect
            .state
            .and_then(|s| s.running)
            .ok_or_else(|| anyhow::anyhow!("No state info"))?;
        assert!(running, "Running container should still exist");

        manager.stop(&container_id, Some(10)).await?;
        manager.remove(&container_id, true, true).await?;
        drop(temp_dir);

        Ok(())
    }
}
