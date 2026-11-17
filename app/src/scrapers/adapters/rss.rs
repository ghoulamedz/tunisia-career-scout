//! RSS Adapter
//!
//! Fetches and parses RSS/Atom feeds for job postings.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use md5;
use reqwest::Client;
use rss::{Channel, Item};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::posting::{JobPosting, PostingStatus};
use crate::models::source::SourceConfig;
use crate::scrapers::pipeline::{ContentType, RawContent, SourceAdapter};

/// Default poll interval in hours
const DEFAULT_POLL_INTERVAL_HOURS: u32 = 1;

/// Default field mapping from RSS fields to JobPosting fields
fn default_field_mapping() -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    mapping.insert("title".to_string(), "title".to_string());
    mapping.insert("description".to_string(), "description".to_string());
    mapping.insert("content_encoded".to_string(), "description".to_string());
    mapping.insert("link".to_string(), "application_url".to_string());
    mapping.insert("pub_date".to_string(), "posted_date".to_string());
    mapping.insert("guid".to_string(), "source_posting_id".to_string());
    mapping.insert("categories".to_string(), "skills".to_string());
    mapping.insert("author".to_string(), "company".to_string());
    mapping.insert("dc_creator".to_string(), "company".to_string());
    mapping
}

/// RSS Adapter for fetching and parsing RSS/Atom feeds
pub struct RssAdapter {
    client: Client,
}

