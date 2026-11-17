//! CV Repository

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::cv::{CVProfile, CVData, CVTemplate, CVTheme};

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

        Ok(row.map(self::row_to_profile))
    }

    pub async fn get_default(&self) -> Result<Option<CVProfile>> {
        let row = sqlx::query!(
            "SELECT * FROM cv_profiles WHERE is_default = 1 LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(self::row_to_profile))
    }

    pub async fn list(&self) -> Result<Vec<CVProfile>> {
        let rows = sqlx::query!(
            "SELECT * FROM cv_profiles ORDER BY is_default DESC, updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(self::row_to_profile).collect())
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

fn row_to_profile(row: sqlx::sqlite::SqliteRow) -> CVProfile {
    CVProfile {
        id: row.get("id"),
        name: row.get("name"),
        data: serde_json::from_value(row.get("data")).unwrap_or_default(),
        template: serde_json::from_value(row.get("template")).unwrap_or(CVTemplate::Modern),
        theme: serde_json::from_value(row.get("theme")).unwrap_or(CVTheme::Auto),
        is_default: row.get("is_default"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}