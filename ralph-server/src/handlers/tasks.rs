//! Task CRUD HTTP handlers for Ralph Loop Manager server.
//!
//! This module provides handlers for creating, listing, retrieving, and deleting tasks.
//! All handlers require authentication via auth middleware and check ownership
//! to ensure users can only access their own loop's tasks.

use askama::Template;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use ralph_models::CreateTask;
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
use crate::handlers::loops::LoopDetail;
use crate::middleware::csrf::CsrfToken;
use crate::templates::NewTaskTemplate;

/// Query parameters for listing tasks with pagination.
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    /// Page number (1-based, defaults to 1)
    #[serde(default = "default_page")]
    pub page: usize,
    /// Number of items per page (defaults to 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Response structure for creating a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateTaskResponse {
    /// Whether creation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The ID of newly created task (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Response structure for listing tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListTasksResponse {
    /// Whether listing was successful
    pub success: bool,
    /// The list of tasks
    pub tasks: Vec<TaskSummary>,
    /// Current page number
    pub page: usize,
    /// Number of items per page
    pub limit: usize,
    /// Total number of tasks for the loop
    pub total: usize,
}

/// A lightweight task representation for list responses (without description, error_message).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskSummary {
    pub id: String,
    pub loop_id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    pub parent_task_id: Option<String>,
    pub created_by: String,
    pub iteration_id: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ralph_models::Task> for TaskSummary {
    fn from(task: ralph_models::Task) -> Self {
        Self {
            id: task.id,
            loop_id: task.loop_id,
            title: task.title,
            status: task.status.to_string(),
            priority: task.priority,
            parent_task_id: task.parent_task_id,
            created_by: task.created_by,
            iteration_id: task.iteration_id,
            started_at: task.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: task.completed_at.map(|dt| dt.to_rfc3339()),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for retrieving a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetTaskResponse {
    /// Whether retrieval was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The task (if found and user owns the loop)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskDetail>,
}

/// A detailed task representation with full description and error_message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskDetail {
    pub id: String,
    pub loop_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub parent_task_id: Option<String>,
    pub created_by: String,
    pub iteration_id: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ralph_models::Task> for TaskDetail {
    fn from(task: ralph_models::Task) -> Self {
        Self {
            id: task.id,
            loop_id: task.loop_id,
            title: task.title,
            description: task.description,
            status: task.status.to_string(),
            priority: task.priority,
            parent_task_id: task.parent_task_id,
            created_by: task.created_by,
            iteration_id: task.iteration_id,
            started_at: task.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: task.completed_at.map(|dt| dt.to_rfc3339()),
            error_message: task.error_message,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for deleting a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteTaskResponse {
    /// Whether deletion was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
}

/// Default page number
fn default_page() -> usize {
    1
}

/// Default limit
fn default_limit() -> usize {
    10
}

/// Handles task creation requests.
///
/// This endpoint:
/// 1. Validates title (required)
/// 2. Extracts user_id from auth middleware
/// 3. Extracts loop_id from path
/// 4. Checks loop ownership via loop_repo.find_by_id(loop_id)
/// 5. Returns 401 if user doesn't own the loop
/// 6. Creates Task from payload with created_by = "user" and loop_id from path
/// 7. Calls task_repo.create()
/// 8. Returns task ID on success
///
/// # Arguments
/// * `payload` - The task creation data (JSON)
/// * `state` - The application state containing task and loop repositories
/// * `user_id` - The authenticated user's ID (from auth middleware)
/// * `loop_id` - The loop ID (from path)
///
/// # Returns
/// * `201 Created` with task ID on success
/// * `400 Bad Request` for validation errors
/// * `401 Unauthorized` if user doesn't own the loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn create_task(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(loop_id): Path<String>,
    Json(payload): Json<CreateTask>,
) -> (StatusCode, Json<CreateTaskResponse>) {
    if payload.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreateTaskResponse {
                success: false,
                message: "Title is required".to_string(),
                task_id: None,
            }),
        );
    }

    match state.loop_repository.find_by_id(&loop_id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(CreateTaskResponse {
                        success: false,
                        message: "You do not have permission to add tasks to this loop".to_string(),
                        task_id: None,
                    }),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(CreateTaskResponse {
                    success: false,
                    message: "Loop not found".to_string(),
                    task_id: None,
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateTaskResponse {
                    success: false,
                    message: format!("Failed to verify loop: {}", e),
                    task_id: None,
                }),
            );
        }
    }

    let create_task = CreateTask {
        loop_id: loop_id.clone(),
        title: payload.title,
        description: payload.description,
        priority: payload.priority,
        parent_task_id: payload.parent_task_id,
        created_by: "user".to_string(),
    };

    match state.task_repository.create(create_task).await {
        Ok(task) => (
            StatusCode::CREATED,
            Json(CreateTaskResponse {
                success: true,
                message: "Task created successfully".to_string(),
                task_id: Some(task.id),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateTaskResponse {
                success: false,
                message: format!("Failed to create task: {}", e),
                task_id: None,
            }),
        ),
    }
}

/// Handles task listing requests with pagination.
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Extracts loop_id from path
/// 3. Checks loop ownership
/// 4. Returns 401 if user doesn't own the loop
/// 5. Parses pagination params (page, limit) from query params
/// 6. Calls task_repo.list_by_loop(loop_id)
/// 7. Returns paginated results
///
/// Note: list_by_loop returns Vec<Task> with empty description and error_message for performance.
///
/// # Arguments
/// * `query` - Pagination query parameters (page, limit)
/// * `state` - The application state containing task and loop repositories
/// * `user_id` - The authenticated user's ID (from auth middleware)
/// * `loop_id` - The loop ID (from path)
///
/// # Returns
/// * `200 OK` with paginated task list on success
/// * `401 Unauthorized` if user doesn't own the loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn list_tasks(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(loop_id): Path<String>,
    Query(q): Query<ListTasksQuery>,
) -> (StatusCode, Json<ListTasksResponse>) {
    match state.loop_repository.find_by_id(&loop_id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ListTasksResponse {
                        success: false,
                        tasks: Vec::new(),
                        page: q.page,
                        limit: q.limit,
                        total: 0,
                    }),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ListTasksResponse {
                    success: false,
                    tasks: Vec::new(),
                    page: q.page,
                    limit: q.limit,
                    total: 0,
                }),
            );
        }
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ListTasksResponse {
                    success: false,
                    tasks: Vec::new(),
                    page: q.page,
                    limit: q.limit,
                    total: 0,
                }),
            );
        }
    }

    match state.task_repository.list_by_loop(&loop_id).await {
        Ok(mut all_tasks) => {
            let total = all_tasks.len();

            let start = if q.page > 0 {
                (q.page - 1) * q.limit
            } else {
                0
            };

            let end = std::cmp::min(start + q.limit, total);

            let tasks: Vec<TaskSummary> = if start < total {
                all_tasks.drain(start..end).map(TaskSummary::from).collect()
            } else {
                Vec::new()
            };

            (
                StatusCode::OK,
                Json(ListTasksResponse {
                    success: true,
                    tasks,
                    page: q.page,
                    limit: q.limit,
                    total,
                }),
            )
        }
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListTasksResponse {
                success: false,
                tasks: Vec::new(),
                page: q.page,
                limit: q.limit,
                total: 0,
            }),
        ),
    }
}

