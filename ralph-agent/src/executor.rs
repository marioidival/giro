use anyhow::{Context, Result};
use bollard::{
    Docker,
    exec::{CreateExecOptions, StartExecOptions},
};
use futures::TryStreamExt;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

pub struct ExecutionContext {
    container_id: String,
    working_dir: String,
    env_vars: HashMap<String, String>,
    docker: Docker,
}

impl ExecutionContext {
    pub fn new(
        container_id: String,
        working_dir: String,
        env_vars: HashMap<String, String>,
    ) -> Self {
        let docker =
            Docker::connect_with_local_defaults().expect("Failed to connect to Docker daemon");

        Self {
            container_id,
            working_dir,
            env_vars,
            docker,
        }
    }

    async fn execute_in_container(&self, cmd: &[&str]) -> Result<String> {
        let env: Vec<String> = self
            .env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let mut full_cmd: Vec<&str> = vec!["/bin/sh", "-c"];
        full_cmd.extend_from_slice(cmd);

        debug!("Executing command in container: {:?}", full_cmd);

        let exec_options = CreateExecOptions {
            cmd: Some(full_cmd.into_iter().map(|s| s.to_string()).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            working_dir: Some(self.working_dir.clone()),
            env: Some(env),
            ..Default::default()
        };

        let exec_id = self
            .docker
            .create_exec(&self.container_id, exec_options)
            .await
            .context("Failed to create exec instance")?
            .id;

        debug!("Created exec instance: {}", exec_id);

        let exec_result = self
            .docker
            .start_exec(&exec_id, None::<StartExecOptions>)
            .await
            .context("Failed to start exec")?;

        let output = match exec_result {
            bollard::exec::StartExecResults::Attached { output, .. } => {
                let logs: Vec<u8> = output
                    .try_fold(Vec::new(), |mut acc, chunk| async move {
                        acc.extend_from_slice(&chunk.into_bytes());
                        Ok(acc)
                    })
                    .await
                    .context("Failed to read exec output")?;
                self.decode_docker_output(logs).await
            }
            bollard::exec::StartExecResults::Detached => Ok(String::new()),
        };

        let inspect = self
            .docker
            .inspect_exec(&exec_id)
            .await
            .context("Failed to inspect exec")?;

        if let Some(exit_code) = inspect.exit_code
            && exit_code != 0
        {
            let output_str = output.unwrap_or_else(|_| String::new());
            return Err(anyhow::anyhow!(
                "Command failed with exit code {}: {}",
                exit_code,
                output_str
            ));
        }

        output
    }

    async fn decode_docker_output(&self, log: Vec<u8>) -> Result<String> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let mut pos = 0;
        while pos + 8 <= log.len() {
            let header = &log[pos..pos + 8];
            let _stream_type = header[0];
            let _header_ignored = &header[1..4];
            let length = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;

            pos += 8;

            if pos + length <= log.len() {
                let data = &log[pos..pos + length];

                if header[0] == 1 {
                    stdout.extend_from_slice(data);
                } else if header[0] == 2 {
                    stderr.extend_from_slice(data);
                }

                pos += length;
            } else {
                break;
            }
        }

        if !stderr.is_empty() {
            warn!("Command stderr: {}", String::from_utf8_lossy(&stderr));
        }

        String::from_utf8(stdout)
            .map_err(|e| anyhow::anyhow!("Failed to decode output as UTF-8: {}", e))
    }

    pub async fn execute_command(&self, command: &str) -> Result<String> {
        info!("Executing command: {}", command);

        let result = timeout(
            Duration::from_secs(300),
            self.execute_in_container(&[command]),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after 300 seconds"))??;

        let trimmed = result.trim().to_string();
        debug!("Command output length: {} chars", trimmed.len());

        Ok(trimmed)
    }

    pub async fn read_file(&self, path: &str) -> Result<String> {
        info!("Reading file: {}", path);

        let full_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.working_dir, path)
        };

        let content = self
            .execute_in_container(&[&format!("cat {}", full_path)])
            .await
            .context("Failed to read file")?;

