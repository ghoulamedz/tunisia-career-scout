//! Rule Extractor Integration Tests
//!
//! Tests for deterministic rule-based extraction from various sources.

use anyhow::Result;
use tunisia_career_scout::scrapers::{RuleExtractor, pipeline::{RawContent, ContentType, ExtractionMethod}};
use uuid::Uuid;
use chrono::Utc;

fn create_html_raw(html: &str, url: &str) -> RawContent {
    RawContent {
        source_url: url.to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    }
}

#[tokio::test]
async fn test_json_ld_job_posting_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Senior Rust Developer",
                "hiringOrganization": {
                    "@type": "Organization",
                    "name": "RustCorp"
                },
                "description": "We are looking for a Rust expert.",
                "jobLocation": {
                    "@type": "Place",
                    "address": {
                        "@type": "PostalAddress",
                        "addressLocality": "Tunis",
                        "addressCountry": "TN"
                    }
                },
                "employmentType": "FULL_TIME",
                "baseSalary": {
                    "@type": "MonetaryAmount",
                    "currency": "TND",
                    "value": {
                        "@type": "QuantitativeValue",
                        "minValue": "50000",
                        "maxValue": "80000",
                        "unitText": "YEAR"
                    }
                },
                "datePosted": "2025-01-15T10:00:00Z",
                "validThrough": "2025-06-15T23:59:59Z",
                "jobLocationType": "REMOTE",
                "skills": ["Rust", "PostgreSQL", "Docker"],
                "experienceRequirements": "3+ years Rust experience",
                "educationRequirements": "BS Computer Science"
            }
            </script>
        </head>
        <body></body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/job/1");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Senior Rust Developer");
    assert_eq!(result.posting.company, "RustCorp");
    assert_eq!(result.posting.description, "We are looking for a Rust expert.");
    assert_eq!(result.posting.location, Some("Tunis, TN".to_string()));
    assert_eq!(result.posting.job_type, Some("FULL_TIME".to_string()));
    assert!(result.posting.salary_range.as_ref().unwrap().contains("50000"));
    assert!(result.posting.salary_range.as_ref().unwrap().contains("80000"));
    assert!(result.posting.posted_date.is_some());
    assert!(result.posting.deadline.is_some());
    assert_eq!(result.posting.remote_policy, Some("REMOTE".to_string()));
    assert_eq!(result.posting.skills.0, vec!["Rust", "PostgreSQL", "Docker"]);
    assert_eq!(result.posting.experience_required, Some("3+ years Rust experience".to_string()));
    assert_eq!(result.posting.education_required, Some("BS Computer Science".to_string()));
    assert!(result.confidence > 0.8);
    assert_eq!(result.extraction_method, ExtractionMethod::Rules);
}

#[tokio::test]
async fn test_microdata_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1 itemprop="title">Software Engineer</h1>
            <div itemprop="hiringOrganization" itemscope itemtype="https://schema.org/Organization">
                <span itemprop="name">TechCorp</span>
            </div>
            <div itemprop="description">Great job opportunity.</div>
            <div itemprop="jobLocation" itemscope itemtype="https://schema.org/Place">
                <div itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
                    <span itemprop="addressLocality">San Francisco</span>
                    <span itemprop="addressRegion">CA</span>
                    <span itemprop="addressCountry">US</span>
                </div>
            </div>
            <span itemprop="employmentType">FULL_TIME</span>
        </body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/job/2");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Software Engineer");
    assert_eq!(result.posting.company, "TechCorp");
    assert_eq!(result.posting.description, "Great job opportunity.");
    assert_eq!(result.posting.location, Some("San Francisco, CA, US".to_string()));
    assert_eq!(result.posting.job_type, Some("FULL_TIME".to_string()));
    assert!(result.confidence > 0.7);
}