impl RssAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("TunisiaCareerScout/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Validate the RSS source configuration
    pub fn validate_config(config: &Value) -> Result<()> {
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("RSS config must contain 'url' field"))?;

        if url.is_empty() {
            return Err(anyhow!("RSS feed URL cannot be empty"));
        }

        // Validate URL format
        url::Url::parse(url).context("Invalid RSS feed URL")?;

        // Validate poll_interval_hours if present
        if let Some(interval) = config.get("poll_interval_hours") {
            if !interval.is_number() || interval.as_u64().unwrap_or(0) == 0 {
                return Err(anyhow!("poll_interval_hours must be a positive integer"));
            }
        }

        // Validate field_mapping if present
        if let Some(mapping) = config.get("field_mapping") {
            if !mapping.is_object() {
                return Err(anyhow!("field_mapping must be a JSON object"));
            }
        }

        Ok(())
    }

    /// Get the poll interval in hours from config (default: 1 hour)
    pub fn get_poll_interval_hours(config: &SourceConfig) -> u32 {
        match config {
            SourceConfig::Rss { poll_interval_hours, .. } => poll_interval_hours.unwrap_or(DEFAULT_POLL_INTERVAL_HOURS),
            _ => DEFAULT_POLL_INTERVAL_HOURS,
        }
    }

    /// Parse an RSS feed and extract raw content items
    async fn fetch_feed(&self, url: &str) -> Result<Channel> {
        let response = self.client.get(url).send().await?;
        response.error_for_status_ref()?;
        let body = response.text().await?;

        // Parse using rss crate (handles both RSS 2.0 and Atom 1.0)
        let channel = Channel::read_from(body.as_bytes())
            .context("Failed to parse RSS/Atom feed")?;

        Ok(channel)
    }

    /// Extract text content from an RSS item, handling namespaces
    fn extract_item_content(&self, item: &Item) -> String {
        // Try content:encoded first (common in WordPress feeds)
        if let Some(content) = item.content() {
            return content.to_string();
        }

        // Try description
        if let Some(desc) = item.description() {
            return desc.to_string();
        }

        // Try Atom content
        if let Some(ext_map) = item.extensions.get("content") {
            if let Some(content_vec) = ext_map.get("content") {
                if let Some(first) = content_vec.first() {
                    if let Some(value) = first.value.as_ref() {
                        return value.clone();
                    }
                }
            }
        }

        String::new()
    }

    /// Extract GUID or link for deduplication
    fn extract_guid(&self, item: &Item) -> String {
        // Try GUID first
        if let Some(guid) = item.guid() {
            return guid.value().to_string();
        }

        // Fall back to link
        if let Some(link) = item.link() {
            return link.to_string();
        }

        // Fall back to title + pub_date hash
        let title = item.title().unwrap_or("");
        let pub_date = item.pub_date().unwrap_or("");
        format!("{:x}", md5::compute(format!("{}{}", title, pub_date)))
    }

    /// Extract categories/tags from item
    fn extract_categories(&self, item: &Item) -> Vec<String> {
        let mut categories = Vec::new();

        // Standard RSS categories
        for cat in item.categories() {
            categories.push(cat.name().to_string());
        }

        // Atom categories (in extensions)
        if let Some(ext_map) = item.extensions.get("category") {
            for cat_vec in ext_map.values() {
                for cat in cat_vec {
                    if let Some(term) = cat.attrs.get("term") {
                        categories.push(term.clone());
                    } else if let Some(value) = cat.value.as_ref() {
                        categories.push(value.clone());
                    }
                }
            }
        }

        categories
    }

    /// Extract author/company from item (handles dc:creator namespace)
    fn extract_author(&self, item: &Item) -> Option<String> {
        // Standard RSS author
        if let Some(author) = item.author() {
            return Some(author.to_string());
        }

        // Dublin Core creator
        if let Some(dc_map) = item.extensions.get("creator") {
            if let Some(dc_vec) = dc_map.get("creator") {
                if let Some(first) = dc_vec.first() {
                    if let Some(value) = first.value.as_ref() {
                        return Some(value.clone());
                    }
                }
            }
        }

        // Try dc:creator in default namespace
        if let Some(dc_map) = item.extensions.get("dc") {
            if let Some(creator_vec) = dc_map.get("creator") {
                if let Some(first) = creator_vec.first() {
                    if let Some(value) = first.value.as_ref() {
                        return Some(value.clone());
                    }
                }
            }
        }

        None
    }

    /// Parse date from RSS item (handles multiple formats)
    fn parse_date(&self, item: &Item, custom_format: Option<&str>) -> Option<DateTime<Utc>> {
        // Try custom format first
        if let Some(fmt) = custom_format {
            if let Some(pub_date) = item.pub_date() {
                if let Ok(dt) = DateTime::parse_from_str(pub_date, fmt) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }

        // Try standard RSS pubDate (RFC 822 / RFC 2822)
        if let Some(pub_date) = item.pub_date() {
            // Try RFC 2822
            if let Ok(dt) = DateTime::parse_from_rfc2822(pub_date) {
                return Some(dt.with_timezone(&Utc));
            }
            // Try RFC 3339
            if let Ok(dt) = DateTime::parse_from_rfc3339(pub_date) {
                return Some(dt.with_timezone(&Utc));
            }
            // Try common formats
            for fmt in &["%a, %d %b %Y %H:%M:%S %z", "%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%d %H:%M:%S"] {
                if let Ok(dt) = DateTime::parse_from_str(pub_date, fmt) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }

        // Try Atom updated/published in extensions
        for key in ["updated", "published"] {
            if let Some(ext_map) = item.extensions.get(key) {
                if let Some(vec) = ext_map.get(key) {
                    if let Some(first) = vec.first() {
                        if let Some(value) = first.value.as_ref() {
                            for fmt in &["%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M:%S%.f%z", "%Y-%m-%dT%H:%M:%SZ"] {
                                if let Ok(dt) = DateTime::parse_from_str(value, fmt) {
                                    return Some(dt.with_timezone(&Utc));
                                }
                            }
                            if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                                return Some(dt.with_timezone(&Utc));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Apply custom field mapping to extract JobPosting fields from RSS item
    fn apply_field_mapping(
        &self,
        item: &Item,
        mapping: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut result = HashMap::new();

        for (rss_field, job_field) in mapping {
            let value = match rss_field.as_str() {
                "title" => item.title().map(|s| s.to_string()),
                "description" => item.description().map(|s| s.to_string()),
                "content_encoded" => item.content().map(|s| s.to_string()),
                "link" => item.link().map(|s| s.to_string()),
                "pub_date" => item.pub_date().map(|s| s.to_string()),
                "guid" => item.guid().map(|g| g.value().to_string()),
                "author" => item.author().map(|s| s.to_string()),
                "dc_creator" => self.extract_author(item),
                _ => {
                    // Try to get from extensions (namespaces)
                    if let Some(ext_map) = item.extensions.get(rss_field) {
                        if let Some(vec) = ext_map.get(rss_field) {
                            vec.first().and_then(|v| v.value.clone())
                        } else {
                            // Try first value in the namespace
                            ext_map.values().next().and_then(|v| v.first()).and_then(|v| v.value.clone())
                        }
                    } else {
                        None
                    }
                }
            };

            if let Some(v) = value {
                if !v.is_empty() {
                    result.insert(job_field.clone(), v);
                }
            }
        }

        result
    }

    /// Convert RSS item to RawContent
    fn item_to_raw_content(
        &self,
        item: &Item,
        _feed_url: &str,
        custom_mapping: Option<&HashMap<String, String>>,
        date_format: Option<&str>,
    ) -> RawContent {
        let content = self.extract_item_content(item);
        let content_hash = format!("{:x}", md5::compute(&content));
        let guid = self.extract_guid(item);
        let categories = self.extract_categories(item);
        let author = self.extract_author(item);
        let posted_date = self.parse_date(item, date_format);

        let default_mapping = default_field_mapping();
        let mapping = custom_mapping.unwrap_or(&default_mapping);
        let mapped_fields = self.apply_field_mapping(item, mapping);

        let mut metadata = serde_json::json!({
            "title": item.title(),
            "link": item.link(),
            "description": item.description(),
            "content_encoded": item.content(),
            "pub_date": item.pub_date(),
            "guid": item.guid().map(|g| g.value()),
            "categories": categories,
            "author": author,
            "mapped_fields": mapped_fields,
        });

        // Add link as application_url if present
        if let Some(link) = item.link() {
            metadata["application_url"] = serde_json::json!(link);
        }

        RawContent {
            source_url: guid.clone(),
            content,
            content_type: ContentType::Rss,
            fetched_at: posted_date.unwrap_or_else(Utc::now),
            content_hash,
            metadata,
        }
    }

    /// Parse RawContent into a JobPosting
    pub fn parse(&self, raw: &RawContent, source_id: Uuid) -> Result<JobPosting> {
        let metadata = &raw.metadata;

        // Extract fields from metadata.mapped_fields or fall back to direct metadata
        let mapped = metadata
            .get("mapped_fields")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let get_mapped = |key: &str| -> Option<String> {
            mapped
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        let get_meta_str = |key: &str| -> Option<String> {
            metadata.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
        };

        let title = get_mapped("title")
            .or_else(|| get_meta_str("title"))
            .ok_or_else(|| anyhow!("Missing required field: title"))?;

        let company = get_mapped("company")
            .or_else(|| get_meta_str("author"))
            .unwrap_or_else(|| "Unknown Company".to_string());

        let description = get_mapped("description")
            .or_else(|| get_meta_str("content_encoded"))
            .or_else(|| get_meta_str("description"))
            .unwrap_or_else(|| raw.content.clone());

        let source_posting_id = get_mapped("source_posting_id")
            .or_else(|| get_meta_str("guid"))
            .or_else(|| get_meta_str("link"))
            .unwrap_or_else(|| raw.content_hash.clone());

        let application_url = get_mapped("application_url")
            .or_else(|| get_meta_str("application_url"))
            .or_else(|| get_meta_str("link"));

        let posted_date = get_mapped("posted_date")
            .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|| {
                get_meta_str("pub_date")
                    .and_then(|s| DateTime::parse_from_rfc2822(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
            });

        // Extract skills from categories
        let skills = metadata
            .get("categories")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let scraped_at = raw.fetched_at;

        let mut posting = JobPosting::new(
            source_id,
            source_posting_id,
            title,
            company,
            description,
            scraped_at,
            raw.content_hash.clone(),
        );

        posting.application_url = application_url;
        posting.posted_date = posted_date;
        posting.skills = sqlx::types::Json(skills);

        // Set status
        posting.status = PostingStatus::Active;

        Ok(posting)
    }
}

#[async_trait]
impl SourceAdapter for RssAdapter {
    async fn fetch(&self, config: &SourceConfig) -> Result<Vec<RawContent>> {
        let SourceConfig::Rss {
            url,
            field_mapping,
            date_format,
            ..
        } = config
        else {
            return Err(anyhow!("RssAdapter only handles Rss source type"));
        };

        // Validate config
        let config_value = serde_json::to_value(config)?;
        Self::validate_config(&config_value)?;

        let channel = self.fetch_feed(url).await?;

        // Build custom field mapping if provided
        let custom_mapping = if let Some(mapping_value) = field_mapping {
            if let Some(obj) = mapping_value.as_object() {
                let mut mapping = HashMap::new();
                for (k, v) in obj {
                    if let Some(v_str) = v.as_str() {
                        mapping.insert(k.clone(), v_str.to_string());
                    }
                }
                Some(mapping)
            } else {
                None
            }
        } else {
            None
        };

        let mut results = Vec::new();
        let mut seen_guids = std::collections::HashSet::new();

        for item in channel.items() {
            let guid = self.extract_guid(item);

            // Deduplication by GUID or link
            if seen_guids.contains(&guid) {
                continue;
            }
            seen_guids.insert(guid);

            let raw_content = self.item_to_raw_content(
                item,
                url,
                custom_mapping.as_ref(),
                date_format.as_deref(),
            );

            results.push(raw_content);
        }

        Ok(results)
    }

    fn source_type(&self) -> &'static str {
        "rss"
    }
}

impl Default for RssAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:dc="http://purl.org/dc/elements/1.1/">
<channel>
    <title>Test Job Board</title>
    <link>https://example.com/jobs</link>
    <description>Latest job postings</description>
    <item>
        <title>Senior Rust Developer</title>
        <link>https://example.com/jobs/123</link>
        <description>We are looking for a Rust developer</description>
        <content:encoded><![CDATA[<p>Full description with <strong>HTML</strong></p>]]></content:encoded>
        <pubDate>Mon, 15 Jan 2024 10:00:00 +0000</pubDate>
        <guid isPermaLink="true">https://example.com/jobs/123</guid>
        <category>Rust</category>
        <category>Backend</category>
        <dc:creator>Acme Corp</dc:creator>
    </item>
    <item>
        <title>Python Engineer</title>
        <link>https://example.com/jobs/456</link>
        <description>Python ML role</description>
        <pubDate>Tue, 16 Jan 2024 10:00:00 +0000</pubDate>
        <guid>job-456</guid>
        <category>Python</category>
        <category>ML</category>
    </item>
</channel>
</rss>"#;

    const SAMPLE_ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Test Job Board</title>
    <link href="https://example.com/jobs"/>
    <updated>2024-01-16T10:00:00Z</updated>
    <entry>
        <title>Go Developer</title>
        <link href="https://example.com/jobs/789"/>
        <id>https://example.com/jobs/789</id>
        <updated>2024-01-16T10:00:00Z</updated>
        <summary>Go backend position</summary>
        <content type="html"><![CDATA[<p>Full <strong>HTML</strong> description</p>]]></content>
        <category term="Go"/>
        <category term="Backend"/>
        <author>
            <name>TechStart Inc</name>
        </author>
    </entry>
</feed>"#;

    #[tokio::test]
    async fn test_parse_rss_feed() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        assert_eq!(channel.title(), "Test Job Board");
        assert_eq!(channel.items().len(), 2);

        let item = &channel.items()[0];
        assert_eq!(item.title(), Some("Senior Rust Developer"));
        assert_eq!(item.link(), Some("https://example.com/jobs/123"));
        assert_eq!(item.description(), Some("We are looking for a Rust developer"));
        assert_eq!(item.content(), Some("<p>Full description with <strong>HTML</strong></p>"));
        assert_eq!(item.pub_date(), Some("Mon, 15 Jan 2024 10:00:00 +0000"));
        assert_eq!(item.guid().map(|g| g.value()), Some("https://example.com/jobs/123"));
        assert_eq!(item.categories().len(), 2);
    }

    #[tokio::test]
    async fn test_parse_atom_feed() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_ATOM.as_bytes()).unwrap();

        assert_eq!(channel.title(), "Test Job Board");
        assert_eq!(channel.items().len(), 1);

        let item = &channel.items()[0];
        assert_eq!(item.title(), Some("Go Developer"));
        assert_eq!(item.link(), Some("https://example.com/jobs/789"));
        assert_eq!(item.guid().map(|g| g.value()), Some("https://example.com/jobs/789"));
    }

    #[test]
    fn test_validate_config_valid() {
        let config = serde_json::json!({
            "url": "https://example.com/feed.xml",
            "poll_interval_hours": 2
        });
        assert!(RssAdapter::validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_url() {
        let config = serde_json::json!({
            "poll_interval_hours": 2
        });
        assert!(RssAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_url() {
        let config = serde_json::json!({
            "url": "not-a-url",
            "poll_interval_hours": 2
        });
        assert!(RssAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_poll_interval() {
        let config = serde_json::json!({
            "url": "https://example.com/feed.xml",
            "poll_interval_hours": 0
        });
        assert!(RssAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_invalid_field_mapping() {
        let config = serde_json::json!({
            "url": "https://example.com/feed.xml",
            "field_mapping": "not-an-object"
        });
        assert!(RssAdapter::validate_config(&config).is_err());
    }

    #[test]
    fn test_extract_guid() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        let item = &channel.items()[0];
        let guid = adapter.extract_guid(item);
        assert_eq!(guid, "https://example.com/jobs/123");

        let item = &channel.items()[1];
        let guid = adapter.extract_guid(item);
        assert_eq!(guid, "job-456");
    }

    #[test]
    fn test_extract_categories() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        let item = &channel.items()[0];
        let cats = adapter.extract_categories(item);
        assert_eq!(cats, vec!["Rust", "Backend"]);
    }

    #[test]
    fn test_extract_author() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        let item = &channel.items()[0];
        let author = adapter.extract_author(item);
        assert_eq!(author, Some("Acme Corp".to_string()));
    }

    #[test]
    fn test_parse_date_rfc2822() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        let item = &channel.items()[0];
        let date = adapter.parse_date(item, None);
        assert!(date.is_some());
        let dt = date.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_item_to_raw_content() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        let item = &channel.items()[0];
        let raw = adapter.item_to_raw_content(item, "https://example.com/feed.xml", None, None);

        assert_eq!(raw.content_type, ContentType::Rss);
        assert!(!raw.content.is_empty());
        assert!(!raw.content_hash.is_empty());
        assert!(raw.metadata.get("title").is_some());
        assert!(raw.metadata.get("categories").is_some());
    }

    #[test]
    fn test_parse_raw_to_job_posting() {
        let adapter = RssAdapter::new();
        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();

        let item = &channel.items()[0];
        let raw = adapter.item_to_raw_content(item, "https://example.com/feed.xml", None, None);

        let source_id = Uuid::new_v4();
        let posting = adapter.parse(&raw, source_id).unwrap();

        assert_eq!(posting.title, "Senior Rust Developer");
        assert_eq!(posting.company, "Acme Corp");
        assert!(posting.description.contains("Rust"));
        assert_eq!(posting.application_url, Some("https://example.com/jobs/123".to_string()));
        assert!(posting.posted_date.is_some());
        assert_eq!(posting.skills.0, vec!["Rust", "Backend"]);
        assert_eq!(posting.status, PostingStatus::Active);
    }

    #[test]
    fn test_custom_field_mapping() {
        let adapter = RssAdapter::new();
        let mut custom_mapping = HashMap::new();
        custom_mapping.insert("title".to_string(), "title".to_string());
        custom_mapping.insert("dc_creator".to_string(), "company".to_string());
        custom_mapping.insert("content_encoded".to_string(), "description".to_string());
        custom_mapping.insert("link".to_string(), "application_url".to_string());
        custom_mapping.insert("pub_date".to_string(), "posted_date".to_string());
        custom_mapping.insert("guid".to_string(), "source_posting_id".to_string());

        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();
        let item = &channel.items()[0];
        let raw = adapter.item_to_raw_content(item, "https://example.com/feed.xml", Some(&custom_mapping), None);

        let source_id = Uuid::new_v4();
        let posting = adapter.parse(&raw, source_id).unwrap();

        assert_eq!(posting.title, "Senior Rust Developer");
        assert_eq!(posting.company, "Acme Corp");
        assert!(posting.description.contains("HTML"));
    }

    #[test]
    fn test_deduplication() {
        let adapter = RssAdapter::new();
        let mut seen = std::collections::HashSet::new();

        let channel = Channel::read_from(SAMPLE_RSS.as_bytes()).unwrap();
        for item in channel.items() {
            let guid = adapter.extract_guid(item);
            let is_new = seen.insert(guid.clone());
            if is_new {
                // First time seeing this GUID
            } else {
                // Duplicate
            }
        }

        // Both items should have unique GUIDs
        assert_eq!(seen.len(), 2);
    }
}