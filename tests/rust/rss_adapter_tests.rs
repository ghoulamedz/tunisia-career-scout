//! RSS Adapter Integration Tests
//!
//! Tests for the RSS adapter with mocked HTTP responses.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use tunisia_career_scout::scrapers::adapters::RssAdapter;
use tunisia_career_scout::scrapers::pipeline::{SourceAdapter, RawContent, ContentType};
use tunisia_career_scout::models::source::SourceConfig;

/// Mock HTTP server for testing RSS feed fetching
struct MockRssServer {
    server: Arc<MockServer>,
}

impl MockRssServer {
    async fn new() -> Self {
        let server = Arc::new(MockServer::start().await);
        Self { server }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.server.uri(), path)
    }

    async fn mock_get(&self, path: &str, status: u16, body: &str) {
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.to_string()))
            .mount(self.server.as_ref())
            .await;
    }
}

const SAMPLE_RSS_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
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

const SAMPLE_ATOM_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
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
    <entry>
        <title>DevOps Engineer</title>
        <link href="https://example.com/jobs/999"/>
        <id>https://example.com/jobs/999</id>
        <updated>2024-01-17T10:00:00Z</updated>
        <summary>Infrastructure role</summary>
        <content type="html"><![CDATA[<p>Kubernetes and AWS experience required</p>]]></content>
        <category term="DevOps"/>
        <category term="Kubernetes"/>
        <author>
            <name>CloudCorp</name>
        </author>
    </entry>
</feed>"#;

const RSS_FEED_WITH_NAMESPACES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" 
     xmlns:content="http://purl.org/rss/1.0/modules/content/"
     xmlns:dc="http://purl.org/dc/elements/1.1/"
     xmlns:media="http://search.yahoo.com/mrss/">
<channel>
    <title>Job Board with Namespaces</title>
    <link>https://example.com/jobs</link>
    <item>
        <title>Frontend Developer</title>
        <link>https://example.com/jobs/frontend</link>
        <description>React developer needed</description>
        <content:encoded><![CDATA[<div><h2>Full Description</h2><p>We need a <strong>React</strong> expert.</p></div>]]></content:encoded>
        <pubDate>Wed, 17 Jan 2024 10:00:00 +0000</pubDate>
        <guid>frontend-123</guid>
        <category>Frontend</category>
        <category>React</category>
        <dc:creator>WebAgency</dc:creator>
        <media:thumbnail url="https://example.com/logo.png"/>
    </item>
</channel>
</rss>"#;

const EMPTY_RSS_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
    <title>Empty Job Board</title>
    <link>https://example.com/jobs</link>
    <description>No jobs</description>
</channel>
</rss>"#;

#[tokio::test]
async fn test_validate_config_valid() {
    let config = serde_json::json!({
        "url": "https://example.com/feed.xml",
        "poll_interval_hours": 2,
        "field_mapping": {
            "title": "title",
            "description": "description"
        },
        "date_format": "%Y-%m-%d"
    });
    assert!(RssAdapter::validate_config(&config).is_ok());
}

