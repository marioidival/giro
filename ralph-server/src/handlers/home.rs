//! Home page handler for Ralph Loop Manager server.
//!
//! This module provides handler for rendering the home/landing page.
//! The home page displays welcome message, quick start cards,
//! and recent loops if the user is logged in.

use askama::Template;
use axum::{Extension, extract::State, http::StatusCode, response::Html};

use crate::handlers::auth::AppState;
use crate::handlers::loops::LoopSummary;
use crate::middleware::csrf::CsrfToken;
use crate::templates::{HomeTemplate, LoopDisplay};

/// Handles rendering of home/landing page (HTML).
///
/// This endpoint:
/// 1. Extracts user_id from auth middleware
/// 2. Checks if user is logged in
/// 3. Generates CSRF token
/// 4. Fetches recent loops if user is logged in
/// 5. Renders home page template
///
/// # Arguments
/// * `state` - The application state containing repositories
/// * `user_id` - The authenticated user's ID (from auth middleware)
///
/// # Returns
/// * `200 OK` with HTML template on success
/// * `500 Internal Server Error` for server errors
pub async fn home_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    let recent_loops: Vec<LoopDisplay> = if logged_in {
        match state.loop_repository.list_by_owner(&user_id).await {
            Ok(mut loops) => {
                let recent_count = std::cmp::min(loops.len(), 3);
                loops.truncate(recent_count);
                loops
                    .into_iter()
                    .map(LoopSummary::from)
                    .map(LoopDisplay::from)
                    .collect()
            }
            Err(_e) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let csrf_token = CsrfToken::generate().to_string();

    let template = HomeTemplate {
        logged_in,
        csrf_token,
        recent_loops,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render home page: {}", e)),
        ),
    }
}

impl From<LoopSummary> for LoopDisplay {
    fn from(loop_: LoopSummary) -> Self {
        Self {
            id: loop_.id,
            name: loop_.name,
            description: loop_
                .description
                .unwrap_or_else(|| "No description".to_string()),
            status: loop_.status,
            provider: loop_.provider,
            model: loop_.model,
            created_at: loop_.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_page_logged_in_with_loops() {
        // Test would verify template renders with logged_in=true and loops
        // This is a placeholder for future testing
    }

    #[test]
    fn test_home_page_logged_in_no_loops() {
        // Test would verify template renders with logged_in=true and empty loops
        // This is a placeholder for future testing
    }

    #[test]
    fn test_home_page_not_logged_in() {
        // Test would verify template renders with logged_in=false and empty loops
        // This is a placeholder for future testing
    }
}
