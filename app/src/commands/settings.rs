//! Settings Commands

use anyhow::Result;
use tauri::{State, command};
use crate::db::DbPool;
use crate::db::repositories::SettingsRepository;
use crate::models::settings::{Settings, LLMSettings, PushSettings, SyncSettings, SettingsState};

#[command]
pub async fn get_settings(
    pool: State<'_, DbPool>,
) -> Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.get_all().await.map_err(|e| e.to_string())
}

#[command]
pub async fn update_settings(
    pool: State<'_, DbPool>,
    settings: Settings,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.save_all(&settings).await.map_err(|e| e.to_string())
}

#[command]
pub async fn update_llm_settings(
    pool: State<'_, DbPool>,
    settings: LLMSettings,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.set("llm", serde_json::to_value(settings).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn update_push_settings(
    pool: State<'_, DbPool>,
    settings: PushSettings,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.set("push", serde_json::to_value(settings).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn update_sync_settings(
    pool: State<'_, DbPool>,
    settings: SyncSettings,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.set("sync", serde_json::to_value(settings).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn reset_settings(
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.reset().await.map_err(|e| e.to_string())
}

#[command]
pub async fn gmail_configured(
    pool: State<'_, DbPool>,
    settings: State<'_, SettingsState>,
) -> Result<bool, String> {
    let settings_guard = settings.inner().read().await;
    Ok(settings_guard.gmail_client_id.is_some() && settings_guard.gmail_client_secret.is_some())
}