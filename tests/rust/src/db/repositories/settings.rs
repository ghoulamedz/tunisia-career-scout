//! Settings Repository

use anyhow::Result;
use sqlx::SqlitePool;
use serde_json::Value;
use crate::models::settings::{Settings, SettingRow};

#[derive(Clone)]
pub struct SettingsRepository {
    pool: SqlitePool,
}

impl SettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Value>> {
        let row = sqlx::query!(
            "SELECT value FROM settings WHERE key = ?",
            key
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.value.0))
    }

    pub async fn set(&self, key: &str, value: Value) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            key,
            value,
            chrono::Utc::now(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_all(&self) -> Result<Settings> {
        let rows = sqlx::query!("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await?;

        let mut settings = Settings::default();
        for row in rows {
            match row.key.as_str() {
                "llm" => settings.llm = serde_json::from_value(row.value.0).unwrap_or_default(),
                "push" => settings.push = serde_json::from_value(row.value.0).unwrap_or_default(),
                "sync" => settings.sync = serde_json::from_value(row.value.0).unwrap_or_default(),
                "theme" => settings.theme = row.value.0.as_str().unwrap_or("auto").to_string(),
                "language" => settings.language = row.value.0.as_str().unwrap_or("en").to_string(),
                "scan_concurrency" => settings.scan_concurrency = row.value.0.as_u64().unwrap_or(3) as u32,
                "retention_days" => settings.retention_days = row.value.0.as_u64().unwrap_or(90) as u32,
                "gmail_client_id" => settings.gmail_client_id = row.value.0.as_str().map(|s| s.to_string()),
                "gmail_client_secret" => settings.gmail_client_secret = row.value.0.as_str().map(|s| s.to_string()),
                "gmail_redirect_uri" => settings.gmail_redirect_uri = row.value.0.as_str().map(|s| s.to_string()),
                "gmail_android_scheme" => settings.gmail_android_scheme = row.value.0.as_str().map(|s| s.to_string()),
                _ => {}
            }
        }
        Ok(settings)
    }

    pub async fn save_all(&self, settings: &Settings) -> Result<()> {
        self.set("llm", serde_json::to_value(&settings.llm)?).await?;
        self.set("push", serde_json::to_value(&settings.push)?).await?;
        self.set("sync", serde_json::to_value(&settings.sync)?).await?;
        self.set("theme", serde_json::to_value(&settings.theme)?).await?;
        self.set("language", serde_json::to_value(&settings.language)?).await?;
        self.set("scan_concurrency", serde_json::to_value(settings.scan_concurrency)?).await?;
        self.set("retention_days", serde_json::to_value(settings.retention_days)?).await?;
        if let Some(v) = &settings.gmail_client_id { self.set("gmail_client_id", serde_json::to_value(v)?).await?; }
        if let Some(v) = &settings.gmail_client_secret { self.set("gmail_client_secret", serde_json::to_value(v)?).await?; }
        if let Some(v) = &settings.gmail_redirect_uri { self.set("gmail_redirect_uri", serde_json::to_value(v)?).await?; }
        if let Some(v) = &settings.gmail_android_scheme { self.set("gmail_android_scheme", serde_json::to_value(v)?).await?; }
        Ok(())
    }

    pub async fn reset(&self) -> Result<()> {
        sqlx::query!("DELETE FROM settings").execute(&self.pool).await?;
        Ok(())
    }
}