use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LoopStatus {
    Created,
    Running,
    Paused,
    Completed,
    Error,
}

impl fmt::Display for LoopStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopStatus::Created => write!(f, "created"),
            LoopStatus::Running => write!(f, "running"),
            LoopStatus::Paused => write!(f, "paused"),
            LoopStatus::Completed => write!(f, "completed"),
            LoopStatus::Error => write!(f, "error"),
        }
    }
}

impl FromStr for LoopStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "created" => Ok(LoopStatus::Created),
            "running" => Ok(LoopStatus::Running),
            "paused" => Ok(LoopStatus::Paused),
            "completed" => Ok(LoopStatus::Completed),
            "error" => Ok(LoopStatus::Error),
            _ => Err(format!("Invalid LoopStatus: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Loop {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub prd: String,
    pub owner_id: String,
    pub provider: String,
    pub model: String,
    pub docker_image: String,
    pub cpu_limit: i32,
    pub memory_limit: i32,
    pub max_iterations: i32,
    pub iteration_timeout: i32,
    pub iteration_delay: i32,
    pub status: LoopStatus,
    pub current_iteration: i32,
    pub container_id: Option<String>,
    pub git_repo_url: Option<String>,
    pub git_branch_pattern: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Loop {
    pub fn new(create_loop: CreateLoop) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: create_loop.name,
            description: create_loop.description,
            prd: create_loop.prd,
            owner_id: create_loop.owner_id,
            provider: create_loop.provider,
            model: create_loop.model,
            docker_image: create_loop
                .docker_image
                .unwrap_or_else(|| "ralph-loop-manager:latest".to_string()),
            cpu_limit: create_loop.cpu_limit.unwrap_or(1),
            memory_limit: create_loop.memory_limit.unwrap_or(1024),
            max_iterations: create_loop.max_iterations.unwrap_or(100),
            iteration_timeout: create_loop.iteration_timeout.unwrap_or(300),
            iteration_delay: create_loop.iteration_delay.unwrap_or(0),
            status: LoopStatus::Created,
            current_iteration: 0,
            container_id: None,
            git_repo_url: create_loop.git_repo_url,
            git_branch_pattern: create_loop
                .git_branch_pattern
                .unwrap_or_else(|| "ralph/{loop_id}/{timestamp}".to_string()),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateLoop {
    pub name: String,
    pub description: Option<String>,
    pub prd: String,
    pub owner_id: String,
    pub provider: String,
    pub model: String,
    pub docker_image: Option<String>,
    pub cpu_limit: Option<i32>,
    pub memory_limit: Option<i32>,
    pub max_iterations: Option<i32>,
    pub iteration_timeout: Option<i32>,
    pub iteration_delay: Option<i32>,
    pub git_repo_url: Option<String>,
    pub git_branch_pattern: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_new_generates_unique_id() {
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: "user123".to_string(),
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

        let loop1 = Loop::new(create_loop.clone());
        let loop2 = Loop::new(create_loop);

        assert_ne!(loop1.id, loop2.id);
    }

    #[test]
    fn test_loop_new_with_defaults() {
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: "user123".to_string(),
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

        let loop_ = Loop::new(create_loop);

        assert_eq!(loop_.docker_image, "ralph-loop-manager:latest");
        assert_eq!(loop_.cpu_limit, 1);
        assert_eq!(loop_.memory_limit, 1024);
        assert_eq!(loop_.max_iterations, 100);
        assert_eq!(loop_.iteration_timeout, 300);
        assert_eq!(loop_.iteration_delay, 0);
        assert_eq!(loop_.current_iteration, 0);
        assert_eq!(loop_.status, LoopStatus::Created);
        assert!(loop_.container_id.is_none());
    }

    #[test]
    fn test_loop_new_with_custom_values() {
        let create_loop = CreateLoop {
            name: "Custom Loop".to_string(),
            description: Some("Custom description".to_string()),
            prd: "Custom PRD".to_string(),
            owner_id: "user456".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            docker_image: Some("custom-image:latest".to_string()),
            cpu_limit: Some(2),
            memory_limit: Some(2048),
            max_iterations: Some(50),
            iteration_timeout: Some(600),
            iteration_delay: Some(5),
            git_repo_url: Some("https://github.com/test/repo.git".to_string()),
            git_branch_pattern: Some("custom/{loop_id}".to_string()),
        };

        let loop_ = Loop::new(create_loop);

        assert_eq!(loop_.name, "Custom Loop");
        assert_eq!(loop_.description, Some("Custom description".to_string()));
        assert_eq!(loop_.docker_image, "custom-image:latest");
        assert_eq!(loop_.cpu_limit, 2);
        assert_eq!(loop_.memory_limit, 2048);
        assert_eq!(loop_.max_iterations, 50);
        assert_eq!(loop_.iteration_timeout, 600);
        assert_eq!(loop_.iteration_delay, 5);
        assert_eq!(
            loop_.git_repo_url,
            Some("https://github.com/test/repo.git".to_string())
        );
        assert_eq!(loop_.git_branch_pattern, "custom/{loop_id}");
    }

    #[test]
    fn test_loop_status_display() {
        assert_eq!(format!("{}", LoopStatus::Created), "created");
        assert_eq!(format!("{}", LoopStatus::Running), "running");
        assert_eq!(format!("{}", LoopStatus::Paused), "paused");
        assert_eq!(format!("{}", LoopStatus::Completed), "completed");
        assert_eq!(format!("{}", LoopStatus::Error), "error");
    }

    #[test]
    fn test_loop_status_from_str() {
        assert_eq!(LoopStatus::from_str("created"), Ok(LoopStatus::Created));
        assert_eq!(LoopStatus::from_str("Running"), Ok(LoopStatus::Running));
        assert_eq!(LoopStatus::from_str("PAUSED"), Ok(LoopStatus::Paused));
        assert_eq!(LoopStatus::from_str("completed"), Ok(LoopStatus::Completed));
        assert_eq!(LoopStatus::from_str("error"), Ok(LoopStatus::Error));
    }

    #[test]
    fn test_loop_status_from_str_invalid() {
        assert!(LoopStatus::from_str("invalid").is_err());
        assert!(LoopStatus::from_str("").is_err());
        assert!(LoopStatus::from_str("pending").is_err());
    }

    #[test]
    fn test_loop_status_roundtrip() {
        let statuses = vec![
            LoopStatus::Created,
            LoopStatus::Running,
            LoopStatus::Paused,
            LoopStatus::Completed,
            LoopStatus::Error,
        ];

        for status in statuses {
            let string = format!("{}", status);
            let parsed = LoopStatus::from_str(&string).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_loop_status_serialization() {
        let status = LoopStatus::Running;

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: LoopStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_loop_serialization() {
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: Some("Test description".to_string()),
            prd: "Test PRD".to_string(),
            owner_id: "user123".to_string(),
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

        let loop_ = Loop::new(create_loop);

        let json = serde_json::to_string(&loop_).unwrap();
        let deserialized: Loop = serde_json::from_str(&json).unwrap();

        assert_eq!(loop_.id, deserialized.id);
        assert_eq!(loop_.name, deserialized.name);
        assert_eq!(loop_.prd, deserialized.prd);
        assert_eq!(loop_.status, deserialized.status);
    }

    #[test]
    fn test_create_loop_serialization() {
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: Some("Test description".to_string()),
            prd: "Test PRD".to_string(),
            owner_id: "user123".to_string(),
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

        let json = serde_json::to_string(&create_loop).unwrap();
        let deserialized: CreateLoop = serde_json::from_str(&json).unwrap();

        assert_eq!(create_loop, deserialized);
    }

    #[test]
    fn test_loop_timestamps() {
        let before_creation = Utc::now();
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: "user123".to_string(),
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

        let loop_ = Loop::new(create_loop);
        let after_creation = Utc::now();

        assert!(loop_.created_at >= before_creation);
        assert!(loop_.created_at <= after_creation);
        assert_eq!(loop_.created_at, loop_.updated_at);
    }
}
