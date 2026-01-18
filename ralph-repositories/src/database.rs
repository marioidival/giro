use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, sqlite::SqliteConnectOptions};
use std::str::FromStr;

/// Database connection pool with optimized SQLite settings
#[derive(Clone, Debug)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Create a new Database instance with optimized SQLite settings
    ///
    /// Pragmas configured:
    /// - foreign_keys=ON: Enable foreign key constraints
    /// - journal_mode=WAL: Write-Ahead Logging for better concurrency
    /// - synchronous=NORMAL: Balance between safety and performance
    /// - cache_size=64MB: Larger cache for better performance
    /// - auto_vacuum=INCREMENTAL: Automatic vacuuming of freed pages
    ///
    /// # Arguments
    /// * `database_url` - SQLite database URL (e.g., "sqlite:ralph.db")
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::database::Database;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = Database::new("sqlite:ralph.db").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .pragma("foreign_keys", "1")
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("cache_size", "-65536")
            .pragma("auto_vacuum", "INCREMENTAL");

        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .context("Failed to create database connection pool")?;

        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .context("Failed to run database migrations")?;

        Self::verify_pragmas(&pool).await?;

        Ok(Database { pool })
    }

    /// Verify that critical pragmas are correctly configured
    ///
    /// # Panics
    /// Panics if pragmas don't match expected values
    async fn verify_pragmas(pool: &Pool<Sqlite>) -> Result<()> {
        let fk_result: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(pool)
            .await
            .context("Failed to verify foreign_keys pragma")?;

        if fk_result.0 != 1 {
            anyhow::bail!("Foreign keys pragma is not enabled");
        }

        tracing::info!("Database pragmas verified successfully");
        Ok(())
    }

    /// Get a reference to the underlying SqlitePool
    ///
    /// # Example
    /// ```no_run
    /// # use ralph_repositories::database::Database;
    /// # async fn example(db: Database) -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = db.pool();
    /// // Use pool for queries
    /// # Ok(())
    /// # }
    /// ```
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test for database connection and initialization
    #[tokio::test]
    async fn test_database_initialization() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;
        assert!(db.pool().size() > 0);
        Ok(())
    }

    /// Test that pragmas are correctly applied
    #[tokio::test]
    async fn test_pragmas_applied() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;

        let fk_result: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(db.pool())
            .await?;

        assert_eq!(fk_result.0, 1, "Foreign keys should be enabled");

        let sync_result: (i64,) = sqlx::query_as("PRAGMA synchronous")
            .fetch_one(db.pool())
            .await?;

        assert_eq!(sync_result.0, 1, "Synchronous mode should be NORMAL");

        let cache_result: (i64,) = sqlx::query_as("PRAGMA cache_size")
            .fetch_one(db.pool())
            .await?;

        assert_eq!(cache_result.0, -65536, "Cache size should be 64MB");

        Ok(())
    }

    /// Test that migrations run successfully
    #[tokio::test]
    async fn test_migrations_run() -> Result<()> {
        let db = Database::new("sqlite::memory:").await?;

        let tables: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(db.pool())
                .await?;

        let table_names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();

        assert!(
            table_names.contains(&"users"),
            "users table should exist from migration"
        );
        assert!(
            table_names.contains(&"loops"),
            "loops table should exist from migration"
        );
        assert!(
            table_names.contains(&"tasks"),
            "tasks table should exist from migration"
        );
        assert!(
            table_names.contains(&"iterations"),
            "iterations table should exist from migration"
        );
        assert!(
            table_names.contains(&"files"),
            "files table should exist from migration"
        );

        Ok(())
    }
}
