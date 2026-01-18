//! Integration tests for authentication templates

use askama::Template;
use ralph_server::templates::{LoginTemplate, RegisterTemplate};

#[test]
fn test_login_template_renders_with_csrf_token() {
    let csrf_token = "test-csrf-token-12345".to_string();
    let template = LoginTemplate {
        logged_in: false,
        csrf_token: csrf_token.clone(),
    };

    let result = template.render();

    assert!(result.is_ok(), "Login template should render successfully");

    let html = result.unwrap();
    assert!(
        html.contains("<!DOCTYPE html>"),
        "Should have HTML5 doctype"
    );
    assert!(html.contains("Login"), "Should contain login title");
    assert!(html.contains("username"), "Should contain username field");
    assert!(html.contains("password"), "Should contain password field");
    assert!(
        html.contains(&csrf_token),
        "Should include CSRF token in form"
    );
    assert!(
        html.contains("hx-post=\"/api/auth/login\""),
        "Should have hx-post attribute pointing to login endpoint"
    );
    assert!(
        html.contains("hx-swap=\"outerHTML\""),
        "Should have hx-swap attribute for HTMX"
    );
    assert!(
        html.contains("Register here"),
        "Should contain link to registration page"
    );
}

#[test]
fn test_login_template_has_tailwind_classes() {
    let template = LoginTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("max-w-md"),
        "Should use max-w-md for centered container"
    );
    assert!(html.contains("bg-white"), "Should have white background");
    assert!(html.contains("shadow-md"), "Should have shadow for depth");
    assert!(
        html.contains("bg-indigo-600"),
        "Should use indigo color for button"
    );
}

#[test]
fn test_register_template_renders_with_csrf_token() {
    let csrf_token = "test-csrf-token-67890".to_string();
    let empty_errors: &[String] = &[];
    let template = RegisterTemplate {
        logged_in: false,
        csrf_token: csrf_token.clone(),
        errors: empty_errors,
    };

    let result = template.render();

    assert!(
        result.is_ok(),
        "Register template should render successfully"
    );

    let html = result.unwrap();
    assert!(
        html.contains("<!DOCTYPE html>"),
        "Should have HTML5 doctype"
    );
    assert!(html.contains("Register"), "Should contain register title");
    assert!(html.contains("username"), "Should contain username field");
    assert!(html.contains("email"), "Should contain email field");
    assert!(html.contains("password"), "Should contain password field");
    assert!(
        html.contains(&csrf_token),
        "Should include CSRF token in form"
    );
    assert!(
        html.contains("hx-post=\"/api/auth/register\""),
        "Should have hx-post attribute pointing to register endpoint"
    );
    assert!(
        html.contains("Sign in here"),
        "Should contain link to login page"
    );
}

#[test]
fn test_register_template_displays_validation_errors() {
    let errors_vec = vec![
        "Username already exists".to_string(),
        "Invalid email format".to_string(),
        "Password too short".to_string(),
    ];
    let errors: &[String] = &errors_vec;

    let template = RegisterTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
        errors,
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("bg-red-50"),
        "Should have red background for errors"
    );
    assert!(
        html.contains("Registration failed"),
        "Should display error header"
    );

    for error in errors_vec {
        assert!(
            html.contains(&error),
            "Should display specific error: {}",
            error
        );
    }
}

#[test]
fn test_register_template_no_errors() {
    let empty_errors: &[String] = &[];
    let template = RegisterTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
        errors: empty_errors,
    };

    let html = template.render().unwrap();

    assert!(
        !html.contains("bg-red-50"),
        "Should not have error styling when no errors"
    );
    assert!(
        !html.contains("Registration failed"),
        "Should not display error message when no errors"
    );
}

#[test]
fn test_login_template_has_proper_form_attributes() {
    let template = LoginTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("action=\"/api/auth/login\""),
        "Should have correct form action"
    );
    assert!(html.contains("method=\"post\""), "Should use POST method");
    assert!(
        html.contains("type=\"hidden\""),
        "Should have hidden CSRF input"
    );
    assert!(
        html.contains("name=\"csrf_token\""),
        "Should have csrf_token field name"
    );
}

#[test]
fn test_register_template_has_proper_form_attributes() {
    let template = RegisterTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("action=\"/api/auth/register\""),
        "Should have correct form action"
    );
    assert!(html.contains("method=\"post\""), "Should use POST method");
    assert!(
        html.contains("type=\"hidden\""),
        "Should have hidden CSRF input"
    );
    assert!(
        html.contains("name=\"csrf_token\""),
        "Should have csrf_token field name"
    );
    assert!(
        html.contains("type=\"email\""),
        "Email field should be of type email"
    );
    assert!(
        html.contains("type=\"password\""),
        "Password field should be of type password"
    );
}

#[test]
fn test_login_template_has_required_attributes() {
    let template = LoginTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("required"),
        "Fields should have required attribute"
    );
    assert!(
        html.contains("placeholder"),
        "Fields should have placeholder text"
    );
    assert!(
        html.contains("placeholder=\"Enter your username\""),
        "Username field should have specific placeholder"
    );
    assert!(
        html.contains("placeholder=\"Enter your password\""),
        "Password field should have specific placeholder"
    );
}

#[test]
fn test_register_template_has_required_attributes() {
    let template = RegisterTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("required"),
        "Fields should have required attribute"
    );
    assert!(
        html.contains("placeholder"),
        "Fields should have placeholder text"
    );
    assert!(
        html.contains("placeholder=\"Choose a username\""),
        "Username field should have specific placeholder"
    );
    assert!(
        html.contains("placeholder=\"your@email.com\""),
        "Email field should have specific placeholder"
    );
    assert!(
        html.contains("placeholder=\"Create a password\""),
        "Password field should have specific placeholder"
    );
}

#[test]
fn test_login_and_register_use_tailwind_consistently() {
    let login_template = LoginTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
    };
    let register_template = RegisterTemplate {
        logged_in: false,
        csrf_token: "test-token".to_string(),
        errors: &[],
    };

    let login_html = login_template.render().unwrap();
    let register_html = register_template.render().unwrap();

    let common_classes = vec![
        "max-w-md",
        "bg-white",
        "shadow-md",
        "border-gray-300",
        "rounded-md",
        "text-sm",
        "font-medium",
    ];

    for class in common_classes {
        assert!(
            login_html.contains(class),
            "Login template should use class: {}",
            class
        );
        assert!(
            register_html.contains(class),
            "Register template should use class: {}",
            class
        );
    }
}
