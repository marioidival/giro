use crate::handlers::loops::LoopSummary;
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
}
