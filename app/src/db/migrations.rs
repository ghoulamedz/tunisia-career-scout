//! Database Migrations
//!
//! This module re-exports the SQLx migration macro for compile-time checking.

/// Run all pending migrations.
pub async fn run_migrations(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "./migrations")]
    async fn test_migrations(pool: SqlitePool) {
        // Migrations are run automatically by the test macro
        // Just verify we can query the tables
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sources")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }
}