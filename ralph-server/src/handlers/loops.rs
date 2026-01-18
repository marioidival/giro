//! Loop CRUD HTTP handlers for Ralph Loop Manager server.
//!
//! This module provides handlers for creating, listing, retrieving, and deleting loops.
//! All handlers require authentication via auth middleware and check ownership
//! to ensure users can only access their own loops.

use askama::Template;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use ralph_models::CreateLoop;
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
use crate::handlers::tasks::TaskSummary;
use crate::middleware::csrf::CsrfToken;
use crate::templates::{LoopDetailTemplate, LoopFormTemplate, LoopListTemplate};
use crate::validation::validate_loop_name;

/// Query parameters for listing loops with pagination.
#[derive(Debug, Deserialize)]
pub struct ListLoopsQuery {
    /// Page number (1-based, defaults to 1)
    #[serde(default = "default_page")]
    pub page: usize,
    /// Number of items per page (defaults to 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Handles rendering the loop detail page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Checks if user is logged in
/// 3. Generates CSRF token
/// 4. Retrieves loop by id with ownership check
/// 5. Retrieves tasks for the loop
/// 6. Renders the loop detail template with WebSocket for real-time updates
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
pub async fn get_loop_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    // Get loop with ownership check
    let loop_detail = match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Html("You do not have permission to view this loop".to_string()),
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

    // Get tasks for the loop
    let tasks = match state.task_repository.list_by_loop(&id).await {
        Ok(task_list) => task_list.into_iter().map(TaskSummary::from).collect(),
        Err(_e) => Vec::new(),
    };

    let csrf_token = CsrfToken::generate().to_string();

    let template = LoopDetailTemplate {
        logged_in,
        csrf_token,
        loop_detail,
        tasks,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

/// Response structure for creating a loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateLoopResponse {
    /// Whether creation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The ID of newly created loop (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_id: Option<String>,
}

/// Response structure for listing loops.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListLoopsResponse {
    /// Whether listing was successful
    pub success: bool,
    /// The list of loops
    pub loops: Vec<LoopSummary>,
    /// Current page number
    pub page: usize,
    /// Number of items per page
    pub limit: usize,
    /// Total number of loops owned by user
    pub total: usize,
}

