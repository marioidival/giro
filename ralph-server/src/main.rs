//! Ralph Loop Manager - Main Server Binary
//!
//! This is main entry point for Ralph Loop Manager HTTP server.
//! It initializes all services and starts web server with all routes.

use anyhow::{Context, Result};
use ralph_agent::agent::AgentConfig;
use ralph_repositories::{Database, LoopRepository, TaskRepository, UserRepository};
use ralph_services::{AuthService, DockerManager, LoopExecutor};
use std::env;
use std::sync::Arc;
use tracing::{Level, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

use ralph_server::{create_router, handlers::auth::AppState, middleware::auth::SessionStore};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    dotenvy::dotenv().ok();
    info!("Environment variables loaded");

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:ralph.db".to_string());
    info!("Connecting to database: {}", database_url);
    let db = Database::new(&database_url)
        .await
        .context("Failed to initialize database")?;
    info!("Database initialized and migrations applied");

    let pool = db.pool().clone();

    let docker = Arc::new(DockerManager::new());
    info!("Running container cleanup on startup");
    match docker.cleanup_orphaned_containers().await {
        Ok(count) => info!("Cleaned up {} orphaned containers", count),
        Err(e) => warn!("Failed to cleanup orphaned containers: {}", e),
    }

    let user_repo = UserRepository::new(pool.clone());
    let auth_service = AuthService::new(user_repo);

    let session_store = SessionStore::new();

    let loop_repository = LoopRepository::new(pool.clone());
    let task_repository = TaskRepository::new(pool.clone());

    let agent_config = AgentConfig::default();
    let loop_executor = LoopExecutor::new(Arc::new(pool), docker.clone(), agent_config);

    let state = AppState::new(
        auth_service,
        session_store,
        loop_repository,
        task_repository,
        loop_executor,
    );
    info!("All services initialized");

    let app = create_router(state);

    let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    info!("Starting Ralph Loop Manager server on {}", server_addr);

    let listener = tokio::net::TcpListener::bind(&server_addr)
        .await
        .context(format!("Failed to bind to address {}", server_addr))?;

    info!("Server listening on {}", server_addr);
    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_level(true)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();

    info!("Tracing initialized");
}