        Ok(content)
    }

    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        info!("Writing file: {} ({} bytes)", path, content.len());

        let full_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.working_dir, path)
        };

        let dir_path = std::path::Path::new(&full_path)
            .parent()
            .map(|p| p.to_str().unwrap_or(""))
            .unwrap_or("");

        let command = format!(
            "mkdir -p {} && echo '{}' | tee {} > /dev/null",
            dir_path,
            content.replace('\'', "'\\''"),
            full_path
        );

        self.execute_in_container(&[&command])
            .await
            .context("Failed to write file")?;

        Ok(())
    }

    pub async fn list_files(&self, path: Option<&str>) -> Result<Vec<String>> {
        let target_path = path.unwrap_or(&self.working_dir);
        info!("Listing files in: {}", target_path);

        let output = self
            .execute_in_container(&[&format!("ls -1 {}", target_path)])
            .await
            .context("Failed to list files")?;

        let files: Vec<String> = output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        debug!("Found {} files/directories", files.len());

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
    use bollard::image::CreateImageOptions;
    use futures::TryStreamExt;
    use tokio::time::sleep;
    use uuid::Uuid;

    async fn setup_test_container() -> Result<String> {
        let docker = Docker::connect_with_local_defaults()?;

        let container_name = format!("ralph-exec-test-{}", Uuid::new_v4());

        let image = if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            "alpine:latest"
        } else {
            return Err(anyhow::anyhow!("Test requires Docker on macOS/Linux"));
        };

        let pull_options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        docker
            .create_image(Some(pull_options), None, None)
            .try_collect::<Vec<_>>()
            .await?;

        let config = Config {
            image: Some(image),
            tty: Some(true),
            cmd: Some(vec!["/bin/sh", "-c", "tail -f /dev/null"]),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name.as_str(),
            platform: None,
        };

        let container = docker.create_container(Some(options), config).await?;

        docker
            .start_container(
                &container.id,
                None::<bollard::container::StartContainerOptions<String>>,
            )
            .await?;

        sleep(Duration::from_secs(2)).await;

        Ok(container.id)
    }

    async fn cleanup_container(container_id: &str) -> Result<()> {
        let docker = Docker::connect_with_local_defaults()?;

        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        docker.remove_container(container_id, Some(options)).await?;

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_command() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        let result = ctx
            .execute_command("echo 'Hello, World!'")
            .await
            .expect("Command should succeed");

        assert_eq!(result, "Hello, World!");

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_command_with_env_vars() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

        let ctx = ExecutionContext::new(container_id.clone(), "/workspace".to_string(), env_vars);

        let result = ctx
            .execute_command("echo $TEST_VAR")
            .await
            .expect("Command should succeed");

        assert_eq!(result, "test_value");

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_command_failure() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        let result = ctx.execute_command("exit 1").await;

        assert!(result.is_err());

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_read_file() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        ctx.execute_command("mkdir -p /workspace && echo 'test content' > /workspace/test.txt")
            .await
            .expect("Failed to create test file");

        let content = ctx
            .read_file("/workspace/test.txt")
            .await
            .expect("Failed to read file");

        assert_eq!(content, "test content");

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_write_file() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        ctx.write_file("/workspace/test.txt", "new content")
            .await
            .expect("Failed to write file");

        let content = ctx
            .execute_command("cat /workspace/test.txt")
            .await
            .expect("Failed to verify file");

        assert_eq!(content, "new content");

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_write_file_with_subdirs() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        ctx.write_file("/workspace/nested/deep/file.txt", "nested content")
            .await
            .expect("Failed to write file");

        let content = ctx
            .execute_command("cat /workspace/nested/deep/file.txt")
            .await
            .expect("Failed to verify file");

        assert_eq!(content, "nested content");

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_files() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        ctx.execute_command(
            "mkdir -p /workspace && touch /workspace/file1.txt /workspace/file2.txt",
        )
        .await
        .expect("Failed to setup test files");

        let files = ctx.list_files(None).await.expect("Failed to list files");

        assert_eq!(files.len(), 2);
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"file2.txt".to_string()));

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_files_with_path() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        ctx.execute_command("mkdir -p /workspace/subdir && touch /workspace/subdir/file.txt")
            .await
            .expect("Failed to setup test files");

        let files = ctx
            .list_files(Some("/workspace/subdir"))
            .await
            .expect("Failed to list files");

        assert_eq!(files.len(), 1);
        assert!(files.contains(&"file.txt".to_string()));

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_command_timeout() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        let result = timeout(Duration::from_secs(5), ctx.execute_command("sleep 10")).await;

        assert!(result.is_err());

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }

    #[tokio::test]
    #[ignore]
    async fn test_large_output() {
        let container_id = setup_test_container()
            .await
            .expect("Failed to setup test container");

        let ctx = ExecutionContext::new(
            container_id.clone(),
            "/workspace".to_string(),
            HashMap::new(),
        );

        ctx.execute_command("for i in $(seq 1 1000); do echo \"Line $i\"; done")
            .await
            .expect("Failed to generate large output");

        let output = ctx
            .execute_command("for i in $(seq 1 1000); do echo \"Line $i\"; done")
            .await
            .expect("Failed to get large output");

        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 1000"));
        assert_eq!(output.lines().count(), 1000);

        cleanup_container(&container_id)
            .await
            .expect("Failed to cleanup container");
    }
}
