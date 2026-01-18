use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub owner_id: Option<String>,
    pub prd: String,
    pub provider: String,
    pub model: String,
    pub docker_image: Option<String>,
    pub cpu_limit: Option<i32>,
    pub memory_limit: Option<i32>,
    pub max_iterations: Option<i32>,
    pub iteration_timeout: Option<i32>,
    pub iteration_delay: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LoopTemplate {
    pub fn new(create_template: CreateLoopTemplate) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: create_template.name,
            description: create_template.description,
            is_public: create_template.is_public,
            owner_id: create_template.owner_id,
            prd: create_template.prd,
            provider: create_template.provider,
            model: create_template.model,
            docker_image: create_template.docker_image,
            cpu_limit: create_template.cpu_limit,
            memory_limit: create_template.memory_limit,
            max_iterations: create_template.max_iterations,
            iteration_timeout: create_template.iteration_timeout,
            iteration_delay: create_template.iteration_delay,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateLoopTemplate {
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub owner_id: Option<String>,
    pub prd: String,
    pub provider: String,
    pub model: String,
    pub docker_image: Option<String>,
    pub cpu_limit: Option<i32>,
    pub memory_limit: Option<i32>,
    pub max_iterations: Option<i32>,
    pub iteration_timeout: Option<i32>,
    pub iteration_delay: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateLoopTemplate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub prd: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub docker_image: Option<String>,
    pub cpu_limit: Option<i32>,
    pub memory_limit: Option<i32>,
    pub max_iterations: Option<i32>,
    pub iteration_timeout: Option<i32>,
    pub iteration_delay: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_template_new_generates_unique_id() {
        let create_template = CreateLoopTemplate {
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: false,
            owner_id: Some("user123".to_string()),
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

        let template1 = LoopTemplate::new(create_template.clone());
        let template2 = LoopTemplate::new(create_template);

        assert_ne!(template1.id, template2.id);
    }

    #[test]
    fn test_loop_template_public_without_owner() {
        let create_template = CreateLoopTemplate {
            name: "Public Template".to_string(),
            description: "Public Description".to_string(),
            is_public: true,
            owner_id: None,
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

        let template = LoopTemplate::new(create_template);

        assert!(template.is_public);
        assert!(template.owner_id.is_none());
    }

    #[test]
    fn test_loop_template_private_with_owner() {
        let create_template = CreateLoopTemplate {
            name: "Private Template".to_string(),
            description: "Private Description".to_string(),
            is_public: false,
            owner_id: Some("user123".to_string()),
            prd: "Test PRD".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            docker_image: Some("custom-image:latest".to_string()),
            cpu_limit: Some(2),
            memory_limit: Some(2048),
            max_iterations: Some(50),
            iteration_timeout: Some(600),
            iteration_delay: Some(5),
        };

        let template = LoopTemplate::new(create_template);

        assert!(!template.is_public);
        assert_eq!(template.owner_id, Some("user123".to_string()));
        assert_eq!(
            template.docker_image,
            Some("custom-image:latest".to_string())
        );
        assert_eq!(template.cpu_limit, Some(2));
        assert_eq!(template.memory_limit, Some(2048));
    }

    #[test]
    fn test_loop_template_serialization() {
        let create_template = CreateLoopTemplate {
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: true,
            owner_id: None,
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

        let template = LoopTemplate::new(create_template);

        let json = serde_json::to_string(&template).unwrap();
        let deserialized: LoopTemplate = serde_json::from_str(&json).unwrap();

        assert_eq!(template.id, deserialized.id);
        assert_eq!(template.name, deserialized.name);
        assert_eq!(template.is_public, deserialized.is_public);
    }

    #[test]
    fn test_create_loop_template_serialization() {
        let create_template = CreateLoopTemplate {
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: false,
            owner_id: Some("user123".to_string()),
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

        let json = serde_json::to_string(&create_template).unwrap();
        let deserialized: CreateLoopTemplate = serde_json::from_str(&json).unwrap();

        assert_eq!(create_template, deserialized);
    }

    #[test]
    fn test_loop_template_timestamps() {
        let before_creation = Utc::now();
        let create_template = CreateLoopTemplate {
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: true,
            owner_id: None,
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

        let template = LoopTemplate::new(create_template);
        let after_creation = Utc::now();

        assert!(template.created_at >= before_creation);
        assert!(template.created_at <= after_creation);
        assert_eq!(template.created_at, template.updated_at);
    }
}
