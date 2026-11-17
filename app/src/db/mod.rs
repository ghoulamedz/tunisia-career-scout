//! Database Module
//!
//! Re-exports database connection, migrations, and repositories.

pub mod connection;
pub mod migrations;
pub mod repositories;

use sqlx::SqlitePool;

pub use connection::{DbPool, init_pool, run_migrations};
pub use repositories::{SourceRepository, PostingRepository, CVRepository, MatchRepository, SettingsRepository, ScanLogRepository};