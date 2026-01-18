use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// File record - represents a file created/modified during an iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub iteration_id: String,
    pub path: String,
    pub content_hash: Option<String>,
    pub size: Option<i64>,
    pub file_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl File {
    /// Create a new File instance
    ///
    /// # Arguments
    /// * `iteration_id` - ID of the iteration that created this file
    /// * `path` - File path in the container
    ///
    /// # Returns
    /// New File instance with generated UUID and current timestamp
    ///
    /// # Example
    /// ```
    /// # use ralph_models::File;
    /// let file = File::new("iteration-id-123", "/workspace/src/main.rs");
    /// assert_eq!(file.iteration_id, "iteration-id-123");
    /// assert_eq!(file.path, "/workspace/src/main.rs");
    /// ```
    pub fn new(iteration_id: String, path: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            iteration_id,
            path,
            content_hash: None,
            size: None,
            file_type: None,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_new_generates_uuid() {
        let file = File::new("iteration-id".to_string(), "/path/to/file.rs".to_string());

        uuid::Uuid::parse_str(&file.id).expect("ID should be a valid UUID");
    }

    #[test]
    fn test_file_new_sets_fields() {
        let iteration_id = "iteration-123".to_string();
        let path = "/workspace/src/main.rs".to_string();
        let file = File::new(iteration_id.clone(), path.clone());

        assert_eq!(file.iteration_id, iteration_id);
        assert_eq!(file.path, path);
    }

    #[test]
    fn test_file_new_sets_created_at() {
        let before = Utc::now();
        let file = File::new("iteration-id".to_string(), "/path/to/file.rs".to_string());
        let after = Utc::now();

        assert!(file.created_at >= before);
        assert!(file.created_at <= after);
    }

    #[test]
    fn test_file_new_optional_fields_none() {
        let file = File::new("iteration-id".to_string(), "/path/to/file.rs".to_string());

        assert!(file.content_hash.is_none());
        assert!(file.size.is_none());
        assert!(file.file_type.is_none());
    }

    #[test]
    fn test_file_serialization() {
        let file = File {
            id: "file-123".to_string(),
            iteration_id: "iteration-123".to_string(),
            path: "/workspace/src/main.rs".to_string(),
            content_hash: Some("abc123".to_string()),
            size: Some(1024),
            file_type: Some("code".to_string()),
            created_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&file).expect("Failed to serialize");
        let deserialized: File = serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.id, file.id);
        assert_eq!(deserialized.iteration_id, file.iteration_id);
        assert_eq!(deserialized.path, file.path);
        assert_eq!(deserialized.content_hash, file.content_hash);
        assert_eq!(deserialized.size, file.size);
        assert_eq!(deserialized.file_type, file.file_type);
    }
}
