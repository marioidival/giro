use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            _ => Err(format!("Invalid TaskStatus: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub loop_id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: i32,
    pub parent_task_id: Option<String>,
    pub created_by: String,
    pub iteration_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(create_task: CreateTask) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            loop_id: create_task.loop_id,
            title: create_task.title,
            description: create_task.description,
            status: TaskStatus::Pending,
            priority: create_task.priority.unwrap_or(0),
            parent_task_id: create_task.parent_task_id,
            created_by: create_task.created_by,
            iteration_id: None,
            started_at: None,
            completed_at: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateTask {
    pub loop_id: String,
    pub title: String,
    pub description: String,
    pub priority: Option<i32>,
    pub parent_task_id: Option<String>,
    pub created_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new_generates_unique_id() {
        let create_task = CreateTask {
            loop_id: "loop123".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };

        let task1 = Task::new(create_task.clone());
        let task2 = Task::new(create_task);

        assert_ne!(task1.id, task2.id);
    }

    #[test]
    fn test_task_new_with_defaults() {
        let create_task = CreateTask {
            loop_id: "loop123".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };

        let task = Task::new(create_task);

        assert_eq!(task.priority, 0);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.parent_task_id.is_none());
        assert!(task.iteration_id.is_none());
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
        assert!(task.error_message.is_none());
    }

    #[test]
    fn test_task_new_with_custom_values() {
        let create_task = CreateTask {
            loop_id: "loop123".to_string(),
            title: "Custom Task".to_string(),
            description: "Custom description".to_string(),
            priority: Some(5),
            parent_task_id: Some("parent123".to_string()),
            created_by: "user".to_string(),
        };

        let task = Task::new(create_task);

        assert_eq!(task.title, "Custom Task");
        assert_eq!(task.loop_id, "loop123");
        assert_eq!(task.priority, 5);
        assert_eq!(task.parent_task_id, Some("parent123".to_string()));
        assert_eq!(task.created_by, "user");
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(format!("{}", TaskStatus::Pending), "pending");
        assert_eq!(format!("{}", TaskStatus::InProgress), "in_progress");
        assert_eq!(format!("{}", TaskStatus::Completed), "completed");
        assert_eq!(format!("{}", TaskStatus::Failed), "failed");
        assert_eq!(format!("{}", TaskStatus::Cancelled), "cancelled");
    }

    #[test]
    fn test_task_status_from_str() {
        assert_eq!(TaskStatus::from_str("pending"), Ok(TaskStatus::Pending));
        assert_eq!(
            TaskStatus::from_str("in_progress"),
            Ok(TaskStatus::InProgress)
        );
        assert_eq!(TaskStatus::from_str("Completed"), Ok(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_str("FAILED"), Ok(TaskStatus::Failed));
        assert_eq!(TaskStatus::from_str("cancelled"), Ok(TaskStatus::Cancelled));
    }

    #[test]
    fn test_task_status_from_str_invalid() {
        assert!(TaskStatus::from_str("invalid").is_err());
        assert!(TaskStatus::from_str("").is_err());
        assert!(TaskStatus::from_str("running").is_err());
    }

    #[test]
    fn test_task_status_roundtrip() {
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];

        for status in statuses {
            let string = format!("{}", status);
            let parsed = TaskStatus::from_str(&string).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::InProgress;

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: TaskStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_task_serialization() {
        let create_task = CreateTask {
            loop_id: "loop123".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };

        let task = Task::new(create_task);

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.title, deserialized.title);
        assert_eq!(task.status, deserialized.status);
        assert_eq!(task.loop_id, deserialized.loop_id);
    }

    #[test]
    fn test_create_task_serialization() {
        let create_task = CreateTask {
            loop_id: "loop123".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: Some(5),
            parent_task_id: Some("parent123".to_string()),
            created_by: "user".to_string(),
        };

        let json = serde_json::to_string(&create_task).unwrap();
        let deserialized: CreateTask = serde_json::from_str(&json).unwrap();

        assert_eq!(create_task, deserialized);
    }

    #[test]
    fn test_task_timestamps() {
        let before_creation = Utc::now();
        let create_task = CreateTask {
            loop_id: "loop123".to_string(),
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            priority: None,
            parent_task_id: None,
            created_by: "user".to_string(),
        };

        let task = Task::new(create_task);
        let after_creation = Utc::now();

        assert!(task.created_at >= before_creation);
        assert!(task.created_at <= after_creation);
        assert_eq!(task.created_at, task.updated_at);
    }
}