/// Handles retrieving a specific task.
///
/// This endpoint:
/// 1. Calls task_repo.find_by_id(task_id)
/// 2. Checks loop ownership via task.loop_id -> loop_repo.find_by_id
/// 3. Returns 404 if task not found
/// 4. Returns 401 if user doesn't own the loop
/// 5. Returns TaskDetail with full description and error_message on success
///
/// # Arguments
/// * `path` - Path parameters containing task ID
/// * `state` - The application state containing task and loop repositories
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with task details on success
/// * `401 Unauthorized` if user doesn't own the loop
/// * `404 Not Found` if task or loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn get_task(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<GetTaskResponse>) {
    match state.task_repository.find_by_id(&task_id).await {
        Ok(Some(task)) => match state.loop_repository.find_by_id(&task.loop_id).await {
            Ok(Some(loop_)) => {
                if loop_.owner_id != user_id {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(GetTaskResponse {
                            success: false,
                            message: "You do not have permission to access this task".to_string(),
                            task: None,
                        }),
                    );
                }

                (
                    StatusCode::OK,
                    Json(GetTaskResponse {
                        success: true,
                        message: "Task retrieved successfully".to_string(),
                        task: Some(TaskDetail::from(task)),
                    }),
                )
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(GetTaskResponse {
                    success: false,
                    message: "Loop not found".to_string(),
                    task: None,
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetTaskResponse {
                    success: false,
                    message: format!("Failed to verify loop ownership: {}", e),
                    task: None,
                }),
            ),
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(GetTaskResponse {
                success: false,
                message: "Task not found".to_string(),
                task: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetTaskResponse {
                success: false,
                message: format!("Failed to retrieve task: {}", e),
                task: None,
            }),
        ),
    }
}

