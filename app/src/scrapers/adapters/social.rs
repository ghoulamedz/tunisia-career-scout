//! Social Media Adapter
//!
//! Delegates to Python sidecar (Playwright) for Facebook, TikTok, LinkedIn scraping.
//!
//! The adapter communicates with the Python sidecar via JSON-RPC over stdin/stdout.
//! The sidecar is managed as a Tauri sidecar process, with support for both bundled
//! (production) and development paths.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use md5;
use crate::scrapers::pipeline::{RawContent, SourceAdapter, ContentType};
use crate::models::source::{SourceConfig, SocialPlatform, SocialTarget, AuthData};
use crate::sidecar::python::PythonSidecar;

/// JSON-RPC request sent to the Python sidecar
#[derive(Debug, Serialize)]
#[serde(tag = "command")]
enum SidecarCommand {
    Login { platform: String, url: String },
    Scrape { platform: String, target: SocialTargetConfig, cookies: Option<String>, scroll_depth: Option<u32> },
}

/// Target configuration for scraping, serialized to match Python sidecar expectations
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum SocialTargetConfig {
    Group { id: String },
    Page { id: String },
    Hashtag { tag: String },
    User { username: String },
    Search { query: String, location: Option<String> },
}

/// Response from the Python sidecar
#[derive(Debug, Clone, Deserialize)]
struct SidecarResponse {
    success: bool,
    data: Option<SidecarData>,
    error: Option<SidecarError>,
}

/// Error information from sidecar
#[derive(Debug, Clone, Deserialize)]
struct SidecarError {
    code: String,
    message: String,
    #[serde(default)]
    details: serde_json::Value,
}

/// Data payload from successful sidecar response
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum SidecarData {
    Login { cookies: String },
    Scrape { items: Vec<SidecarItem> },
}

/// Individual scraped item from sidecar
#[derive(Debug, Clone, Deserialize)]
struct SidecarItem {
    url: String,
    title: String,
    content: String,
    author: Option<String>,
    posted_at: Option<String>,
    metadata: serde_json::Value,
}

/// Default timeout for scrape operations (5 minutes)
const SCRAPE_TIMEOUT: Duration = Duration::from_secs(300);
/// Default timeout for login operations (10 minutes - user interaction required)
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);

/// Errors specific to the SocialAdapter
#[derive(Debug, thiserror::Error)]
pub enum SocialAdapterError {
    #[error("Sidecar error: {code} - {message}")]
    SidecarError { code: String, message: String },
    #[error("Sidecar timeout after {seconds}s")]
    Timeout { seconds: u64 },
    #[error("Invalid sidecar response: {0}")]
    InvalidResponse(String),
    #[error("No cookies returned from login")]
    NoCookies,
    #[error("SocialAdapter only handles Social source type")]
    WrongSourceType,
}

/// SocialAdapter bridges the Rust Tauri app to the Python Playwright sidecar.
///
/// It handles:
/// - Sidecar path resolution (bundled resource vs dev path)
/// - JSON-RPC communication over stdin/stdout
/// - Timeout handling for long-running scrapes
/// - Cookie persistence via AuthData
pub struct SocialAdapter {
    sidecar: PythonSidecar,
}

impl SocialAdapter {
    /// Create a new SocialAdapter with the given PythonSidecar.
    ///
    /// The sidecar handles path resolution for both bundled (production) and
    /// development environments.
    pub fn new(sidecar: PythonSidecar) -> Self {
        Self { sidecar }
    }

