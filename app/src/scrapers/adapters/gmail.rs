//! Gmail Adapter
//!
//! Fetches job alert emails via Gmail API with OAuth2.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use google_gmail1::Gmail;
use google_gmail1::api::{Message, MessagePart, MessagePartBody};
use md5;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::source::{AuthData, SourceConfig};
use crate::scrapers::pipeline::{ContentType, RawContent, SourceAdapter};
use crate::gmail::oauth::GmailOAuth;

/// Configuration for Gmail source
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GmailSourceConfig {
    pub label_ids: Vec<String>,
    pub query: Option<String>,
    pub max_results: Option<u32>,
    pub sender_filters: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
}

/// Job email parser configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailParserConfig {
    pub extract_from_html: bool,
    pub extract_from_text: bool,
    pub job_keywords: Vec<String>,
    pub company_patterns: Vec<String>,
    pub location_patterns: Vec<String>,
    pub salary_patterns: Vec<String>,
    pub url_patterns: Vec<String>,
}

/// Gmail API adapter for fetching job alert emails
pub struct GmailAdapter {
    oauth: GmailOAuth,
    parser_config: EmailParserConfig,
}

impl GmailAdapter {
    /// Create a new Gmail adapter with default parser configuration
    pub fn new(oauth: GmailOAuth) -> Self {
        Self {
            oauth,
            parser_config: EmailParserConfig::default(),
        }
    }

    /// Create a new Gmail adapter with custom parser configuration
    pub fn with_parser_config(mut self, config: EmailParserConfig) -> Self {
        self.parser_config = config;
        self
    }

    /// Get a valid Gmail API client
    async fn get_gmail_client(&self) -> Result<Gmail<Client>> {
        let access_token = self.oauth.get_valid_access_token().await?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        Ok(Gmail::new(client, access_token))
    }

    /// Build search query from configuration
    fn build_query(config: &GmailSourceConfig) -> String {
        let mut parts = Vec::new();

        // Add label filters
        for label in &config.label_ids {
            parts.push(format!("label:{}", label));
        }

        // Add sender filters
        if let Some(senders) = &config.sender_filters {
            for sender in senders {
                parts.push(format!("from:{}", sender));
            }
        }

        // Add keyword filters
        if let Some(keywords) = &config.keywords {
            for keyword in keywords {
                parts.push(format!("({})", keyword));
            }
        }

        // Add custom query
        if let Some(query) = &config.query {
            parts.push(query.clone());
        }

        // Default to last 30 days
        parts.push("newer_than:30d".to_string());

        parts.join(" ")
    }

    /// Fetch messages matching the query
    async fn fetch_message_ids(
        &self,
        gmail: &Gmail<Client>,
        config: &GmailSourceConfig,
    ) -> Result<Vec<String>> {
        let query = Self::build_query(config);
        let max_results = config.max_results.unwrap_or(50) as i32;

        let response = gmail
            .users()
            .messages()
            .list("me")
            .q(query)
            .max_results(max_results)
            .doit()
            .await
            .context("Failed to list messages")?;

        let messages = response.1.messages.unwrap_or_default();
        let ids: Vec<String> = messages
            .into_iter()
            .filter_map(|m| m.id)
            .collect();

        Ok(ids)
    }

    /// Fetch full message content
    async fn fetch_full_message(&self, gmail: &Gmail<Client>, msg_id: &str) -> Result<Message> {
        let response = gmail
            .users()
            .messages()
            .get("me", msg_id)
            .format("full")
            .doit()
            .await
            .context("Failed to fetch message")?;

        Ok(response.1)
    }

