use anyhow::{Context, Result, anyhow};
use git2::{Cred, Oid, Repository, build::RepoBuilder};
use ralph_models::GitCredential;
use ralph_repositories::crypto::decrypt_token;
use std::path::PathBuf;
use tokio::task::spawn_blocking;
use tracing::info;

/// GitService provides async wrappers for Git operations
///
/// This service handles all Git-related operations for Ralph Loop Manager,
/// including cloning repositories, creating branches, staging files,
/// committing changes, and pushing to remotes.
#[derive(Clone, Debug)]
pub struct GitService {
    /// Path to local git repository
    pub repo_path: PathBuf,

    /// Git credentials for authentication
    pub credentials: GitCredential,
}

impl GitService {
    /// Create a new GitService instance
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local git repository
    /// * `credentials` - Git credentials for authentication (encrypted tokens are used internally)
    pub fn new(repo_path: PathBuf, credentials: GitCredential) -> Self {
        Self {
            repo_path,
            credentials,
        }
    }

    /// Clone a Git repository into specified directory
    ///
    /// # Arguments
    /// * `url` - URL of the repository to clone
    ///
    /// # Returns
    /// Oid of the cloned repository's HEAD
    ///
    /// # Errors
    /// Returns error if clone fails, URL is invalid, or credentials are invalid
    pub async fn clone_repo(&self, url: &str) -> Result<Oid> {
        let url = url.to_string();
        let repo_path = self.repo_path.clone();

        let decrypted_token = decrypt_token(&self.credentials.encrypted_token)
            .context("Failed to decrypt Git credential token")?;

        spawn_blocking(move || {
            let auth_url = Self::build_auth_url(&url, &decrypted_token)?;

            let mut builder = RepoBuilder::new();
            let repo = builder
                .clone(&auth_url, &repo_path)
                .context("Failed to clone repository")?;

            let head = repo.head().context("Failed to get repository HEAD")?;
            let oid = head.target().context("HEAD is not a direct reference")?;

            Ok(oid)
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Create a new branch from current HEAD
    ///
    /// # Arguments
    /// * `branch_name` - Name of the branch to create
    ///
    /// # Returns
    /// Oid of the branch's commit
    ///
    /// # Errors
    /// Returns error if branch creation fails or repository is invalid
    pub async fn create_branch(&self, branch_name: &str) -> Result<Oid> {
        let repo_path = self.repo_path.clone();
        let branch_name = branch_name.to_string();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let head = repo.head().context("Failed to get repository HEAD")?;
            let head_oid = head.target().context("HEAD is not a direct reference")?;

            let commit = repo
                .find_commit(head_oid)
                .context("Failed to find HEAD commit")?;
            let branch = repo
                .branch(&branch_name, &commit, false)
                .context("Failed to create branch")?;

            let oid = branch.get().target().context("Branch has no target")?;

            Ok(oid)
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Checkout to a specific branch
    ///
    /// # Arguments
    /// * `branch_name` - Name of the branch to checkout
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// Returns error if checkout fails or branch doesn't exist
    pub async fn checkout(&self, branch_name: &str) -> Result<()> {
        let repo_path = self.repo_path.clone();
        let branch_name = branch_name.to_string();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let obj = repo
                .revparse_single(&branch_name)
                .context("Failed to find branch")?;

            let tree = obj.peel_to_tree().context("Failed to peel to tree")?;
            let tree_obj = tree.as_object();
            repo.checkout_tree(tree_obj, None)
                .context("Failed to checkout tree")?;
            repo.set_head(&format!("refs/heads/{}", branch_name))
                .context("Failed to set HEAD")?;

            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Stage all changes in the repository
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// Returns error if repository is invalid or staging fails
    pub async fn stage_all(&self) -> Result<()> {
        let repo_path = self.repo_path.clone();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let mut index = repo.index().context("Failed to get repository index")?;

            index
                .update_all(None::<&str>, None)
                .context("Failed to update index")?;
            index.write().context("Failed to write index")?;

            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Stage specific files in the repository
    ///
    /// # Arguments
    /// * `paths` - Slice of file paths to stage
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// Returns error if repository is invalid or files don't exist
    pub async fn stage_files(&self, paths: &[&str]) -> Result<()> {
        let repo_path = self.repo_path.clone();
        let paths: Vec<String> = paths.iter().map(|s| s.to_string()).collect();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let mut index = repo.index().context("Failed to get repository index")?;

            for path in &paths {
                index
                    .add_path(path.as_ref())
                    .with_context(|| format!("Failed to stage file: {}", path))?;
            }

            index.write().context("Failed to write index")?;

            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Create a commit with staged changes
    ///
    /// # Arguments
    /// * `message` - Commit message
    ///
    /// # Returns
    /// Oid of the created commit
    ///
    /// # Errors
    /// Returns error if commit creation fails or no staged changes
    pub async fn commit(&self, message: &str) -> Result<Oid> {
        let repo_path = self.repo_path.clone();
        let message = message.to_string();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let mut index = repo.index().context("Failed to get repository index")?;
            let tree_id = index.write_tree().context("Failed to write tree")?;

            let tree = repo.find_tree(tree_id).context("Failed to find tree")?;

            let head = repo.head().context("Failed to get repository HEAD")?;
            let head_oid = head.target().context("HEAD is not a direct reference")?;

            let parent_commit = repo
                .find_commit(head_oid)
                .context("Failed to find parent commit")?;

            let sig = repo.signature().context("Failed to create signature")?;

            let oid = repo
                .commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit])
                .context("Failed to create commit")?;

            Ok(oid)
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Push changes to a remote
    ///
    /// # Arguments
    /// * `remote_name` - Name of the remote (e.g., "origin")
    /// * `branch_name` - Name of the branch to push
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// Returns error if push fails or credentials are invalid
    pub async fn push(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        let repo_path = self.repo_path.clone();
        let remote_name = remote_name.to_string();
        let branch_name = branch_name.to_string();
        let encrypted_token = self.credentials.encrypted_token.clone();

        spawn_blocking(move || {
            let decrypted_token = decrypt_token(&encrypted_token)
                .context("Failed to decrypt Git credential token")?;

            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let mut remote = repo
                .find_remote(&remote_name)
                .with_context(|| format!("Failed to find remote: {}", remote_name))?;

            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                Cred::userpass_plaintext(
                    username_from_url.unwrap_or(&remote_name),
                    &decrypted_token,
                )
            });

            let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

            let mut push_options = git2::PushOptions::new();
            push_options.remote_callbacks(callbacks);

            remote
                .push(&[&refspec], Some(&mut push_options))
                .context("Failed to push to remote")?;

            Ok(())
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Get the name of the current branch
    ///
    /// # Returns
    /// Name of the current branch
    ///
    /// # Errors
    /// Returns error if HEAD is not a branch or repository is invalid
    pub async fn current_branch(&self) -> Result<String> {
        let repo_path = self.repo_path.clone();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let head = repo.head().context("Failed to get repository HEAD")?;

            if head.is_branch() {
                let shorthand = head.shorthand().context("Failed to get branch name")?;
                Ok(shorthand.to_string())
            } else {
                Err(anyhow!("HEAD is not a branch (detached state)"))
            }
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Get the OID of the current HEAD commit
    ///
    /// # Returns
    /// Oid of the current HEAD commit
    ///
    /// # Errors
    /// Returns error if HEAD is invalid or repository is invalid
    pub async fn current_commit(&self) -> Result<Oid> {
        let repo_path = self.repo_path.clone();

        spawn_blocking(move || {
            let repo = Repository::open(&repo_path).context("Failed to open repository")?;

            let head = repo.head().context("Failed to get repository HEAD")?;

            head.target().context("HEAD is not a direct reference")
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))?
    }

    /// Create a pull request for the current branch
    ///
    /// # Arguments
    /// * `title` - PR title
    /// * `body` - PR description/body
    ///
    /// # Returns
    /// PR URL on success
    ///
    /// # Errors
    /// Returns error if PR creation fails
    ///
    /// # Note
    /// This is a placeholder implementation. In a future sprint, this will be
    /// integrated with GitHub (octocrab) or GitLab (gitlab-sdk) APIs.
    pub async fn create_pr(&self, title: &str, _body: &str) -> Result<String> {
        let current_branch = self.current_branch().await?;
        let pr_url = format!(
            "https://github.com/{}/pull/new/{}",
            self.credentials.username.as_deref().unwrap_or("ralph"),
            current_branch
        );
        info!(
            "PR creation placeholder - would create PR '{}' at branch '{}'",
            title, current_branch
        );
        Ok(pr_url)
    }

    /// Build an authenticated URL with credentials embedded
    ///
    /// # Arguments
    /// * `url` - Original repository URL
    /// * `token` - Personal access token for authentication
    ///
    /// # Returns
    /// URL with token embedded for authentication
    ///
    /// # Note
    /// This method handles both HTTPS and HTTP URLs
    ///
    /// # Errors
    /// Returns error if URL format is unsupported
    fn build_auth_url(url: &str, token: &str) -> Result<String> {
        if url.starts_with("https://") {
            Ok(url.replace("https://", &format!("https://oauth2:{}@", token)))
        } else if url.starts_with("http://") {
            Ok(url.replace("http://", &format!("http://oauth2:{}@", token)))
        } else if url.starts_with("git@") || url.starts_with("ssh://") {
            Err(anyhow!("SSH URLs require SSH keys, not tokens"))
        } else {
            Err(anyhow!("Unsupported URL format: {}", url))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_service_new() {
        let repo_path = PathBuf::from("/tmp/test-repo");
        let credentials = GitCredential::new(
            "user123".to_string(),
            "github".to_string(),
            "encrypted_token".to_string(),
            Some("alice".to_string()),
            Some("alice@example.com".to_string()),
        );

        let service = GitService::new(repo_path.clone(), credentials);

        assert_eq!(service.repo_path, repo_path);
        assert_eq!(service.credentials.provider, "github");
        assert_eq!(service.credentials.username, Some("alice".to_string()));
    }

    #[test]
    fn test_build_auth_url_https() {
        let url = "https://github.com/user/repo.git";
        let token = "ghp_test_token";
        let result = GitService::build_auth_url(url, token);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "https://oauth2:ghp_test_token@github.com/user/repo.git"
        );
    }

    #[test]
    fn test_build_auth_url_http() {
        let url = "http://github.com/user/repo.git";
        let token = "ghp_test_token";
        let result = GitService::build_auth_url(url, token);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "http://oauth2:ghp_test_token@github.com/user/repo.git"
        );
    }

    #[test]
    fn test_build_auth_url_ssh_fails() {
        let url = "git@github.com:user/repo.git";
        let token = "ghp_test_token";
        let result = GitService::build_auth_url(url, token);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SSH URLs require"));
    }
}