    /// Create a SocialAdapter from an AppHandle (production use).
    ///
    /// This resolves the bundled sidecar path via Tauri's resource_dir.
    pub fn from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        let sidecar = PythonSidecar::new(app_handle)?;
        Ok(Self::new(sidecar))
    }

    /// Create a SocialAdapter with a custom sidecar script path (development/testing).
    ///
    /// Uses the system Python interpreter with the given script path.
    pub fn from_script_path(script_path: impl Into<std::path::PathBuf>) -> Self {
        let sidecar = PythonSidecar::from_script_path(script_path);
        Self::new(sidecar)
    }

    fn target_to_config(target: &SocialTarget) -> SocialTargetConfig {
        match target {
            SocialTarget::Group { id } => SocialTargetConfig::Group { id: id.clone() },
            SocialTarget::Page { id } => SocialTargetConfig::Page { id: id.clone() },
            SocialTarget::Hashtag { tag } => SocialTargetConfig::Hashtag { tag: tag.clone() },
            SocialTarget::User { username } => SocialTargetConfig::User { username: username.clone() },
            SocialTarget::Search { query, location } => SocialTargetConfig::Search { query: query.clone(), location: location.clone() },
        }
    }

    fn platform_to_str(platform: &SocialPlatform) -> &'static str {
        match platform {
            SocialPlatform::Facebook => "facebook",
            SocialPlatform::TikTok => "tiktok",
            SocialPlatform::LinkedIn => "linkedin",
        }
    }

    /// Send a command to the sidecar with timeout handling.
    async fn send_command_with_timeout(&self, cmd: SidecarCommand, timeout_duration: Duration) -> Result<SidecarResponse> {
        let input = serde_json::to_value(&cmd)?;
        let response = timeout(timeout_duration, self.sidecar.run_command(input))
            .await
            .map_err(|_| SocialAdapterError::Timeout { seconds: timeout_duration.as_secs() })??;

        // Parse the JSON-RPC response
        let success = response.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let data = response.get("data").cloned().and_then(|v| serde_json::from_value(v).ok());
        let error = response.get("error").cloned().and_then(|v| serde_json::from_value(v).ok());

        let sidecar_response = SidecarResponse { success, data, error };
        Ok(sidecar_response)
    }

    /// Parse SidecarData::Scrape items into RawContent vec
    fn parse_scrape_items(&self, items: Vec<SidecarItem>, platform_str: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        for item in items {
            let content_hash = format!("{:x}", md5::compute(&item.content));
            let posted_at = item.posted_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            results.push(RawContent {
                source_url: item.url,
                content: item.content,
                content_type: ContentType::Html,
                fetched_at: posted_at,
                content_hash,
                metadata: serde_json::json!({
                    "platform": platform_str,
                    "author": item.author,
                    "metadata": item.metadata,
                }),
            });
        }
        results
    }
}

#[async_trait]
impl SourceAdapter for SocialAdapter {
    async fn fetch(&self, config: &SourceConfig) -> Result<Vec<RawContent>> {
        let SourceConfig::Social { platform, target, cookies, scroll_depth } = config else {
            return Err(SocialAdapterError::WrongSourceType.into());
        };

        let platform_str = Self::platform_to_str(platform);
        let target_config = Self::target_to_config(target);

        // Prepare scrape command
        let cmd = SidecarCommand::Scrape {
            platform: platform_str.to_string(),
            target: target_config,
            cookies: cookies.clone(),
            scroll_depth: *scroll_depth,
        };

        // Send with scrape timeout
        let response = self.send_command_with_timeout(cmd, SCRAPE_TIMEOUT).await?;

        if !response.success {
            let err = response.error.unwrap_or(SidecarError {
                code: "UNKNOWN_ERROR".to_string(),
                message: "Unknown sidecar error".to_string(),
                details: serde_json::Value::Null,
            });
            return Err(SocialAdapterError::SidecarError { code: err.code, message: err.message }.into());
        }

        if let Some(SidecarData::Scrape { items }) = response.data {
            Ok(self.parse_scrape_items(items, platform_str))
        } else {
            Err(SocialAdapterError::InvalidResponse("Expected scrape data in response".to_string()).into())
        }
    }

    fn source_type(&self) -> &'static str {
        "social"
    }
}

impl SocialAdapter {
    /// Perform login for a social platform.
    ///
    /// Opens a browser window for the user to authenticate, then returns
    /// the session cookies as a JSON string. These cookies should be stored
    /// in `Source.auth_data.cookies` for subsequent scrapes.
    ///
    /// # Arguments
    /// * `platform` - The social platform to log into
    /// * `url` - The login URL (e.g., "https://www.facebook.com/login")
    ///
    /// # Returns
    /// JSON string of cookies for the authenticated session
    pub async fn login(&self, platform: &SocialPlatform, url: &str) -> Result<String> {
        let platform_str = Self::platform_to_str(platform);
        let cmd = SidecarCommand::Login {
            platform: platform_str.to_string(),
            url: url.to_string(),
        };

        let response = self.send_command_with_timeout(cmd, LOGIN_TIMEOUT).await?;

        if !response.success {
            let err = response.error.unwrap_or(SidecarError {
                code: "LOGIN_FAILED".to_string(),
                message: "Login failed".to_string(),
                details: serde_json::Value::Null,
            });
            return Err(SocialAdapterError::SidecarError { code: err.code, message: err.message }.into());
        }

        if let Some(SidecarData::Login { cookies }) = response.data {
            if cookies.is_empty() {
                return Err(SocialAdapterError::NoCookies.into());
            }
            Ok(cookies)
        } else {
            Err(SocialAdapterError::InvalidResponse("Expected login data in response".to_string()).into())
        }
    }

