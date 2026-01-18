pub mod api_keys;
pub mod auth;
pub mod git;
pub mod health;
pub mod home;
pub mod loops;
pub mod tasks;
pub mod templates;
pub mod user;

pub use api_keys::api_keys_page;
pub use api_keys::{ApiKeySummary, ListApiKeysResponse, create_api_key, list_api_keys};
pub use api_keys::{
    CreateApiKeyRequest, CreateApiKeyResponse, DeactivateApiKeyResponse, deactivate_api_key,
};
pub use auth::{AppState, login, login_page, logout, register, register_page};
pub use git::{
    CreateGitCredentialRequest, DeleteGitCredentialResponse, GitCredentialSummary,
    ListGitCredentialsResponse, create_git_credentials, delete_git_credentials,
    git_credentials_page, list_git_credentials,
};
pub use health::{HealthResponse, health_check};
pub use home::home_page;
pub use loops::{
    create_loop, delete_loop, get_loop, get_loop_page, list_loops, list_loops_page, new_loop_form,
    pause_loop, resume_loop, start_loop, stop_loop,
};
pub use tasks::{create_task, delete_task, get_task, list_tasks};
pub use templates::{
    CreateTemplateRequest, TemplateDetail, TemplateSummary, create_template, delete_template,
    get_template, list_templates, templates_page,
};
pub use user::{UserDisplay, profile_page};
