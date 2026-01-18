use anyhow::{Context, Result};
use ralph_models::File;
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Debug, FromRow)]
struct FileRow {
    id: String,
    iteration_id: String,
    path: String,
    content_hash: Option<String>,
    size: Option<i64>,
    file_type: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<FileRow> for File {
    fn from(row: FileRow) -> Self {
        Self {
            id: row.id,
            iteration_id: row.iteration_id,
            path: row.path,
            content_hash: row.content_hash,
            size: row.size,
            file_type: row.file_type,
            created_at: row.created_at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileRepository {
    pool: Pool<Sqlite>,
}

impl FileRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, file: File) -> Result<File> {
        sqlx::query(
            r#"
            INSERT INTO files (id, iteration_id, path, content_hash, size, file_type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&file.id)
        .bind(&file.iteration_id)
        .bind(&file.path)
        .bind(&file.content_hash)
        .bind(file.size)
        .bind(&file.file_type)
        .bind(file.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert file into database")?;

        Ok(file)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<File>> {
        let result = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT id, iteration_id, path, content_hash, size, file_type, created_at
            FROM files
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query file by id")?;

        Ok(result.map(|row| row.into()))
    }

    pub async fn list_by_iteration(&self, iteration_id: &str) -> Result<Vec<File>> {
        let results = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT id, iteration_id, path, content_hash, size, file_type, created_at
            FROM files
            WHERE iteration_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(iteration_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query files by iteration")?;

        Ok(results.into_iter().map(|row| row.into()).collect())
    }

    pub async fn delete(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(r#"DELETE FROM files WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete file")?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_find_by_id() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = FileRepository::new(db.pool().clone());

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

        let iteration_repo = crate::iteration::IterationRepository::new(db.pool().clone());
        let iteration = ralph_models::Iteration {
            id: uuid::Uuid::new_v4().to_string(),
            loop_id: loop_.id.clone(),
            task_id: task.id.clone(),
            iteration_number: 1,
            output: Some("Test output".to_string()),
            error: None,
            status: ralph_models::IterationStatus::Completed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            tokens_used: Some(1000),
        };
        let created_iteration = iteration_repo.create(iteration).await?;

        let file = File {
            id: uuid::Uuid::new_v4().to_string(),
            iteration_id: created_iteration.id.clone(),
            path: "/workspace/src/main.rs".to_string(),
            content_hash: Some("abc123".to_string()),
            size: Some(1024),
            file_type: Some("code".to_string()),
            created_at: chrono::Utc::now(),
        };

        let created_file = repo.create(file.clone()).await?;

        let found_file = repo.find_by_id(&created_file.id).await?;
        assert!(found_file.is_some());
        let f = found_file.unwrap();
        assert_eq!(f.id, created_file.id);
        assert_eq!(f.iteration_id, created_iteration.id);
        assert_eq!(f.path, "/workspace/src/main.rs");
        assert_eq!(f.content_hash, Some("abc123".to_string()));
        assert_eq!(f.size, Some(1024));
        assert_eq!(f.file_type, Some("code".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_iteration() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = FileRepository::new(db.pool().clone());

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

        let iteration_repo = crate::iteration::IterationRepository::new(db.pool().clone());
        let iteration = ralph_models::Iteration {
            id: uuid::Uuid::new_v4().to_string(),
            loop_id: loop_.id.clone(),
            task_id: task.id.clone(),
            iteration_number: 1,
            output: Some("Test output".to_string()),
            error: None,
            status: ralph_models::IterationStatus::Completed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            tokens_used: Some(1000),
        };
        let created_iteration = iteration_repo.create(iteration).await?;

        for i in 1..=3 {
            let file = File {
                id: uuid::Uuid::new_v4().to_string(),
                iteration_id: created_iteration.id.clone(),
                path: format!("/workspace/src/file{}.rs", i),
                content_hash: Some(format!("hash{}", i)),
                size: Some(i * 100),
                file_type: Some("code".to_string()),
                created_at: chrono::Utc::now(),
            };
            repo.create(file).await?;
        }

        let files = repo.list_by_iteration(&created_iteration.id).await?;
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, "/workspace/src/file1.rs");
        assert_eq!(files[1].path, "/workspace/src/file2.rs");
        assert_eq!(files[2].path, "/workspace/src/file3.rs");

        Ok(())
    }
}
