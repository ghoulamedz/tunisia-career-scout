//! Scan Log Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::scan_log::{ScanLog, ScanStatus};

#[derive(Clone)]
pub struct ScanLogRepository {
    pool: SqlitePool,
}

impl ScanLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, log: &ScanLog) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO scan_logs (id, source_id, status, postings_found, postings_new, postings_updated, error_message, started_at, completed_at, duration_ms, raw_response_sample)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            log.id,
            log.source_id,
            log.status as _,
            log.postings_found,
            log.postings_new,
            log.postings_updated,
            log.error_message,
            log.started_at,
            log.completed_at,
            log.duration_ms,
            log.raw_response_sample,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update(&self, log: &ScanLog) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE scan_logs SET
                status = ?, postings_found = ?, postings_new = ?, postings_updated = ?,
                error_message = ?, completed_at = ?, duration_ms = ?, raw_response_sample = ?
            WHERE id = ?
            "#,
            log.status as _,
            log.postings_found,
            log.postings_new,
            log.postings_updated,
            log.error_message,
            log.completed_at,
            log.duration_ms,
            log.raw_response_sample,
            log.id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<ScanLog>> {
        let row = sqlx::query!("SELECT * FROM scan_logs WHERE id = ?", id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| ScanLog {
            id: Uuid::parse_str(&r.id).unwrap(),
            source_id: Uuid::parse_str(&r.source_id).unwrap(),
            status: serde_json::from_str(&r.status).unwrap_or(ScanStatus::Running),
            postings_found: r.postings_found,
            postings_new: r.postings_new,
            postings_updated: r.postings_updated,
            error_message: r.error_message,
            started_at: chrono::DateTime::parse_from_rfc3339(&r.started_at).unwrap().with_timezone(&chrono::Utc),
            completed_at: r.completed_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            duration_ms: r.duration_ms.map(|v| v as u64),
            raw_response_sample: r.raw_response_sample,
        }))
    }

    pub async fn list_for_source(&self, source_id: Uuid, limit: u32) -> Result<Vec<ScanLog>> {
        let rows = sqlx::query!(
            "SELECT * FROM scan_logs WHERE source_id = ? ORDER BY started_at DESC LIMIT ?",
            source_id, limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| ScanLog {
            id: Uuid::parse_str(&r.id).unwrap(),
            source_id: Uuid::parse_str(&r.source_id).unwrap(),
            status: serde_json::from_str(&r.status).unwrap_or(ScanStatus::Running),
            postings_found: r.postings_found,
            postings_new: r.postings_new,
            postings_updated: r.postings_updated,
            error_message: r.error_message,
            started_at: chrono::DateTime::parse_from_rfc3339(&r.started_at).unwrap().with_timezone(&chrono::Utc),
            completed_at: r.completed_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            duration_ms: r.duration_ms.map(|v| v as u64),
            raw_response_sample: r.raw_response_sample,
        }).collect())
    }

    pub async fn list_recent(&self, limit: u32) -> Result<Vec<ScanLog>> {
        let rows = sqlx::query!(
            "SELECT * FROM scan_logs ORDER BY started_at DESC LIMIT ?",
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| ScanLog {
            id: Uuid::parse_str(&r.id).unwrap(),
            source_id: Uuid::parse_str(&r.source_id).unwrap(),
            status: serde_json::from_str(&r.status).unwrap_or(ScanStatus::Running),
            postings_found: r.postings_found,
            postings_new: r.postings_new,
            postings_updated: r.postings_updated,
            error_message: r.error_message,
            started_at: chrono::DateTime::parse_from_rfc3339(&r.started_at).unwrap().with_timezone(&chrono::Utc),
            completed_at: r.completed_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            duration_ms: r.duration_ms.map(|v| v as u64),
            raw_response_sample: r.raw_response_sample,
        }).collect())
    }

    pub async fn delete_old(&self, older_than: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM scan_logs WHERE started_at < ?",
            older_than
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}