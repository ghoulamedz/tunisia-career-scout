//! Database Connection & Pool Management
//!
//! Uses SQLx with SQLite. Provides a connection pool and migration runner.

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;
use directories::ProjectDirs;
use tracing::info;

pub type DbPool = SqlitePool;

/// Initialize the database connection pool.
///
/// Creates the database file in the platform-appropriate app data directory:
/// - Linux: ~/.local/share/tunisia-career-scout/app.db
/// - macOS: ~/Library/Application Support/tunisia-career-scout/app.db
/// - Windows: %APPDATA%/tunisia-career-scout/app.db
/// - Android: /data/data/com.tunisiacareerscout.app/files/app.db
pub async fn init_pool() -> Result<DbPool> {
    let db_path = get_db_path()?;

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let database_url = format!("sqlite://{}?mode=rwc", db_path.display());
    info!("Connecting to database: {}", database_url);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await?;

    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;
    sqlx::query("PRAGMA busy_timeout = 5000;").execute(&pool).await?;

    info!("Database connected successfully");
    Ok(pool)
}

/// Get the platform-appropriate database file path.
fn get_db_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "tunisiacareerscout", "app")
        .ok_or_else(|| anyhow::anyhow!("Could not determine app data directory"))?;

    Ok(proj_dirs.data_dir().join("app.db"))
}

/// Run database migrations.
pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(pool).await?;
    info!("Migrations completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_pool() {
        let pool = init_pool().await.expect("Failed to init pool");
        // Test a simple query
        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .expect("Query failed");
        assert_eq!(result.0, 1);
    }
}