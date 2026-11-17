//! FCM Client
//!
//! Firebase Cloud Messaging integration for push notifications.

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::settings::PushSettings;

#[derive(Debug, Serialize)]
struct FCMMessage {
    token: String,
    notification: FCMNotification,
    data: Option<serde_json::Value>,
    android: Option<FCMAndroidConfig>,
}

#[derive(Debug, Serialize)]
struct FCMNotification {
    title: String,
    body: String,
    image: Option<String>,
}

#[derive(Debug, Serialize)]
struct FCMAndroidConfig {
    priority: String,
    notification: FCMAndroidNotification,
}

#[derive(Debug, Serialize)]
struct FCMAndroidNotification {
    channel_id: String,
    icon: String,
    color: String,
    click_action: String,
}

#[derive(Debug, Deserialize)]
struct FCMResponse {
    name: Option<String>,
    error: Option<FCMError>,
}

#[derive(Debug, Deserialize)]
struct FCMError {
    code: i32,
    message: String,
    status: String,
}

pub struct FCMClient {
    client: Client,
    server_key: String,
    project_id: String,
}

impl FCMClient {
    pub fn new(server_key: String, project_id: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, server_key, project_id }
    }

    pub async fn send(&self, token: &str, title: &str, body: &str, data: Option<serde_json::Value>) -> Result<()> {
        let message = FCMMessage {
            token: token.to_string(),
            notification: FCMNotification {
                title: title.to_string(),
                body: body.to_string(),
                image: None,
            },
            data,
            android: Some(FCMAndroidConfig {
                priority: "high".to_string(),
                notification: FCMAndroidNotification {
                    channel_id: "job_alerts".to_string(),
                    icon: "ic_notification".to_string(),
                    color: "#2196F3".to_string(),
                    click_action: "OPEN_APP".to_string(),
                },
            }),
        };

        let response = self.client
            .post(&format!("https://fcm.googleapis.com/v1/projects/{}/messages:send", self.project_id))
            .bearer_auth(&self.server_key)
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("FCM error: {}", error_text));
        }

        Ok(())
    }

    pub async fn send_match_notification(&self, settings: &PushSettings, posting_title: &str, company: &str, score: f32, posting_id: Uuid) -> Result<()> {
        if !settings.enabled || settings.fcm_token.is_none() {
            return Ok(());
        }

        if score < settings.min_match_score {
            return Ok(());
        }

        // Check quiet hours
        if self.is_quiet_hours(settings) {
            return Ok(());
        }

        let token = settings.fcm_token.as_ref().unwrap();
        let title = format!("New Match: {}", posting_title);
        let body = format!("{} - {}% match", company, score as u32);

        let data = serde_json::json!({
            "type": "job_match",
            "posting_id": posting_id.to_string(),
            "score": score,
        });

        self.send(token, &title, &body, Some(data)).await
    }

    fn is_quiet_hours(&self, settings: &PushSettings) -> bool {
        if let (Some(start), Some(end)) = (&settings.quiet_hours_start, &settings.quiet_hours_end) {
            let now = chrono::Local::now().time();
            let start_time = start.parse::<chrono::NaiveTime>().ok();
            let end_time = end.parse::<chrono::NaiveTime>().ok();

            if let (Some(start), Some(end)) = (start_time, end_time) {
                if start < end {
                    return now >= start && now <= end;
                } else {
                    // Overnight (e.g., 22:00 to 08:00)
                    return now >= start || now <= end;
                }
            }
        }
        false
    }
}

pub async fn register_device(token: String, project_id: String, server_key: String) -> Result<()> {
    // In a real implementation, this would register the token with your backend
    // For now, just store locally
    Ok(())
}

pub async fn send_notification(
    fcm_client: &FCMClient,
    settings: &PushSettings,
    title: &str,
    body: &str,
    data: Option<serde_json::Value>,
) -> Result<()> {
    if let Some(token) = &settings.fcm_token {
        fcm_client.send(token, title, body, data).await
    } else {
        Ok(())
    }
}