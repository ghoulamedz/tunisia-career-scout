//! Source Repository

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::source::{Source, SourceStatus, SourceConfig, AuthData};
use crate::models::scan_log::ScanLog;

pub struct SourceRepository {
    pool: SqlitePool,
}

impl SourceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, source: &Source) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sources (id, name, source_type, config, schedule, enabled, status, auth_data, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            source.id,
            source.name,
            source.source_type as _,
            serde_json::to_value(&source.config)?,
            source.schedule,
            source.enabled,
            source.status as _,
            source.auth_data.as_ref().map(|a| serde_json::to_value(a)).transpose()?,
            source.created_at,
            source.updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Source>> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, source_type, config, schedule, enabled, status, auth_data, last_scan_at, last_error, created_at, updated_at
            FROM sources WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Source {
            id: r.id,
            name: r.name,
            source_type: serde_json::from_value(r.source_type).unwrap(),
            config: serde_json::from_value(r.config).unwrap(),
            schedule: r.schedule,
            enabled: r.enabled,
            status: serde_json::from_value(r.status).unwrap(),
            auth_data: r.auth_data.map(|v| serde_json::from_value(v).unwrap()),
            last_scan_at: r.last_scan_at,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    pub async fn list(&self, enabled_only: bool) -> Result<Vec<Source>> {
        let query = if enabled_only {
            sqlx::query!(
                r#"
                SELECT id, name, source_type, config, schedule, enabled, status, auth_data, last_scan_at, last_error, created_at, updated_at
                FROM sources WHERE enabled = 1 ORDER BY created_at DESC
                "#
            )
        } else {
            sqlx::query!(
                r#"
                SELECT id, name, source_type, config, schedule, enabled, status, auth_data, last_scan_at, last_error, created_at, updated_at
                FROM sources ORDER BY created_at DESC
                "#
            )
        };

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| Source {
            id: r.id,
            name: r.name,
            source_type: serde_json::from_value(r.source_type).unwrap(),
            config: serde_json::from_value(r.config).unwrap(),
            schedule: r.schedule,
            enabled: r.enabled,
            status: serde_json::from_value(r.status).unwrap(),
            auth_data: r.auth_data.map(|v| serde_json::from_value(v).unwrap()),
            last_scan_at: r.last_scan_at,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }

    pub async fn update(&self, source: &Source) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE sources SET
                name = ?, source_type = ?, config = ?, schedule = ?,
                enabled = ?, status = ?, auth_data = ?, last_scan_at = ?,
                last_error = ?, updated_at = ?
            WHERE id = ?
            "#,
            source.name,
            source.source_type as _,
            serde_json::to_value(&source.config)?,
            source.schedule,
            source.enabled,
            source.status as _,
            source.auth_data.as_ref().map(|a| serde_json::to_value(a)).transpose()?,
            source.last_scan_at,
            source.last_error,
            Utc::now(),
            source.id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM sources WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: Uuid, status: SourceStatus, error: Option<String>) -> Result<()> {
        sqlx::query!(
            "UPDATE sources SET status = ?, last_error = ?, updated_at = ? WHERE id = ?",
            status as _,
            error,
            Utc::now(),
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_last_scan(&self, id: Uuid, scanned_at: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            "UPDATE sources SET last_scan_at = ?, updated_at = ? WHERE id = ?",
            scanned_at,
            Utc::now(),
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}