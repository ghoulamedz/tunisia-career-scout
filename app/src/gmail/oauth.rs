//! Gmail OAuth2
//!
//! Handles OAuth2 flow for Gmail API access with PKCE support,
//! CSRF protection via state parameter, local callback server
//! (desktop) and custom scheme (Android), and secure token storage.

use anyhow::{anyhow, Context, Result};
use google_gmail1::Gmail;
use oauth2::basic::{
    BasicClient, BasicErrorResponse, BasicTokenResponse, BasicTokenIntrospectionResponse,
    BasicRevocationErrorResponse, BasicErrorResponseType, BasicTokenType,
};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
    AuthorizationCode, RefreshToken, AccessToken,
    PkceCodeChallenge, PkceCodeVerifier, PkceCodeChallengeMethod,
    StandardRevocableToken,
    EndpointSet, EndpointNotSet,
    TokenResponse,
};
use oauth2::Client as OAuth2Client;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{info, warn};
use url::Url;

use crate::models::source::AuthData;
use crate::models::settings::SettingsState;
use crate::db::DbPool;
use once_cell::sync::Lazy;

/// OAuth2 configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,        // Desktop: http://localhost:PORT/callback
    pub android_scheme: Option<String>, // Android: com.app:/oauth2redirect
}

/// Stored token data (encrypted at rest via keyring)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp
    pub scope: String,
}

impl StoredToken {
    pub fn is_expired(&self, buffer_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at.saturating_sub(buffer_seconds) <= now
    }
}

/// PKCE challenge/verifier pair for public clients
#[derive(Debug, Clone)]
pub struct PkcePair {
    pub verifier: String,  // Store as String since PkceCodeVerifier is not Clone
    pub challenge: PkceCodeChallenge,
}

impl PkcePair {
    /// Generate a new PKCE pair (S256 method)
    pub fn new() -> Self {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        Self {
            verifier: verifier.secret().to_string(),
            challenge,
        }
    }
}

/// OAuth2 session state for CSRF protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub pkce_verifier: String,
    pub created_at: u64,
    pub redirect_uri: String,
    pub android_scheme: Option<String>,
}

/// Global state store for OAuth flows (shared across command invocations)
static OAUTH_STATE_STORE: Lazy<Arc<RwLock<HashMap<String, OAuthState>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Gmail OAuth2 handler
pub struct GmailOAuth {
    config: OAuthConfig,
    http_client: Client,
    basic_client: BasicClient<
        EndpointSet,     // HasAuthUrl
        EndpointNotSet,  // HasDeviceAuthUrl
        EndpointNotSet,  // HasIntrospectionUrl
        EndpointNotSet,  // HasRevocationUrl
        EndpointSet,     // HasTokenUrl
    >,
    // Database pool for token persistence
    db_pool: Option<DbPool>,
}

impl GmailOAuth {
    /// Create a new GmailOAuth instance
    pub fn new(config: OAuthConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())
            .context("Invalid auth URL")?;
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .context("Invalid token URL")?;

