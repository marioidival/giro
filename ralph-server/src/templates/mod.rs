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
pub struct RegisterTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub errors: Vec<String>,
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
        assert!(html.contains("Home"), "Should include navigation");
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
}