/// A lightweight loop representation for list responses (without PRD).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub provider: String,
    pub model: String,
    pub docker_image: String,
    pub cpu_limit: i32,
    pub memory_limit: i32,
    pub max_iterations: i32,
    pub iteration_timeout: i32,
    pub iteration_delay: i32,
    pub git_repo_url: Option<String>,
    pub git_branch_pattern: String,
    pub status: String,
    pub current_iteration: i32,
    pub container_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ralph_models::Loop> for LoopSummary {
    fn from(loop_: ralph_models::Loop) -> Self {
        Self {
            id: loop_.id,
            name: loop_.name,
            description: loop_.description,
            owner_id: loop_.owner_id,
            provider: loop_.provider,
            model: loop_.model,
            docker_image: loop_.docker_image,
            cpu_limit: loop_.cpu_limit,
            memory_limit: loop_.memory_limit,
            max_iterations: loop_.max_iterations,
            iteration_timeout: loop_.iteration_timeout,
            iteration_delay: loop_.iteration_delay,
            git_repo_url: loop_.git_repo_url,
            git_branch_pattern: loop_.git_branch_pattern,
            status: loop_.status.to_string(),
            current_iteration: loop_.current_iteration,
            container_id: loop_.container_id,
            created_at: loop_.created_at.to_rfc3339(),
            updated_at: loop_.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for retrieving a loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetLoopResponse {
    /// Whether retrieval was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The loop (if found and user owns it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_: Option<LoopDetail>,
}

/// A detailed loop representation with full PRD.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopDetail {
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
    pub git_repo_url: Option<String>,
    pub git_branch_pattern: String,
    pub status: String,
    pub current_iteration: i32,
    pub container_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ralph_models::Loop> for LoopDetail {
    fn from(loop_: ralph_models::Loop) -> Self {
        Self {
            id: loop_.id,
            name: loop_.name,
            description: loop_.description,
            prd: loop_.prd,
            owner_id: loop_.owner_id,
            provider: loop_.provider,
            model: loop_.model,
            docker_image: loop_.docker_image,
            cpu_limit: loop_.cpu_limit,
            memory_limit: loop_.memory_limit,
            max_iterations: loop_.max_iterations,
            iteration_timeout: loop_.iteration_timeout,
            iteration_delay: loop_.iteration_delay,
            git_repo_url: loop_.git_repo_url,
            git_branch_pattern: loop_.git_branch_pattern,
            status: loop_.status.to_string(),
            current_iteration: loop_.current_iteration,
            container_id: loop_.container_id,
            created_at: loop_.created_at.to_rfc3339(),
            updated_at: loop_.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for deleting a loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteLoopResponse {
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

/// Handles rendering the loop list page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Checks if user is logged in (via logged_in header)
/// 3. Generates CSRF token
/// 4. Parses pagination params (page, limit) from query params
/// 5. Calls loop_repo.list_by_owner(user_id)
/// 6. Renders the loop list template with HTMX interactions
///
/// # Arguments
/// * `query` - Pagination query parameters (page, limit)
/// * `state` - The application state containing loop repository and CSRF store
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `500 Internal Server Error` for server errors
pub async fn list_loops_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<ListLoopsQuery>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    match state.loop_repository.list_by_owner(&user_id).await {
        Ok(mut all_loops) => {
            let total = all_loops.len();
            let limit = query.limit;
            let total_pages = if total == 0 { 1 } else { total.div_ceil(limit) };

            let start = if query.page > 0 {
                (query.page - 1) * limit
            } else {
                0
            };

            let end = std::cmp::min(start + limit, total);

            let loops: Vec<LoopSummary> = if start < total {
                all_loops.drain(start..end).map(LoopSummary::from).collect()
            } else {
                Vec::new()
            };

            let page = query.page as u32;
            let csrf_token = CsrfToken::generate().to_string();

            let template = LoopListTemplate {
                logged_in,
                csrf_token,
                loops,
                page,
                total_pages: total_pages as u32,
                has_prev: page > 1,
                has_next: page < total_pages as u32,
            };

            match template.render() {
                Ok(html) => (StatusCode::OK, Html(html)),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("Failed to render template: {}", e)),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to retrieve loops: {}", e)),
        ),
    }
}

/// Handles rendering the loop creation form (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Checks if user is logged in
/// 3. Generates CSRF token
/// 4. Renders the loop creation form template
///
/// # Arguments
/// * `state` - The application state containing loop repository and CSRF store
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML form on success
/// * `500 Internal Server Error` for server errors
pub async fn new_loop_form(
    State(_state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let csrf_token = CsrfToken::generate().to_string();

    let empty_errors: &[String] = &[];
    let template = LoopFormTemplate {
        logged_in,
        csrf_token,
        errors: empty_errors,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

/// Handles loop creation requests.
///
/// This endpoint:
/// 1. Validates loop name
/// 2. Extracts user_id from auth middleware
/// 3. Creates a Loop from CreateLoop payload with user_id as owner
/// 4. Calls loop_repo.create()
/// 5. Returns loop ID on success
///
/// # Arguments
/// * `payload` - The loop creation data (JSON)
/// * `state` - The application state containing loop repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `201 Created` with loop ID on success
/// * `400 Bad Request` for validation errors
/// * `500 Internal Server Error` for server errors
pub async fn create_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(mut payload): Json<CreateLoop>,
) -> (StatusCode, Json<CreateLoopResponse>) {
    if let Err(e) = validate_loop_name(&payload.name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreateLoopResponse {
                success: false,
                message: format!("Invalid loop name: {}", e.message.unwrap_or_default()),
                loop_id: None,
            }),
        );
    }

    payload.owner_id = user_id;

    match state.loop_repository.create(payload).await {
        Ok(loop_) => (
            StatusCode::CREATED,
            Json(CreateLoopResponse {
                success: true,
                message: "Loop created successfully".to_string(),
                loop_id: Some(loop_.id),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateLoopResponse {
                success: false,
                message: format!("Failed to create loop: {}", e),
                loop_id: None,
            }),
        ),
    }
}

/// Handles loop listing requests with pagination.
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Parses pagination params (page, limit) from query params
/// 3. Calls loop_repo.list_by_owner(user_id)
/// 4. Returns paginated results
///
/// Note: list_by_owner returns Vec<Loop> with empty PRD for performance.
///
/// # Arguments
/// * `query` - Pagination query parameters (page, limit)
/// * `state` - The application state containing loop repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with paginated loop list on success
/// * `500 Internal Server Error` for server errors
pub async fn list_loops(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<ListLoopsQuery>,
) -> (StatusCode, Json<ListLoopsResponse>) {
    match state.loop_repository.list_by_owner(&user_id).await {
        Ok(mut all_loops) => {
            let total = all_loops.len();

            let start = if query.page > 0 {
                (query.page - 1) * query.limit
            } else {
                0
            };

            let end = std::cmp::min(start + query.limit, total);

            let loops: Vec<LoopSummary> = if start < total {
                all_loops.drain(start..end).map(LoopSummary::from).collect()
            } else {
                Vec::new()
            };

            (
                StatusCode::OK,
                Json(ListLoopsResponse {
                    success: true,
                    loops,
                    page: query.page,
                    limit: query.limit,
                    total,
                }),
            )
        }
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListLoopsResponse {
                success: false,
                loops: Vec::new(),
                page: query.page,
                limit: query.limit,
                total: 0,
            }),
        ),
    }
}

/// Handles retrieving a specific loop.
///
/// This endpoint:
/// 1. Calls loop_repo.find_by_id(id)
/// 2. Checks ownership (verify loop.owner_id == user_id)
/// 3. Returns 404 if not found
/// 4. Returns Loop with full PRD on success
/// 5. Returns LoopDetail with full PRD on success
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with loop details on success
/// * `401 Unauthorized` if user doesn't own loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn get_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<GetLoopResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(GetLoopResponse {
                        success: false,
                        message: "You do not have permission to access this loop".to_string(),
                        loop_: None,
                    }),
                );
            }

            (
                StatusCode::OK,
                Json(GetLoopResponse {
                    success: true,
                    message: "Loop retrieved successfully".to_string(),
                    loop_: Some(LoopDetail::from(loop_)),
                }),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(GetLoopResponse {
                success: false,
                message: "Loop not found".to_string(),
                loop_: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetLoopResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                loop_: None,
            }),
        ),
    }
}