#[tokio::test]
async fn test_greenhouse_ats_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <script id="job-search-results-data">
            {
                "job": {
                    "title": "Backend Engineer",
                    "company": { "name": "Greenhouse Inc" },
                    "content": "Build scalable systems.",
                    "location": { "name": "Remote" },
                    "employment_type": "full-time",
                    "remote": true,
                    "requirements": "5 years experience"
                }
            }
            </script>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "https://boards.greenhouse.io/jobs/123".to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Backend Engineer");
    assert_eq!(result.posting.company, "Greenhouse Inc");
    assert_eq!(result.posting.description, "Build scalable systems.");
    assert_eq!(result.posting.location, Some("Remote".to_string()));
    assert_eq!(result.posting.job_type, Some("full-time".to_string()));
    assert_eq!(result.posting.remote_policy, Some("remote".to_string()));
}

#[tokio::test]
async fn test_lever_ats_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1 data-qa="job-title">Frontend Developer</h1>
            <div data-qa="company-name">Lever Corp</div>
            <div data-qa="location">New York, NY</div>
            <div data-qa="description">React and TypeScript role.</div>
            <span data-qa="commitment">Full-time</span>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "https://jobs.lever.co/levercorp/123".to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Frontend Developer");
    assert_eq!(result.posting.company, "Lever Corp");
    assert_eq!(result.posting.location, Some("New York, NY".to_string()));
    assert_eq!(result.posting.description, "React and TypeScript role.");
    assert_eq!(result.posting.job_type, Some("Full-time".to_string()));
}

#[tokio::test]
async fn test_workable_ats_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1 data-ui="job-title">DevOps Engineer</h1>
            <div data-ui="company-name">Workable Ltd</div>
            <div data-ui="location">London, UK</div>
            <div data-ui="job-description">Kubernetes and AWS.</div>
            <span data-ui="employment-type">Contract</span>
            <span data-ui="workplace-type">Hybrid</span>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "https://apply.workable.com/j/123".to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "DevOps Engineer");
    assert_eq!(result.posting.company, "Workable Ltd");
    assert_eq!(result.posting.location, Some("London, UK".to_string()));
    assert_eq!(result.posting.job_type, Some("Contract".to_string()));
    assert_eq!(result.posting.remote_policy, Some("Hybrid".to_string()));
}

#[tokio::test]
async fn test_smartrecruiters_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1 data-qa="job-title">SmartRecruiters Job</h1>
            <div data-qa="company-name">SmartRecruiters Inc</div>
            <div data-qa="location">Berlin, Germany</div>
            <div data-qa="job-description">Description here.</div>
            <span data-qa="job-type">Full-time</span>
            <span data-qa="remote-type">Remote</span>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "https://careers.smartrecruiters.com/company/123".to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "SmartRecruiters Job");
    assert_eq!(result.posting.company, "SmartRecruiters Inc");
    assert_eq!(result.posting.location, Some("Berlin, Germany".to_string()));
    assert_eq!(result.posting.job_type, Some("Full-time".to_string()));
    assert_eq!(result.posting.remote_policy, Some("Remote".to_string()));
}

#[tokio::test]
async fn test_bamboohr_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1 class="job-title">BambooHR Position</h1>
            <div class="company-name">BambooHR Corp</div>
            <div class="job-location">Remote</div>
            <div class="job-description">Job description content.</div>
            <span class="job-type">Full-time</span>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "https://company.bamboohr.com/jobs/123".to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "BambooHR Position");
    assert_eq!(result.posting.company, "BambooHR Corp");
    assert_eq!(result.posting.location, Some("Remote".to_string()));
    assert_eq!(result.posting.job_type, Some("Full-time".to_string()));
}

#[tokio::test]
async fn test_meta_tags_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <head>
            <meta property="og:title" content="Meta Tag Job">
            <meta property="og:description" content="Found via meta tags">
            <meta property="og:site_name" content="MetaCorp">
            <meta name="job-location" content="Paris, France">
            <meta name="job-salary" content="€60k-€80k">
            <meta name="job-type" content="full-time">
            <meta name="job-remote" content="hybrid">
        </head>
        <body></body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/job/3");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Meta Tag Job");
    assert_eq!(result.posting.description, "Found via meta tags");
    assert_eq!(result.posting.company, "MetaCorp");
    assert_eq!(result.posting.location, Some("Paris, France".to_string()));
    assert_eq!(result.posting.salary_range, Some("€60k-€80k".to_string()));
    assert_eq!(result.posting.job_type, Some("full-time".to_string()));
    assert_eq!(result.posting.remote_policy, Some("hybrid".to_string()));
}

