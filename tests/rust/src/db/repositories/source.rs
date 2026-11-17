//! Source Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::source::{Source, SourceStatus, SourceConfig, AuthData};

#[derive(Clone)]
pub struct SourceRepository {
    pool: SqlitePool,
}

impl SourceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, source: &Source) -> Result<()> {
        let config = serde_json::to_value(&source.config)?;
        let auth_data = source.auth_data.as_ref().map(|a| serde_json::to_value(a)).transpose()?;
        let source_type_str = source.source_type.as_str();
        let status_str = source.status.as_str();
        sqlx::query!(
            r#"
            INSERT INTO sources (id, name, source_type, config, schedule, enabled, status, auth_data, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            source.id,
            source.name,
            source_type_str,
            config,
            source.schedule,
            source.enabled,
            status_str,
            auth_data,
            source.created_at,
            source.updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Source>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, source_type, config, schedule, enabled, status, auth_data, last_scan_at, last_error, created_at, updated_at
            FROM sources WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let source_type_str = r.get::<String, _>("source_type");
            let capitalized = source_type_str.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default() + &source_type_str[1..];
            Source {
                id: r.get::<Uuid, _>("id"),
                name: r.get("name"),
                source_type: serde_json::from_str(&format!("\"{}\"", capitalized)).unwrap(),
                config: serde_json::from_str(&r.get::<String, _>("config")).unwrap(),
                schedule: r.get("schedule"),
                enabled: r.get::<i64, _>("enabled") != 0,
                status: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("status"))).unwrap(),
                auth_data: r.get::<Option<String>, _>("auth_data").map(|v| serde_json::from_str(&v).unwrap()),
                last_scan_at: r.get::<Option<String>, _>("last_scan_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
                last_error: r.get("last_error"),
                created_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
            }
        }))
    }

    pub async fn list(&self, enabled_only: bool) -> Result<Vec<Source>> {
        let query_str = if enabled_only {
            "SELECT id, name, source_type, config, schedule, enabled, status, auth_data, last_scan_at, last_error, created_at, updated_at FROM sources WHERE enabled = 1 ORDER BY created_at DESC"
        } else {
            "SELECT id, name, source_type, config, schedule, enabled, status, auth_data, last_scan_at, last_error, created_at, updated_at FROM sources ORDER BY created_at DESC"
        };

        let rows = sqlx::query(query_str).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| Source {
            id: r.get::<Uuid, _>("id"),
            name: r.get("name"),
            source_type: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("source_type"))).unwrap(),
            config: serde_json::from_str(&r.get::<String, _>("config")).unwrap(),
            schedule: r.get("schedule"),
            enabled: r.get::<i64, _>("enabled") != 0,
            status: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("status"))).unwrap(),
            auth_data: r.get::<Option<String>, _>("auth_data").map(|v| serde_json::from_str(&v).unwrap()),
            last_scan_at: r.get::<Option<String>, _>("last_scan_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            last_error: r.get("last_error"),
            created_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        }).collect())
    }

    pub async fn update(&self, source: &Source) -> Result<()> {
        let config = serde_json::to_value(&source.config)?;
        let auth_data = source.auth_data.as_ref().map(|a| serde_json::to_value(a)).transpose()?;
        let source_type_str = source.source_type.as_str();
        let status_str = source.status.as_str();
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE sources SET
                name = ?, source_type = ?, config = ?, schedule = ?,
                enabled = ?, status = ?, auth_data = ?, last_scan_at = ?,
                last_error = ?, updated_at = ?
            WHERE id = ?
            "#,
            source.name,
            source_type_str,
            config,
            source.schedule,
            source.enabled,
            status_str,
            auth_data,
            source.last_scan_at,
            source.last_error,
            now,
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
        let now = Utc::now();
        let status_str = status.as_str();
        sqlx::query!(
            "UPDATE sources SET status = ?, last_error = ?, updated_at = ? WHERE id = ?",
            status_str,
            error,
            now,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_last_scan(&self, id: Uuid, scanned_at: DateTime<Utc>) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            "UPDATE sources SET last_scan_at = ?, updated_at = ? WHERE id = ?",
            scanned_at,
            now,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}