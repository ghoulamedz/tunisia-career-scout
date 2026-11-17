//! Scheduler State Repository
//!
//! Persists next_run times and scheduler state for job recovery after app restart.

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStateRow {
    pub job_id: String,
    pub job_type: String,
    pub source_id: Option<String>, // Nullable for non-source jobs
    pub cron_expression: String,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub metadata: Option<String>, // JSON for additional job-specific data
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    SourceScan,
    MatchRerank,
    NotificationCheck,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::SourceScan => "source_scan",
            JobType::MatchRerank => "match_rerank",
            JobType::NotificationCheck => "notification_check",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "source_scan" => Some(JobType::SourceScan),
            "match_rerank" => Some(JobType::MatchRerank),
            "notification_check" => Some(JobType::NotificationCheck),
            _ => None,
        }
    }
}

pub struct SchedulerStateRepository {
    pool: SqlitePool,
}

impl SchedulerStateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the scheduler_state table if not exists
    pub async fn init_table(&self) -> Result<()> {
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS scheduler_state (
                job_id TEXT PRIMARY KEY NOT NULL,
                job_type TEXT NOT NULL CHECK (job_type IN ('source_scan', 'match_rerank', 'notification_check')),
                source_id TEXT REFERENCES sources(id) ON DELETE CASCADE,
                cron_expression TEXT NOT NULL,
                next_run TEXT NOT NULL,
                last_run TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                metadata TEXT, -- JSON
                updated_at TEXT NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Upsert a scheduler state entry
    pub async fn upsert(
        &self,
        job_id: &str,
        job_type: JobType,
        source_id: Option<Uuid>,
        cron_expression: &str,
        next_run: DateTime<Utc>,
        last_run: Option<DateTime<Utc>>,
        enabled: bool,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let source_id_str = source_id.map(|id| id.to_string());
        let metadata_str = metadata.map(|v| v.to_string());
        let now = Utc::now();
        let job_type_str = job_type.as_str();
        let next_run_str = next_run.to_rfc3339();
        let last_run_str = last_run.map(|dt| dt.to_rfc3339());
        let enabled_int = enabled as i64;
        let now_str = now.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO scheduler_state (job_id, job_type, source_id, cron_expression, next_run, last_run, enabled, metadata, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(job_id) DO UPDATE SET
                job_type = excluded.job_type,
                source_id = excluded.source_id,
                cron_expression = excluded.cron_expression,
                next_run = excluded.next_run,
                last_run = excluded.last_run,
                enabled = excluded.enabled,
                metadata = excluded.metadata,
                updated_at = excluded.updated_at
            "#,
            job_id,
            job_type_str,
            source_id_str,
            cron_expression,
            next_run_str,
            last_run_str,
            enabled_int,
            metadata_str,
            now_str,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get next_run for a specific job
    pub async fn get_next_run(&self, job_id: &str) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query!("SELECT next_run FROM scheduler_state WHERE job_id = ?", job_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| DateTime::parse_from_rfc3339(&r.next_run).ok()).map(|dt| dt.with_timezone(&Utc)))
    }

    /// Update last_run and calculate next_run for a job
    pub async fn update_last_run(&self, job_id: &str, last_run: DateTime<Utc>, next_run: DateTime<Utc>) -> Result<()> {
        let last_run_str = last_run.to_rfc3339();
        let next_run_str = next_run.to_rfc3339();
        let now_str = Utc::now().to_rfc3339();

        sqlx::query!(
            "UPDATE scheduler_state SET last_run = ?, next_run = ?, updated_at = ? WHERE job_id = ?",
            last_run_str, next_run_str, now_str, job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get all enabled scheduler states
    pub async fn list_enabled(&self) -> Result<Vec<SchedulerStateRow>> {
        let rows = sqlx::query!(
            "SELECT * FROM scheduler_state WHERE enabled = 1 ORDER BY next_run ASC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| row_to_state(r)).collect())
    }

    /// Get scheduler states by job type
    pub async fn list_by_type(&self, job_type: JobType) -> Result<Vec<SchedulerStateRow>> {
        let job_type_str = job_type.as_str();
        let rows = sqlx::query!(
            "SELECT * FROM scheduler_state WHERE job_type = ? ORDER BY next_run ASC",
            job_type_str
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| row_to_state(r)).collect())
    }

    /// Get scheduler state by job_id
    pub async fn get(&self, job_id: &str) -> Result<Option<SchedulerStateRow>> {
        let row = sqlx::query!("SELECT * FROM scheduler_state WHERE job_id = ?", job_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(row_to_state))
    }

    /// Disable a job
    pub async fn disable(&self, job_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE scheduler_state SET enabled = 0, updated_at = ? WHERE job_id = ?",
            now_str, job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Enable a job
    pub async fn enable(&self, job_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE scheduler_state SET enabled = 1, updated_at = ? WHERE job_id = ?",
            now_str, job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a job state
    pub async fn delete(&self, job_id: &str) -> Result<()> {
        sqlx::query!("DELETE FROM scheduler_state WHERE job_id = ?", job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all source scan jobs with their next_run times
    pub async fn list_source_scans(&self) -> Result<Vec<(Uuid, DateTime<Utc>, String)>> {
        let rows = sqlx::query!(
            r#"
            SELECT source_id, next_run, cron_expression
            FROM scheduler_state
            WHERE job_type = 'source_scan' AND enabled = 1 AND source_id IS NOT NULL
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| {
            let source_id = Uuid::parse_str(&r.source_id?).ok()?;
            let next_run = DateTime::parse_from_rfc3339(&r.next_run).ok()?.with_timezone(&Utc);
            Some((source_id, next_run, r.cron_expression))
        }).collect())
    }

    /// Update next_run for a source scan job
    pub async fn update_source_scan_next_run(&self, source_id: Uuid, next_run: DateTime<Utc>) -> Result<()> {
        let job_id = format!("source_scan_{}", source_id);
        let next_run_str = next_run.to_rfc3339();
        let now_str = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE scheduler_state SET next_run = ?, updated_at = ? WHERE job_id = ?",
            next_run_str, now_str, job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_state<'r, R: sqlx::Row>(row: &'r R) -> Option<SchedulerStateRow> {
    let next_run = DateTime::parse_from_rfc3339(row.try_get("next_run").ok()?).ok()?.with_timezone(&Utc);
    let last_run = row.try_get::<Option<String>, _>("last_run").ok()??.parse::<DateTime<Utc>>().ok();
    let enabled: i64 = row.try_get("enabled").ok()?;
    let updated_at = DateTime::parse_from_rfc3339(row.try_get("updated_at").ok()?).ok()?.with_timezone(&Utc);
    
    Some(SchedulerStateRow {
        job_id: row.try_get("job_id").ok()?,
        job_type: row.try_get("job_type").ok()?,
        source_id: row.try_get("source_id").ok()?,
        cron_expression: row.try_get("cron_expression").ok()?,
        next_run,
        last_run,
        enabled: enabled != 0,
        metadata: row.try_get("metadata").ok()?,
        updated_at,
    })
}