//! Settings Models
//!
//! Application-wide configuration stored in SQLite.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: LLMProvider,
    pub model: String,
    pub base_url: Option<String>,      // For Ollama/local
    pub api_key: Option<String>,       // Encrypted
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LLMProvider {
    OpenAI,
    OpenAICompatible,
    Ollama,
    LlamaCpp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSettings {
    pub enabled: bool,
    pub fcm_token: Option<String>,
    pub min_match_score: f32,          // Threshold for notifications
    pub quiet_hours_start: Option<String>, // "22:00"
    pub quiet_hours_end: Option<String>,   // "08:00"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub enabled: bool,
    pub method: SyncMethod,
    pub peer_id: Option<String>,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMethod {
    LocalNetwork,
    QRCode,
    FileExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub llm: LLMSettings,
    pub push: PushSettings,
    pub sync: SyncSettings,
    pub theme: String,                 // "light", "dark", "auto"
    pub language: String,              // "en", "fr", "ar"
    pub scan_concurrency: u32,         // Max parallel source scans
    pub retention_days: u32,           // Auto-delete old postings
    // Gmail OAuth settings
    pub gmail_client_id: Option<String>,
    pub gmail_client_secret: Option<String>, // Encrypted
    pub gmail_redirect_uri: Option<String>,
    pub gmail_android_scheme: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm: LLMSettings {
                provider: LLMProvider::Ollama,
                model: "llama3.2".to_string(),
                base_url: Some("http://localhost:11434".to_string()),
                api_key: None,
                temperature: 0.3,
                max_tokens: Some(4096),
                timeout_seconds: 60,
            },
            push: PushSettings {
                enabled: false,
                fcm_token: None,
                min_match_score: 75.0,
                quiet_hours_start: Some("22:00".to_string()),
                quiet_hours_end: Some("08:00".to_string()),
            },
            sync: SyncSettings {
                enabled: false,
                method: SyncMethod::FileExport,
                peer_id: None,
                last_sync: None,
            },
            theme: "auto".to_string(),
            language: "en".to_string(),
            scan_concurrency: 3,
            retention_days: 90,
            gmail_client_id: None,
            gmail_client_secret: None,
            gmail_redirect_uri: Some("http://localhost:0/callback".to_string()),
            gmail_android_scheme: Some("com.tunisiacareerscout:/oauth2redirect".to_string()),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: Json<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

/// Shared state for settings (wrapped in RwLock for concurrent access)
pub type SettingsState = std::sync::Arc<tokio::sync::RwLock<Settings>>;