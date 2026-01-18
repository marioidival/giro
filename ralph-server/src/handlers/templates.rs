//! Loop template CRUD HTTP handlers for Ralph Loop Manager server.

use askama::Template;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use ralph_models::{CreateLoopTemplate, LoopTemplate};
use serde::{Deserialize, Serialize};

use crate::handlers::auth::AppState;
use crate::middleware::csrf::CsrfToken;

/// Summary representation of a loop template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub owner_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub created_at: String,
}

impl From<LoopTemplate> for TemplateSummary {
    fn from(template: LoopTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            description: template.description,
            is_public: template.is_public,
            owner_id: template.owner_id,
            provider: template.provider,
            model: template.model,
            created_at: template.created_at.to_rfc3339(),
        }
    }
}

/// Response structure for creating a template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateTemplateResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
}

/// Response structure for listing templates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListTemplatesResponse {
    pub success: bool,
    pub templates: Vec<TemplateSummary>,
    pub total: usize,
}

/// Response structure for getting a template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetTemplateResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateDetail>,
}

/// Detailed template representation with full configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateDetail {
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
    pub created_at: String,
    pub updated_at: String,
}

impl From<LoopTemplate> for TemplateDetail {
    fn from(template: LoopTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            description: template.description,
            is_public: template.is_public,
            owner_id: template.owner_id,
            prd: template.prd,
            provider: template.provider,
            model: template.model,
            docker_image: template.docker_image,
            cpu_limit: template.cpu_limit,
            memory_limit: template.memory_limit,
            max_iterations: template.max_iterations,
            iteration_timeout: template.iteration_timeout,
            iteration_delay: template.iteration_delay,
            created_at: template.created_at.to_rfc3339(),
            updated_at: template.updated_at.to_rfc3339(),
        }
    }
}

/// Response structure for deleting a template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteTemplateResponse {
    pub success: bool,
    pub message: String,
}

/// Request structure for creating a template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: String,
    pub is_public: bool,
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

/// Template list page template
#[derive(Template)]
#[template(path = "loops/templates.html")]
pub struct TemplatesListTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub public_templates: Vec<TemplateSummary>,
    pub user_templates: Vec<TemplateSummary>,
}

/// Handles rendering the templates library page (HTML)
pub async fn templates_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();

    let all_templates = match state.template_repository.list_for_user(&user_id).await {
        Ok(templates) => templates,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("Failed to retrieve templates: {}", e)),
            );
        }
    };

    let public_templates: Vec<TemplateSummary> = all_templates
        .iter()
        .filter(|t| t.is_public)
        .map(|t| TemplateSummary::from(t.clone()))
        .collect();

    let user_templates: Vec<TemplateSummary> = all_templates
        .iter()
        .filter(|t| t.owner_id.as_ref() == Some(&user_id))
        .map(|t| TemplateSummary::from(t.clone()))
        .collect();

    let csrf_token = CsrfToken::generate().to_string();

    let template = TemplatesListTemplate {
        logged_in,
        csrf_token,
        public_templates,
        user_templates,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}

/// Handles creating a new template
pub async fn create_template(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateTemplateRequest>,
) -> (StatusCode, Json<CreateTemplateResponse>) {
    let create_template = CreateLoopTemplate {
        name: payload.name,
        description: payload.description,
        is_public: payload.is_public,
        owner_id: Some(user_id),
        prd: payload.prd,
        provider: payload.provider,
        model: payload.model,
        docker_image: payload.docker_image,
        cpu_limit: payload.cpu_limit,
        memory_limit: payload.memory_limit,
        max_iterations: payload.max_iterations,
        iteration_timeout: payload.iteration_timeout,
        iteration_delay: payload.iteration_delay,
    };

    match state.template_repository.create(create_template).await {
        Ok(template) => (
            StatusCode::CREATED,
            Json(CreateTemplateResponse {
                success: true,
                message: "Template created successfully".to_string(),
                template_id: Some(template.id),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateTemplateResponse {
                success: false,
                message: format!("Failed to create template: {}", e),
                template_id: None,
            }),
        ),
    }
}

/// Handles listing all templates for the user
pub async fn list_templates(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> (StatusCode, Json<ListTemplatesResponse>) {
    match state.template_repository.list_for_user(&user_id).await {
        Ok(templates) => {
            let summaries: Vec<TemplateSummary> =
                templates.into_iter().map(TemplateSummary::from).collect();
            let total = summaries.len();
            (
                StatusCode::OK,
                Json(ListTemplatesResponse {
                    success: true,
                    templates: summaries,
                    total,
                }),
            )
        }
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListTemplatesResponse {
                success: false,
                templates: Vec::new(),
                total: 0,
            }),
        ),
    }
}

/// Handles getting a specific template
pub async fn get_template(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<GetTemplateResponse>) {
    match state.template_repository.find_by_id(&id).await {
        Ok(Some(template)) => {
            // Check if user has access (public or owned)
            if !template.is_public && template.owner_id.as_ref() != Some(&user_id) {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(GetTemplateResponse {
                        success: false,
                        message: "You do not have permission to access this template".to_string(),
                        template: None,
                    }),
                );
            }

            (
                StatusCode::OK,
                Json(GetTemplateResponse {
                    success: true,
                    message: "Template retrieved successfully".to_string(),
                    template: Some(TemplateDetail::from(template)),
                }),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(GetTemplateResponse {
                success: false,
                message: "Template not found".to_string(),
                template: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetTemplateResponse {
                success: false,
                message: format!("Failed to retrieve template: {}", e),
                template: None,
            }),
        ),
    }
}

/// Handles deleting a template
pub async fn delete_template(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Json<DeleteTemplateResponse>) {
    match state.template_repository.delete(&id, &user_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(DeleteTemplateResponse {
                success: true,
                message: "Template deleted successfully".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeleteTemplateResponse {
                success: false,
                message: format!("Failed to delete template: {}", e),
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_summary_serialization() {
        let summary = TemplateSummary {
            id: "test-id".to_string(),
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: false,
            owner_id: Some("user-id".to_string()),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: TemplateSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(summary, deserialized);
    }

    #[test]
    fn test_create_template_response_serialization() {
        let response = CreateTemplateResponse {
            success: true,
            message: "Template created".to_string(),
            template_id: Some("template-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CreateTemplateResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_template_detail_serialization() {
        let detail = TemplateDetail {
            id: "test-id".to_string(),
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: true,
            owner_id: None,
            prd: "Test PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: Some("image:latest".to_string()),
            cpu_limit: Some(2),
            memory_limit: Some(2048),
            max_iterations: Some(100),
            iteration_timeout: Some(300),
            iteration_delay: Some(0),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&detail).unwrap();
        let deserialized: TemplateDetail = serde_json::from_str(&json).unwrap();

        assert_eq!(detail, deserialized);
    }

    #[test]
    fn test_create_template_request_serialization() {
        let request = CreateTemplateRequest {
            name: "Test Template".to_string(),
            description: "Test Description".to_string(),
            is_public: true,
            prd: "Test PRD".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus".to_string(),
            docker_image: Some("image:latest".to_string()),
            cpu_limit: Some(2),
            memory_limit: Some(2048),
            max_iterations: Some(100),
            iteration_timeout: Some(300),
            iteration_delay: Some(0),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateTemplateRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }
}