#[tokio::test]
async fn test_application_steps_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1>Test Job</h1>
            <a href="/apply/123" class="apply-button">Apply Now</a>
            <a href="mailto:jobs@example.com">Email Us</a>
            <form action="/submit-application" id="apply-form">
                <input type="text" name="name">
            </form>
        </body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/job/4");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert!(!result.posting.application_steps.0.is_empty());
    let steps = &result.posting.application_steps.0;
    
    // Should have link, email, and form steps
    assert!(steps.iter().any(|s| s.action_type == tunisia_career_scout::models::posting::ActionType::Link));
    assert!(steps.iter().any(|s| s.action_type == tunisia_career_scout::models::posting::ActionType::Email));
    assert!(steps.iter().any(|s| s.action_type == tunisia_career_scout::models::posting::ActionType::Form));
}

#[tokio::test]
async fn test_skills_extraction_from_description() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1>Rust Engineer</h1>
            <div class="company">RustCorp</div>
            <div class="description">
                We need a Rust expert with PostgreSQL and Docker experience.
                Skills: Rust, PostgreSQL, Docker, Kubernetes, AWS.
                Technologies: Git, Linux, CI/CD.
            </div>
        </body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/job/5");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert!(!result.posting.skills.0.is_empty());
    let skills: Vec<String> = result.posting.skills.0.iter().map(|s| s.to_lowercase()).collect();
    assert!(skills.iter().any(|s| s.contains("rust")));
    assert!(skills.iter().any(|s| s.contains("postgresql")));
    assert!(skills.iter().any(|s| s.contains("docker")));
}

#[tokio::test]
async fn test_rss_extraction() {
    let extractor = RuleExtractor::new();
    let rss = r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0">
        <channel>
            <title>Job Feed</title>
            <item>
                <title>RSS Job</title>
                <link>https://example.com/job/1</link>
                <description>RSS description</description>
                <pubDate>Mon, 15 Jan 2025 10:00:00 GMT</pubDate>
                <guid>job-1</guid>
            </item>
        </channel>
    </rss>
    "#;

    let raw = RawContent {
        source_url: "https://example.com/feed.xml".to_string(),
        content: rss.to_string(),
        content_type: ContentType::Rss,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "RSS Job");
    assert_eq!(result.posting.description, "RSS description");
    assert_eq!(result.posting.application_url, Some("https://example.com/job/1".to_string()));
    assert!(!result.posting.application_steps.0.is_empty());
}

#[tokio::test]
async fn test_email_extraction() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1>Email Job</h1>
            <div class="company">EmailCorp</div>
            <div class="description">Job from email.</div>
            <a href="mailto:hr@emailcorp.com">Apply</a>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "mailto:job@example.com".to_string(),
        content: html.to_string(),
        content_type: ContentType::Email,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({
            "subject": "New Job: Email Job",
            "from": "recruiter@emailcorp.com",
            "message_id": "<msg-123@example.com>",
            "html_body": html
        }),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Email Job");
    assert_eq!(result.posting.company, "EmailCorp");
    assert!(!result.posting.application_steps.0.is_empty());
    assert!(result.posting.application_steps.0.iter().any(|s| s.action_type == tunisia_career_scout::models::posting::ActionType::Email));
}

#[tokio::test]
async fn test_json_extraction() {
    let extractor = RuleExtractor::new();
    let json = serde_json::json!({
        "title": "API Job",
        "company": "API Corp",
        "description": "Job from API",
        "location": "Remote",
        "salary": "$100k-$150k",
        "job_type": "full-time",
        "remote": "remote",
        "skills": ["Rust", "Go", "Kubernetes"]
    });

    let raw = RawContent {
        source_url: "https://api.example.com/jobs/1".to_string(),
        content: json.to_string(),
        content_type: ContentType::Json,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "API Job");
    assert_eq!(result.posting.company, "API Corp");
    assert_eq!(result.posting.description, "Job from API");
    assert_eq!(result.posting.location, Some("Remote".to_string()));
    assert_eq!(result.posting.salary_range, Some("$100k-$150k".to_string()));
    assert_eq!(result.posting.job_type, Some("full-time".to_string()));
    assert_eq!(result.posting.remote_policy, Some("remote".to_string()));
    assert!(!result.posting.skills.0.is_empty());
}