/// Handles deleting a loop.
///
/// This endpoint:
/// 1. Calls loop_repo.find_by_id(id) to check ownership
/// 2. Returns 404 if not found
/// 3. Returns 401 if user doesn't own loop
/// 4. Calls loop_repo.delete(id)
/// 5. Returns appropriate status codes
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful deletion
/// * `401 Unauthorized` if user doesn't own loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn delete_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<DeleteLoopResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(DeleteLoopResponse {
                        success: false,
                        message: "You do not have permission to delete this loop".to_string(),
                    }),
                );
            }

            match state.loop_repository.delete(&id).await {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        (
                            StatusCode::OK,
                            Json(DeleteLoopResponse {
                                success: true,
                                message: "Loop deleted successfully".to_string(),
                            }),
                        )
                    } else {
                        (
                            StatusCode::NOT_FOUND,
                            Json(DeleteLoopResponse {
                                success: false,
                                message: "Loop not found".to_string(),
                            }),
                        )
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DeleteLoopResponse {
                        success: false,
                        message: format!("Failed to delete loop: {}", e),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(DeleteLoopResponse {
                success: false,
                message: "Loop not found".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeleteLoopResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
            }),
        ),
    }
}

/// Response structure for loop control operations (start, pause, resume, stop).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopControlResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// A message describing result
    pub message: String,
    /// The new status of loop
    pub status: String,
}

