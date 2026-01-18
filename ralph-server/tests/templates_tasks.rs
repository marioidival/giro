//! Integration tests for task templates

use askama::Template;
use ralph_server::handlers::tasks::TaskSummary;
use ralph_server::templates::NewTaskTemplate;

#[test]
fn test_new_task_template_renders_with_csrf_token() {
    let csrf_token = "test-csrf-token-12345".to_string();
    let loop_id = "loop-123".to_string();
    let loop_name = "Test Loop".to_string();
    let empty_tasks: &[TaskSummary] = &[];
    let empty_errors: &[String] = &[];

    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: csrf_token.clone(),
        loop_id: loop_id.clone(),
        loop_name: loop_name.clone(),
        existing_tasks: empty_tasks,
        errors: empty_errors,
    };

    let result = template.render();

    assert!(
        result.is_ok(),
        "New task template should render successfully"
    );

    let html = result.unwrap();
    assert!(
        html.contains("<!DOCTYPE html>"),
        "Should have HTML5 doctype"
    );
    assert!(
        html.contains("Create New Task"),
        "Should contain form title"
    );
    assert!(html.contains(&loop_name), "Should display loop name");
    assert!(
        html.contains(&csrf_token),
        "Should include CSRF token in form"
    );
    assert!(
        html.contains("hx-post=\"/api/loops/loop-123/tasks\""),
        "Should have hx-post attribute pointing to task creation endpoint"
    );
    assert!(
        html.contains("name=\"title\""),
        "Should contain title field"
    );
    assert!(
        html.contains("name=\"description\""),
        "Should contain description field"
    );
    assert!(
        html.contains("name=\"priority\""),
        "Should contain priority field"
    );
    assert!(
        html.contains("name=\"parent_task_id\""),
        "Should contain parent_task_id field"
    );
}

#[test]
fn test_new_task_template_has_tailwind_classes() {
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("max-w-7xl"),
        "Should use max-w-7xl for main container"
    );
    assert!(html.contains("bg-white"), "Should have white background");
    assert!(html.contains("shadow"), "Should have shadow for depth");
    assert!(
        html.contains("bg-indigo-600"),
        "Should use indigo color for button"
    );
}

#[test]
fn test_new_task_template_displays_validation_errors() {
    let errors_vec = vec![
        "Title is required".to_string(),
        "Description too long".to_string(),
    ];
    let errors: &[String] = &errors_vec;

    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors,
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("bg-red-50"),
        "Should have red background for errors"
    );
    assert!(
        html.contains("Task creation failed"),
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
fn test_new_task_template_no_errors() {
    let empty_errors: &[String] = &[];
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: empty_errors,
    };

    let html = template.render().unwrap();

    assert!(
        !html.contains("bg-red-50"),
        "Should not have error styling when no errors"
    );
    assert!(
        !html.contains("Task creation failed"),
        "Should not display error message when no errors"
    );
}

#[test]
fn test_new_task_template_has_proper_form_attributes() {
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-abc".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("hx-post=\"/api/loops/loop-abc/tasks\""),
        "Should have correct hx-post action"
    );
    assert!(
        html.contains("hx-swap=\"outerHTML\""),
        "Should have hx-swap attribute"
    );
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
fn test_new_task_template_has_required_attributes() {
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("required"),
        "Title field should have required attribute"
    );
    assert!(
        html.contains("maxlength=\"200\""),
        "Title should have max length"
    );
    assert!(
        html.contains("placeholder"),
        "Fields should have placeholder text"
    );
}

#[test]
fn test_new_task_template_with_existing_tasks() {
    let task1 = TaskSummary {
        id: "task-1".to_string(),
        loop_id: "loop-123".to_string(),
        title: "Parent Task 1".to_string(),
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
        loop_id: "loop-123".to_string(),
        title: "Parent Task 2".to_string(),
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

    let tasks = vec![task1, task2];

    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &tasks,
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("Parent Task 1"),
        "Should display parent task 1"
    );
    assert!(
        html.contains("Parent Task 2"),
        "Should display parent task 2"
    );
    assert!(
        html.contains("value=\"task-1\""),
        "Should have task-1 as option value"
    );
    assert!(
        html.contains("value=\"task-2\""),
        "Should have task-2 as option value"
    );
    assert!(
        html.contains("No parent (root task)"),
        "Should have default option for no parent"
    );
}

#[test]
fn test_new_task_template_priority_options() {
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: &[],
    };

    let html = template.render().unwrap();

    // Check for all priority options
    assert!(html.contains("1 - Lowest"), "Should have priority 1 option");
    assert!(html.contains("2 - Low"), "Should have priority 2 option");
    assert!(html.contains("3 - Medium"), "Should have priority 3 option");
    assert!(html.contains("4 - High"), "Should have priority 4 option");
    assert!(
        html.contains("5 - Highest"),
        "Should have priority 5 option"
    );

    // Check that medium (3) is selected by default
    assert!(
        html.contains("value=\"3\" selected"),
        "Priority 3 should be selected by default"
    );
}

#[test]
fn test_new_task_template_cancel_link() {
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-xyz".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("href=\"/loops/loop-xyz\""),
        "Cancel link should point back to loop detail page"
    );
    assert!(html.contains("Cancel"), "Should have cancel button");
}

#[test]
fn test_new_task_template_helper_text() {
    let template = NewTaskTemplate {
        logged_in: true,
        csrf_token: "test-token".to_string(),
        loop_id: "loop-123".to_string(),
        loop_name: "Test Loop".to_string(),
        existing_tasks: &[],
        errors: &[],
    };

    let html = template.render().unwrap();

    assert!(
        html.contains("Provide clear instructions for the LLM agent"),
        "Should have helper text for description"
    );
    assert!(
        html.contains("Higher priority tasks will be executed first"),
        "Should have helper text for priority"
    );
    assert!(
        html.contains("If this task depends on another task"),
        "Should have helper text for parent task"
    );
}
