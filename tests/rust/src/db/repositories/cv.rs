//! CV Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::cv::{CVProfile, CVData, CVTemplate, CVTheme};

#[derive(Clone)]
pub struct CVRepository {
    pool: SqlitePool,
}

impl CVRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, profile: &CVProfile) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO cv_profiles (id, name, data, template, theme, is_default, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            profile.id,
            profile.name,
            serde_json::to_value(&profile.data)?,
            profile.template as _,
            profile.theme as _,
            profile.is_default,
            profile.created_at,
            profile.updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<CVProfile>> {
        let row = sqlx::query!(
            "SELECT * FROM cv_profiles WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| CVProfile {
            id: r.id,
            name: r.name,
            data: serde_json::from_value(r.data).unwrap_or_default(),
            template: serde_json::from_value(r.template).unwrap_or(CVTemplate::Modern),
            theme: serde_json::from_value(r.theme).unwrap_or(CVTheme::Auto),
            is_default: r.is_default != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap().with_timezone(&chrono::Utc),
        }))
    }

    pub async fn get_default(&self) -> Result<Option<CVProfile>> {
        let row = sqlx::query!(
            "SELECT * FROM cv_profiles WHERE is_default = 1 LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| CVProfile {
            id: r.id,
            name: r.name,
            data: serde_json::from_value(r.data).unwrap_or_default(),
            template: serde_json::from_value(r.template).unwrap_or(CVTemplate::Modern),
            theme: serde_json::from_value(r.theme).unwrap_or(CVTheme::Auto),
            is_default: r.is_default != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap().with_timezone(&chrono::Utc),
        }))
    }

    pub async fn list(&self) -> Result<Vec<CVProfile>> {
        let rows = sqlx::query!(
            "SELECT * FROM cv_profiles ORDER BY is_default DESC, updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| CVProfile {
            id: r.id,
            name: r.name,
            data: serde_json::from_value(r.data).unwrap_or_default(),
            template: serde_json::from_value(r.template).unwrap_or(CVTemplate::Modern),
            theme: serde_json::from_value(r.theme).unwrap_or(CVTheme::Auto),
            is_default: r.is_default != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at).unwrap().with_timezone(&chrono::Utc),
        }).collect())
    }

    pub async fn update(&self, profile: &CVProfile) -> Result<()> {
        // If this is set as default, unset others
        if profile.is_default {
            sqlx::query!("UPDATE cv_profiles SET is_default = 0 WHERE id != ?", profile.id)
                .execute(&self.pool)
                .await?;
        }

        sqlx::query!(
            r#"
            UPDATE cv_profiles SET
                name = ?, data = ?, template = ?, theme = ?, is_default = ?, updated_at = ?
            WHERE id = ?
            "#,
            profile.name,
            serde_json::to_value(&profile.data)?,
            profile.template as _,
            profile.theme as _,
            profile.is_default,
            Utc::now(),
            profile.id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM cv_profiles WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_default(&self, id: Uuid) -> Result<()> {
        sqlx::query!("UPDATE cv_profiles SET is_default = 0").execute(&self.pool).await?;
        sqlx::query!("UPDATE cv_profiles SET is_default = 1, updated_at = ? WHERE id = ?", Utc::now(), id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}