    /// Create AuthData with cookies for storage in Source.auth_data
    pub fn create_auth_data(&self, cookies: String) -> AuthData {
        AuthData {
            auth_type: crate::models::source::AuthType::Cookie,
            token: None,
            refresh_token: None,
            cookies: Some(cookies),
            expires_at: None, // Cookies don't have a standard expiry in this format
        }
    }
}

/// Trait for mocking sidecar communication in tests
#[async_trait]
pub trait SidecarTransport: Send + Sync {
    async fn send_command(&self, command: SidecarCommand) -> Result<SidecarResponse>;
}

/// Mock transport for unit testing
#[cfg(test)]
pub struct MockSidecarTransport {
    responses: std::sync::Mutex<std::collections::HashMap<String, SidecarResponse>>,
    default_response: std::sync::Mutex<Option<SidecarResponse>>,
}

#[cfg(test)]
impl MockSidecarTransport {
    pub fn new() -> Self {
        Self {
            responses: std::sync::Mutex::new(std::collections::HashMap::new()),
            default_response: std::sync::Mutex::new(None),
        }
    }

    pub fn set_response(&self, command_type: &str, response: SidecarResponse) {
        self.responses.lock().unwrap().insert(command_type.to_string(), response);
    }

    pub fn set_default_response(&self, response: SidecarResponse) {
        *self.default_response.lock().unwrap() = Some(response);
    }
}

#[cfg(test)]
#[async_trait]
impl SidecarTransport for MockSidecarTransport {
    async fn send_command(&self, command: SidecarCommand) -> Result<SidecarResponse> {
        let cmd_type = match &command {
            SidecarCommand::Login { .. } => "login",
            SidecarCommand::Scrape { .. } => "scrape",
        };

        if let Some(resp) = self.responses.lock().unwrap().get(cmd_type).cloned() {
            return Ok(resp);
        }

        if let Some(resp) = self.default_response.lock().unwrap().clone() {
            return Ok(resp);
        }

        Err(anyhow::anyhow!("No mock response configured for {}", cmd_type))
    }
}

/// Test-only adapter that uses a mock transport
#[cfg(test)]
pub struct TestSocialAdapter {
    transport: Box<dyn SidecarTransport>,
}

#[cfg(test)]
impl TestSocialAdapter {
    pub fn new(transport: Box<dyn SidecarTransport>) -> Self {
        Self { transport }
    }

    fn target_to_config(target: &SocialTarget) -> SocialTargetConfig {
        match target {
            SocialTarget::Group { id } => SocialTargetConfig::Group { id: id.clone() },
            SocialTarget::Page { id } => SocialTargetConfig::Page { id: id.clone() },
            SocialTarget::Hashtag { tag } => SocialTargetConfig::Hashtag { tag: tag.clone() },
            SocialTarget::User { username } => SocialTargetConfig::User { username: username.clone() },
            SocialTarget::Search { query, location } => SocialTargetConfig::Search { query: query.clone(), location: location.clone() },
        }
    }

