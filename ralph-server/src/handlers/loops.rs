//! Loop CRUD HTTP handlers for the Ralph Loop Manager server.
//!
//! This module provides handlers for creating, listing, retrieving, and deleting loops.
//! All handlers require authentication via the auth middleware and check ownership
//! to ensure users can only access their own loops.

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use ralph_models::CreateLoop;
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
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

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    10
}

/// Response structure for creating a loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateLoopResponse {
    /// Whether the creation was successful
    pub success: bool,
    /// A message describing the result
    pub message: String,
    /// The ID of the newly created loop (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_id: Option<String>,
}

/// Response structure for listing loops.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListLoopsResponse {
    /// Whether the listing was successful
    pub success: bool,
    /// The list of loops
    pub loops: Vec<LoopSummary>,
    /// Current page number
    pub page: usize,
    /// Number of items per page
    pub limit: usize,
    /// Total number of loops owned by the user
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
    /// Whether the retrieval was successful
    pub success: bool,
    /// A message describing the result
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
    /// Whether the deletion was successful
    pub success: bool,
    /// A message describing the result
    pub message: String,
}

/// Handles loop creation requests.
///
/// This endpoint:
/// 1. Validates the loop name
/// 2. Extracts user_id from auth middleware
/// 3. Creates a Loop from CreateLoop payload with user_id as owner
/// 4. Calls loop_repo.create()
/// 5. Returns the loop ID on success
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
/// 4. Returns 401 if user doesn't own the loop
/// 5. Returns Loop with full PRD on success
///
/// # Arguments
/// * `path` - Path parameters containing loop ID
/// * `state` - The application state containing loop repository
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with loop details on success
/// * `401 Unauthorized` if user doesn't own the loop
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
/// 3. Returns 401 if user doesn't own the loop
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
/// * `401 Unauthorized` if user doesn't own the loop
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
            message: "Loop deleted".to_string(),
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
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::handlers::auth::AppState;
    use crate::middleware::auth::SessionStore;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::{delete, get, post},
    };
    use ralph_repositories::Database;
    use tower::ServiceExt;

    fn create_test_app(session_store: SessionStore, db: Database) -> Router {
        let user_repo = ralph_repositories::UserRepository::new(db.pool().clone());
        let app_state = AppState {
            auth_service: ralph_services::AuthService::new(user_repo),
            session_store: session_store.clone(),
            loop_repository: ralph_repositories::LoopRepository::new(db.pool().clone()),
        };

        Router::new()
            .without_v07_checks()
            .route("/api/loops", post(create_loop))
            .route("/api/loops", get(list_loops))
            .route("/api/loops/:id", get(get_loop))
            .route("/api/loops/:id", delete(delete_loop))
            .layer(axum::middleware::from_fn_with_state(
                session_store.clone(),
                crate::middleware::auth::auth_middleware,
            ))
            .layer(Extension(session_store))
            .with_state(app_state)
    }

    /// Integration test: POST /api/loops creates loop
    #[tokio::test]
    async fn test_post_api_loops_creates_loop() -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::new("sqlite::memory:").await?;
        let session_store = SessionStore::new();

        let user_repo = ralph_repositories::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;
        let session_id = session_store.create_session(user.id.clone()).await;

        let app = create_test_app(session_store.clone(), db);

        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: Some("Test description".to_string()),
            prd: "Test PRD content".to_string(),
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

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/loops")
                    .header("session", &session_id)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_loop)?))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        if status != StatusCode::CREATED {
            eprintln!("Expected 201, got {:?}", status);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            eprintln!("Response body: {}", body_str);
            panic!("Test failed");
        }

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: CreateLoopResponse = serde_json::from_str(&body_str)?;

        assert!(response_data.success);
        assert!(response_data.loop_id.is_some());
        assert_eq!(response_data.message, "Loop created successfully");

        Ok(())
    }

    /// Integration test: GET /api/loops?page=1&limit=10 paginates
    #[tokio::test]
    async fn test_get_api_loops_paginates() -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::new("sqlite::memory:").await?;
        let session_store = SessionStore::new();

        let user_repo = ralph_repositories::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;
        let session_id = session_store.create_session(user.id.clone()).await;

        let loop_repo = ralph_repositories::LoopRepository::new(db.pool().clone());
        for i in 1..=25 {
            let create_loop = CreateLoop {
                name: format!("Loop {}", i),
                description: None,
                prd: format!("PRD {}", i),
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
            loop_repo.create(create_loop).await?;
        }

        let app = create_test_app(session_store.clone(), db);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/loops?page=1&limit=10")
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: ListLoopsResponse = serde_json::from_str(&body_str)?;

        assert!(response_data.success);
        assert_eq!(response_data.loops.len(), 10);
        assert_eq!(response_data.page, 1);
        assert_eq!(response_data.limit, 10);
        assert_eq!(response_data.total, 25);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/loops?page=2&limit=10")
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: ListLoopsResponse = serde_json::from_str(&body_str)?;

        assert!(response_data.success);
        assert_eq!(response_data.loops.len(), 10);
        assert_eq!(response_data.page, 2);
        assert_eq!(response_data.total, 25);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/loops?page=3&limit=10")
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: ListLoopsResponse = serde_json::from_str(&body_str)?;

        assert!(response_data.success);
        assert_eq!(response_data.loops.len(), 5);
        assert_eq!(response_data.page, 3);
        assert_eq!(response_data.total, 25);

        Ok(())
    }

    /// Integration test: GET /api/loops/:id returns loop
    #[tokio::test]
    async fn test_get_api_loops_id_returns_loop() -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::new("sqlite::memory:").await?;
        let session_store = SessionStore::new();

        let user_repo = ralph_repositories::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;
        let session_id = session_store.create_session(user.id.clone()).await;

        let loop_repo = ralph_repositories::LoopRepository::new(db.pool().clone());
        let create_loop = CreateLoop {
            name: "Test Loop".to_string(),
            description: Some("Test description".to_string()),
            prd: "Test PRD content".to_string(),
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

        let app = create_test_app(session_store.clone(), db);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/loops/{}", loop_.id))
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: GetLoopResponse = serde_json::from_str(&body_str)?;

        assert!(response_data.success);
        assert!(response_data.loop_.is_some());
        assert_eq!(response_data.message, "Loop retrieved successfully");

        let loop_detail = response_data.loop_.unwrap();
        assert_eq!(loop_detail.id, loop_.id);
        assert_eq!(loop_detail.name, "Test Loop");
        assert_eq!(loop_detail.prd, "Test PRD content");
        assert_eq!(loop_detail.status, "created");

        Ok(())
    }

    /// Integration test: GET /api/loops/:id unauthorized for wrong user (ownership check)
    #[tokio::test]
    async fn test_get_api_loops_id_unauthorized_for_wrong_user()
    -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::new("sqlite::memory:").await?;
        let session_store = SessionStore::new();

        let user_repo = ralph_repositories::UserRepository::new(db.pool().clone());
        let create_user1 = ralph_models::CreateUser {
            username: "testuser1".to_string(),
            email: "test1@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user1 = user_repo.create(create_user1).await?;

        let create_user2 = ralph_models::CreateUser {
            username: "testuser2".to_string(),
            email: "test2@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user2 = user_repo.create(create_user2).await?;

        let loop_repo = ralph_repositories::LoopRepository::new(db.pool().clone());
        let create_loop = CreateLoop {
            name: "User1 Loop".to_string(),
            description: None,
            prd: "User1 PRD".to_string(),
            owner_id: user1.id.clone(),
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

        let app = create_test_app(session_store.clone(), db);

        let session_id = session_store.create_session(user1.id.clone()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/loops/{}", loop_.id))
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: GetLoopResponse = serde_json::from_str(&body_str)?;

        assert!(!response_data.success);
        assert!(response_data.loop_.is_none());
        assert!(response_data.message.contains("permission"));

        Ok(())
    }

    /// Integration test: DELETE /api/loops/:id deletes loop
    #[tokio::test]
    async fn test_delete_api_loops_id_deletes_loop() -> Result<(), Box<dyn std::error::Error>> {
        let db = Database::new("sqlite::memory:").await?;
        let session_store = SessionStore::new();

        let user_repo = ralph_repositories::UserRepository::new(db.pool().clone());
        let create_user = ralph_models::CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
        };
        let user = user_repo.create(create_user).await?;
        let session_id = session_store.create_session(user.id.clone()).await;

        let loop_repo = ralph_repositories::LoopRepository::new(db.pool().clone());
        let create_loop = CreateLoop {
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

        let app = create_test_app(session_store.clone(), db);

        let found = loop_repo.find_by_id(&loop_.id).await?;
        assert!(found.is_some());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/loops/{}", loop_.id))
                    .header("session", &session_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_data: DeleteLoopResponse = serde_json::from_str(&body_str)?;

        assert!(response_data.success);
        assert_eq!(response_data.message, "Loop deleted successfully");

        let found = loop_repo.find_by_id(&loop_.id).await?;
        assert!(found.is_none());

        Ok(())
    }
}
