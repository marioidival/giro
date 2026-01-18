use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Status of an iteration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq)]
pub enum IterationStatus {
    Running,
    Completed,
    Error,
}

impl FromStr for IterationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" => Ok(IterationStatus::Running),
            "completed" => Ok(IterationStatus::Completed),
            "error" => Ok(IterationStatus::Error),
            _ => Err(format!("Invalid iteration status: {}", s)),
        }
    }
}

impl fmt::Display for IterationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IterationStatus::Running => write!(f, "running"),
            IterationStatus::Completed => write!(f, "completed"),
            IterationStatus::Error => write!(f, "error"),
        }
    }
}

/// An iteration of a loop (execution of a task)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iteration {
    pub id: String,
    pub loop_id: String,
    pub task_id: String,
    pub iteration_number: i32,
    pub output: Option<String>,
    pub error: Option<String>,
    pub status: IterationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub tokens_used: Option<i32>,
}

impl Iteration {
    /// Create a new Iteration with generated id and current timestamp
    ///
    /// # Arguments
    /// * `loop_id` - Loop ID this iteration belongs to
    /// * `task_id` - Task ID this iteration executes
    /// * `iteration_number` - Sequential iteration number
    ///
    /// # Returns
    /// New Iteration instance
    pub fn new(loop_id: String, task_id: String, iteration_number: i32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            loop_id,
            task_id,
            iteration_number,
            output: None,
            error: None,
            status: IterationStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            tokens_used: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration_status_from_str() {
        assert_eq!(
            IterationStatus::from_str("running").unwrap(),
            IterationStatus::Running
        );
        assert_eq!(
            IterationStatus::from_str("completed").unwrap(),
            IterationStatus::Completed
        );
        assert_eq!(
            IterationStatus::from_str("error").unwrap(),
            IterationStatus::Error
        );
        assert!(IterationStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_iteration_status_to_string() {
        assert_eq!(IterationStatus::Running.to_string(), "running");
        assert_eq!(IterationStatus::Completed.to_string(), "completed");
        assert_eq!(IterationStatus::Error.to_string(), "error");
    }

    #[test]
    fn test_iteration_new() {
        let iteration = Iteration::new("loop123".to_string(), "task456".to_string(), 1);

        assert_eq!(iteration.loop_id, "loop123");
        assert_eq!(iteration.task_id, "task456");
        assert_eq!(iteration.iteration_number, 1);
        assert_eq!(iteration.status, IterationStatus::Running);
        assert!(iteration.started_at <= Utc::now());
        assert!(iteration.completed_at.is_none());
        assert!(iteration.output.is_none());
        assert!(iteration.error.is_none());
        assert!(iteration.tokens_used.is_none());
    }
}