    fn platform_to_str(platform: &SocialPlatform) -> &'static str {
        match platform {
            SocialPlatform::Facebook => "facebook",
            SocialPlatform::TikTok => "tiktok",
            SocialPlatform::LinkedIn => "linkedin",
        }
    }

    fn parse_scrape_items(&self, items: Vec<SidecarItem>, platform_str: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        for item in items {
            let content_hash = format!("{:x}", md5::compute(&item.content));
            let posted_at = item.posted_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            results.push(RawContent {
                source_url: item.url,
                content: item.content,
                content_type: ContentType::Html,
                fetched_at: posted_at,
                content_hash,
                metadata: serde_json::json!({
                    "platform": platform_str,
                    "author": item.author,
                    "metadata": item.metadata,
                }),
            });
        }
        results
    }

    pub async fn fetch(&self, config: &SourceConfig) -> Result<Vec<RawContent>> {
        let SourceConfig::Social { platform, target, cookies, scroll_depth } = config else {
            return Err(SocialAdapterError::WrongSourceType.into());
        };

        let platform_str = Self::platform_to_str(platform);
        let target_config = Self::target_to_config(target);

        let cmd = SidecarCommand::Scrape {
            platform: platform_str.to_string(),
            target: target_config,
            cookies: cookies.clone(),
            scroll_depth: *scroll_depth,
        };

        let response = self.transport.send_command(cmd).await?;

        if !response.success {
            let err = response.error.unwrap_or(SidecarError {
                code: "UNKNOWN_ERROR".to_string(),
                message: "Unknown sidecar error".to_string(),
                details: serde_json::Value::Null,
            });
            return Err(SocialAdapterError::SidecarError { code: err.code, message: err.message }.into());
        }

        if let Some(SidecarData::Scrape { items }) = response.data {
            Ok(self.parse_scrape_items(items, platform_str))
        } else {
            Err(SocialAdapterError::InvalidResponse("Expected scrape data in response".to_string()).into())
        }
    }

    pub async fn login(&self, platform: &SocialPlatform, url: &str) -> Result<String> {
        let platform_str = Self::platform_to_str(platform);
        let cmd = SidecarCommand::Login {
            platform: platform_str.to_string(),
            url: url.to_string(),
        };

        let response = self.transport.send_command(cmd).await?;

        if !response.success {
            let err = response.error.unwrap_or(SidecarError {
                code: "LOGIN_FAILED".to_string(),
                message: "Login failed".to_string(),
                details: serde_json::Value::Null,
            });
            return Err(SocialAdapterError::SidecarError { code: err.code, message: err.message }.into());
        }

        if let Some(SidecarData::Login { cookies }) = response.data {
            if cookies.is_empty() {
                return Err(SocialAdapterError::NoCookies.into());
            }
            Ok(cookies)
        } else {
            Err(SocialAdapterError::InvalidResponse("Expected login data in response".to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::models::source::{SocialPlatform, SocialTarget, SourceConfig};

    fn make_mock_scrape_response(items: Vec<SidecarItem>) -> SidecarResponse {
        SidecarResponse {
            success: true,
            data: Some(SidecarData::Scrape { items }),
            error: None,
        }
    }

    fn make_mock_login_response(cookies: String) -> SidecarResponse {
        SidecarResponse {
            success: true,
            data: Some(SidecarData::Login { cookies }),
            error: None,
        }
    }

    fn make_mock_error_response(code: &str, message: &str) -> SidecarResponse {
        SidecarResponse {
            success: false,
            data: None,
            error: Some(SidecarError {
                code: code.to_string(),
                message: message.to_string(),
                details: serde_json::Value::Null,
            }),
        }
    }

    fn sample_sidecar_item() -> SidecarItem {
        SidecarItem {
            url: "https://facebook.com/groups/123/posts/456".to_string(),
            title: "Job Posting Title".to_string(),
            content: "We are hiring a Rust developer in Tunis. Contact us at jobs@example.com".to_string(),
            author: Some("Jane Doe".to_string()),
            posted_at: Some("2024-01-15T10:30:00Z".to_string()),
            metadata: serde_json::json!({"platform": "facebook", "group_id": "123"}),
        }
    }

    fn sample_cookies() -> String {
        r#"[{"name": "c_user", "value": "12345", "domain": ".facebook.com", "path": "/", "expires": 1735689600, "httpOnly": true, "secure": true, "sameSite": "None"}]"#.to_string()
    }

    #[tokio::test]
    async fn test_fetch_facebook_group_success() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("scrape", make_mock_scrape_response(vec![sample_sidecar_item()]));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Social {
            platform: SocialPlatform::Facebook,
            target: SocialTarget::Group { id: "123".to_string() },
            cookies: Some(sample_cookies()),
            scroll_depth: Some(5),
        };

        let results = adapter.fetch(&config).await.unwrap();

        assert_eq!(results.len(), 1);
        let content = &results[0];
        assert_eq!(content.source_url, "https://facebook.com/groups/123/posts/456");
        assert!(content.content.contains("Rust developer"));
        assert_eq!(content.metadata["platform"], "facebook");
        assert_eq!(content.metadata["author"], "Jane Doe");
        assert_eq!(content.content_type, ContentType::Html);
    }

    #[tokio::test]
    async fn test_fetch_tiktok_hashtag_success() {
        let mut transport = MockSidecarTransport::new();
        let item = SidecarItem {
            url: "https://tiktok.com/@user/video/789".to_string(),
            title: "TikTok Job Video".to_string(),
            content: "Hiring developers! #jobs #tunisia".to_string(),
            author: Some("recruiter_tn".to_string()),
            posted_at: Some("2024-01-16T14:00:00Z".to_string()),
            metadata: serde_json::json!({"platform": "tiktok", "hashtags": ["jobs", "tunisia"]}),
        };
        transport.set_response("scrape", make_mock_scrape_response(vec![item]));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Social {
            platform: SocialPlatform::TikTok,
            target: SocialTarget::Hashtag { tag: "jobs".to_string() },
            cookies: None,
            scroll_depth: Some(3),
        };

        let results = adapter.fetch(&config).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata["platform"], "tiktok");
        assert_eq!(results[0].metadata["author"], "recruiter_tn");
    }

    #[tokio::test]
    async fn test_fetch_linkedin_search_success() {
        let mut transport = MockSidecarTransport::new();
        let item = SidecarItem {
            url: "https://linkedin.com/jobs/view/999".to_string(),
            title: "Senior Rust Engineer".to_string(),
            content: "Senior Rust Engineer at TechCorp in Tunis".to_string(),
            author: Some("TechCorp".to_string()),
            posted_at: Some("2024-01-17T09:00:00Z".to_string()),
            metadata: serde_json::json!({"platform": "linkedin", "company": "TechCorp", "location": "Tunis", "job_id": "999"}),
        };
        transport.set_response("scrape", make_mock_scrape_response(vec![item]));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Social {
            platform: SocialPlatform::LinkedIn,
            target: SocialTarget::Search { query: "Rust".to_string(), location: Some("Tunisia".to_string()) },
            cookies: Some(sample_cookies()),
            scroll_depth: Some(10),
        };

        let results = adapter.fetch(&config).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata["platform"], "linkedin");
        assert_eq!(results[0].metadata["company"], "TechCorp");
        assert_eq!(results[0].metadata["location"], "Tunis");
    }

    #[tokio::test]
    async fn test_fetch_multiple_items() {
        let mut transport = MockSidecarTransport::new();
        let items = vec![
            sample_sidecar_item(),
            SidecarItem {
                url: "https://facebook.com/groups/123/posts/789".to_string(),
                title: "Another Job".to_string(),
                content: "Python developer needed".to_string(),
                author: Some("John Smith".to_string()),
                posted_at: Some("2024-01-15T11:00:00Z".to_string()),
                metadata: serde_json::json!({"platform": "facebook"}),
            },
        ];
        transport.set_response("scrape", make_mock_scrape_response(items));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Social {
            platform: SocialPlatform::Facebook,
            target: SocialTarget::Group { id: "123".to_string() },
            cookies: Some(sample_cookies()),
            scroll_depth: Some(5),
        };

        let results = adapter.fetch(&config).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[1].content, "Python developer needed");
        assert_eq!(results[1].metadata["author"], "John Smith");
    }

    #[tokio::test]
    async fn test_fetch_sidecar_error() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("scrape", make_mock_error_response("SCRAPE_FAILED", "Failed to load page"));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Social {
            platform: SocialPlatform::Facebook,
            target: SocialTarget::Group { id: "123".to_string() },
            cookies: Some(sample_cookies()),
            scroll_depth: Some(5),
        };

        let result = adapter.fetch(&config).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("SCRAPE_FAILED"));
        assert!(err.contains("Failed to load page"));
    }

    #[tokio::test]
    async fn test_login_facebook_success() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("login", make_mock_login_response(sample_cookies()));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let cookies = adapter.login(&SocialPlatform::Facebook, "https://facebook.com/login").await.unwrap();

        assert!(!cookies.is_empty());
        assert!(cookies.contains("c_user"));
        assert!(cookies.contains(".facebook.com"));
    }

    #[tokio::test]
    async fn test_login_tiktok_success() {
        let mut transport = MockSidecarTransport::new();
        let cookies = r#"[{"name": "sessionid", "value": "abc123", "domain": ".tiktok.com", "path": "/", "expires": 1735689600, "httpOnly": true, "secure": true, "sameSite": "None"}]"#.to_string();
        transport.set_response("login", make_mock_login_response(cookies));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let result = adapter.login(&SocialPlatform::TikTok, "https://tiktok.com/login").await.unwrap();

        assert!(result.contains("sessionid"));
        assert!(result.contains(".tiktok.com"));
    }

    #[tokio::test]
    async fn test_login_linkedin_success() {
        let mut transport = MockSidecarTransport::new();
        let cookies = r#"[{"name": "li_at", "value": "xyz789", "domain": ".linkedin.com", "path": "/", "expires": 1735689600, "httpOnly": true, "secure": true, "sameSite": "None"}]"#.to_string();
        transport.set_response("login", make_mock_login_response(cookies));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let result = adapter.login(&SocialPlatform::LinkedIn, "https://linkedin.com/login").await.unwrap();

        assert!(result.contains("li_at"));
        assert!(result.contains(".linkedin.com"));
    }

    #[tokio::test]
    async fn test_login_sidecar_error() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("login", make_mock_error_response("LOGIN_TIMEOUT", "User did not complete login in time"));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let result = adapter.login(&SocialPlatform::Facebook, "https://facebook.com/login").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("LOGIN_TIMEOUT"));
        assert!(err.contains("User did not complete login"));
    }

    #[tokio::test]
    async fn test_login_empty_cookies() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("login", make_mock_login_response("[]".to_string()));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let result = adapter.login(&SocialPlatform::Facebook, "https://facebook.com/login").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No cookies returned"));
    }

    #[tokio::test]
    async fn test_create_auth_data() {
        let cookies = sample_cookies();
        let auth_data = AuthData {
            auth_type: crate::models::source::AuthType::Cookie,
            token: None,
            refresh_token: None,
            cookies: Some(cookies.clone()),
            expires_at: None,
        };

        assert_eq!(auth_data.auth_type, crate::models::source::AuthType::Cookie);
        assert_eq!(auth_data.cookies, Some(cookies));
        assert!(auth_data.token.is_none());
        assert!(auth_data.refresh_token.is_none());
    }

    #[tokio::test]
    async fn test_fetch_wrong_source_type() {
        let transport = MockSidecarTransport::new();
        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Url {
            url: "https://example.com".to_string(),
            selector: None,
            detail_selector: None,
            pagination: None,
            headers: None,
        };

        let result = adapter.fetch(&config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SocialAdapter only handles Social source type"));
    }

    #[tokio::test]
    async fn test_fetch_all_target_types() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("scrape", make_mock_scrape_response(vec![sample_sidecar_item()]));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        // Test Group
        let config = SourceConfig::Social {
            platform: SocialPlatform::Facebook,
            target: SocialTarget::Group { id: "123".to_string() },
            cookies: None,
            scroll_depth: Some(5),
        };
        adapter.fetch(&config).await.unwrap();

        // Test Page
        let config = SourceConfig::Social {
            platform: SocialPlatform::Facebook,
            target: SocialTarget::Page { id: "page123".to_string() },
            cookies: None,
            scroll_depth: Some(5),
        };
        adapter.fetch(&config).await.unwrap();

        // Test Hashtag
        let config = SourceConfig::Social {
            platform: SocialPlatform::TikTok,
            target: SocialTarget::Hashtag { tag: "jobs".to_string() },
            cookies: None,
            scroll_depth: Some(5),
        };
        adapter.fetch(&config).await.unwrap();

        // Test User
        let config = SourceConfig::Social {
            platform: SocialPlatform::TikTok,
            target: SocialTarget::User { username: "recruiter".to_string() },
            cookies: None,
            scroll_depth: Some(5),
        };
        adapter.fetch(&config).await.unwrap();

        // Test Search
        let config = SourceConfig::Social {
            platform: SocialPlatform::LinkedIn,
            target: SocialTarget::Search { query: "Rust".to_string(), location: Some("Tunisia".to_string()) },
            cookies: None,
            scroll_depth: Some(5),
        };
        adapter.fetch(&config).await.unwrap();
    }

    #[tokio::test]
    async fn test_fetch_empty_results() {
        let mut transport = MockSidecarTransport::new();
        transport.set_response("scrape", make_mock_scrape_response(vec![]));

        let adapter = TestSocialAdapter::new(Box::new(transport));

        let config = SourceConfig::Social {
            platform: SocialPlatform::Facebook,
            target: SocialTarget::Group { id: "empty".to_string() },
            cookies: Some(sample_cookies()),
            scroll_depth: Some(5),
        };

        let results = adapter.fetch(&config).await.unwrap();

        assert_eq!(results.len(), 0);
    }
}