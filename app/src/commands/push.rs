//! Push Commands

use anyhow::Result;
use tauri::{State, command};
use crate::db::DbPool;
use crate::db::repositories::SettingsRepository;
use crate::models::settings::PushSettings;
use crate::push::fcm::FCMClient;

#[command]
pub async fn register_device(
    pool: State<'_, DbPool>,
    fcm_token: String,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    let mut settings = repo.get_all().await.map_err(|e| e.to_string())?;
    settings.push.fcm_token = Some(fcm_token);
    settings.push.enabled = true;
    repo.save_all(&settings).await.map_err(|e| e.to_string())
}

#[command]
pub async fn unregister_device(
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    let mut settings = repo.get_all().await.map_err(|e| e.to_string())?;
    settings.push.fcm_token = None;
    settings.push.enabled = false;
    repo.save_all(&settings).await.map_err(|e| e.to_string())
}

#[command]
pub async fn send_test_notification(
    pool: State<'_, DbPool>,
    title: String,
    body: String,
) -> Result<(), String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    let settings = repo.get_all().await.map_err(|e| e.to_string())?;

    let server_key = std::env::var("FCM_SERVER_KEY").map_err(|_| "FCM_SERVER_KEY not set".to_string())?;
    let project_id = std::env::var("FCM_PROJECT_ID").map_err(|_| "FCM_PROJECT_ID not set".to_string())?;

    let fcm = FCMClient::new(server_key, project_id);
    fcm.send(&settings.push.fcm_token.unwrap_or_default(), &title, &body, None).await
        .map_err(|e| e.to_string())
}