        let basic_client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        Ok(Self {
            config,
            http_client,
            basic_client,
            db_pool: None,
        })
    }

    /// Set database pool for token persistence
    pub fn with_db_pool(mut self, pool: DbPool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Create a new GmailOAuth instance with individual parameters (compat)
    pub fn new_with_params(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Result<Self> {
        let config = OAuthConfig {
            client_id,
            client_secret,
            redirect_uri,
            android_scheme: None,
        };
        Self::new(config)
    }

    /// Create GmailOAuth from settings
    pub async fn from_settings(settings: &SettingsState) -> Result<Self> {
        let settings_guard = settings.inner().read().await;
        let client_id = settings_guard
            .gmail_client_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Gmail client ID not configured"))?
            .clone();
        let client_secret = settings_guard
            .gmail_client_secret
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Gmail client secret not configured"))?
            .clone();
        let redirect_uri = settings_guard
            .gmail_redirect_uri
            .clone()
            .unwrap_or_else(|| "http://localhost/callback".to_string());
        let android_scheme = settings_guard.gmail_android_scheme.clone();

        let config = OAuthConfig {
            client_id,
            client_secret,
            redirect_uri,
            android_scheme,
        };
        Self::new(config)
    }

    /// Get valid access token for Gmail API
    /// Returns the access token string that can be used with any HTTP client
    pub async fn get_valid_access_token(&self) -> Result<String> {
        self.get_valid_token().await
    }

    /// Get Gmail API service with valid access token
    /// Note: This requires a hyper-based HTTP client compatible with google-gmail1
    /// For now, use get_valid_access_token() and create your own client
    pub async fn get_gmail_service(&self, _auth: &AuthData) -> Result<Gmail<reqwest::Client>> {
        Err(anyhow::anyhow!("Gmail service creation requires hyper client; use get_valid_access_token() instead"))
    }

    /// Generate PKCE pair for public clients (Android/desktop)
    pub fn generate_pkce() -> PkcePair {
        PkcePair::new()
    }

    /// Start OAuth2 flow - returns authorization URL and state
    /// Call this from frontend to initiate the flow
    pub async fn start_auth(&self) -> Result<(String, String)> {
        // Generate CSRF state token
        let state = CsrfToken::new_random();
        let state_str = state.secret().to_string();

        // Generate PKCE pair
        let pkce = Self::generate_pkce();
        let pkce_verifier = pkce.verifier.clone();

        // Build authorization URL with PKCE and state
        let (auth_url, _csrf_token) = self.basic_client
            .authorize_url(|| CsrfToken::new(state_str.clone()))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/gmail.readonly".to_string(),
            ))
            .set_pkce_challenge(pkce.challenge)
            .url();

        // Store state with PKCE verifier for callback validation
        let oauth_state = OAuthState {
            state: state_str.clone(),
            pkce_verifier,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            redirect_uri: self.config.redirect_uri.clone(),
            android_scheme: self.config.android_scheme.clone(),
        };

        OAUTH_STATE_STORE
            .write()
            .await
            .insert(state_str.clone(), oauth_state);

        // Clean up old states (older than 10 minutes)
        Self::cleanup_old_states_global().await;

        info!("OAuth2 auth started with state: {}", &state_str[..8.min(state_str.len())]);
        Ok((auth_url.to_string(), state_str))
    }

    /// Handle OAuth2 callback with authorization code
    /// Validates state, exchanges code for tokens, stores tokens securely
    pub async fn handle_callback(
        &self,
        code: String,
        state: String,
    ) -> Result<AuthData> {
        // Retrieve and validate state
        let oauth_state = {
            let mut store = OAUTH_STATE_STORE.write().await;
            store.remove(&state).ok_or_else(|| anyhow!("Invalid or expired OAuth state"))?
        };

        // Verify state timestamp (max 10 minutes)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now.saturating_sub(oauth_state.created_at) > 600 {
            return Err(anyhow!("OAuth state expired"));
        }

        // Reconstruct PKCE verifier
        let pkce_verifier = PkceCodeVerifier::new(oauth_state.pkce_verifier);

        // Exchange authorization code for tokens
        let token_result = self.basic_client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http_client)
            .await
            .context("Failed to exchange authorization code")?;

        // Build AuthData
        let expires_in = token_result
            .expires_in()
            .map(|d| d.as_secs())
            .unwrap_or(3600);
        let expires_at = now + expires_in;

        let auth_data = AuthData {
            auth_type: crate::models::source::AuthType::OAuth2,
            token: Some(token_result.access_token().secret().to_string()),
            refresh_token: token_result.refresh_token().map(|t| t.secret().to_string()),
            cookies: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64)),
        };

        // Store tokens securely (keyring + database)
        let stored_token = StoredToken {
            access_token: token_result.access_token().secret().to_string(),
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().to_string()),
            expires_at,
            scope: token_result.scopes().map(|s| s.iter().map(|sc| sc.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default(),
        };

        self.store_token(&stored_token).await?;

        info!("OAuth2 callback successful, tokens stored");
        Ok(auth_data)
    }

    /// Store token in keyring (encrypted) and database
    async fn store_token(&self, token: &StoredToken) -> Result<()> {
        // Store in keyring (platform secure storage)
        let entry = keyring::Entry::new("tunisia-career-scout", "gmail-oauth")?;
        let json = serde_json::to_string(token)?;
        entry.set_password(&json)?;

        // Also store in database if pool available (for sync across devices later)
        if let Some(pool) = &self.db_pool {
            sqlx::query(
                r#"
                INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind("gmail_token")
            .bind(serde_json::to_value(token)?)
            .bind(chrono::Utc::now())
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Load token from keyring
    pub async fn load_token(&self) -> Result<Option<StoredToken>> {
        let entry = keyring::Entry::new("tunisia-career-scout", "gmail-oauth")?;
        match entry.get_password() {
            Ok(json) => {
                let token: StoredToken = serde_json::from_str(&json)?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete stored token (disconnect)
    pub async fn delete_token(&self) -> Result<()> {
        let entry = keyring::Entry::new("tunisia-career-scout", "gmail-oauth")?;
        let _ = entry.delete_credential(); // Ignore if not found

        if let Some(pool) = &self.db_pool {
            sqlx::query("DELETE FROM settings WHERE key = ?")
                .bind("gmail_token")
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    /// Get valid access token, refreshing if necessary
    pub async fn get_valid_token(&self) -> Result<String> {
        let mut token = self.load_token().await?.ok_or_else(|| anyhow!("No Gmail token stored"))?;

        if token.is_expired(300) { // 5-minute buffer
            info!("Access token expired, refreshing...");
            token = self.refresh_token(token).await?;
        }

        Ok(token.access_token)
    }

    /// Refresh access token using refresh token
    async fn refresh_token(&self, mut token: StoredToken) -> Result<StoredToken> {
        let refresh_token = token
            .refresh_token
            .as_ref()
            .ok_or_else(|| anyhow!("No refresh token available"))?
            .clone();

        let token_result = self.basic_client
            .exchange_refresh_token(&RefreshToken::new(refresh_token))
            .request_async(&self.http_client)
            .await
            .context("Failed to refresh access token")?;

        let expires_in = token_result
            .expires_in()
            .map(|d| d.as_secs())
            .unwrap_or(3600);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        token.access_token = token_result.access_token().secret().to_string();
        token.expires_at = now + expires_in;

        // Update refresh token if new one provided
        if let Some(new_refresh) = token_result.refresh_token() {
            token.refresh_token = Some(new_refresh.secret().to_string());
        }

        self.store_token(&token).await?;
        info!("Token refreshed successfully");
        Ok(token)
    }

    /// Check authentication status
    pub async fn status(&self) -> Result<AuthStatus> {
        match self.load_token().await? {
            Some(token) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let is_expired = token.is_expired(0);
                let expires_in = token.expires_at.saturating_sub(now);

                Ok(AuthStatus {
                    authenticated: true,
                    token_expired: is_expired,
                    expires_in_seconds: if is_expired { 0 } else { expires_in },
                    has_refresh_token: token.refresh_token.is_some(),
                })
            }
            None => Ok(AuthStatus {
                authenticated: false,
                token_expired: false,
                expires_in_seconds: 0,
                has_refresh_token: false,
            }),
        }
    }

    /// Start local callback server (desktop only)
    /// Returns the port the server is listening on
    pub async fn start_callback_server(&self) -> Result<u16> {
        // Bind to random port on localhost
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .context("Failed to bind callback server")?;
        let port = listener.local_addr()?.port();

        let state_store = OAUTH_STATE_STORE.clone();
        let basic_client = self.basic_client.clone();
        let http_client = self.http_client.clone();
        let config = self.config.clone();
        let db_pool = self.db_pool.clone();

        // Spawn server task
        tokio::spawn(async move {
            info!("OAuth callback server listening on port {}", port);
            if let Err(e) = run_callback_server(
                listener,
                state_store,
                basic_client,
                http_client,
                config,
                db_pool,
            )
            .await
            {
                warn!("Callback server error: {}", e);
            }
        });

        Ok(port)
    }

    /// Clean up old OAuth states (older than 10 minutes) - static method
    async fn cleanup_old_states_global() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut store = OAUTH_STATE_STORE.write().await;
        store.retain(|_, v| now.saturating_sub(v.created_at) <= 600);
    }
}

/// Authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub token_expired: bool,
    pub expires_in_seconds: u64,
    pub has_refresh_token: bool,
}

/// Run the local HTTP callback server (desktop)
async fn run_callback_server(
    listener: TcpListener,
    state_store: Arc<RwLock<HashMap<String, OAuthState>>>,
    basic_client: BasicClient<
        EndpointSet,     // HasAuthUrl
        EndpointNotSet,  // HasDeviceAuthUrl
        EndpointNotSet,  // HasIntrospectionUrl
        EndpointNotSet,  // HasRevocationUrl
        EndpointSet,     // HasTokenUrl
    >,
    http_client: Client,
    config: OAuthConfig,
    db_pool: Option<DbPool>,
) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let state_store = state_store.clone();
        let basic_client = basic_client.clone();
        let http_client = http_client.clone();
        let _config = config.clone();
        let db_pool = db_pool.clone();

        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut stream = stream;
            let mut buffer = [0; 4096];
            let n = match stream.read(&mut buffer).await {
                Ok(n) => n,
                Err(_) => return,
            };
            let request = String::from_utf8_lossy(&buffer[..n]);

            // Parse request to extract code and state
            let (code, state) = match parse_callback_request(&request) {
                Ok(params) => params,
                Err(_) => {
                    send_response(&mut stream, 400, "Bad Request", "Invalid callback request").await;
                    return;
                }
            };

            // Validate state
            let oauth_state = {
                let mut store = state_store.write().await;
                store.remove(&state)
            };

            let oauth_state = match oauth_state {
                Some(s) => s,
                None => {
                    send_response(&mut stream, 400, "Bad Request", "Invalid or expired state").await;
                    return;
                }
            };

            // Verify state timestamp
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if now.saturating_sub(oauth_state.created_at) > 600 {
                send_response(&mut stream, 400, "Bad Request", "State expired").await;
                return;
            }

            // Exchange code for tokens
            let pkce_verifier = PkceCodeVerifier::new(oauth_state.pkce_verifier);
            let token_result = match basic_client
                .exchange_code(AuthorizationCode::new(code))
                .set_pkce_verifier(pkce_verifier)
                .request_async(&http_client)
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    send_response(&mut stream, 500, "Server Error", &format!("Token exchange failed: {}", e)).await;
                    return;
                }
            };

            let expires_in = token_result
                .expires_in()
                .map(|d| d.as_secs())
                .unwrap_or(3600);
            let expires_at = now + expires_in;

            let stored_token = StoredToken {
                access_token: token_result.access_token().secret().to_string(),
                refresh_token: token_result
                    .refresh_token()
                    .map(|t| t.secret().to_string()),
                expires_at,
                scope: token_result.scopes().map(|s| s.iter().map(|sc| sc.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default(),
            };

            // Store token
            if let Err(e) = store_token_internal(&stored_token, &db_pool).await {
                send_response(&mut stream, 500, "Server Error", &format!("Token storage failed: {}", e)).await;
                return;
            }

            // Success response - close window
            let html = r#"
<!DOCTYPE html>
<html>
<head><title>Authentication Successful</title></head>
<body style="font-family: system-ui; padding: 2rem; text-align: center;">
    <h1 style="color: #22c55e;">✓ Authentication Successful</h1>
    <p>You can now close this window and return to the app.</p>
    <script>window.close();</script>
</body>
</html>
"#;
            send_response(&mut stream, 200, "OK", html).await;
        });
    }
}

