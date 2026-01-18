use crate::handlers::loops::LoopDetail;
use crate::handlers::loops::LoopSummary;
use crate::handlers::tasks::TaskSummary;
use askama::Template;

#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate<'a> {
    pub logged_in: bool,
    pub csrf_token: String,
    pub errors: &'a [String],
}

impl<'a> RegisterTemplate<'a> {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[derive(Template)]
#[template(path = "loops/index.html")]
pub struct LoopListTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loops: Vec<LoopSummary>,
    pub page: u32,
    pub total_pages: u32,
    pub has_prev: bool,
    pub has_next: bool,
}

#[derive(Template)]
#[template(path = "loops/new.html")]
pub struct LoopFormTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "loops/show.html")]
pub struct LoopDetailTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loop_detail: LoopDetail,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Template)]
#[template(path = "tasks/new.html")]
pub struct NewTaskTemplate<'a> {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loop_id: String,
    pub loop_name: String,
    pub existing_tasks: &'a [TaskSummary],
    pub errors: &'a [String],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_template_renders() {
        let template = BaseTemplate { logged_in: true };
        let result = template.render();

        assert!(result.is_ok(), "Template should render successfully");

        let html = result.unwrap();
        assert!(
            html.contains("<!DOCTYPE html>"),
            "Should have HTML5 doctype"
        );
        assert!(
            html.contains("tailwindcss.com"),
            "Should include Tailwind CDN"
        );
        assert!(html.contains("htmx.org"), "Should include HTMX CDN");
        assert!(html.contains("Loops"), "Should include navigation");
        assert!(
            html.contains("Logout"),
            "Should include logout when logged in"
        );
    }

    #[test]
    fn test_base_template_not_logged_in() {
        let template = BaseTemplate { logged_in: false };
        let result = template.render();

        assert!(result.is_ok(), "Template should render successfully");

        let html = result.unwrap();
        assert!(
            !html.contains("nav"),
            "Should not show navigation when not logged in"
        );
        assert!(
            html.contains("tailwindcss.com"),
            "Should still include Tailwind CDN"
        );
        assert!(html.contains("htmx.org"), "Should still include HTMX CDN");
    }

    #[test]
    fn test_loop_form_template_renders() {
        let template = LoopFormTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
        };
        let result = template.render();

        assert!(result.is_ok(), "Template should render successfully");

        let html = result.unwrap();
        assert!(
            html.contains("<!DOCTYPE html>"),
            "Should have HTML5 doctype"
        );
        assert!(
            html.contains("Create New Loop"),
            "Should include form title"
        );
        assert!(
            html.contains("test-csrf-token"),
            "Should include CSRF token"
        );
        assert!(html.contains("name=\"prd\""), "Should include PRD field");
        assert!(html.contains("name=\"name\""), "Should include name field");
        assert!(
            html.contains("hx-post=\"/api/loops\""),
            "Should include form post action"
        );
    }

    #[test]
    fn test_loop_form_has_all_required_fields() {
        let template = LoopFormTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
        };
        let html = template.render().unwrap();

        // Basic info fields
        assert!(html.contains("name=\"name\""));
        assert!(html.contains("name=\"description\""));
        assert!(html.contains("name=\"prd\""));

        // LLM config fields
        assert!(html.contains("name=\"provider\""));
        assert!(html.contains("name=\"model\""));

        // Docker config fields
        assert!(html.contains("name=\"docker_image\""));
        assert!(html.contains("name=\"cpu_limit\""));
        assert!(html.contains("name=\"memory_limit\""));

        // Execution config fields
        assert!(html.contains("name=\"max_iterations\""));
        assert!(html.contains("name=\"iteration_timeout\""));
        assert!(html.contains("name=\"iteration_delay\""));

        // Git config fields
        assert!(html.contains("name=\"git_repo_url\""));
        assert!(html.contains("name=\"git_branch_pattern\""));
    }

    #[test]
    fn test_loop_form_has_validation_indicators() {
        let template = LoopFormTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
        };
        let html = template.render().unwrap();

        // Check for required field indicators
        let required_count = html.matches("text-red-500").count();
        assert!(
            required_count >= 3,
            "Should mark required fields with red asterisk"
        );

        // Check for input constraints
        assert!(
            html.contains("maxlength=\"100\""),
            "Name should have max length"
        );
        assert!(
            html.contains("required"),
            "Required fields should be marked"
        );
    }

    #[test]
    fn test_loop_detail_template_renders() {
        let loop_detail = LoopDetail {
            id: "test-loop-id".to_string(),
            name: "Test Loop".to_string(),
            description: Some("Test Description".to_string()),
            prd: "# Test PRD\n\nThis is a test PRD.".to_string(),
            owner_id: "test-user-id".to_string(),
            provider: "claude".to_string(),
            model: "claude-3-opus-20240229".to_string(),
            docker_image: "ralph-loop-manager:latest".to_string(),
            cpu_limit: 1,
            memory_limit: 1024,
            max_iterations: 100,
            iteration_timeout: 300,
            iteration_delay: 0,
            git_repo_url: Some("https://github.com/test/repo".to_string()),
            git_branch_pattern: "ralph/{loop_id}/{timestamp}".to_string(),
            status: "running".to_string(),
            current_iteration: 5,
            container_id: Some("container-123".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T01:00:00Z".to_string(),
        };

        let template = LoopDetailTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
            loop_detail: loop_detail,
            tasks: vec![],
        };
        let result = template.render();

        assert!(result.is_ok(), "Template should render successfully");

        let html = result.unwrap();
        assert!(
            html.contains("<!DOCTYPE html>"),
            "Should have HTML5 doctype"
        );
        assert!(html.contains("Test Loop"), "Should include loop name");
        assert!(html.contains("Test PRD"), "Should include PRD content");
        assert!(html.contains("claude"), "Should include provider");
        assert!(html.contains("running"), "Should include status");
        assert!(
            html.contains("Configuration"),
            "Should include configuration section"
        );
        assert!(html.contains("Tasks"), "Should include tasks section");
        assert!(
            html.contains("WebSocket"),
            "Should include WebSocket script"
        );
    }

    #[test]
    fn test_loop_detail_template_with_tasks() {
        let loop_detail = LoopDetail {
            id: "test-loop-id".to_string(),
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: "test-user-id".to_string(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: "ralph-loop-manager:latest".to_string(),
            cpu_limit: 1,
            memory_limit: 1024,
            max_iterations: 100,
            iteration_timeout: 300,
            iteration_delay: 0,
            git_repo_url: None,
            git_branch_pattern: "ralph/{loop_id}/{timestamp}".to_string(),
            status: "created".to_string(),
            current_iteration: 0,
            container_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let task1 = TaskSummary {
            id: "task-1".to_string(),
            loop_id: "test-loop-id".to_string(),
            title: "Test Task 1".to_string(),
            status: "pending".to_string(),
            priority: 5,
            parent_task_id: None,
            created_by: "user".to_string(),
            iteration_id: None,
            started_at: None,
            completed_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let task2 = TaskSummary {
            id: "task-2".to_string(),
            loop_id: "test-loop-id".to_string(),
            title: "Test Task 2".to_string(),
            status: "completed".to_string(),
            priority: 3,
            parent_task_id: None,
            created_by: "llm".to_string(),
            iteration_id: Some("iter-1".to_string()),
            started_at: Some("2024-01-01T00:01:00Z".to_string()),
            completed_at: Some("2024-01-01T00:05:00Z".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:05:00Z".to_string(),
        };

        let template = LoopDetailTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
            loop_detail: loop_detail,
            tasks: vec![task1, task2],
        };
        let html = template.render().unwrap();

        assert!(html.contains("Test Task 1"), "Should display task 1");
        assert!(html.contains("Test Task 2"), "Should display task 2");
        assert!(html.contains("pending"), "Should show pending status");
        assert!(html.contains("completed"), "Should show completed status");
        assert!(html.contains("Priority: 5"), "Should show priority");
        assert!(html.contains("By: user"), "Should show created by");
        assert!(html.contains("By: llm"), "Should show LLM created by");
    }

    #[test]
    fn test_loop_detail_empty_tasks_state() {
        let loop_detail = LoopDetail {
            id: "test-loop-id".to_string(),
            name: "Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: "test-user-id".to_string(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: "ralph-loop-manager:latest".to_string(),
            cpu_limit: 1,
            memory_limit: 1024,
            max_iterations: 100,
            iteration_timeout: 300,
            iteration_delay: 0,
            git_repo_url: None,
            git_branch_pattern: "ralph/{loop_id}/{timestamp}".to_string(),
            status: "created".to_string(),
            current_iteration: 0,
            container_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let template = LoopDetailTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
            loop_detail: loop_detail,
            tasks: vec![],
        };
        let html = template.render().unwrap();

        assert!(html.contains("No tasks"), "Should show empty state message");
        assert!(
            html.contains("Get started by adding a task"),
            "Should show empty state description"
        );
    }

    #[test]
    fn test_loop_detail_websocket_connection_script() {
        let loop_detail = LoopDetail {
            id: "ws-test-loop-id".to_string(),
            name: "WebSocket Test Loop".to_string(),
            description: None,
            prd: "Test PRD".to_string(),
            owner_id: "test-user-id".to_string(),
            provider: "mock".to_string(),
            model: "mock".to_string(),
            docker_image: "ralph-loop-manager:latest".to_string(),
            cpu_limit: 1,
            memory_limit: 1024,
            max_iterations: 100,
            iteration_timeout: 300,
            iteration_delay: 0,
            git_repo_url: None,
            git_branch_pattern: "ralph/{loop_id}/{timestamp}".to_string(),
            status: "running".to_string(),
            current_iteration: 1,
            container_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let template = LoopDetailTemplate {
            logged_in: true,
            csrf_token: "test-csrf-token".to_string(),
            loop_detail: loop_detail,
            tasks: vec![],
        };
        let html = template.render().unwrap();

        assert!(
            html.contains("WebSocket"),
            "Should include WebSocket connection script"
        );
        assert!(
            html.contains("connectWebSocket"),
            "Should define connectWebSocket function"
        );
        assert!(
            html.contains("ws.onmessage"),
            "Should handle WebSocket messages"
        );
        assert!(
            html.contains("ws.onclose"),
            "Should handle WebSocket reconnection"
        );
        assert!(
            html.contains("updateStatusBadge"),
            "Should define status badge update function"
        );
    }
}
