//! URL Adapter Integration Tests
//!
//! Tests for the URL adapter with mocked HTTP responses.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use tunisia_career_scout::scrapers::adapters::url::{UrlAdapter, UrlSourceConfig, FieldSelectors, PaginationConfig, PaginationType};
use tunisia_career_scout::scrapers::pipeline::{SourceAdapter, RawContent, ContentType};
use tunisia_career_scout::models::source::SourceConfig;

/// Mock HTTP client that records requests
struct MockHttpClient {
    server: Arc<MockServer>,
}

impl MockHttpClient {
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

#[tokio::test]
async fn test_validate_config_valid() {
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

#[tokio::test]
async fn test_validate_config_invalid_url() {
    let config = serde_json::json!({
        "url": "not-a-url",
        "selector": ".job"
    });
    assert!(UrlAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_invalid_selector() {
    let config = serde_json::json!({
        "url": "https://example.com",
        "selector": "[invalid"
    });
    assert!(UrlAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_validate_config_invalid_rate_limit() {
    let config = serde_json::json!({
        "url": "https://example.com",
        "rate_limit_rps": 200.0
    });
    assert!(UrlAdapter::validate_config(&config).is_err());
}

#[tokio::test]
async fn test_extract_listing_urls() {
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

#[tokio::test]
async fn test_extract_listing_urls_absolute() {
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

#[tokio::test]
async fn test_parse_detail_page() {
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

#[tokio::test]
async fn test_token_bucket() {
    let mut bucket = tunisia_career_scout::scrapers::adapters::url::TokenBucket::new(2.0);
    assert!(bucket.try_take());
    assert!(bucket.try_take());
    assert!(!bucket.try_take());
}

#[tokio::test]
async fn test_url_source_config_deserialize() {
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

#[tokio::test]
async fn test_pagination_config_deserialize() {
    let json = serde_json::json!({
        "type_": "url_param",
        "selector": ".next",
        "max_pages": 5,
        "url_param": "page"
    });
    let config: PaginationConfig = serde_json::from_value(json).unwrap();
    assert!(matches!(config.type_, PaginationType::UrlParam));
    assert_eq!(config.max_pages, Some(5));
    assert_eq!(config.url_param, Some("page".to_string()));
}

#[tokio::test]
async fn test_fetch_single_page() {
    let mock_client = MockHttpClient::new().await;
    
    let html = r#"
        <html>
            <body>
                <h1>Software Engineer</h1>
                <div class="company">Acme Corp</div>
                <p>We are hiring a software engineer</p>
            </body>
        </html>
    "#;
    
    mock_client.mock_get("/job/1", 200, html).await;
    
    let adapter = UrlAdapter::new();
    let config = SourceConfig::Url {
        url: mock_client.url("/job/1"),
        selector: None,
        detail_selector: None,
        pagination: None,
        headers: None,
    };
    
    let results = adapter.fetch(&config).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content_type, ContentType::Html);
    assert!(results[0].content.contains("Software Engineer"));
    assert!(results[0].content.contains("Acme Corp"));
}

#[tokio::test]
async fn test_fetch_listing_page_with_detail_urls() {
    let mock_client = MockHttpClient::new().await;
    
    let listing_html = r#"
        <html>
            <body>
                <a class="job-link" href="/jobs/1">Job 1</a>
                <a class="job-link" href="/jobs/2">Job 2</a>
            </body>
        </html>
    "#;
    
    let detail_html_1 = r#"
        <html>
            <body>
                <h1 class="title">Software Engineer</h1>
                <div class="company">Acme Corp</div>
                <div class="desc">Job 1 description</div>
            </body>
        </html>
    "#;
    
    let detail_html_2 = r#"
        <html>
            <body>
                <h1 class="title">Data Scientist</h1>
                <div class="company">Data Inc</div>
                <div class="desc">Job 2 description</div>
            </body>
        </html>
    "#;
    
    mock_client.mock_get("/jobs", 200, listing_html).await;
    mock_client.mock_get("/jobs/1", 200, detail_html_1).await;
    mock_client.mock_get("/jobs/2", 200, detail_html_2).await;
    
    let adapter = UrlAdapter::new();
    let config = SourceConfig::Url {
        url: mock_client.url("/jobs"),
        selector: Some(".job-link".to_string()),
        detail_selector: None,
        pagination: None,
        headers: None,
    };
    
    let results = adapter.fetch(&config).await.unwrap();
    assert_eq!(results.len(), 2);
    
    let titles: Vec<&str> = results.iter().map(|r| {
        // Extract title from content (simplified)
        if r.content.contains("Software Engineer") { "Software Engineer" }
        else if r.content.contains("Data Scientist") { "Data Scientist" }
        else { "Unknown" }
    }).collect();
    
    assert!(titles.contains(&"Software Engineer"));
    assert!(titles.contains(&"Data Scientist"));
}

#[tokio::test]
async fn test_fetch_with_pagination_url_param() {
    let mock_client = MockHttpClient::new().await;
    
    let page1_html = r#"
        <html>
            <body>
                <a class="job-link" href="/jobs/1">Job 1</a>
                <a class="job-link" href="/jobs/2">Job 2</a>
            </body>
        </html>
    "#;
    
    let page2_html = r#"
        <html>
            <body>
                <a class="job-link" href="/jobs/3">Job 3</a>
            </body>
        </html>
    "#;
    
    let detail_html = r#"
        <html>
            <body>
                <h1 class="title">Test Job</h1>
                <div class="company">Test Corp</div>
                <div class="desc">Test description</div>
            </body>
        </html>
    "#;
    
    mock_client.mock_get("/jobs", 200, page1_html).await;
    mock_client.mock_get("/jobs?page=2", 200, page2_html).await;
    mock_client.mock_get("/jobs/1", 200, detail_html).await;
    mock_client.mock_get("/jobs/2", 200, detail_html).await;
    mock_client.mock_get("/jobs/3", 200, detail_html).await;
    
    let adapter = UrlAdapter::new();
    let config = SourceConfig::Url {
        url: mock_client.url("/jobs"),
        selector: Some(".job-link".to_string()),
        detail_selector: None,
        pagination: Some(tunisia_career_scout::models::source::PaginationConfig {
            type_: tunisia_career_scout::models::source::PaginationType::UrlParam,
            selector: None,
            max_pages: Some(2),
        }),
        headers: None,
    };
    
    let results = adapter.fetch(&config).await.unwrap();
    // Should have fetched 3 jobs across 2 pages
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_user_agent_rotation() {
    let adapter = UrlAdapter::new();
    let ua1 = adapter.get_user_agent();
    let ua2 = adapter.get_user_agent();
    let ua3 = adapter.get_user_agent();
    
    // Should return one of the default user agents
    assert!(!ua1.is_empty());
    assert!(!ua2.is_empty());
    assert!(!ua3.is_empty());
    
    // All should be from the default pool
    let default_uas: Vec<&str> = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Edg/120.0.0.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
    ];
    
    for ua in [ua1, ua2, ua3] {
        assert!(default_uas.iter().any(|d| ua.contains(d)) || ua.contains("Mozilla"));
    }
}

#[tokio::test]
async fn test_source_type() {
    let adapter = UrlAdapter::new();
    assert_eq!(adapter.source_type(), "url");
}

#[tokio::test]
async fn test_field_selectors_extraction() {
    let adapter = UrlAdapter::new();
    
    let html = r#"
        <html>
            <body>
                <h1 itemprop="title">Senior Rust Developer</h1>
                <div itemprop="hiringOrganization" itemscope itemtype="https://schema.org/Organization">
                    <span itemprop="name">RustCorp</span>
                </div>
                <div itemprop="description">We need a Rust expert</div>
                <div itemprop="jobLocation" itemscope itemtype="https://schema.org/Place">
                    <span itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
                        <span itemprop="addressLocality">Tunis</span>
                    </span>
                </div>
                <span itemprop="baseSalary" itemscope itemtype="https://schema.org/MonetaryAmount">
                    <span itemprop="value" itemscope itemtype="https://schema.org/QuantitativeValue">
                        <span itemprop="minValue">50000</span>
                        <span itemprop="maxValue">80000</span>
                    </span>
                </span>
                <a itemprop="applicationUrl" href="/apply/rust-123">Apply Now</a>
            </body>
        </html>
    "#;
    
    let field_selectors = Some(FieldSelectors {
        title: Some("[itemprop='title']".to_string()),
        company: Some("[itemprop='hiringOrganization'] [itemprop='name']".to_string()),
        description: Some("[itemprop='description']".to_string()),
        location: Some("[itemprop='addressLocality']".to_string()),
        salary: Some("[itemprop='baseSalary'] [itemprop='value']".to_string()),
        application_url: Some("[itemprop='applicationUrl']".to_string()),
        requirements: None,
        benefits: None,
        posted_date: None,
        job_type: None,
        remote_policy: None,
        experience_required: None,
        education_required: None,
    });
    
    let fields = adapter.parse_detail_page(html, "https://example.com/job/1", &field_selectors);
    
    assert_eq!(fields["title"], "Senior Rust Developer");
    assert_eq!(fields["company"], "RustCorp");
    assert_eq!(fields["description"], "We need a Rust expert");
    assert_eq!(fields["location"], "Tunis");
    assert_eq!(fields["application_url"], "/apply/rust-123");
}