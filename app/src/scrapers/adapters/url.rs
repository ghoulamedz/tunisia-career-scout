//! URL Adapter
//!
//! Fetches and parses HTML pages from URLs with configurable selectors,
//! rate limiting, user-agent rotation, and robots.txt compliance.

use anyhow::{Result, Context};
use async_trait::async_trait;
use reqwest::{Client, header::{HeaderValue, USER_AGENT}};
use scraper::{Html, Selector};
use robotstxt::DefaultMatcher;
use url::Url;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, warn, info};
use crate::scrapers::pipeline::{RawContent, SourceAdapter, ContentType};
use crate::models::source::{SourceConfig, PaginationConfig as ModelPaginationConfig, PaginationType as ModelPaginationType};

/// Configuration for a URL source, extending SourceConfig::Url with detailed selectors
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UrlSourceConfig {
    pub url: String,
    /// CSS selector for listing items on index page
    pub selector: Option<String>,
    /// CSS selector for detail page content (optional)
    pub detail_selector: Option<String>,
    /// Pagination configuration
    pub pagination: Option<PaginationConfig>,
    /// Custom HTTP headers
    pub headers: Option<serde_json::Value>,
    /// Per-field CSS selectors for detail page extraction
    pub field_selectors: Option<FieldSelectors>,
    /// Rate limit: max requests per second per domain
    pub rate_limit_rps: Option<f64>,
    /// Whether to respect robots.txt (default: true)
    pub respect_robots_txt: Option<bool>,
    /// Custom user agents for rotation (default: built-in list)
    pub user_agents: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PaginationConfig {
    pub type_: PaginationType,
    pub selector: Option<String>,      // Next page button/link selector
    pub max_pages: Option<u32>,
    pub url_param: Option<String>,     // Parameter name for page number (default: "page")
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaginationType {
    NextButton,
    PageNumbers,
    UrlParam,
    InfiniteScroll, // Not supported for static HTML
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FieldSelectors {
    pub title: Option<String>,
    pub company: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub salary: Option<String>,
    pub requirements: Option<String>,
    pub benefits: Option<String>,
    pub application_url: Option<String>,
    pub posted_date: Option<String>,
    pub job_type: Option<String>,
    pub remote_policy: Option<String>,
    pub experience_required: Option<String>,
    pub education_required: Option<String>,
}

/// Convert model pagination config to adapter pagination config
fn convert_pagination_config(model: Option<&ModelPaginationConfig>) -> Option<PaginationConfig> {
    model.map(|p| PaginationConfig {
        type_: match p.type_ {
            ModelPaginationType::NextButton => PaginationType::NextButton,
            ModelPaginationType::PageNumbers => PaginationType::PageNumbers,
            ModelPaginationType::UrlParam => PaginationType::UrlParam,
            ModelPaginationType::InfiniteScroll => PaginationType::InfiniteScroll,
        },
        selector: p.selector.clone(),
        max_pages: p.max_pages,
        url_param: None, // Not in model config
    })
}

/// Rate limiter per domain using token bucket
struct DomainRateLimiter {
    permits: Mutex<HashMap<String, TokenBucket>>,
    default_rps: f64,
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64, // tokens per second
}

impl TokenBucket {
    fn new(rps: f64) -> Self {
        Self {
            tokens: rps,
            last_refill: Instant::now(),
            capacity: rps,
            refill_rate: rps,
        }
    }

    fn try_take(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    async fn take(&mut self) {
        loop {
            self.refill();
            if self.tokens >= 1.0 {
                self.tokens -= 1.0;
                return;
            }
            // Wait until next token is available
            let wait_time = Duration::from_secs_f64((1.0 - self.tokens) / self.refill_rate);
            tokio::time::sleep(wait_time).await;
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}

impl DomainRateLimiter {
    fn new(default_rps: f64) -> Self {
        Self {
            permits: Mutex::new(HashMap::new()),
            default_rps,
        }
    }

    async fn acquire(&self, domain: &str, rps: Option<f64>) {
        let rate = rps.unwrap_or(self.default_rps);
        let mut permits = self.permits.lock().await;
        let bucket = permits.entry(domain.to_string()).or_insert_with(|| TokenBucket::new(rate));
        bucket.take().await;
    }
}

/// Robots.txt cache per domain
struct RobotsTxtCache {
    cache: Mutex<HashMap<String, CachedRobotsTxt>>,
    client: Client,
}

struct CachedRobotsTxt {
    robots_txt: String,
    fetched_at: Instant,
    ttl: Duration,
}

impl RobotsTxtCache {
    fn new(client: Client) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            client,
        }
    }

    async fn can_fetch(&self, url: &Url, user_agent: &str) -> bool {
        let domain = match url.host_str() {
            Some(h) => h.to_string(),
            None => return true, // No domain, allow
        };

        // Check cache
        let robots_txt = {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&domain) {
                if cached.fetched_at.elapsed() < cached.ttl {
                    Some(cached.robots_txt.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Fetch robots.txt if not cached or expired
        let robots_txt = if let Some(txt) = robots_txt {
            txt
        } else {
            let robots_url = format!("{}://{}/robots.txt", url.scheme(), domain);
            match self.client.get(&robots_url).send().await {
                Ok(response) if response.status().is_success() => {
                    match response.text().await {
                        Ok(body) => {
                            let cached = CachedRobotsTxt {
                                robots_txt: body.clone(),
                                fetched_at: Instant::now(),
                                ttl: Duration::from_secs(3600), // Cache for 1 hour
                            };
                            self.cache.lock().await.insert(domain.clone(), cached);
                            body
                        }
                        Err(_) => {
                            // Failed to read body - allow
                            let cached = CachedRobotsTxt {
                                robots_txt: String::new(),
                                fetched_at: Instant::now(),
                                ttl: Duration::from_secs(3600),
                            };
                            self.cache.lock().await.insert(domain.clone(), cached);
                            String::new()
                        }
                    }
                }
                Ok(_) => {
                    // No robots.txt or error - allow all
                    let cached = CachedRobotsTxt {
                        robots_txt: String::new(),
                        fetched_at: Instant::now(),
                        ttl: Duration::from_secs(3600),
                    };
                    self.cache.lock().await.insert(domain, cached);
                    String::new()
                }
                Err(_) => {
                    // Network error - allow but log
                    warn!("Failed to fetch robots.txt for {}", domain);
                    let cached = CachedRobotsTxt {
                        robots_txt: String::new(),
                        fetched_at: Instant::now(),
                        ttl: Duration::from_secs(3600),
                    };
                    self.cache.lock().await.insert(domain, cached);
                    String::new()
                }
            }
        };

        // Check if allowed using DefaultMatcher
        if robots_txt.is_empty() {
            return true;
        }

        let mut matcher = DefaultMatcher::default();
        // URL must be %-encoded according to RFC3986
        let path = robotstxt::get_path_params_query(url.as_str());
        matcher.one_agent_allowed_by_robots(&robots_txt, user_agent, &path)
    }
}

/// Built-in user agent pool for rotation
const DEFAULT_USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
];

pub struct UrlAdapter {
    client: Client,
    rate_limiter: Arc<DomainRateLimiter>,
    robots_cache: Arc<RobotsTxtCache>,
    user_agents: Vec<String>,
    semaphore: Arc<Semaphore>, // Global concurrency limit
}

impl UrlAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");

        let rate_limiter = Arc::new(DomainRateLimiter::new(1.0)); // Default 1 req/s per domain
        let robots_cache = Arc::new(RobotsTxtCache::new(client.clone()));
        let semaphore = Arc::new(Semaphore::new(3)); // Max 3 concurrent requests globally

        Self {
            client,
            rate_limiter,
            robots_cache,
            user_agents: DEFAULT_USER_AGENTS.iter().map(|s| s.to_string()).collect(),
            semaphore,
        }
    }

    pub fn with_config(
        client: Client,
        default_rps: f64,
        max_concurrent: usize,
        user_agents: Vec<String>,
    ) -> Self {
        let rate_limiter = Arc::new(DomainRateLimiter::new(default_rps));
        let robots_cache = Arc::new(RobotsTxtCache::new(client.clone()));
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        Self {
            client,
            rate_limiter,
            robots_cache,
            user_agents: if user_agents.is_empty() {
                DEFAULT_USER_AGENTS.iter().map(|s| s.to_string()).collect()
            } else {
                user_agents
            },
            semaphore,
        }
    }

    /// Get a random user agent from the pool
    fn get_user_agent(&self) -> &str {
        use rand::seq::IndexedRandom;
        let mut rng = rand::rng();
        self.user_agents.choose(&mut rng).map(|s| s.as_str()).unwrap_or(DEFAULT_USER_AGENTS[0])
    }

    /// Fetch HTML with rate limiting, robots.txt check, and user-agent rotation
    async fn fetch_html(&self, url: &str, headers: Option<&serde_json::Value>) -> Result<String> {
        let parsed_url = Url::parse(url).context("Invalid URL")?;
        let domain = parsed_url.host_str().unwrap_or("unknown");
        let user_agent = self.get_user_agent();

        // Check robots.txt
        if !self.robots_cache.can_fetch(&parsed_url, user_agent).await {
            anyhow::bail!("Disallowed by robots.txt: {}", url);
        }

        // Acquire rate limit permit
        self.rate_limiter.acquire(domain, None).await;

        // Acquire global semaphore
        let _permit = self.semaphore.acquire().await.context("Semaphore closed")?;

        // Build request
        let mut request = self.client.get(url)
            .header(USER_AGENT, HeaderValue::from_str(user_agent)?);

        if let Some(h) = headers {
            if let Some(obj) = h.as_object() {
                for (k, v) in obj {
                    if let Some(v_str) = v.as_str() {
                        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
                            if let Ok(header_value) = HeaderValue::from_str(v_str) {
                                request = request.header(header_name, header_value);
                            }
                        }
                    }
                }
            }
        }

        debug!("Fetching {} with UA: {}", url, user_agent);
        let response = request.send().await?;
        response.error_for_status_ref()?;
        let text = response.text().await?;
        Ok(text)
    }

    /// Extract listing URLs from index page using CSS selector
    fn extract_listing_urls(&self, html: &str, selector: &str, base_url: &str) -> Vec<String> {
        let document = Html::parse_document(html);
        let sel = match Selector::parse(selector) {
            Ok(s) => s,
            Err(e) => {
                warn!("Invalid selector '{}': {}", selector, e);
                return Vec::new();
            }
        };
        let mut urls = Vec::new();

        for el in document.select(&sel) {
            if let Some(href) = el.value().attr("href") {
                let absolute = if href.starts_with("http") {
                    href.to_string()
                } else {
                    let base = Url::parse(base_url).ok();
                    base.and_then(|b| b.join(href).ok()).map(|u| u.to_string()).unwrap_or_else(|| href.to_string())
                };
                urls.push(absolute);
            }
        }
        urls
    }

    /// Handle pagination to collect all listing URLs
    async fn handle_pagination(&self, html: &str, config: &UrlSourceConfig, base_url: &str) -> Result<Vec<String>> {
        let mut all_urls = Vec::new();
        let selector = match &config.selector {
            Some(s) => s,
            None => return Ok(all_urls),
        };

        // First page
        let urls = self.extract_listing_urls(html, selector, base_url);
        all_urls.extend(urls);

        // Handle pagination if configured
        if let Some(pag) = &config.pagination {
            match pag.type_ {
                PaginationType::NextButton => {
                    if let Some(sel_str) = &pag.selector {
                        // Parse initial next link in a separate scope to drop Html before loop
                        let next_url = {
                            let document = Html::parse_document(html);
                            if let Ok(initial_sel) = Selector::parse(sel_str) {
                                document.select(&initial_sel).next()
                                    .and_then(|el| el.value().attr("href"))
                                    .and_then(|href| Url::parse(base_url).ok().and_then(|b| b.join(href).ok()).map(|u| u.to_string()).or_else(|| Some(href.to_string())))
                            } else {
                                None
                            }
                        };
                        
                        if let Some(mut current_url) = next_url {
                            let max_pages = pag.max_pages.unwrap_or(10);
                            // Clone selector string to avoid holding Selector across await
                            let sel_str = sel_str.clone();
                            for page in 2..=max_pages {
                                match self.fetch_html(&current_url, config.headers.as_ref()).await {
                                    Ok(next_html) => {
                                        let urls = self.extract_listing_urls(&next_html, selector, &current_url);
                                        if urls.is_empty() { break; }
                                        all_urls.extend(urls);

                                        // Find next page link again - parse selector fresh
                                        let doc = Html::parse_document(&next_html);
                                        if let Ok(sel) = Selector::parse(&sel_str) {
                                            if let Some(next_el) = doc.select(&sel).next() {
                                                if let Some(next_href) = next_el.value().attr("href") {
                                                    current_url = Url::parse(&current_url)
                                                        .ok()
                                                        .and_then(|b| b.join(next_href).ok())
                                                        .map(|u| u.to_string())
                                                        .unwrap_or_else(|| next_href.to_string());
                                                    continue;
                                                }
                                            }
                                        }
                                        break;
                                    }
                                    Err(e) => {
                                        warn!("Failed to fetch page {}: {}", page, e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                PaginationType::UrlParam => {
                    let _param = pag.url_param.as_deref().unwrap_or("page");
                    let max_pages = pag.max_pages.unwrap_or(10);
                    for page in 2..=max_pages {
                        let sep = if base_url.contains('?') { "&" } else { "?" };
                        let page_url = format!("{}{}={}", base_url, sep, page);
                        match self.fetch_html(&page_url, config.headers.as_ref()).await {
                            Ok(page_html) => {
                                let urls = self.extract_listing_urls(&page_html, selector, &page_url);
                                if urls.is_empty() { break; }
                                all_urls.extend(urls);
                            }
                            Err(e) => {
                                warn!("Failed to fetch page {}: {}", page, e);
                                break;
                            }
                        }
                    }
                }
                PaginationType::PageNumbers => {
                    // Similar to UrlParam but with explicit page number links
                    let max_pages = pag.max_pages.unwrap_or(10);
                    if let Some(sel_str) = &pag.selector {
                        // Collect page URLs in a separate scope to drop Html before await
                        let page_urls = {
                            let document = Html::parse_document(html);
                            if let Ok(sel) = Selector::parse(sel_str) {
                                let mut urls = Vec::new();
                                for el in document.select(&sel) {
                                    if let Some(href) = el.value().attr("href") {
                                        let page_url = Url::parse(base_url)
                                            .ok()
                                            .and_then(|b| b.join(href).ok())
                                            .map(|u| u.to_string())
                                            .unwrap_or_else(|| href.to_string());
                                        urls.push(page_url);
                                    }
                                    if urls.len() >= max_pages as usize { break; }
                                }
                                urls
                            } else {
                                Vec::new()
                            }
                        };
                        // Now fetch each page
                        for page_url in page_urls {
                            match self.fetch_html(&page_url, config.headers.as_ref()).await {
                                Ok(page_html) => {
                                    let urls = self.extract_listing_urls(&page_html, selector, &page_url);
                                    all_urls.extend(urls);
                                }
                                Err(e) => warn!("Failed to fetch page: {}", e),
                            }
                        }
                    }
                }
                PaginationType::InfiniteScroll => {
                    warn!("InfiniteScroll pagination not supported for static HTML fetching");
                }
            }
        }

        // Deduplicate
        all_urls.sort();
        all_urls.dedup();
        Ok(all_urls)
    }

    /// Parse a detail page using field selectors from config
    fn parse_detail_page(&self, html: &str, _url: &str, field_selectors: &Option<FieldSelectors>) -> serde_json::Value {
        let document = Html::parse_document(html);
        let mut fields = serde_json::Map::new();

        if let Some(selectors) = field_selectors {
            let selector_map = [
                ("title", &selectors.title),
                ("company", &selectors.company),
                ("description", &selectors.description),
                ("location", &selectors.location),
                ("salary", &selectors.salary),
                ("requirements", &selectors.requirements),
                ("benefits", &selectors.benefits),
                ("application_url", &selectors.application_url),
                ("posted_date", &selectors.posted_date),
                ("job_type", &selectors.job_type),
                ("remote_policy", &selectors.remote_policy),
                ("experience_required", &selectors.experience_required),
                ("education_required", &selectors.education_required),
            ];

            for (field_name, selector_opt) in selector_map {
                if let Some(sel_str) = selector_opt {
                    if let Ok(sel) = Selector::parse(sel_str) {
                        if let Some(el) = document.select(&sel).next() {
                            // For application_url, get href attribute
                            let value = if field_name == "application_url" {
                                el.value().attr("href").unwrap_or("").to_string()
                            } else {
                                el.text().collect::<String>().trim().to_string()
                            };
                            if !value.is_empty() {
                                fields.insert(field_name.to_string(), serde_json::json!(value));
                            }
                        }
                    }
                }
            }
        }

        serde_json::Value::Object(fields)
    }

    /// Validate URL source configuration
    pub fn validate_config(config: &serde_json::Value) -> Result<()> {
        let url_config: UrlSourceConfig = serde_json::from_value(config.clone())
            .context("Invalid URL source config")?;

        // Validate URL
        Url::parse(&url_config.url).context("Invalid URL")?;

        // Validate selectors if present
        if let Some(sel) = &url_config.selector {
            Selector::parse(sel).map_err(|e| anyhow::anyhow!("Invalid listing selector: {}", e))?;
        }
        if let Some(sel) = &url_config.detail_selector {
            Selector::parse(sel).map_err(|e| anyhow::anyhow!("Invalid detail selector: {}", e))?;
        }
        if let Some(pag) = &url_config.pagination {
            if let Some(sel) = &pag.selector {
                Selector::parse(sel).map_err(|e| anyhow::anyhow!("Invalid pagination selector: {}", e))?;
            }
        }
        if let Some(field_sel) = &url_config.field_selectors {
            let selector_fields = [
                &field_sel.title, &field_sel.company, &field_sel.description,
                &field_sel.location, &field_sel.salary, &field_sel.requirements,
                &field_sel.benefits, &field_sel.application_url, &field_sel.posted_date,
                &field_sel.job_type, &field_sel.remote_policy, &field_sel.experience_required,
                &field_sel.education_required,
            ];
            for sel_opt in selector_fields {
                if let Some(sel) = sel_opt {
                    Selector::parse(sel).map_err(|e| anyhow::anyhow!("Invalid field selector '{}': {}", sel, e))?;
                }
            }
        }

        // Validate rate limit
        if let Some(rps) = url_config.rate_limit_rps {
            if rps <= 0.0 || rps > 100.0 {
                anyhow::bail!("rate_limit_rps must be between 0 and 100");
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SourceAdapter for UrlAdapter {
    async fn fetch(&self, config: &SourceConfig) -> Result<Vec<RawContent>> {
        let SourceConfig::Url { url, selector, pagination, headers, .. } = config else {
            return Err(anyhow::anyhow!("UrlAdapter only handles Url source type"));
        };

        // Parse extended config from the base config
        // Note: SourceConfig::Url doesn't have all the new fields, so we need to handle backwards compatibility
        let url_config = UrlSourceConfig {
            url: url.clone(),
            selector: selector.clone(),
            detail_selector: None, // Not in base config
            pagination: convert_pagination_config(pagination.as_ref()),
            headers: headers.clone(),
            field_selectors: None, // Not in base config
            rate_limit_rps: None,
            respect_robots_txt: Some(true),
            user_agents: None,
        };

        // Fetch index page
        let html = self.fetch_html(url, headers.as_ref()).await?;

        // If it's a listing page with selector, extract detail URLs
        if selector.is_some() {
            let detail_urls = self.handle_pagination(&html, &url_config, url).await?;
            let mut results = Vec::new();

            for detail_url in detail_urls {
                match self.fetch_html(&detail_url, None).await {
                    Ok(detail_html) => {
                        let content_hash = format!("{:x}", md5::compute(&detail_html));
                        let extracted_fields = self.parse_detail_page(&detail_html, &detail_url, &url_config.field_selectors);
                        let mut metadata = serde_json::json!({"parent_url": url});
                        if let serde_json::Value::Object(mut obj) = metadata {
                            obj.extend(extracted_fields.as_object().cloned().unwrap_or_default());
                            metadata = serde_json::Value::Object(obj);
                        }

                        results.push(RawContent {
                            source_url: detail_url,
                            content: detail_html,
                            content_type: ContentType::Html,
                            fetched_at: Utc::now(),
                            content_hash,
                            metadata,
                        });
                    }
                    Err(e) => {
                        warn!("Failed to fetch detail page {}: {}", detail_url, e);
                    }
                }
            }

            Ok(results)
        } else {
            // Single page - treat as job posting
            let content_hash = format!("{:x}", md5::compute(&html));
            let extracted_fields = self.parse_detail_page(&html, url, &url_config.field_selectors);
            let mut metadata = serde_json::json!({});
            if let serde_json::Value::Object(mut obj) = metadata {
                obj.extend(extracted_fields.as_object().cloned().unwrap_or_default());
                metadata = serde_json::Value::Object(obj);
            }

            Ok(vec![RawContent {
                source_url: url.clone(),
                content: html,
                content_type: ContentType::Html,
                fetched_at: Utc::now(),
                content_hash,
                metadata,
            }])
        }
    }

    fn source_type(&self) -> &'static str {
        "url"
    }
}

impl Default for UrlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::SourceConfig;

    #[test]
    fn test_validate_config_valid() {
        let config = serde_json::json!({
            "url": "https://example.com/jobs",
            "selector": ".job-listing",
            "field_selectors": {
                "title": "h1.title",
                "company": ".company-name",
                "description": ".description"
            },
            "rate_limit_rps": 2.0
        });
        assert!(UrlAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_url() {
        let config = serde_json::json!({
            "url": "not-a-url",
            "selector": ".job"
        });
        assert!(UrlAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_selector() {
        let config = serde_json::json!({
            "url": "https://example.com",
            "selector": "[invalid"
        });
        assert!(UrlAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_rate_limit() {
        let config = serde_json::json!({
            "url": "https://example.com",
            "rate_limit_rps": 200.0
        });
        assert!(UrlAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_extract_listing_urls() {
        let adapter = UrlAdapter::new();
        let html = r#"
            <html>
                <body>
                    <a class="job-link" href="/jobs/1">Job 1</a>
                    <a class="job-link" href="/jobs/2">Job 2</a>
                    <a class="other" href="/other">Other</a>
                </body>
            </html>
        "#;
        let urls = adapter.extract_listing_urls(html, ".job-link", "https://example.com");
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com/jobs/1".to_string()));
        assert!(urls.contains(&"https://example.com/jobs/2".to_string()));
    }

    #[test]
    fn test_extract_listing_urls_absolute() {
        let adapter = UrlAdapter::new();
        let html = r#"
            <html>
                <body>
                    <a class="job-link" href="https://other.com/jobs/1">Job 1</a>
                </body>
            </html>
        "#;
        let urls = adapter.extract_listing_urls(html, ".job-link", "https://example.com");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://other.com/jobs/1");
    }

    #[test]
    fn test_parse_detail_page() {
        let adapter = UrlAdapter::new();
        let html = r#"
            <html>
                <body>
                    <h1 class="title">Software Engineer</h1>
                    <div class="company">Acme Corp</div>
                    <div class="desc">We are hiring</div>
                    <a class="apply-btn" href="/apply/123">Apply</a>
                </body>
            </html>
        "#;
        let field_selectors = Some(FieldSelectors {
            title: Some("h1.title".to_string()),
            company: Some(".company".to_string()),
            description: Some(".desc".to_string()),
            application_url: Some(".apply-btn".to_string()),
            location: None,
            salary: None,
            requirements: None,
            benefits: None,
            posted_date: None,
            job_type: None,
            remote_policy: None,
            experience_required: None,
            education_required: None,
        });
        let fields = adapter.parse_detail_page(html, "https://example.com/job/1", &field_selectors);
        assert_eq!(fields["title"], "Software Engineer");
        assert_eq!(fields["company"], "Acme Corp");
        assert_eq!(fields["description"], "We are hiring");
        assert_eq!(fields["application_url"], "/apply/123");
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(2.0); // 2 tokens per second
        assert!(bucket.try_take()); // First token
        assert!(bucket.try_take()); // Second token
        assert!(!bucket.try_take()); // No more tokens
    }

    #[test]
    fn test_url_source_config_deserialize() {
        let json = serde_json::json!({
            "url": "https://example.com/jobs",
            "selector": ".job",
            "field_selectors": {
                "title": "h1",
                "company": ".company"
            },
            "rate_limit_rps": 1.5
        });
        let config: UrlSourceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.url, "https://example.com/jobs");
        assert_eq!(config.selector, Some(".job".to_string()));
        assert_eq!(config.rate_limit_rps, Some(1.5));
        assert!(config.field_selectors.is_some());
    }

    #[test]
    fn test_convert_pagination_config() {
        let model_config = Some(ModelPaginationConfig {
            type_: ModelPaginationType::NextButton,
            selector: Some(".next".to_string()),
            max_pages: Some(5),
        });
        let adapter_config = convert_pagination_config(model_config.as_ref());
        assert!(adapter_config.is_some());
        let adapter_config = adapter_config.unwrap();
        assert!(matches!(adapter_config.type_, PaginationType::NextButton));
        assert_eq!(adapter_config.selector, Some(".next".to_string()));
        assert_eq!(adapter_config.max_pages, Some(5));
    }
}