/// Handles deleting a task.
///
/// This endpoint:
/// 1. Calls task_repo.find_by_id(task_id) to check ownership
/// 2. Returns 404 if task not found
/// 3. Checks loop ownership via task.loop_id
/// 4. Returns 401 if user doesn't own the loop
/// 5. Calls task_repo.delete(task_id)
/// 6. Checks rows_affected and returns 404 if 0
///
/// # Arguments
/// * `path` - Path parameters containing task ID
/// * `state` - The application state containing task and loop repositories
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful deletion
/// * `401 Unauthorized` if user doesn't own the loop
/// * `404 Not Found` if task or loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn delete_task(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(task_id): Path<String>,
) -> (StatusCode, Json<DeleteTaskResponse>) {
    match state.task_repository.find_by_id(&task_id).await {
        Ok(Some(task)) => match state.loop_repository.find_by_id(&task.loop_id).await {
            Ok(Some(loop_)) => {
                if loop_.owner_id != user_id {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(DeleteTaskResponse {
                            success: false,
                            message: "You do not have permission to delete this task".to_string(),
                        }),
                    );
                }

                match state.task_repository.delete(&task_id).await {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            (
                                StatusCode::OK,
                                Json(DeleteTaskResponse {
                                    success: true,
                                    message: "Task deleted successfully".to_string(),
                                }),
                            )
                        } else {
                            (
                                StatusCode::NOT_FOUND,
                                Json(DeleteTaskResponse {
                                    success: false,
                                    message: "Task not found".to_string(),
                                }),
                            )
                        }
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(DeleteTaskResponse {
                            success: false,
                            message: format!("Failed to delete task: {}", e),
                        }),
                    ),
                }
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(DeleteTaskResponse {
                    success: false,
                    message: "Loop not found".to_string(),
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DeleteTaskResponse {
                    success: false,
                    message: format!("Failed to verify loop ownership: {}", e),
                }),
            ),
        },
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(DeleteTaskResponse {
                success: false,
                message: "Task not found".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeleteTaskResponse {
                success: false,
                message: format!("Failed to retrieve task: {}", e),
            }),
        ),
    }
}

/// Handles rendering the new task form page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Checks if user is logged in
/// 3. Generates CSRF token
/// 4. Retrieves loop by id with ownership check
/// 5. Retrieves existing tasks for parent task dropdown
/// 6. Renders the new task form template
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing repositories
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `401 Unauthorized` if user doesn't own the loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn new_task_form(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(loop_id): Path<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    // Get loop with ownership check
    let loop_detail = match state.loop_repository.find_by_id(&loop_id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Html("You do not have permission to add tasks to this loop".to_string()),
                );
            }
            LoopDetail::from(loop_)
        }
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Html("Loop not found".to_string()));
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("Failed to retrieve loop: {}", e)),
            );
        }
    };

    // Get existing tasks for parent task dropdown
    let existing_tasks = match state.task_repository.list_by_loop(&loop_id).await {
        Ok(task_list) => task_list
            .into_iter()
            .map(TaskSummary::from)
            .collect::<Vec<_>>(),
        Err(_e) => Vec::new(),
    };

    let csrf_token = CsrfToken::generate().to_string();
    let loop_name = loop_detail.name.clone();
    let empty_errors: Vec<String> = Vec::new();

    // Use Box::leak for 'static lifetime required by template
    // This is safe because the template is rendered immediately and dropped
    let existing_tasks_static: &'static [TaskSummary] =
        Box::leak(existing_tasks.into_boxed_slice());
    let errors_static: &'static [String] = Box::leak(empty_errors.into_boxed_slice());

    let template = NewTaskTemplate {
        logged_in,
        csrf_token,
        loop_id,
        loop_name,
        existing_tasks: existing_tasks_static,
        errors: errors_static,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 10);
    }

    #[test]
    fn test_create_task_response_serialization() {
        let response = CreateTaskResponse {
            success: true,
            message: "Task created".to_string(),
            task_id: Some("task-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CreateTaskResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_list_tasks_response_serialization() {
        let response = ListTasksResponse {
            success: true,
            tasks: vec![],
            page: 1,
            limit: 10,
            total: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ListTasksResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_get_task_response_serialization() {
        let response = GetTaskResponse {
            success: true,
            message: "Task retrieved".to_string(),
            task: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: GetTaskResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_delete_task_response_serialization() {
        let response = DeleteTaskResponse {
            success: true,
            message: "Task deleted successfully".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: DeleteTaskResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_list_tasks_query_defaults() {
        let parsed: ListTasksQuery = serde_urlencoded::from_str("").unwrap();

        assert_eq!(parsed.page, 1);
        assert_eq!(parsed.limit, 10);
    }

    // Integration tests for task handlers will be added later
    // These tests will require proper Router setup with mock dependencies
}