#[tokio::test]
async fn test_validate_config_missing_url() {
    let config = serde_json::json!({
        "poll_interval_hours": 2
    });
    assert!(RssAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_empty_url() {
    let config = serde_json::json!({
        "url": "",
        "poll_interval_hours": 2
    });
    assert!(RssAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_invalid_url() {
    let config = serde_json::json!({
        "url": "not-a-url",
        "poll_interval_hours": 2
    });
    assert!(RssAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_invalid_poll_interval_zero() {
    let config = serde_json::json!({
        "url": "https://example.com/feed.xml",
        "poll_interval_hours": 0
    });
    assert!(RssAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_invalid_poll_interval_negative() {
    let config = serde_json::json!({
        "url": "https://example.com/feed.xml",
        "poll_interval_hours": -1
    });
    assert!(RssAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_invalid_field_mapping() {
    let config = serde_json::json!({
        "url": "https://example.com/feed.xml",
        "field_mapping": "not-an-object"
    });
    assert!(RssAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_valid_field_mapping() {
    let config = serde_json::json!({
        "url": "https://example.com/feed.xml",
        "field_mapping": {
            "title": "title",
            "link": "application_url",
            "custom_field": "custom_mapping"
        }
    });
    assert!(RssAdapter::validate_config(&config).is_ok());
}

#[tokio::test]
async fn test_fetch_rss_feed() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/feed.xml", 200, SAMPLE_RSS_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/feed.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    assert_eq!(results.len(), 2);

    // Check first item
    let item1 = &results[0];
    assert_eq!(item1.content_type, ContentType::Rss);
    assert!(item1.content.contains("Rust"));
    assert!(!item1.content_hash.is_empty());
    assert!(item1.metadata.get("title").is_some());
    assert!(item1.metadata.get("categories").is_some());
    assert_eq!(item1.metadata["title"], "Senior Rust Developer");

    // Check categories
    let categories = item1.metadata["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 2);
    assert!(categories.iter().any(|c| c.as_str() == Some("Rust")));
    assert!(categories.iter().any(|c| c.as_str() == Some("Backend")));

    // Check author/company from dc:creator
    assert_eq!(item1.metadata["author"], "Acme Corp");

    // Check second item
    let item2 = &results[1];
    assert_eq!(item2.metadata["title"], "Python Engineer");
    assert_eq!(item2.metadata["author"], serde_json::Value::Null); // No author for second item
}

#[tokio::test]
async fn test_fetch_atom_feed() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/atom.xml", 200, SAMPLE_ATOM_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/atom.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    assert_eq!(results.len(), 2);

    let item1 = &results[0];
    assert_eq!(item1.metadata["title"], "Go Developer");
    assert_eq!(item1.metadata["author"], "TechStart Inc");

    let item2 = &results[1];
    assert_eq!(item2.metadata["title"], "DevOps Engineer");
    assert_eq!(item2.metadata["author"], "CloudCorp");
}

#[tokio::test]
async fn test_fetch_rss_with_namespaces() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/namespaces.xml", 200, RSS_FEED_WITH_NAMESPACES).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/namespaces.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    assert_eq!(results.len(), 1);

    let item = &results[0];
    assert_eq!(item.metadata["title"], "Frontend Developer");
    assert_eq!(item.metadata["author"], "WebAgency");

    // Check content:encoded is extracted
    assert!(item.content.contains("Full Description"));
    assert!(item.content.contains("React"));
}

#[tokio::test]
async fn test_fetch_empty_feed() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/empty.xml", 200, EMPTY_RSS_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/empty.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_fetch_with_custom_field_mapping() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/feed.xml", 200, SAMPLE_RSS_FEED).await;

    let adapter = RssAdapter::new();
    let custom_mapping = serde_json::json!({
        "title": "title",
        "dc_creator": "company",
        "content_encoded": "description",
        "link": "application_url",
        "pub_date": "posted_date",
        "guid": "source_posting_id"
    });

    let config = SourceConfig::Rss {
        url: mock_server.url("/feed.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: Some(custom_mapping),
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    assert_eq!(results.len(), 2);

    let item = &results[0];
    // Check that mapped fields are in metadata.mapped_fields
    let mapped = item.metadata["mapped_fields"].as_object().unwrap();
    assert_eq!(mapped["title"], "Senior Rust Developer");
    assert_eq!(mapped["company"], "Acme Corp");
    assert!(mapped["description"].as_str().unwrap().contains("HTML"));
    assert_eq!(mapped["application_url"], "https://example.com/jobs/123");
}

#[tokio::test]
async fn test_fetch_with_date_format() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/feed.xml", 200, SAMPLE_RSS_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/feed.xml"),
        item_xpath: None,
        date_format: Some("%a, %d %b %Y %H:%M:%S %z".to_string()),
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    assert_eq!(results.len(), 2);
    // Date format should be used for parsing
    assert!(results[0].fetched_at.year() == 2024);
    assert!(results[0].fetched_at.month() == 1);
    assert!(results[0].fetched_at.day() == 15);
}

#[tokio::test]
async fn test_parse_raw_to_job_posting() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/feed.xml", 200, SAMPLE_RSS_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/feed.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();
    let source_id = uuid::Uuid::new_v4();

    let posting = adapter.parse(&results[0], source_id).unwrap();

    assert_eq!(posting.source_id, source_id);
    assert_eq!(posting.title, "Senior Rust Developer");
    assert_eq!(posting.company, "Acme Corp");
    assert!(posting.description.contains("Rust"));
    assert_eq!(posting.application_url, Some("https://example.com/jobs/123".to_string()));
    assert!(posting.posted_date.is_some());
    assert_eq!(posting.skills.0, vec!["Rust", "Backend"]);
    assert_eq!(posting.status, tunisia_career_scout::models::posting::PostingStatus::Active);
    assert_eq!(posting.source_posting_id, "https://example.com/jobs/123");
}

#[tokio::test]
async fn test_parse_atom_to_job_posting() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/atom.xml", 200, SAMPLE_ATOM_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/atom.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();
    let source_id = uuid::Uuid::new_v4();

    let posting = adapter.parse(&results[0], source_id).unwrap();

    assert_eq!(posting.title, "Go Developer");
    assert_eq!(posting.company, "TechStart Inc");
    assert!(posting.description.contains("Go"));
    assert_eq!(posting.application_url, Some("https://example.com/jobs/789".to_string()));
    assert!(posting.posted_date.is_some());
    assert_eq!(posting.skills.0, vec!["Go", "Backend"]);
    assert_eq!(posting.source_posting_id, "https://example.com/jobs/789");
}

#[tokio::test]
async fn test_deduplication_by_guid() {
    // Feed with duplicate GUIDs
    let duplicate_feed = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
    <title>Test</title>
    <item>
        <title>Job 1</title>
        <link>https://example.com/job/1</link>
        <guid>same-guid</guid>
        <pubDate>Mon, 15 Jan 2024 10:00:00 +0000</pubDate>
    </item>
    <item>
        <title>Job 2</title>
        <link>https://example.com/job/2</link>
        <guid>same-guid</guid>
        <pubDate>Tue, 16 Jan 2024 10:00:00 +0000</pubDate>
    </item>
    <item>
        <title>Job 3</title>
        <link>https://example.com/job/3</link>
        <guid>different-guid</guid>
        <pubDate>Wed, 17 Jan 2024 10:00:00 +0000</pubDate>
    </item>
</channel>
</rss>"#;

    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/dup.xml", 200, duplicate_feed).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/dup.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    // Should only have 2 items (second item with same GUID is deduplicated)
    assert_eq!(results.len(), 2);
    let titles: Vec<String> = results.iter().map(|r| r.metadata["title"].as_str().unwrap().to_string()).collect();
    assert!(titles.contains(&"Job 1".to_string()));
    assert!(titles.contains(&"Job 3".to_string()));
    assert!(!titles.contains(&"Job 2".to_string()));
}

#[tokio::test]
async fn test_deduplication_by_link_when_no_guid() {
    // Feed with duplicate links but no GUIDs
    let duplicate_feed = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
    <title>Test</title>
    <item>
        <title>Job 1</title>
        <link>https://example.com/job/1</link>
        <pubDate>Mon, 15 Jan 2024 10:00:00 +0000</pubDate>
    </item>
    <item>
        <title>Job 2</title>
        <link>https://example.com/job/1</link>
        <pubDate>Tue, 16 Jan 2024 10:00:00 +0000</pubDate>
    </item>
    <item>
        <title>Job 3</title>
        <link>https://example.com/job/3</link>
        <pubDate>Wed, 17 Jan 2024 10:00:00 +0000</pubDate>
    </item>
</channel>
</rss>"#;

    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/dup2.xml", 200, duplicate_feed).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/dup2.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    // Should only have 2 items (second item with same link is deduplicated)
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_fetch_handles_http_error() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/error.xml", 500, "Internal Server Error").await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/error.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let result = adapter.fetch(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_fetch_handles_invalid_xml() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/invalid.xml", 200, "not xml at all").await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/invalid.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let result = adapter.fetch(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_source_type() {
    let adapter = RssAdapter::new();
    assert_eq!(adapter.source_type(), "rss");
}

#[tokio::test]
async fn test_get_poll_interval_hours() {
    let config = SourceConfig::Rss {
        url: "https://example.com/feed.xml".to_string(),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(3),
    };
    assert_eq!(RssAdapter::get_poll_interval_hours(&config), 3);

    let config = SourceConfig::Rss {
        url: "https://example.com/feed.xml".to_string(),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: None,
    };
    assert_eq!(RssAdapter::get_poll_interval_hours(&config), 1); // default
}

#[tokio::test]
async fn test_extract_categories_from_atom() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/atom.xml", 200, SAMPLE_ATOM_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/atom.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results = adapter.fetch(&config).await.unwrap();

    let item1 = &results[0];
    let categories = item1.metadata["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 2);
    assert!(categories.iter().any(|c| c.as_str() == Some("Go")));
    assert!(categories.iter().any(|c| c.as_str() == Some("Backend")));
}

#[tokio::test]
async fn test_content_hash_consistency() {
    let mock_server = MockRssServer::new().await;
    mock_server.mock_get("/feed.xml", 200, SAMPLE_RSS_FEED).await;

    let adapter = RssAdapter::new();
    let config = SourceConfig::Rss {
        url: mock_server.url("/feed.xml"),
        item_xpath: None,
        date_format: None,
        field_mapping: None,
        poll_interval_hours: Some(1),
    };

    let results1 = adapter.fetch(&config).await.unwrap();
    let results2 = adapter.fetch(&config).await.unwrap();

    // Content hash should be consistent for same content
    assert_eq!(results1[0].content_hash, results2[0].content_hash);
    assert_eq!(results1[1].content_hash, results2[1].content_hash);
}