/// Handles starting a loop.
///
/// This endpoint:
/// 1. Calls loop_repo.find_by_id(id) to check ownership
/// 2. Returns 404 if not found
/// 3. Returns 401 if user doesn't own loop
/// 4. Calls executor.start(loop_id)
/// 5. Returns {"success": true, "status": "running"}
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository and executor
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful start
/// * `401 Unauthorized` if user doesn't own loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn start_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<LoopControlResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(LoopControlResponse {
                        success: false,
                        message: "You do not have permission to control this loop".to_string(),
                        status: loop_.status.to_string(),
                    }),
                );
            }

            match state.loop_executor.start(&id).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(LoopControlResponse {
                        success: true,
                        message: "Loop started successfully".to_string(),
                        status: "running".to_string(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LoopControlResponse {
                        success: false,
                        message: format!("Failed to start loop: {}", e),
                        status: loop_.status.to_string(),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(LoopControlResponse {
                success: false,
                message: "Loop not found".to_string(),
                status: "unknown".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoopControlResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                status: "unknown".to_string(),
            }),
        ),
    }
}

/// Handles pausing a running loop.
///
/// This endpoint:
/// 1. Calls loop_repo.find_by_id(id) to check ownership
/// 2. Returns 404 if not found
/// 3. Returns 401 if user doesn't own loop
/// 4. Calls executor.pause(loop_id)
/// 5. Returns {"success": true, "status": "paused"}
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository and executor
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful pause
/// * `401 Unauthorized` if user doesn't own loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn pause_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<LoopControlResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(LoopControlResponse {
                        success: false,
                        message: "You do not have permission to control this loop".to_string(),
                        status: loop_.status.to_string(),
                    }),
                );
            }

            match state.loop_executor.pause(&id).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(LoopControlResponse {
                        success: true,
                        message: "Loop paused successfully".to_string(),
                        status: "paused".to_string(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LoopControlResponse {
                        success: false,
                        message: format!("Failed to pause loop: {}", e),
                        status: loop_.status.to_string(),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(LoopControlResponse {
                success: false,
                message: "Loop not found".to_string(),
                status: "unknown".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoopControlResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                status: "unknown".to_string(),
            }),
        ),
    }
}

/// Handles resuming a paused loop.
///
/// This endpoint:
/// 1. Calls loop_repo.find_by_id(id) to check ownership
/// 2. Returns 404 if not found
/// 3. Returns 401 if user doesn't own loop
/// 4. Calls executor.resume(loop_id)
/// 5. Returns {"success": true, "status": "running"}
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository and executor
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful resume
/// * `401 Unauthorized` if user doesn't own loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn resume_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<LoopControlResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(LoopControlResponse {
                        success: false,
                        message: "You do not have permission to control this loop".to_string(),
                        status: loop_.status.to_string(),
                    }),
                );
            }

            match state.loop_executor.resume(&id).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(LoopControlResponse {
                        success: true,
                        message: "Loop resumed successfully".to_string(),
                        status: "running".to_string(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LoopControlResponse {
                        success: false,
                        message: format!("Failed to resume loop: {}", e),
                        status: loop_.status.to_string(),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(LoopControlResponse {
                success: false,
                message: "Loop not found".to_string(),
                status: "unknown".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoopControlResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                status: "unknown".to_string(),
            }),
        ),
    }
}

/// Handles stopping a loop.
///
/// This endpoint:
/// 1. Calls loop_repo.find_by_id(id) to check ownership
/// 2. Returns 404 if not found
/// 3. Returns 401 if user doesn't own loop
/// 4. Calls executor.stop(loop_id)
/// 5. Returns {"success": true, "status": "completed"}
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository and executor
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` on successful stop
/// * `401 Unauthorized` if user doesn't own loop
/// * `404 Not Found` if loop doesn't exist
/// * `500 Internal Server Error` for server errors
pub async fn stop_loop(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<LoopControlResponse>) {
    match state.loop_repository.find_by_id(&id).await {
        Ok(Some(loop_)) => {
            if loop_.owner_id != user_id {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(LoopControlResponse {
                        success: false,
                        message: "You do not have permission to control this loop".to_string(),
                        status: loop_.status.to_string(),
                    }),
                );
            }

            match state.loop_executor.stop(&id).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(LoopControlResponse {
                        success: true,
                        message: "Loop stopped successfully".to_string(),
                        status: "completed".to_string(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LoopControlResponse {
                        success: false,
                        message: format!("Failed to stop loop: {}", e),
                        status: loop_.status.to_string(),
                    }),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(LoopControlResponse {
                success: false,
                message: "Loop not found".to_string(),
                status: "unknown".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoopControlResponse {
                success: false,
                message: format!("Failed to retrieve loop: {}", e),
                status: "unknown".to_string(),
            }),
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
    fn test_create_loop_response_serialization() {
        let response = CreateLoopResponse {
            success: true,
            message: "Loop created".to_string(),
            loop_id: Some("loop-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CreateLoopResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_list_loops_response_serialization() {
        let response = ListLoopsResponse {
            success: true,
            loops: vec![],
            page: 1,
            limit: 10,
            total: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ListLoopsResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_get_loop_response_serialization() {
        let response = GetLoopResponse {
            success: true,
            message: "Loop retrieved".to_string(),
            loop_: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: GetLoopResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_delete_loop_response_serialization() {
        let response = DeleteLoopResponse {
            success: true,
            message: "Loop deleted successfully".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: DeleteLoopResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_list_loops_query_defaults() {
        let parsed: ListLoopsQuery = serde_urlencoded::from_str("").unwrap();

        assert_eq!(parsed.page, 1);
        assert_eq!(parsed.limit, 10);
    }

    // Integration tests for loop control require Docker daemon
    // These tests will be added once test infrastructure is fully set up
    // with proper Router, request builders, and mock dependencies.

    #[test]
    fn test_list_loops_page_logic() {
        let total = 25;
        let limit = 10;
        let total_pages = (total + limit - 1) / limit;

        assert_eq!(total_pages, 3);

        let page = 2;
        let start = (page - 1) * limit;
        let end = std::cmp::min(start + limit, total);

        assert_eq!(start, 10);
        assert_eq!(end, 20);

        let has_prev = page > 1;
        let has_next = page < total_pages;

        assert!(has_prev);
        assert!(has_next);
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;
        use crate::middleware::auth::SessionStore;
        use crate::middleware::csrf::CsrfTokenStore;
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode},
        };
        use ralph_repositories::{LoopRepository, TaskRepository};
        use ralph_services::{AuthService, LoopExecutor};
        use sqlx::SqlitePool;
        use std::sync::Arc;
        use tower::ServiceExt;

        async fn create_test_state() -> AppState {
            let pool = SqlitePool::connect(":memory:").await.unwrap();

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE,
                    email TEXT NOT NULL UNIQUE,
                    password_hash TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )
                "#,
            )
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS loops (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    prd TEXT NOT NULL,
                    owner_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    model TEXT NOT NULL,
                    docker_image TEXT NOT NULL,
                    cpu_limit INTEGER NOT NULL,
                    memory_limit INTEGER NOT NULL,
                    max_iterations INTEGER NOT NULL,
                    iteration_timeout INTEGER NOT NULL,
                    iteration_delay INTEGER NOT NULL,
                    git_repo_url TEXT,
                    git_branch_pattern TEXT NOT NULL,
                    status TEXT NOT NULL,
                    current_iteration INTEGER NOT NULL DEFAULT 0,
                    container_id TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
                )
                "#,
            )
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS api_keys (
                    id TEXT PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    encrypted_key TEXT NOT NULL,
                    is_active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    UNIQUE(user_id, provider),
                    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
                )
                "#,
            )
            .execute(&pool)
            .await
            .unwrap();

            // Insert test users to satisfy foreign key constraints
            sqlx::query(
                r#"
                INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
                VALUES ('test-user-id', 'testuser', 'test@example.com', 'hash', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z'),
                       ('owner-user-id', 'owner', 'owner@example.com', 'hash', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z'),
                       ('attacker-user-id', 'attacker', 'attacker@example.com', 'hash', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')
                "#,
            )
            .execute(&pool)
            .await
            .unwrap();

            let user_repo = ralph_repositories::UserRepository::new(pool.clone());
            let auth_service = AuthService::new(user_repo);
            let session_store = SessionStore::new();
            let csrf_store = CsrfTokenStore::new();
            let loop_repository = LoopRepository::new(pool.clone());
            let task_repository = TaskRepository::new(pool.clone());
            let git_credentials_repository =
                ralph_repositories::GitCredentialsRepository::new(pool.clone());
            let api_key_repository = ralph_repositories::ApiKeyRepository::new(pool.clone());

            let docker = Arc::new(ralph_services::DockerManager::new());
            let agent_config = ralph_agent::agent::AgentConfig::default();
            let loop_executor =
                LoopExecutor::new(Arc::new(pool), docker, agent_config, None, Arc::new(api_key_repository.clone()));

            let broadcast_manager = crate::websocket::BroadcastManager::new();

            AppState::new(
                auth_service,
                session_store.clone(),
                csrf_store,
                loop_repository,
                task_repository,
                git_credentials_repository,
                api_key_repository,
                loop_executor,
                broadcast_manager,
            )
        }

        /// Check if Docker daemon is running
        fn is_docker_running() -> bool {
            std::process::Command::new("docker")
                .args(["ps"])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }

        #[tokio::test]
        async fn test_list_loops_page_renders_html() {
            let state = create_test_state().await;
            let app = crate::router::create_router(state.clone());

            let user_id = "test-user-id".to_string();
            let session_id = state.session_store.create_session(user_id.clone()).await;

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/loops?page=1&limit=10")
                        .method(Method::GET)
                        .header("session", &session_id)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let html = String::from_utf8(body.to_vec()).unwrap();

            assert!(
                html.contains("<!DOCTYPE html>"),
                "Should have HTML5 doctype"
            );
            assert!(
                html.contains("tailwindcss.com"),
                "Should include Tailwind CDN"
            );
            assert!(html.contains("htmx.org"), "Should include HTMX CDN");
            assert!(html.contains("Loops"), "Should include page title");
        }

        #[tokio::test]
        async fn test_start_loop_starts_execution() {
            if !is_docker_running() {
                eprintln!(
                    "Skipping test_start_loop_starts_execution: Docker daemon is not running"
                );
                return;
            }

            let state = create_test_state().await;
            let app = crate::router::create_router(state.clone());

            let user_id = "test-user-id".to_string();
            let session_id = state.session_store.create_session(user_id.clone()).await;

            // Generate and store CSRF token
            let csrf_token = crate::middleware::csrf::CsrfToken::generate();
            state
                .csrf_store
                .store(&session_id, csrf_token.as_str())
                .await;

            // Create loop via HTTP handler
            let create_loop = ralph_models::CreateLoop {
                name: "Test Loop".to_string(),
                description: None,
                prd: "Test PRD".to_string(),
                owner_id: user_id.clone(),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                docker_image: Some("ralph-loop-manager:latest".to_string()),
                cpu_limit: Some(1),
                memory_limit: Some(1024),
                max_iterations: Some(100),
                iteration_timeout: Some(300),
                iteration_delay: Some(0),
                git_repo_url: None,
                git_branch_pattern: Some("ralph/{loop_id}/{timestamp}".to_string()),
            };

            let create_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/loops")
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&create_loop).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_response.status(), StatusCode::CREATED);
            let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
                .await
                .unwrap();
            let create_json: CreateLoopResponse = serde_json::from_slice(&create_body).unwrap();
            let created_loop_id = create_json.loop_id.unwrap();

            // Start the loop
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/loops/{}/start", created_loop_id))
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: LoopControlResponse = serde_json::from_slice(&body).unwrap();

            assert!(json.success);
            assert_eq!(json.status, "running");
            assert!(json.message.contains("started"));
        }

        #[tokio::test]
        async fn test_pause_loop_stops_execution() {
            if !is_docker_running() {
                eprintln!("Skipping test_pause_loop_stops_execution: Docker daemon is not running");
                return;
            }

            let state = create_test_state().await;
            let app = crate::router::create_router(state.clone());

            let user_id = "test-user-id".to_string();
            let session_id = state.session_store.create_session(user_id.clone()).await;

            // Generate and store CSRF token
            let csrf_token = crate::middleware::csrf::CsrfToken::generate();
            state
                .csrf_store
                .store(&session_id, csrf_token.as_str())
                .await;

            // Create loop via HTTP handler
            let create_loop = ralph_models::CreateLoop {
                name: "Test Loop".to_string(),
                description: None,
                prd: "Test PRD".to_string(),
                owner_id: user_id.clone(),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                docker_image: Some("ralph-loop-manager:latest".to_string()),
                cpu_limit: Some(1),
                memory_limit: Some(1024),
                max_iterations: Some(100),
                iteration_timeout: Some(300),
                iteration_delay: Some(0),
                git_repo_url: None,
                git_branch_pattern: Some("ralph/{loop_id}/{timestamp}".to_string()),
            };

            let create_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/loops")
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&create_loop).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_response.status(), StatusCode::CREATED);
            let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
                .await
                .unwrap();
            let create_json: CreateLoopResponse = serde_json::from_slice(&create_body).unwrap();
            let created_loop_id = create_json.loop_id.unwrap();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/loops/{}/pause", created_loop_id))
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: LoopControlResponse = serde_json::from_slice(&body).unwrap();

            assert!(json.success);
            assert_eq!(json.status, "paused");
            assert!(json.message.contains("paused"));
        }

        #[tokio::test]
        async fn test_resume_loop_continues_execution() {
            if !is_docker_running() {
                eprintln!(
                    "Skipping test_resume_loop_continues_execution: Docker daemon is not running"
                );
                return;
            }

            let state = create_test_state().await;
            let app = crate::router::create_router(state.clone());

            let user_id = "test-user-id".to_string();
            let session_id = state.session_store.create_session(user_id.clone()).await;

            // Generate and store CSRF token
            let csrf_token = crate::middleware::csrf::CsrfToken::generate();
            state
                .csrf_store
                .store(&session_id, csrf_token.as_str())
                .await;

            // Create loop via HTTP handler
            let create_loop = ralph_models::CreateLoop {
                name: "Test Loop".to_string(),
                description: None,
                prd: "Test PRD".to_string(),
                owner_id: user_id.clone(),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                docker_image: Some("ralph-loop-manager:latest".to_string()),
                cpu_limit: Some(1),
                memory_limit: Some(1024),
                max_iterations: Some(100),
                iteration_timeout: Some(300),
                iteration_delay: Some(0),
                git_repo_url: None,
                git_branch_pattern: Some("ralph/{loop_id}/{timestamp}".to_string()),
            };

            let create_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/loops")
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&create_loop).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_response.status(), StatusCode::CREATED);
            let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
                .await
                .unwrap();
            let create_json: CreateLoopResponse = serde_json::from_slice(&create_body).unwrap();
            let created_loop_id = create_json.loop_id.unwrap();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/loops/{}/resume", created_loop_id))
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: LoopControlResponse = serde_json::from_slice(&body).unwrap();

            assert!(json.success);
            assert_eq!(json.status, "running");
            assert!(json.message.contains("resumed"));
        }

        #[tokio::test]
        async fn test_stop_loop_terminates_execution() {
            if !is_docker_running() {
                eprintln!(
                    "Skipping test_stop_loop_terminates_execution: Docker daemon is not running"
                );
                return;
            }

            let state = create_test_state().await;
            let app = crate::router::create_router(state.clone());

            let user_id = "test-user-id".to_string();
            let session_id = state.session_store.create_session(user_id.clone()).await;

            // Generate and store CSRF token
            let csrf_token = crate::middleware::csrf::CsrfToken::generate();
            state
                .csrf_store
                .store(&session_id, csrf_token.as_str())
                .await;

            // Create loop via HTTP handler
            let create_loop = ralph_models::CreateLoop {
                name: "Test Loop".to_string(),
                description: None,
                prd: "Test PRD".to_string(),
                owner_id: user_id.clone(),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                docker_image: Some("ralph-loop-manager:latest".to_string()),
                cpu_limit: Some(1),
                memory_limit: Some(1024),
                max_iterations: Some(100),
                iteration_timeout: Some(300),
                iteration_delay: Some(0),
                git_repo_url: None,
                git_branch_pattern: Some("ralph/{loop_id}/{timestamp}".to_string()),
            };

            let create_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/loops")
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&create_loop).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_response.status(), StatusCode::CREATED);
            let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
                .await
                .unwrap();
            let create_json: CreateLoopResponse = serde_json::from_slice(&create_body).unwrap();
            let created_loop_id = create_json.loop_id.unwrap();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/loops/{}/stop", created_loop_id))
                        .method(Method::POST)
                        .header("session", &session_id)
                        .header("x-csrf-token", csrf_token.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: LoopControlResponse = serde_json::from_slice(&body).unwrap();

            assert!(json.success);
            assert_eq!(json.status, "completed");
            assert!(json.message.contains("stopped"));
        }

        #[tokio::test]
        async fn test_loop_control_unauthorized_for_wrong_user() {
            let state = create_test_state().await;
            let app = crate::router::create_router(state.clone());

            let owner_id = "owner-user-id".to_string();
            let attacker_id = "attacker-user-id".to_string();

            let owner_session = state.session_store.create_session(owner_id.clone()).await;
            let attacker_session = state
                .session_store
                .create_session(attacker_id.clone())
                .await;

            // Generate and store CSRF token for owner
            let owner_csrf_token = crate::middleware::csrf::CsrfToken::generate();
            state
                .csrf_store
                .store(&owner_session, owner_csrf_token.as_str())
                .await;

            // Generate and store CSRF token for attacker
            let attacker_csrf_token = crate::middleware::csrf::CsrfToken::generate();
            state
                .csrf_store
                .store(&attacker_session, attacker_csrf_token.as_str())
                .await;

            // Create loop via HTTP handler as owner
            let create_loop = ralph_models::CreateLoop {
                name: "Test Loop".to_string(),
                description: None,
                prd: "Test PRD".to_string(),
                owner_id: owner_id.clone(),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                docker_image: Some("ralph-loop-manager:latest".to_string()),
                cpu_limit: Some(1),
                memory_limit: Some(1024),
                max_iterations: Some(100),
                iteration_timeout: Some(300),
                iteration_delay: Some(0),
                git_repo_url: None,
                git_branch_pattern: Some("ralph/{loop_id}/{timestamp}".to_string()),
            };

            let create_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/loops")
                        .method(Method::POST)
                        .header("session", &owner_session)
                        .header("x-csrf-token", owner_csrf_token.as_str())
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&create_loop).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(create_response.status(), StatusCode::CREATED);
            let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
                .await
                .unwrap();
            let create_json: CreateLoopResponse = serde_json::from_slice(&create_body).unwrap();
            let created_loop_id = create_json.loop_id.unwrap();

            // Try to control the loop as attacker
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/loops/{}/start", created_loop_id))
                        .method(Method::POST)
                        .header("session", &attacker_session)
                        .header("x-csrf-token", attacker_csrf_token.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: LoopControlResponse = serde_json::from_slice(&body).unwrap();

            assert!(!json.success);
            assert!(json.message.contains("permission"));
        }
    }
}