    /// Convert Gmail message to RawContent
    fn message_to_raw_content(&self, message: &Message) -> RawContent {
        let headers = message
            .payload
            .as_ref()
            .and_then(|p| p.headers.as_ref())
            .map(|h| {
                h.iter()
                    .map(|hv| (hv.name.clone(), hv.value.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let get_header = |name: &str| -> Option<String> {
            headers
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
        };

        let subject = get_header("Subject").unwrap_or_default();
        let from = get_header("From").unwrap_or_default();
        let date = get_header("Date").unwrap_or_default();
        let message_id = get_header("Message-ID").unwrap_or_default();

        // Extract body (prefer HTML, fallback to text)
        let body = self.extract_email_body(message.payload.as_ref());

        let content_hash = format!("{:x}", md5::compute(&body));

        // Parse date
        let fetched_at = DateTime::parse_from_rfc2822(&date)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let metadata = serde_json::json!({
            "subject": subject,
            "from": from,
            "date": date,
            "message_id": message_id,
            "headers": headers.into_iter().collect::<serde_json::Map<_, _>>(),
        });

        RawContent {
            source_url: format!("gmail:{}", message_id),
            content_type: ContentType::Email,
            content: body,
            content_hash,
            fetched_at,
            metadata,
        }
    }

    /// Extract email body from message payload
    fn extract_email_body(&self, payload: Option<&MessagePart>) -> String {
        if let Some(part) = payload {
            // Try HTML first
            if self.parser_config.extract_from_html {
                if let Some(html) = self.extract_part_by_mime_type(part, "text/html") {
                    if !html.is_empty() {
                        return html;
                    }
                }
            }

            // Fallback to plain text
            if self.parser_config.extract_from_text {
                if let Some(text) = self.extract_part_by_mime_type(part, "text/plain") {
                    if !text.is_empty() {
                        return text;
                    }
                }
            }

            // Recursively search multipart
            if let Some(parts) = part.parts.as_ref() {
                for p in parts {
                    let body = self.extract_email_body(Some(p));
                    if !body.is_empty() {
                        return body;
                    }
                }
            }
        }
        String::new()
    }

    /// Extract a specific MIME type part
    fn extract_part_by_mime_type(&self, part: &MessagePart, mime_type: &str) -> Option<String> {
        if part.mime_type.as_deref() == Some(mime_type) {
            if let Some(body) = &part.body {
                if let Some(data) = &body.data {
                    return URL_SAFE_NO_PAD
                        .decode(data)
                        .ok()
                        .and_then(|b| String::from_utf8(b).ok());
                }
            }
        }
        None
    }

    /// Parse job details from email content
    pub fn parse_job_from_email(&self, raw: &RawContent) -> Result<ParsedJobEmail> {
        let subject = raw
            .metadata
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let from = raw
            .metadata
            .get("from")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let body = &raw.content;

        // Extract various fields using patterns
        let title = self.extract_title(&subject, body);
        let company = self.extract_company(&from, &subject, body);
        let location = self.extract_location(body);
        let salary = self.extract_salary(body);
        let description = self.extract_description(body);
        let application_url = self.extract_application_url(body);
        let requirements = self.extract_requirements(body);
        let posted_date = self.extract_posted_date(&raw.metadata, body);

        Ok(ParsedJobEmail {
            title,
            company,
            location,
            salary_range: salary,
            description,
            application_url,
            requirements,
            posted_date,
            raw_subject: subject,
            raw_from: from,
        })
    }

    /// Extract job title from subject or body
    fn extract_title(&self, subject: &str, body: &str) -> String {
        // Try subject first (most job alerts have title in subject)
        let subject_clean = subject
            .replace("Job Alert:", "")
            .replace("New Job:", "")
            .replace("Job Opportunity:", "")
            .trim()
            .to_string();

        if !subject_clean.is_empty() && subject_clean.len() < 200 {
            return subject_clean;
        }

        // Fallback: try to find title in body using patterns
        for keyword in &self.parser_config.job_keywords {
            if let Some(idx) = body.to_lowercase().find(&keyword.to_lowercase()) {
                let start = idx.saturating_sub(50);
                let end = (idx + keyword.len() + 100).min(body.len());
                let snippet = &body[start..end];
                if let Some(line) = snippet.lines().next() {
                    let cleaned = line.trim().trim_matches(|c: char| c.is_ascii_punctuation());
                    if cleaned.len() > 5 && cleaned.len() < 200 {
                        return cleaned.to_string();
                    }
                }
            }
        }

        "Unknown Position".to_string()
    }

    /// Extract company name from sender, subject, or body
    fn extract_company(&self, from: &str, subject: &str, body: &str) -> String {
        // Try to extract from sender email
        if let Some(company) = self.extract_company_from_sender(from) {
            return company;
        }

        // Try subject
        for pattern in &self.parser_config.company_patterns {
            if let Some(captures) = regex::Regex::new(pattern).ok().and_then(|re| re.captures(subject)) {
                if let Some(m) = captures.get(1) {
                    return m.as_str().trim().to_string();
                }
            }
        }

        // Try body
        for pattern in &self.parser_config.company_patterns {
            if let Some(captures) = regex::Regex::new(pattern).ok().and_then(|re| re.captures(body)) {
                if let Some(m) = captures.get(1) {
                    return m.as_str().trim().to_string();
                }
            }
        }

        // Fallback: look for common patterns
        let patterns = [
            r"(?i)(?:at|@)\s+([A-Z][a-zA-Z0-9\s&.\-']{2,50})",
            r"(?i)company[:\s]+([A-Z][a-zA-Z0-9\s&.\-']{2,50})",
            r"(?i)employer[:\s]+([A-Z][a-zA-Z0-9\s&.\-']{2,50})",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        return m.as_str().trim().to_string();
                    }
                }
            }
        }

        "Unknown Company".to_string()
    }

    /// Extract company from sender email address
    fn extract_company_from_sender(&self, from: &str) -> Option<String> {
        // Extract domain from email
        if let Some(at_idx) = from.find('@') {
            let domain_part = &from[at_idx + 1..];
            if let Some(gt_idx) = domain_part.find('>') {
                let domain = &domain_part[..gt_idx];
                let company = domain
                    .split('.')
                    .next()
                    .unwrap_or(domain)
                    .replace(['-', '_'], " ");
                // Capitalize
                return Some(
                    company
                        .split_whitespace()
                        .map(|w| {
                            let mut c = w.chars();
                            match c.next() {
                                None => String::new(),
                                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
        }
        None
    }

    /// Extract location from email body
    fn extract_location(&self, body: &str) -> Option<String> {
        for pattern in &self.parser_config.location_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        return Some(m.as_str().trim().to_string());
                    }
                }
            }
        }

        // Common location patterns
        let patterns = [
            r"(?i)location[:\s]+([A-Za-z\s,.\-']{3,100})",
            r"(?i)based in[:\s]+([A-Za-z\s,.\-']{3,100})",
            r"(?i)office[:\s]+([A-Za-z\s,.\-']{3,100})",
            r"(?i)(remote|hybrid|on-site|onsite)",
            r"(?i)(Tunis|Sfax|Sousse|Kairouan|Bizerte|Gabes|Ariana|Manouba|Ben Arous|Nabeul|Zaghouan|Béja|Jendouba|Le Kef|Siliana|Kasserine|Sidi Bouzid|Gafsa|Tozeur|Kébili|Medenine|Tataouine)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        let loc = m.as_str().trim();
                        if loc.len() >= 3 {
                            return Some(loc.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract salary information
    fn extract_salary(&self, body: &str) -> Option<String> {
        for pattern in &self.parser_config.salary_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        return Some(m.as_str().trim().to_string());
                    }
                }
            }
        }

        // Common salary patterns
        let patterns = [
            r"(?i)(?:salary|compensation|pay)[:\s]+([\$€£]\s?\d{1,3}(?:,\d{3})*(?:\.\d{2})?(?:\s?[-–]\s?[\$€£]\s?\d{1,3}(?:,\d{3})*(?:\.\d{2})?)?)",
            r"(?i)(\d{1,3}(?:,\d{3})*(?:\.\d{2})?\s?[-–]\s?\d{1,3}(?:,\d{3})*(?:\.\d{2})?\s?(?:USD|EUR|TND|per year|per month|/yr|/mo))",
            r"(?i)(?:up to|maximum|max)[:\s]+([\$€£]\s?\d{1,3}(?:,\d{3})*(?:\.\d{2})?)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        return Some(m.as_str().trim().to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract job description
    fn extract_description(&self, body: &str) -> String {
        // Clean HTML if present
        let cleaned = if body.contains('<') && body.contains('>') {
            self.strip_html_tags(body)
        } else {
            body.to_string()
        };

        // Take first reasonable chunk as description
        let lines: Vec<&str> = cleaned.lines().collect();
        let mut description = String::new();
        let mut char_count = 0;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip lines that look like headers/footers
            if trimmed.len() < 10 && (trimmed.contains("http") || trimmed.contains("@")) {
                continue;
            }
            description.push_str(trimmed);
            description.push('\n');
            char_count += trimmed.len();
            if char_count > 2000 {
                break;
            }
        }

        if description.is_empty() {
            // Fallback: return first 2000 chars
            cleaned.chars().take(2000).collect()
        } else {
            description.trim().to_string()
        }
    }

    /// Extract application URL
    fn extract_application_url(&self, body: &str) -> Option<String> {
        for pattern in &self.parser_config.url_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        let url = m.as_str().trim();
                        if url.starts_with("http") && url.len() > 10 {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }

        // Common apply URL patterns
        let patterns = [
            r#"(?i)(?:apply|application)[:\s]+(https?://[^\s<>"]+)"#,
            r#"(?i)(?:link|url)[:\s]+(https?://[^\s<>"]+)"#,
            r#"(?i)href=["']([^"']*(?:apply|job|career|position)[^"']*)["']"#,
            r#"(https?://[^\s<>"]*(?:greenhouse|lever|workable|smartrecruiters|icims|taleo|jobvite|bamboohr|recruitee|comeet|teamtailor|personio)[^\s<>"]*)"#,
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        let url = m.as_str().trim().trim_matches(&['"', '\'', '>', '<'] as &[_]);
                        if url.starts_with("http") && url.len() > 10 {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract requirements
    fn extract_requirements(&self, body: &str) -> Option<String> {
        let patterns = [
            r"(?i)(?:requirements|qualifications|what you['']ll need|skills)[:\s]+\n?([\s\S]{20,1000}?)(?:\n\s*\n|$)",
            r"(?i)(?:must have|required)[:\s]+([\s\S]{20,500}?)(?:\n\s*\n|$)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        let req = self.strip_html_tags(m.as_str()).trim().to_string();
                        if req.len() > 10 {
                            return Some(req);
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract posted date
    fn extract_posted_date(&self, metadata: &Value, body: &str) -> Option<DateTime<Utc>> {
        // Try email Date header first
        if let Some(date_str) = metadata.get("date").and_then(|v| v.as_str()) {
            if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
                return Some(dt.with_timezone(&Utc));
            }
        }

        // Try to find date in body
        let patterns = [
            r"(?i)posted[:\s]+(\d{1,2}[/\-]\d{1,2}[/\-]\d{2,4})",
            r"(?i)date[:\s]+(\d{1,2}[/\-]\d{1,2}[/\-]\d{2,4})",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(body) {
                    if let Some(m) = captures.get(1) {
                        let date_str = m.as_str();
                        // Try multiple formats
                        for fmt in &["%m/%d/%Y", "%d/%m/%Y", "%Y-%m-%d", "%m-%d-%Y", "%d-%m-%Y"] {
                            if let Ok(dt) = chrono::NaiveDate::parse_from_str(date_str, fmt) {
                                return Some(dt.and_hms_opt(12, 0, 0).unwrap().and_utc());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Strip HTML tags from text
    fn strip_html_tags(&self, html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        let mut tag_buffer = String::new();

        for c in html.chars() {
            match c {
                '<' => {
                    in_tag = true;
                    tag_buffer.clear();
                }
                '>' if in_tag => {
                    in_tag = false;
                    // Handle line breaks for block elements
                    let tag_lower = tag_buffer.to_lowercase();
                    if tag_lower.starts_with("br") || tag_lower.starts_with("p") || tag_lower.starts_with("div") || tag_lower.starts_with("li") {
                        result.push('\n');
                    }
                }
                _ if in_tag => {
                    tag_buffer.push(c);
                }
                _ => {
                    result.push(c);
                }
            }
        }

        // Decode common HTML entities
        result
            .replace("&nbsp;", " ")
            .replace("&", "&")
            .replace("<", "<")
            .replace(">", ">")
            .replace("\\\"", "\\\"")
            .replace("\\'", "\\'")
            .replace("&apos;", "'")
    }
}

impl Default for EmailParserConfig {
    fn default() -> Self {
        Self {
            extract_from_html: true,
            extract_from_text: true,
            job_keywords: vec![
                "job".to_string(),
                "position".to_string(),
                "role".to_string(),
                "opportunity".to_string(),
                "career".to_string(),
                "hiring".to_string(),
                "vacancy".to_string(),
                "opening".to_string(),
            ],
            company_patterns: vec![
                r#"(?i)at\s+([A-Z][a-zA-Z0-9\s&.\-']{2,50})"#.to_string(),
                r#"(?i)company[:\s]+([A-Z][a-zA-Z0-9\s&.\-']{2,50})"#.to_string(),
                r#"(?i)employer[:\s]+([A-Z][a-zA-Z0-9\s&.\-']{2,50})"#.to_string(),
            ],
            location_patterns: vec![
                r#"(?i)location[:\s]+([A-Za-z\s,.\-']{3,100})"#.to_string(),
                r#"(?i)based in[:\s]+([A-Za-z\s,.\-']{3,100})"#.to_string(),
            ],
            salary_patterns: vec![
                r#"(?i)salary[:\s]+([\$€£]\s?\d[\d,\.]*\s?[-–]\s?[\$€£]\s?\d[\d,\.]*)"#.to_string(),
            ],
            url_patterns: vec![
                r#"(?i)apply[:\s]+(https?://[^\s<>"]+)"#.to_string(),
            ],
        }
    }
}

/// Parsed job email data
#[derive(Debug, Clone)]
pub struct ParsedJobEmail {
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub salary_range: Option<String>,
    pub description: String,
    pub application_url: Option<String>,
    pub requirements: Option<String>,
    pub posted_date: Option<DateTime<Utc>>,
    pub raw_subject: String,
    pub raw_from: String,
}

#[async_trait]
impl SourceAdapter for GmailAdapter {
    async fn fetch(&self, config: &SourceConfig) -> Result<Vec<RawContent>> {
        let SourceConfig::Gmail {
            label_ids,
            query,
            max_results,
        } = config
        else {
            return Err(anyhow!("Invalid config for Gmail adapter"));
        };

        // Build Gmail source config
        let gmail_config = GmailSourceConfig {
            label_ids: label_ids.clone(),
            query: query.clone(),
            max_results: *max_results,
            sender_filters: None, // Could be added to SourceConfig::Gmail in future
            keywords: None,
        };

        let gmail = self.get_gmail_client().await?;
        let message_ids = self.fetch_message_ids(&gmail, &gmail_config).await?;

        let mut results = Vec::new();

        for msg_id in message_ids {
            match self.fetch_full_message(&gmail, &msg_id).await {
                Ok(message) => {
                    let raw = self.message_to_raw_content(&message);
                    results.push(raw);
                }
                Err(e) => {
                    // Log error but continue with other messages
                    eprintln!("Failed to fetch message {}: {}", msg_id, e);
                }
            }
        }

        Ok(results)
    }

    fn source_type(&self) -> &'static str {
        "gmail"
    }
}

impl GmailAdapter {
    /// Validate Gmail source configuration
    pub fn validate_config(config: &Value) -> Result<()> {
        let obj = config
            .as_object()
            .ok_or_else(|| anyhow!("Config must be an object"))?;

        // Validate label_ids (required)
        let label_ids = obj
            .get("label_ids")
            .ok_or_else(|| anyhow!("Missing required field: label_ids"))?;

        let labels = label_ids
            .as_array()
            .ok_or_else(|| anyhow!("label_ids must be an array"))?;

        if labels.is_empty() {
            return Err(anyhow!("label_ids cannot be empty"));
        }

        for label in labels {
            if !label.is_string() {
                return Err(anyhow!("All label_ids must be strings"));
            }
            let label_str = label.as_str().unwrap();
            if label_str.is_empty() {
                return Err(anyhow!("label_ids cannot contain empty strings"));
            }
        }

        // Validate query (optional)
        if let Some(query) = obj.get("query") {
            if !query.is_string() && !query.is_null() {
                return Err(anyhow!("query must be a string or null"));
            }
        }

        // Validate max_results (optional)
        if let Some(max_results) = obj.get("max_results") {
            if let Some(num) = max_results.as_u64() {
                if num == 0 || num > 500 {
                    return Err(anyhow!("max_results must be between 1 and 500"));
                }
            } else if !max_results.is_null() {
                return Err(anyhow!("max_results must be a positive integer or null"));
            }
        }

        // Validate sender_filters (optional)
        if let Some(senders) = obj.get("sender_filters") {
            if let Some(arr) = senders.as_array() {
                for sender in arr {
                    if !sender.is_string() {
                        return Err(anyhow!("All sender_filters must be strings"));
                    }
                }
            } else if !senders.is_null() {
                return Err(anyhow!("sender_filters must be an array or null"));
            }
        }

        // Validate keywords (optional)
        if let Some(keywords) = obj.get("keywords") {
            if let Some(arr) = keywords.as_array() {
                for kw in arr {
                    if !kw.is_string() {
                        return Err(anyhow!("All keywords must be strings"));
                    }
                }
            } else if !keywords.is_null() {
                return Err(anyhow!("keywords must be an array or null"));
            }
        }

        Ok(())
    }

    /// Parse raw email content into a structured JobPosting
    pub fn parse(&self, raw: &RawContent, source_id: Uuid) -> Result<crate::models::posting::JobPosting> {
        let parsed = self.parse_job_from_email(raw)?;

        let content_hash = raw.content_hash.clone();
        let source_posting_id = raw
            .metadata
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut posting = crate::models::posting::JobPosting::new(
            source_id,
            source_posting_id,
            parsed.title,
            parsed.company,
            parsed.description,
            raw.fetched_at,
            content_hash,
        );

        posting.location = parsed.location;
        posting.salary_range = parsed.salary_range;
        posting.application_url = parsed.application_url;
        posting.experience_required = parsed.requirements;
        posting.posted_date = parsed.posted_date.or(Some(raw.fetched_at));
        posting.raw_html = Some(raw.content.clone());

        // Add any skills found in the description
        let skills = self.extract_skills_from_text(&posting.description);
        for skill in skills {
            posting.add_skill(skill);
        }

        // Try to infer job type from description
        posting.job_type = self.infer_job_type(&posting.description);

        // Try to infer remote policy
        posting.remote_policy = self.infer_remote_policy(&posting.description, &posting.location);

        Ok(posting)
    }

    /// Extract skills from job description text
    fn extract_skills_from_text(&self, text: &str) -> Vec<String> {
        let common_skills = [
            "Rust", "Python", "JavaScript", "TypeScript", "Java", "Go", "C++", "C#", "PHP", "Ruby",
            "React", "Vue", "Angular", "Svelte", "Next.js", "Node.js", "Django", "Flask", "FastAPI",
            "Spring", "ASP.NET", "Laravel", "Rails", "Express", "Kotlin", "Swift", "Dart", "Flutter",
            "PostgreSQL", "MySQL", "MongoDB", "Redis", "Elasticsearch", "SQL", "NoSQL",
            "Docker", "Kubernetes", "AWS", "GCP", "Azure", "Terraform", "Ansible", "CI/CD",
            "Git", "Linux", "REST", "GraphQL", "gRPC", "Microservices", "Serverless",
            "Machine Learning", "AI", "Data Science", "TensorFlow", "PyTorch", "Pandas", "NumPy",
            "DevOps", "SRE", "Backend", "Frontend", "Fullstack", "Mobile", "iOS", "Android",
            "Testing", "TDD", "Agile", "Scrum", "Kanban", "Architecture", "System Design",
        ];

        let text_lower = text.to_lowercase();
        let mut found = Vec::new();

        for skill in &common_skills {
            if text_lower.contains(&skill.to_lowercase()) {
                found.push(skill.to_string());
            }
        }

        found
    }

    /// Infer job type from description
    fn infer_job_type(&self, text: &str) -> Option<String> {
        let text_lower = text.to_lowercase();

        if text_lower.contains("intern") || text_lower.contains("internship") {
            return Some("internship".to_string());
        }
        if text_lower.contains("part.time") || text_lower.contains("part-time") {
            return Some("part-time".to_string());
        }
        if text_lower.contains("contract") || text_lower.contains("freelance") || text_lower.contains("contractor") {
            return Some("contract".to_string());
        }
        if text_lower.contains("full.time") || text_lower.contains("full-time") || text_lower.contains("permanent") {
            return Some("full-time".to_string());
        }

        Some("full-time".to_string()) // Default
    }

    /// Infer remote policy from description and location
    fn infer_remote_policy(&self, text: &str, location: &Option<String>) -> Option<String> {
        let text_lower = text.to_lowercase();

        if text_lower.contains("fully remote") || text_lower.contains("100% remote") || text_lower.contains("remote-first") {
            return Some("remote".to_string());
        }
        if text_lower.contains("hybrid") {
            return Some("hybrid".to_string());
        }
        if text_lower.contains("remote") && !text_lower.contains("hybrid") {
            return Some("remote".to_string());
        }
        if text_lower.contains("on.site") || text_lower.contains("on-site") || text_lower.contains("onsite") || text_lower.contains("office") {
            return Some("onsite".to_string());
        }

        // Check location for remote indicators
        if let Some(loc) = location {
            let loc_lower = loc.to_lowercase();
            if loc_lower.contains("remote") || loc_lower.contains("anywhere") || loc_lower.contains("worldwide") {
                return Some("remote".to_string());
            }
        }

        Some("onsite".to_string()) // Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_validate_config_valid() {
        let config = json!({
            "label_ids": ["INBOX", "Jobs"],
            "query": "from:linkedin.com",
            "max_results": 25
        });
        assert!(GmailAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_minimal() {
        let config = json!({
            "label_ids": ["INBOX"]
        });
        assert!(GmailAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_label_ids() {
        let config = json!({
            "query": "jobs"
        });
        assert!(GmailAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_empty_label_ids() {
        let config = json!({
            "label_ids": [],
            "query": "jobs"
        });
        assert!(GmailAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_label_ids_type() {
        let config = json!({
            "label_ids": "INBOX",
            "query": "jobs"
        });
        assert!(GmailAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_max_results_zero() {
        let config = json!({
            "label_ids": ["INBOX"],
            "max_results": 0
        });
        assert!(GmailAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_max_results_too_high() {
        let config = json!({
            "label_ids": ["INBOX"],
            "max_results": 1000
        });
        assert!(GmailAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_valid_max_results() {
        let config = json!({
            "label_ids": ["INBOX"],
            "max_results": 100
        });
        assert!(GmailAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_sender_filters() {
        let config = json!({
            "label_ids": ["INBOX"],
            "sender_filters": ["jobs@linkedin.com", "alerts@indeed.com"]
        });
        assert!(GmailAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_sender_filters() {
        let config = json!({
            "label_ids": ["INBOX"],
            "sender_filters": "not-an-array"
        });
        assert!(GmailAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_keywords() {
        let config = json!({
            "label_ids": ["INBOX"],
            "keywords": ["Rust", "Backend", "Senior"]
        });
        assert!(GmailAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_build_query() {
        let config = GmailSourceConfig {
            label_ids: vec!["Jobs".to_string(), "Job Alerts".to_string()],
            query: Some("from:linkedin.com".to_string()),
            max_results: Some(50),
            sender_filters: Some(vec!["jobs@linkedin.com".to_string()]),
            keywords: Some(vec!["Rust".to_string(), "Backend".to_string()]),
        };

        let query = GmailAdapter::build_query(&config);
        assert!(query.contains("label:Jobs"));
        assert!(query.contains("label:Job Alerts"));
        assert!(query.contains("from:linkedin.com"));
        assert!(query.contains("from:jobs@linkedin.com"));
        assert!(query.contains("(Rust)"));
        assert!(query.contains("(Backend)"));
        assert!(query.contains("newer_than:30d"));
    }

    #[test]
    fn test_extract_email_body_html() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        // Create a mock message part with HTML content
        let html_content = "<html><body><h1>Software Engineer</h1><p>We are hiring!</p></body></html>";
        let encoded = URL_SAFE_NO_PAD.encode(html_content);

        let mut body = MessagePartBody::default();
        body.data = Some(encoded);

        let mut part = MessagePart::default();
        part.mime_type = Some("text/html".to_string());
        part.body = Some(body);

        let result = adapter.extract_part_by_mime_type(&part, "text/html");
        assert_eq!(result, Some(html_content.to_string()));
    }

    #[test]
    fn test_extract_email_body_text() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text_content = "Software Engineer\nWe are hiring!";
        let encoded = URL_SAFE_NO_PAD.encode(text_content);

        let mut body = MessagePartBody::default();
        body.data = Some(encoded);

        let mut part = MessagePart::default();
        part.mime_type = Some("text/plain".to_string());
        part.body = Some(body);

        let result = adapter.extract_part_by_mime_type(&part, "text/plain");
        assert_eq!(result, Some(text_content.to_string()));
    }

    #[test]
    fn test_strip_html_tags() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let html = "<html><body><h1>Title</h1><p>Paragraph with <strong>bold</strong></p><br><div>Another div</div></body></html>";
        let result = adapter.strip_html_tags(html);

        assert!(result.contains("Title"));
        assert!(result.contains("Paragraph with bold"));
        assert!(result.contains("Another div"));
        assert!(!result.contains("<"));
        assert!(!result.contains(">"));
    }

    #[test]
    fn test_strip_html_entities() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let html = "Company & Co. <tag> \"quoted\" 'apsos'";
        let result = adapter.strip_html_tags(html);

        assert_eq!(result, "Company & Co. <tag> \"quoted\" 'apos'");
    }

    #[test]
    fn test_extract_title_from_subject() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let subject = "Job Alert: Senior Rust Developer at Acme Corp";
        let body = "Some body content";

        let title = adapter.extract_title(subject, body);
        assert_eq!(title, "Senior Rust Developer at Acme Corp");
    }

    #[test]
    fn test_extract_title_fallback_to_body() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let subject = "Job Alert";
        let body = "We are looking for a Senior Rust Developer to join our team.";

        let title = adapter.extract_title(subject, body);
        assert!(title.contains("Rust") || title.contains("Senior"));
    }

    #[test]
    fn test_extract_company_from_sender() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let from = "Jobs Team <jobs@linkedin.com>";
        let result = adapter.extract_company_from_sender(from);
        assert_eq!(result, Some("Linkedin".to_string()));
    }

    #[test]
    fn test_extract_company_from_sender_github() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let from = "GitHub Jobs <noreply@github.com>";
        let result = adapter.extract_company_from_sender(from);
        assert_eq!(result, Some("Github".to_string()));
    }

    #[test]
    fn test_extract_location() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let body = "Location: Tunis, Tunisia\nRemote: Hybrid";
        let location = adapter.extract_location(body);
        assert!(location.is_some());
        assert!(location.unwrap().contains("Tunis"));
    }

    #[test]
    fn test_extract_location_remote() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let body = "This is a fully remote position.";
        let location = adapter.extract_location(body);
        assert_eq!(location, Some("remote".to_string()));
    }

    #[test]
    fn test_extract_salary() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let body = "Salary: $80,000 - $120,000 per year";
        let salary = adapter.extract_salary(body);
        assert!(salary.is_some());
        assert!(salary.unwrap().contains("80,000"));
    }

    #[test]
    fn test_extract_application_url() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let body = "Apply here: https://company.greenhouse.io/jobs/12345";
        let url = adapter.extract_application_url(body);
        assert!(url.is_some());
        assert!(url.unwrap().contains("greenhouse.io"));
    }

    #[test]
    fn test_extract_application_url_linkedin() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let body = "View job: https://www.linkedin.com/jobs/view/123456789";
        let url = adapter.extract_application_url(body);
        assert!(url.is_some());
        assert!(url.unwrap().contains("linkedin.com"));
    }

    #[test]
    fn test_extract_requirements() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let body = "Requirements:\n- 5+ years Rust\n- Experience with async\n- Strong communication\n\nBenefits: ...";
        let req = adapter.extract_requirements(body);
        assert!(req.is_some());
        assert!(req.unwrap().contains("Rust"));
    }

    #[test]
    fn test_extract_skills_from_text() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "We need a Senior Rust Developer with Python and AWS experience. Kubernetes knowledge is a plus.";
        let skills = adapter.extract_skills_from_text(text);

        assert!(skills.contains(&"Rust".to_string()));
        assert!(skills.contains(&"Python".to_string()));
        assert!(skills.contains(&"AWS".to_string()));
        assert!(skills.contains(&"Kubernetes".to_string()));
    }

    #[test]
    fn test_infer_job_type_internship() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "Summer Internship Program for Students";
        let job_type = adapter.infer_job_type(text);
        assert_eq!(job_type, Some("internship".to_string()));
    }

    #[test]
    fn test_infer_job_type_part_time() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "Part-time position, 20 hours per week";
        let job_type = adapter.infer_job_type(text);
        assert_eq!(job_type, Some("part-time".to_string()));
    }

    #[test]
    fn test_infer_job_type_contract() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "Contract role for 6 months";
        let job_type = adapter.infer_job_type(text);
        assert_eq!(job_type, Some("contract".to_string()));
    }

    #[test]
    fn test_infer_job_type_full_time_default() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "Join our team as a Software Engineer";
        let job_type = adapter.infer_job_type(text);
        assert_eq!(job_type, Some("full-time".to_string()));
    }

    #[test]
    fn test_infer_remote_policy_remote() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "This is a fully remote position, work from anywhere";
        let policy = adapter.infer_remote_policy(text, &None);
        assert_eq!(policy, Some("remote".to_string()));
    }

    #[test]
    fn test_infer_remote_policy_hybrid() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "Hybrid work model: 3 days in office, 2 days remote";
        let policy = adapter.infer_remote_policy(text, &None);
        assert_eq!(policy, Some("hybrid".to_string()));
    }

    #[test]
    fn test_infer_remote_policy_onsite() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let text = "On-site position at our downtown office";
        let policy = adapter.infer_remote_policy(text, &None);
        assert_eq!(policy, Some("onsite".to_string()));
    }

    #[test]
    fn test_parse_job_from_email() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let raw = RawContent {
            source_url: "gmail:test123".to_string(),
            content_type: ContentType::Email,
            content: "Job: Senior Rust Developer\nCompany: Acme Corp\nLocation: Tunis, Tunisia\nSalary: $100k - $150k\nApply: https://acme.com/jobs/123\nRequirements: 5+ years Rust, AWS, Kubernetes".to_string(),
            content_hash: "abc123".to_string(),
            fetched_at: Utc::now(),
            metadata: json!({
                "subject": "Job Alert: Senior Rust Developer at Acme Corp",
                "from": "jobs@linkedin.com",
                "date": "Mon, 15 Jan 2024 10:00:00 +0000",
                "message_id": "test123"
            }),
        };

        let parsed = adapter.parse_job_from_email(&raw).unwrap();

        assert_eq!(parsed.title, "Job Alert: Senior Rust Developer at Acme Corp");
        assert!(parsed.company.contains("Acme") || parsed.company.contains("Linkedin"));
        assert!(parsed.location.is_some());
        assert!(parsed.salary_range.is_some());
        assert!(parsed.application_url.is_some());
        assert!(parsed.requirements.is_some());
        assert!(parsed.posted_date.is_some());
    }

    #[test]
    fn test_parse_to_job_posting() {
        let oauth = create_mock_oauth();
        let adapter = GmailAdapter::new(oauth);

        let raw = RawContent {
            source_url: "gmail:test456".to_string(),
            content_type: ContentType::Email,
            content: "Senior Rust Developer at TechCorp\nLocation: Remote\nSalary: $120k - $180k\nApply: https://techcorp.greenhouse.io/jobs/789\nRequirements: Rust, Python, Docker, Kubernetes, AWS".to_string(),
            content_hash: "def456".to_string(),
            fetched_at: Utc::now(),
            metadata: json!({
                "subject": "Job Alert: Senior Rust Developer",
                "from": "jobs@indeed.com",
                "date": "Tue, 16 Jan 2024 10:00:00 +0000",
                "message_id": "test456"
            }),
        };

        let source_id = Uuid::new_v4();
        let posting = adapter.parse(&raw, source_id).unwrap();

        assert_eq!(posting.source_id, source_id);
        assert_eq!(posting.source_posting_id, "test456");
        assert!(posting.title.contains("Rust"));
        assert!(!posting.company.is_empty());
        assert!(posting.location.is_some());
        assert!(posting.salary_range.is_some());
        assert!(posting.application_url.is_some());
        assert!(posting.experience_required.is_some());
        assert!(!posting.skills.0.is_empty());
        assert!(posting.skills.0.contains(&"Rust".to_string()));
        assert!(posting.job_type.is_some());
        assert!(posting.remote_policy.is_some());
        assert_eq!(posting.content_hash, "def456");
        assert!(posting.raw_html.is_some());
    }

    fn create_mock_oauth() -> GmailOAuth {
        // This won't actually be used in unit tests since we don't call OAuth methods
        // We create a minimal instance for testing the parser
        use crate::gmail::oauth::OAuthConfig;
        
        let config = OAuthConfig {
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
            android_scheme: None,
        };
        
        GmailOAuth::new(config).unwrap()
    }
}