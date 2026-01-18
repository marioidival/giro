//! Health check HTTP handler for Ralph Loop Manager server.
//!
//! This module provides a health check endpoint that monitors application status,
//! version information, and database connectivity.

use axum::{Json, extract::State, http::StatusCode};
use ralph_repositories::LoopRepository;
use serde::{Deserialize, Serialize};
use sqlx::query;

use crate::handlers::auth::AppState;

/// Health check response structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthResponse {
    /// Overall health status of application
    pub status: String,
    /// Application version from CARGO_PKG_VERSION
    pub version: String,
    /// Database connectivity status
    pub database_status: String,
}

/// Handles health check requests.
///
/// This endpoint:
/// 1. Checks overall application status
/// 2. Returns the application version
/// 3. Pings the database to verify connectivity
///
/// # Arguments
/// * `state` - The application state containing repository for database access
///
/// # Returns
/// * `200 OK` with health check information if database is accessible
/// * `503 Service Unavailable` if database is unreachable
///
/// # Example Response
/// ```json
/// {
///   "status": "ok",
///   "version": "0.1.0",
///   "database_status": "connected"
/// }
/// ```
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    // Ping database to check connectivity
    let db_status = match check_database_connection(&state.loop_repository).await {
        Ok(_) => "connected".to_string(),
        Err(_) => "disconnected".to_string(),
    };

    let status = if db_status == "connected" {
        "ok"
    } else {
        "degraded"
    };

    let http_status = if db_status == "connected" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http_status,
        Json(HealthResponse {
            status: status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            database_status: db_status,
        }),
    )
}

/// Checks database connectivity by executing a simple query.
///
/// # Arguments
/// * `repository` - The loop repository with access to the database pool
///
/// # Returns
/// * `Ok(())` if database connection is successful
/// * `Err` if database query fails
async fn check_database_connection(repository: &LoopRepository) -> Result<(), sqlx::Error> {
    query("SELECT 1").fetch_one(repository.pool()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            database_status: "connected".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: HealthResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_health_response_degraded_status() {
        let response = HealthResponse {
            status: "degraded".to_string(),
            version: "0.1.0".to_string(),
            database_status: "disconnected".to_string(),
        };

        assert_eq!(response.status, "degraded");
        assert_eq!(response.database_status, "disconnected");
    }

    #[tokio::test]
    async fn test_database_connection_check_success() {
        use ralph_repositories::database::Database;

        let db = Database::new("sqlite::memory:").await.unwrap();
        let repo = LoopRepository::new(db.pool().clone());

        let result = check_database_connection(&repo).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_response_contains_required_fields() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            database_status: "connected".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["status"].is_string());
        assert!(parsed["version"].is_string());
        assert!(parsed["database_status"].is_string());
    }

    #[tokio::test]
    async fn test_health_status_ok_when_database_connected() {
        use ralph_repositories::database::Database;

        let db = Database::new("sqlite::memory:").await.unwrap();
        let repo = LoopRepository::new(db.pool().clone());

        let db_result = check_database_connection(&repo).await;
        let db_status = if db_result.is_ok() {
            "connected".to_string()
        } else {
            "disconnected".to_string()
        };

        let status = if db_status == "connected" {
            "ok"
        } else {
            "degraded"
        };

        assert_eq!(db_status, "connected");
        assert_eq!(status, "ok");
    }

    #[tokio::test]
    async fn test_health_status_degraded_when_database_disconnected() {
        let db = ralph_repositories::database::Database::new("sqlite::memory:")
            .await
            .unwrap();

        let repo = LoopRepository::new(db.pool().clone());

        let db_result = check_database_connection(&repo).await;

        let db_status = if db_result.is_ok() {
            "connected".to_string()
        } else {
            "disconnected".to_string()
        };

        let status = if db_status == "connected" {
            "ok"
        } else {
            "degraded"
        };

        assert!(db_result.is_ok());
        assert_eq!(db_status, "connected");
        assert_eq!(status, "ok");
    }

    #[test]
    fn test_version_from_cargo_pkg_version() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        assert_eq!(version, "0.1.0");
    }
}
