use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ralph_models::{CreateLoopTemplate, LoopTemplate};
use sqlx::{Pool, Sqlite};

/// Repository for loop template database operations
#[derive(Clone, Debug)]
pub struct LoopTemplateRepository {
    pool: Pool<Sqlite>,
}

impl LoopTemplateRepository {
    /// Create a new LoopTemplateRepository instance
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new loop template in database
    pub async fn create(&self, create_template: CreateLoopTemplate) -> Result<LoopTemplate> {
        let template = LoopTemplate::new(create_template);

        sqlx::query!(
            r#"
            INSERT INTO loop_templates (id, name, description, is_public, owner_id, prd, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations, iteration_timeout, iteration_delay,
                created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            template.id,
            template.name,
            template.description,
            template.is_public,
            template.owner_id,
            template.prd,
            template.provider,
            template.model,
            template.docker_image,
            template.cpu_limit,
            template.memory_limit,
            template.max_iterations,
            template.iteration_timeout,
            template.iteration_delay,
            template.created_at,
            template.updated_at,
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert loop template into database")?;

        Ok(template)
    }

    /// List all public templates and user's private templates
    pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<LoopTemplate>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                bool,
                Option<String>,
                String,
                String,
                String,
                Option<String>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, name, description, is_public, owner_id, prd, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations, iteration_timeout, iteration_delay,
                created_at, updated_at
            FROM loop_templates
            WHERE is_public = 1 OR owner_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query loop templates")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    name,
                    description,
                    is_public,
                    owner_id,
                    prd,
                    provider,
                    model,
                    docker_image,
                    cpu_limit,
                    memory_limit,
                    max_iterations,
                    iteration_timeout,
                    iteration_delay,
                    created_at,
                    updated_at,
                )| {
                    LoopTemplate {
                        id,
                        name,
                        description,
                        is_public,
                        owner_id,
                        prd,
                        provider,
                        model,
                        docker_image,
                        cpu_limit,
                        memory_limit,
                        max_iterations,
                        iteration_timeout,
                        iteration_delay,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }

    /// Find a template by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Option<LoopTemplate>> {
        let result = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                bool,
                Option<String>,
                String,
                String,
                String,
                Option<String>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, name, description, is_public, owner_id, prd, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations, iteration_timeout, iteration_delay,
                created_at, updated_at
            FROM loop_templates
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query loop template by id")?;

        Ok(result.map(
            |(
                id,
                name,
                description,
                is_public,
                owner_id,
                prd,
                provider,
                model,
                docker_image,
                cpu_limit,
                memory_limit,
                max_iterations,
                iteration_timeout,
                iteration_delay,
                created_at,
                updated_at,
            )| {
                LoopTemplate {
                    id,
                    name,
                    description,
                    is_public,
                    owner_id,
                    prd,
                    provider,
                    model,
                    docker_image,
                    cpu_limit,
                    memory_limit,
                    max_iterations,
                    iteration_timeout,
                    iteration_delay,
                    created_at,
                    updated_at,
                }
            },
        ))
    }

    /// Delete a template by ID with ownership check
    pub async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM loop_templates
            WHERE id = ? AND owner_id = ?
            "#,
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete loop template")?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "Loop template not found or unauthorized: id={}, user_id={}",
                id,
                user_id
            );
        }

        Ok(())
    }

    /// List all templates owned by a user
    pub async fn list_by_owner(&self, user_id: &str) -> Result<Vec<LoopTemplate>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                bool,
                Option<String>,
                String,
                String,
                String,
                Option<String>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT id, name, description, is_public, owner_id, prd, provider, model,
                docker_image, cpu_limit, memory_limit, max_iterations, iteration_timeout, iteration_delay,
                created_at, updated_at
            FROM loop_templates
            WHERE owner_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query loop templates by owner")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    name,
                    description,
                    is_public,
                    owner_id,
                    prd,
                    provider,
                    model,
                    docker_image,
                    cpu_limit,
                    memory_limit,
                    max_iterations,
                    iteration_timeout,
                    iteration_delay,
                    created_at,
                    updated_at,
                )| {
                    LoopTemplate {
                        id,
                        name,
                        description,
                        is_public,
                        owner_id,
                        prd,
                        provider,
                        model,
                        docker_image,
                        cpu_limit,
                        memory_limit,
                        max_iterations,
                        iteration_timeout,
                        iteration_delay,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    async fn create_test_user(db: &crate::database::Database) -> Result<ralph_models::User> {
        let user_repo = crate::user::UserRepository::new(db.pool().clone());
        let suffix = Uuid::new_v4();
        user_repo
            .create(ralph_models::CreateUser {
                username: format!("alice_{}", suffix),
                email: format!("alice_{}@example.com", suffix),
                password: "hashed_password".to_string(),
            })
            .await
    }

    #[tokio::test]
    async fn test_create_and_find_template() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopTemplateRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let create_template = CreateLoopTemplate {
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: false,
            owner_id: Some(user.id.clone()),
            prd: "Test PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        };

        let created = repo.create(create_template).await?;
        let found = repo.find_by_id(&created.id).await?;

        assert!(found.is_some());
        let template = found.unwrap();
        assert_eq!(template.id, created.id);
        assert_eq!(template.name, created.name);
        assert!(!template.is_public);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_for_user_includes_public_and_owned() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopTemplateRepository::new(db.pool().clone());

        let user1 = create_test_user(&db).await?;
        let user2 = create_test_user(&db).await?;

        // Create public template (no owner)
        repo.create(CreateLoopTemplate {
            name: "Public Template".to_string(),
            description: "Public".to_string(),
            is_public: true,
            owner_id: None,
            prd: "PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        })
        .await?;

        // Create user1's private template
        repo.create(CreateLoopTemplate {
            name: "User1 Private".to_string(),
            description: "Private".to_string(),
            is_public: false,
            owner_id: Some(user1.id.clone()),
            prd: "PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        })
        .await?;

        // Create user2's private template
        repo.create(CreateLoopTemplate {
            name: "User2 Private".to_string(),
            description: "Private".to_string(),
            is_public: false,
            owner_id: Some(user2.id.clone()),
            prd: "PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        })
        .await?;

        // user1 should see public + their own
        let user1_templates = repo.list_for_user(&user1.id).await?;
        assert_eq!(user1_templates.len(), 2);

        // user2 should see public + their own
        let user2_templates = repo.list_for_user(&user2.id).await?;
        assert_eq!(user2_templates.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_template() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopTemplateRepository::new(db.pool().clone());

        let user = create_test_user(&db).await?;

        let template = repo
            .create(CreateLoopTemplate {
                name: "Test Template".to_string(),
                description: "Test".to_string(),
                is_public: false,
                owner_id: Some(user.id.clone()),
                prd: "PRD".to_string(),
                provider: "claude".to_string(),
                model: "claude-3-opus".to_string(),
                docker_image: None,
                cpu_limit: None,
                memory_limit: None,
                max_iterations: None,
                iteration_timeout: None,
                iteration_delay: None,
            })
            .await?;

        repo.delete(&template.id, &user.id).await?;

        let found = repo.find_by_id(&template.id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_template_unauthorized_fails() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopTemplateRepository::new(db.pool().clone());

        let user1 = create_test_user(&db).await?;
        let user2 = create_test_user(&db).await?;

        let template = repo
            .create(CreateLoopTemplate {
                name: "Test Template".to_string(),
                description: "Test".to_string(),
                is_public: false,
                owner_id: Some(user1.id.clone()),
                prd: "PRD".to_string(),
                provider: "claude".to_string(),
                model: "claude-3-opus".to_string(),
                docker_image: None,
                cpu_limit: None,
                memory_limit: None,
                max_iterations: None,
                iteration_timeout: None,
                iteration_delay: None,
            })
            .await?;

        let result = repo.delete(&template.id, &user2.id).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found or unauthorized")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_owner() -> Result<()> {
        let db = crate::database::Database::new("sqlite::memory:").await?;
        let repo = LoopTemplateRepository::new(db.pool().clone());

        let user1 = create_test_user(&db).await?;
        let user2 = create_test_user(&db).await?;

        // Create templates for user1
        repo.create(CreateLoopTemplate {
            name: "User1 Template 1".to_string(),
            description: "Test".to_string(),
            is_public: false,
            owner_id: Some(user1.id.clone()),
            prd: "PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        })
        .await?;

        repo.create(CreateLoopTemplate {
            name: "User1 Template 2".to_string(),
            description: "Test".to_string(),
            is_public: true,
            owner_id: Some(user1.id.clone()),
            prd: "PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        })
        .await?;

        // Create template for user2
        repo.create(CreateLoopTemplate {
            name: "User2 Template".to_string(),
            description: "Test".to_string(),
            is_public: false,
            owner_id: Some(user2.id.clone()),
            prd: "PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: None,
            cpu_limit: None,
            memory_limit: None,
            max_iterations: None,
            iteration_timeout: None,
            iteration_delay: None,
        })
        .await?;

        let user1_templates = repo.list_by_owner(&user1.id).await?;
        assert_eq!(user1_templates.len(), 2);
        assert!(
            user1_templates
                .iter()
                .all(|t| t.owner_id.as_ref() == Some(&user1.id))
        );

        let user2_templates = repo.list_by_owner(&user2.id).await?;
        assert_eq!(user2_templates.len(), 1);

        Ok(())
    }
}
