//! Source Models
//!
//! User-configured job sources with type-specific configuration.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::posting::ApplicationStep;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "source_type", rename_all = "lowercase")]
pub enum SourceType {
    Url,
    Rss,
    Gmail,
    Social,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Url => "url",
            SourceType::Rss => "rss",
            SourceType::Gmail => "gmail",
            SourceType::Social => "social",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "source_status", rename_all = "lowercase")]
pub enum SourceStatus {
    Active,
    Paused,
    Error,
}

impl SourceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceStatus::Active => "active",
            SourceStatus::Paused => "paused",
            SourceStatus::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SourceConfig {
    Url {
        url: String,
        selector: Option<String>,      // CSS selector for listing items
        detail_selector: Option<String>, // CSS selector for detail page
        pagination: Option<PaginationConfig>,
        headers: Option<serde_json::Value>, // Custom headers
    },
    Rss {
        url: String,
        item_xpath: Option<String>,    // XPath to item elements
        date_format: Option<String>,   // Custom date parsing
        field_mapping: Option<serde_json::Value>, // Custom RSS field -> JobPosting field mapping
        poll_interval_hours: Option<u32>, // Poll interval in hours (default: 1)
    },
    Gmail {
        label_ids: Vec<String>,        // Gmail labels to search
        query: Option<String>,         // Additional Gmail search query
        max_results: Option<u32>,      // Per-scan limit
    },
    Social {
        platform: SocialPlatform,
        target: SocialTarget,          // Group, page, hashtag, user
        cookies: Option<String>,       // Encrypted cookies JSON
        scroll_depth: Option<u32>,     // How far to scroll
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub type_: PaginationType,
    pub selector: Option<String>,      // Next page button/link
    pub max_pages: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaginationType {
    InfiniteScroll,
    NextButton,
    PageNumbers,
    UrlParam,                          // ?page=N
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SocialPlatform {
    Facebook,
    TikTok,
    LinkedIn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SocialTarget {
    Group { id: String },
    Page { id: String },
    Hashtag { tag: String },
    User { username: String },
    Search { query: String, location: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub auth_type: AuthType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,         // Encrypted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>, // Encrypted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<String>,       // Encrypted JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    None,
    Bearer,
    Cookie,
    OAuth2,
    AppPassword,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Source {
    pub id: Uuid,
    pub name: String,
    pub source_type: SourceType,
    pub config: Json<SourceConfig>,
    pub schedule: String,              // Cron expression (e.g., "0 * * * *")
    pub enabled: bool,
    pub status: SourceStatus,
    pub auth_data: Option<Json<AuthData>>,
    pub last_scan_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Source {
    pub fn new(name: String, source_type: SourceType, config: SourceConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            source_type,
            config: Json(config),
            schedule: "0 * * * *".to_string(), // Default: hourly
            enabled: true,
            status: SourceStatus::Active,
            auth_data: None,
            last_scan_at: None,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}