#[tokio::test]
async fn test_confidence_scoring() {
    let extractor = RuleExtractor::new();
    
    // High confidence: full JSON-LD
    let html_full = r#"
    <html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Full Job",
                "hiringOrganization": { "@type": "Organization", "name": "Full Corp" },
                "description": "Complete job posting",
                "jobLocation": { "@type": "Place", "address": { "@type": "PostalAddress", "addressLocality": "Tunis" }},
                "employmentType": "FULL_TIME",
                "skills": ["Rust"]
            }
            </script>
        </head>
        <body></body>
    </html>
    "#;
    
    let raw_full = create_html_raw(html_full, "https://example.com/full");
    let result_full = extractor.extract(&raw_full, Uuid::new_v4()).await.unwrap();
    assert!(result_full.confidence > 0.85);

    // Low confidence: minimal HTML
    let html_minimal = r#"
    <html>
        <body>
            <h1>Minimal Job</h1>
        </body>
    </html>
    "#;
    
    let raw_minimal = create_html_raw(html_minimal, "https://example.com/minimal");
    let result_minimal = extractor.extract(&raw_minimal, Uuid::new_v4()).await.unwrap();
    assert!(result_minimal.confidence < 0.5);
}

#[tokio::test]
async fn test_missing_fields_identification() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1>Incomplete Job</h1>
        </body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/incomplete");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    let missing = &result.missing_fields;
    assert!(missing.contains(&"company".to_string()));
    assert!(missing.contains(&"description".to_string()));
    assert!(missing.contains(&"location".to_string()));
    assert!(missing.contains(&"skills".to_string()));
    assert!(missing.contains(&"application_steps".to_string()));
    assert!(missing.contains(&"salary_range".to_string()));
    assert!(missing.contains(&"job_type".to_string()));
    assert!(missing.contains(&"remote_policy".to_string()));
    assert!(missing.contains(&"experience_required".to_string()));
    assert!(missing.contains(&"education_required".to_string()));
}

#[tokio::test]
async fn test_json_ld_array_format() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <head>
            <script type="application/ld+json">
            [
                {
                    "@type": "WebPage",
                    "name": "Careers Page"
                },
                {
                    "@context": "https://schema.org",
                    "@type": "JobPosting",
                    "title": "Array Format Job",
                    "hiringOrganization": { "@type": "Organization", "name": "Array Corp" },
                    "description": "Job in JSON-LD array"
                }
            ]
            </script>
        </head>
        <body></body>
    </html>
    "#;

    let raw = create_html_raw(html, "https://example.com/array");
    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert_eq!(result.posting.title, "Array Format Job");
    assert_eq!(result.posting.company, "Array Corp");
    assert_eq!(result.posting.description, "Job in JSON-LD array");
}

#[tokio::test]
async fn test_relative_url_resolution() {
    let extractor = RuleExtractor::new();
    let html = r#"
    <html>
        <body>
            <h1>Relative URL Job</h1>
            <div class="company">Relative Corp</div>
            <a href="/apply/relative" class="apply-button">Apply</a>
        </body>
    </html>
    "#;

    let raw = RawContent {
        source_url: "https://example.com/jobs/123".to_string(),
        content: html.to_string(),
        content_type: ContentType::Html,
        fetched_at: Utc::now(),
        content_hash: "test_hash".to_string(),
        metadata: serde_json::json!({}),
    };

    let result = extractor.extract(&raw, Uuid::new_v4()).await.unwrap();

    assert!(!result.posting.application_steps.0.is_empty());
    let link_step = result.posting.application_steps.0.iter()
        .find(|s| s.action_type == tunisia_career_scout::models::posting::ActionType::Link);
    assert!(link_step.is_some());
    assert!(link_step.unwrap().target.starts_with("https://example.com/apply/relative"));
}