/// Parse callback request for code and state parameters
fn parse_callback_request(request: &str) -> Result<(String, String)> {
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return Err(anyhow!("Empty request"));
    }

    // Parse GET /callback?code=...&state=...
    let first_line = lines[0];
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid request line"));
    }

    let path = parts[1];
    let url = Url::parse(&format!("http://localhost{}", path))?;
    let mut code = None;
    let mut state = None;

    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.to_string()),
            "state" => state = Some(v.to_string()),
            _ => {}
        }
    }

    Ok((
        code.ok_or_else(|| anyhow!("Missing code parameter"))?,
        state.ok_or_else(|| anyhow!("Missing state parameter"))?,
    ))
}

/// Send HTTP response
async fn send_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    status_text: &str,
    body: &str,
) {
    use tokio::io::AsyncWriteExt;
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

/// Store token in keyring and database
async fn store_token_internal(token: &StoredToken, db_pool: &Option<DbPool>) -> Result<()> {
    let entry = keyring::Entry::new("tunisia-career-scout", "gmail-oauth")?;
    let json = serde_json::to_string(token)?;
    entry.set_password(&json)?;

    if let Some(pool) = db_pool {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind("gmail_token")
        .bind(serde_json::to_value(token)?)
        .bind(chrono::Utc::now())
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Android custom scheme handler
/// Call this when app receives intent with custom scheme (e.g., com.app:/oauth2redirect?code=...&state=...)
pub async fn handle_android_callback(
    config: &OAuthConfig,
    state_store: Arc<RwLock<HashMap<String, OAuthState>>>,
    db_pool: Option<DbPool>,
    url: &str,
) -> Result<AuthData> {
    let parsed = Url::parse(url)?;
    let mut code = None;
    let mut state = None;

    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.to_string()),
            "state" => state = Some(v.to_string()),
            _ => {}
        }
    }

    let code = code.ok_or_else(|| anyhow!("Missing code in Android callback"))?;
    let state = state.ok_or_else(|| anyhow!("Missing state in Android callback"))?;

    // Retrieve and validate state
    let oauth_state = {
        let mut store = state_store.write().await;
        store.remove(&state).ok_or_else(|| anyhow!("Invalid or expired OAuth state"))?
    };

    // Verify state timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now.saturating_sub(oauth_state.created_at) > 600 {
        return Err(anyhow!("OAuth state expired"));
    }

    // Exchange code
    let pkce_verifier = PkceCodeVerifier::new(oauth_state.pkce_verifier);
    let basic_client = build_basic_client(config)?;
    let http_client = Client::new();
    let token_result = basic_client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await?;

    let expires_in = token_result
        .expires_in()
        .map(|d| d.as_secs())
        .unwrap_or(3600);
    let expires_at = now + expires_in;

    let auth_data = AuthData {
        auth_type: crate::models::source::AuthType::OAuth2,
        token: Some(token_result.access_token().secret().to_string()),
        refresh_token: token_result.refresh_token().map(|t| t.secret().to_string()),
        cookies: None,
        expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64)),
    };

    let stored_token = StoredToken {
        access_token: token_result.access_token().secret().to_string(),
        refresh_token: token_result
            .refresh_token()
            .map(|t| t.secret().to_string()),
        expires_at,
        scope: token_result.scopes().map(|s| s.iter().map(|sc| sc.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default(),
    };

    store_token_internal(&stored_token, &db_pool).await?;

    Ok(auth_data)
}

/// Build basic client from config
fn build_basic_client(config: &OAuthConfig) -> Result<BasicClient<
        EndpointSet,     // HasAuthUrl
        EndpointNotSet,  // HasDeviceAuthUrl
        EndpointNotSet,  // HasIntrospectionUrl
        EndpointNotSet,  // HasRevocationUrl
        EndpointSet,     // HasTokenUrl
    >> {
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?;

    let client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_client_secret(ClientSecret::new(config.client_secret.clone()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

    Ok(client)
}

// ========================================================================
// Tauri Commands
// ========================================================================

use tauri::{State, command};

/// Start Gmail OAuth2 flow
/// Returns authorization URL and state token
#[command]
pub async fn gmail_auth_start(
    pool: State<'_, DbPool>,
    settings: State<'_, SettingsState>,
) -> Result<GmailAuthStartResult, String> {
    let settings_guard = settings.inner().read().await;
    let client_id = settings_guard
        .gmail_client_id
        .as_ref()
        .ok_or("Gmail client ID not configured")?
        .clone();
    let client_secret = settings_guard
        .gmail_client_secret
        .as_ref()
        .ok_or("Gmail client secret not configured")?
        .clone();
    let redirect_uri = settings_guard
        .gmail_redirect_uri
        .clone()
        .unwrap_or_else(|| "http://localhost:0/callback".to_string());
    let android_scheme = settings_guard.gmail_android_scheme.clone();

    // Determine if we're on mobile/Android
    #[cfg(target_os = "android")]
    let is_mobile = true;
    #[cfg(not(target_os = "android"))]
    let is_mobile = false;

    // For desktop, start callback server on random port
    let (redirect_uri, port) = if is_mobile {
        (redirect_uri, None)
    } else {
        // Use port 0 to get random port, will be updated after server starts
        let temp_config = OAuthConfig {
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
            redirect_uri: "http://localhost:0/callback".to_string(),
            android_scheme: android_scheme.clone(),
        };
        let oauth = GmailOAuth::new(temp_config).map_err(|e| e.to_string())?.with_db_pool(pool.inner().clone());
        let port = oauth.start_callback_server().await.map_err(|e| e.to_string())?;
        (format!("http://localhost:{}/callback", port), Some(port))
    };

    let config = OAuthConfig {
        client_id,
        client_secret,
        redirect_uri: redirect_uri.clone(),
        android_scheme,
    };

    let oauth = GmailOAuth::new(config).map_err(|e| e.to_string())?.with_db_pool(pool.inner().clone());
    let (auth_url, state) = oauth.start_auth().await.map_err(|e| e.to_string())?;

    Ok(GmailAuthStartResult {
        auth_url,
        state,
        redirect_uri,
        port,
    })
}

#[derive(Debug, Serialize)]
pub struct GmailAuthStartResult {
    pub auth_url: String,
    pub state: String,
    pub redirect_uri: String,
    pub port: Option<u16>,
}

/// Handle OAuth2 callback (desktop - called by local server)
#[command]
pub async fn gmail_auth_callback(
    pool: State<'_, DbPool>,
    settings: State<'_, SettingsState>,
    code: String,
    state: String,
) -> Result<AuthData, String> {
    let settings_guard = settings.inner().read().await;
    let client_id = settings_guard
        .gmail_client_id
        .as_ref()
        .ok_or("Gmail client ID not configured")?
        .clone();
    let client_secret = settings_guard
        .gmail_client_secret
        .as_ref()
        .ok_or("Gmail client secret not configured")?
        .clone();
    let redirect_uri = settings_guard
        .gmail_redirect_uri
        .clone()
        .unwrap_or_else(|| "http://localhost/callback".to_string());
    let android_scheme = settings_guard.gmail_android_scheme.clone();

    let config = OAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
        android_scheme,
    };

    let oauth = GmailOAuth::new(config).map_err(|e| e.to_string())?.with_db_pool(pool.inner().clone());
    oauth.handle_callback(code, state).await.map_err(|e| e.to_string())
}

/// Handle Android custom scheme callback
#[command]
pub async fn gmail_auth_android_callback(
    pool: State<'_, DbPool>,
    settings: State<'_, SettingsState>,
    callback_url: String,
) -> Result<AuthData, String> {
    let settings_guard = settings.inner().read().await;
    let client_id = settings_guard
        .gmail_client_id
        .as_ref()
        .ok_or("Gmail client ID not configured")?
        .clone();
    let client_secret = settings_guard
        .gmail_client_secret
        .as_ref()
        .ok_or("Gmail client secret not configured")?
        .clone();
    let redirect_uri = settings_guard
        .gmail_redirect_uri
        .clone()
        .unwrap_or_else(|| "http://localhost/callback".to_string());
    let android_scheme = settings_guard.gmail_android_scheme.clone();

    let config = OAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
        android_scheme,
    };

    handle_android_callback(&config, OAUTH_STATE_STORE.clone(), Some(pool.inner().clone()), &callback_url)
        .await
        .map_err(|e| e.to_string())
}

/// Disconnect Gmail (revoke and delete tokens)
#[command]
pub async fn gmail_disconnect(
    pool: State<'_, DbPool>,
    settings: State<'_, SettingsState>,
) -> Result<(), String> {
    let settings_guard = settings.inner().read().await;
    let client_id = settings_guard
        .gmail_client_id
        .as_ref()
        .ok_or("Gmail client ID not configured")?
        .clone();
    let client_secret = settings_guard
        .gmail_client_secret
        .as_ref()
        .ok_or("Gmail client secret not configured")?
        .clone();
    let redirect_uri = settings_guard
        .gmail_redirect_uri
        .clone()
        .unwrap_or_else(|| "http://localhost/callback".to_string());
    let android_scheme = settings_guard.gmail_android_scheme.clone();

    let config = OAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
        android_scheme,
    };

    let oauth = GmailOAuth::new(config).map_err(|e| e.to_string())?.with_db_pool(pool.inner().clone());
    
    // Try to revoke token at Google
    if let Ok(Some(token)) = oauth.load_token().await {
        let revoke_url = format!(
            "https://oauth2.googleapis.com/revoke?token={}",
            token.access_token
        );
        let _ = oauth.http_client.post(&revoke_url).send().await;
    }

    oauth.delete_token().await.map_err(|e| e.to_string())
}

/// Get Gmail authentication status
#[command]
pub async fn gmail_status(
    pool: State<'_, DbPool>,
    settings: State<'_, SettingsState>,
) -> Result<AuthStatus, String> {
    let settings_guard = settings.inner().read().await;
    let client_id = settings_guard
        .gmail_client_id
        .as_ref()
        .ok_or("Gmail client ID not configured")?
        .clone();
    let client_secret = settings_guard
        .gmail_client_secret
        .as_ref()
        .ok_or("Gmail client secret not configured")?
        .clone();
    let redirect_uri = settings_guard
        .gmail_redirect_uri
        .clone()
        .unwrap_or_else(|| "http://localhost/callback".to_string());
    let android_scheme = settings_guard.gmail_android_scheme.clone();

    let config = OAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
        android_scheme,
    };

    let oauth = GmailOAuth::new(config).map_err(|e| e.to_string())?.with_db_pool(pool.inner().clone());
    oauth.status().await.map_err(|e| e.to_string())
}

// ========================================================================
// Unit Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn test_pkce_generation() {
        let pkce = PkcePair::new();
        // Verifier should be 43-128 chars per RFC 7636
        assert!(pkce.verifier.len() >= 43);
        assert!(pkce.verifier.len() <= 128);
        // Challenge should be base64url encoded
        assert!(!pkce.challenge.as_str().is_empty());
    }

    #[test]
    fn test_stored_token_expiry() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Token not expired
        let token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now + 3600,
            scope: "test".to_string(),
        };
        assert!(!token.is_expired(300));

        // Token expired
        let token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now - 100,
            scope: "test".to_string(),
        };
        assert!(token.is_expired(0));
        assert!(token.is_expired(300));
    }

    #[test]
    fn test_oauth_state_serialization() {
        let state = OAuthState {
            state: "test-state".to_string(),
            pkce_verifier: "test-verifier".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            redirect_uri: "http://localhost:1234/callback".to_string(),
            android_scheme: Some("com.app:/oauth2redirect".to_string()),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: OAuthState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.state, deserialized.state);
        assert_eq!(state.pkce_verifier, deserialized.pkce_verifier);
        assert_eq!(state.redirect_uri, deserialized.redirect_uri);
        assert_eq!(state.android_scheme, deserialized.android_scheme);
    }

    #[test]
    fn test_auth_status_serialization() {
        let status = AuthStatus {
            authenticated: true,
            token_expired: false,
            expires_in_seconds: 3600,
            has_refresh_token: true,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: AuthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status.authenticated, deserialized.authenticated);
        assert_eq!(status.token_expired, deserialized.token_expired);
        assert_eq!(status.expires_in_seconds, deserialized.expires_in_seconds);
        assert_eq!(status.has_refresh_token, deserialized.has_refresh_token);
    }

    #[test]
    fn test_parse_callback_request() {
        let request = "GET /callback?code=4%2F0AfJohXn...&state=abc123 HTTP/1.1\r\nHost: localhost:1234\r\n\r\n";
        let (code, state) = parse_callback_request(request).unwrap();
        assert_eq!(code, "4/0AfJohXn...");
        assert_eq!(state, "abc123");
    }

    #[test]
    fn test_parse_callback_request_missing_params() {
        let request = "GET /callback HTTP/1.1\r\nHost: localhost:1234\r\n\r\n";
        assert!(parse_callback_request(request).is_err());

        let request = "GET /callback?code=test HTTP/1.1\r\nHost: localhost:1234\r\n\r\n";
        assert!(parse_callback_request(request).is_err());
    }
}