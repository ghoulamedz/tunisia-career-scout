//! Repository Integration Tests
//!
//! Tests for SourceRepository and JobPostingRepository using sqlx test fixtures
//! with a test SQLite database.

mod models;
mod db;

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Import only source and posting repositories and models
use crate::db::repositories::{SourceRepository, PostingRepository};
use crate::models::{Source, SourceType, SourceConfig, SourceStatus, AuthData, AuthType};
use crate::models::{JobPosting, PostingStatus, ApplicationStep, ActionType};
use crate::models::{PaginationConfig, PaginationType, SocialPlatform, SocialTarget};

// ============================================================================
// Test Database Setup
// ============================================================================

/// Create an in-memory test database pool with migrations applied
async fn create_test_pool() -> Result<SqlitePool> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    
    // Enable WAL mode and foreign keys
    sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;
    
    // Run migrations from the migrations directory
    sqlx::migrate!("../../app/migrations").run(&pool).await?;
    
    Ok(pool)
}

/// Create a test source for use in posting tests
async fn create_test_source(pool: &SqlitePool) -> Result<Source> {
    let source = Source::new(
        "Test Source".to_string(),
        SourceType::Url,
        SourceConfig::Url {
            url: "https://example.com/jobs".to_string(),
            selector: Some(".job-listing".to_string()),
            detail_selector: Some(".job-detail".to_string()),
            pagination: None,
            headers: None,
        },
    );
    let repo = SourceRepository::new(pool.clone());
    repo.create(&source).await?;
    Ok(source)
}

/// Create a test job posting
async fn create_test_posting(pool: &SqlitePool, source_id: Uuid) -> Result<JobPosting> {
    let posting = JobPosting::new(
        source_id,
        "ext-123".to_string(),
        "Software Engineer".to_string(),
        "Test Company".to_string(),
        "We are hiring a software engineer".to_string(),
        Utc::now(),
        "abc123hash".to_string(),
    );
    let repo = PostingRepository::new(pool.clone());
    repo.create(&posting).await?;
    Ok(posting)
}

// ============================================================================
// SourceRepository Tests
// ============================================================================

#[tokio::test]
async fn test_source_repository_crud() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    // Create
    let source = Source::new(
        "Test Source".to_string(),
        SourceType::Url,
        SourceConfig::Url {
            url: "https://example.com/jobs".to_string(),
            selector: Some(".job".to_string()),
            detail_selector: None,
            pagination: None,
            headers: None,
        },
    );
    repo.create(&source).await?;

    // Get
    let fetched = repo.get(source.id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.name, "Test Source");
    assert_eq!(fetched.source_type, SourceType::Url);
    assert!(fetched.enabled);
    assert_eq!(fetched.status, SourceStatus::Active);

    // Update
    let mut updated = fetched.clone();
    updated.name = "Updated Source".to_string();
    updated.enabled = false;
    updated.status = SourceStatus::Paused;
    repo.update(&updated).await?;

    let fetched = repo.get(source.id).await?.unwrap();
    assert_eq!(fetched.name, "Updated Source");
    assert!(!fetched.enabled);
    assert_eq!(fetched.status, SourceStatus::Paused);

    // List (all)
    let all = repo.list(false).await?;
    assert_eq!(all.len(), 1);

    // List (enabled only)
    let enabled = repo.list(true).await?;
    assert_eq!(enabled.len(), 0);

    // Delete
    repo.delete(source.id).await?;
    let fetched = repo.get(source.id).await?;
    assert!(fetched.is_none());

    Ok(())
}

#[tokio::test]
async fn test_source_repository_list_by_status() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    // Create sources with different statuses
    let mut active_source = Source::new(
        "Active Source".to_string(),
        SourceType::Url,
        SourceConfig::Url { url: "https://a.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None },
    );
    active_source.status = SourceStatus::Active;
    active_source.enabled = true;
    repo.create(&active_source).await?;

    let mut paused_source = Source::new(
        "Paused Source".to_string(),
        SourceType::Rss,
        SourceConfig::Rss { url: "https://b.com/feed".to_string(), item_xpath: None, date_format: None, field_mapping: None, poll_interval_hours: None },
    );
    paused_source.status = SourceStatus::Paused;
    paused_source.enabled = true;
    repo.create(&paused_source).await?;

    let mut error_source = Source::new(
        "Error Source".to_string(),
        SourceType::Gmail,
        SourceConfig::Gmail { label_ids: vec![], query: None, max_results: None },
    );
    error_source.status = SourceStatus::Error;
    error_source.enabled = false;
    repo.create(&error_source).await?;

    // List all
    let all = repo.list(false).await?;
    assert_eq!(all.len(), 3);

    // List enabled only
    let enabled = repo.list(true).await?;
    assert_eq!(enabled.len(), 2); // active and paused are enabled

    Ok(())
}

#[tokio::test]
async fn test_source_repository_update_status() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    let source = Source::new(
        "Test".to_string(),
        SourceType::Url,
        SourceConfig::Url { url: "https://test.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None },
    );
    repo.create(&source).await?;

    // Update status to error with message
    repo.update_status(source.id, SourceStatus::Error, Some("Connection timeout".to_string())).await?;

    let fetched = repo.get(source.id).await?.unwrap();
    assert_eq!(fetched.status, SourceStatus::Error);
    assert_eq!(fetched.last_error, Some("Connection timeout".to_string()));

    // Update status to active, clear error
    repo.update_status(source.id, SourceStatus::Active, None).await?;

    let fetched = repo.get(source.id).await?.unwrap();
    assert_eq!(fetched.status, SourceStatus::Active);
    assert_eq!(fetched.last_error, None);

    Ok(())
}

#[tokio::test]
async fn test_source_repository_update_last_scan() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    let source = Source::new(
        "Test".to_string(),
        SourceType::Url,
        SourceConfig::Url { url: "https://test.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None },
    );
    repo.create(&source).await?;

    let before = Utc::now();
    repo.update_last_scan(source.id, before).await?;

    let fetched = repo.get(source.id).await?.unwrap();
    assert!(fetched.last_scan_at.is_some());
    let scanned_at = fetched.last_scan_at.unwrap();
    assert!(scanned_at >= before);
    assert!(scanned_at <= Utc::now());

    Ok(())
}

#[tokio::test]
async fn test_source_repository_with_auth_data() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    let mut source = Source::new(
        "Gmail Source".to_string(),
        SourceType::Gmail,
        SourceConfig::Gmail { label_ids: vec!["JOBS".to_string()], query: Some("is:unread".to_string()), max_results: Some(50) },
    );
    source.auth_data = Some(sqlx::types::Json(AuthData {
        auth_type: AuthType::OAuth2,
        token: Some("encrypted_token".to_string()),
        refresh_token: Some("encrypted_refresh".to_string()),
        cookies: None,
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
    }));
    repo.create(&source).await?;

    let fetched = repo.get(source.id).await?.unwrap();
    assert!(fetched.auth_data.is_some());
    let auth = fetched.auth_data.unwrap().0;
    assert_eq!(auth.auth_type, AuthType::OAuth2);
    assert_eq!(auth.token, Some("encrypted_token".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_source_repository_duplicate_name() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    let source1 = Source::new(
        "Duplicate".to_string(),
        SourceType::Url,
        SourceConfig::Url { url: "https://a.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None },
    );
    repo.create(&source1).await?;

    let source2 = Source::new(
        "Duplicate".to_string(),
        SourceType::Rss,
        SourceConfig::Rss { url: "https://b.com/feed".to_string(), item_xpath: None, date_format: None, field_mapping: None, poll_interval_hours: None },
    );
    // Should succeed - no unique constraint on name
    repo.create(&source2).await?;

    let all = repo.list(false).await?;
    assert_eq!(all.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_source_repository_source_types() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SourceRepository::new(pool.clone());

    let types = vec![
        (SourceType::Url, SourceConfig::Url { url: "https://u.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None }),
        (SourceType::Rss, SourceConfig::Rss { url: "https://r.com/feed".to_string(), item_xpath: None, date_format: None, field_mapping: None, poll_interval_hours: None }),
        (SourceType::Gmail, SourceConfig::Gmail { label_ids: vec!["INBOX".to_string()], query: None, max_results: None }),
        (SourceType::Social, SourceConfig::Social { platform: SocialPlatform::LinkedIn, target: SocialTarget::Search { query: "rust".to_string(), location: None }, cookies: None, scroll_depth: None }),
    ];

    for (i, (stype, config)) in types.into_iter().enumerate() {
        let source = Source::new(format!("Source {}", i), stype.clone(), config);
        repo.create(&source).await?;
        
        let fetched = repo.get(source.id).await?.unwrap();
        assert_eq!(fetched.source_type, stype);
    }

    let all = repo.list(false).await?;
    assert_eq!(all.len(), 4);

    Ok(())
}

// ============================================================================
// PostingRepository Tests
// ============================================================================

#[tokio::test]
async fn test_posting_repository_crud() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    // Create
    let posting = JobPosting::new(
        source.id,
        "ext-456".to_string(),
        "Senior Developer".to_string(),
        "Acme Corp".to_string(),
        "Looking for a senior developer".to_string(),
        Utc::now(),
        "def456hash".to_string(),
    );
    repo.create(&posting).await?;

    // Get by ID
    let fetched = repo.get(posting.id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.title, "Senior Developer");
    assert_eq!(fetched.company, "Acme Corp");
    assert_eq!(fetched.source_id, source.id);
    assert_eq!(fetched.source_posting_id, "ext-456");
    assert_eq!(fetched.status, PostingStatus::Active);

    // Update
    let mut updated = fetched.clone();
    updated.title = "Lead Developer".to_string();
    updated.status = PostingStatus::Saved;
    repo.update(&updated).await?;

    let fetched = repo.get(posting.id).await?.unwrap();
    assert_eq!(fetched.title, "Lead Developer");
    assert_eq!(fetched.status, PostingStatus::Saved);

    // Delete
    repo.delete(posting.id).await?;
    let fetched = repo.get(posting.id).await?;
    assert!(fetched.is_none());

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_upsert_by_source_url() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    // First insert
    let posting1 = JobPosting::new(
        source.id,
        "ext-789".to_string(),
        "Job Title".to_string(),
        "Company".to_string(),
        "Description v1".to_string(),
        Utc::now(),
        "hash1".to_string(),
    );
    let inserted = repo.upsert(&posting1).await?;
    assert!(inserted); // true = inserted

    // Upsert with same source_posting_id but different content_hash -> should update
    let mut posting2 = posting1.clone();
    posting2.description = "Description v2".to_string();
    posting2.content_hash = "hash2".to_string();
    let inserted = repo.upsert(&posting2).await?;
    assert!(!inserted); // false = updated

    let fetched = repo.get_by_source_id(source.id, "ext-789").await?.unwrap();
    assert_eq!(fetched.description, "Description v2");
    assert_eq!(fetched.content_hash, "hash2");

    // Upsert with same content_hash -> should not update (returns false)
    let posting3 = posting2.clone();
    let inserted = repo.upsert(&posting3).await?;
    assert!(!inserted);

    // Verify only one record exists
    let all = repo.list(Some(source.id), None, 10, 0).await?;
    assert_eq!(all.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_get_by_source_id() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let posting = JobPosting::new(
        source.id,
        "ext-unique".to_string(),
        "Unique Job".to_string(),
        "Company".to_string(),
        "Desc".to_string(),
        Utc::now(),
        "uniquehash".to_string(),
    );
    repo.create(&posting).await?;

    // Get by source_id + source_posting_id
    let fetched = repo.get_by_source_id(source.id, "ext-unique").await?;
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().source_posting_id, "ext-unique");

    // Non-existent
    let fetched = repo.get_by_source_id(source.id, "non-existent").await?;
    assert!(fetched.is_none());

    // Wrong source_id
    let other_source = create_test_source(&pool).await?;
    let fetched = repo.get_by_source_id(other_source.id, "ext-unique").await?;
    assert!(fetched.is_none());

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_list_by_source() -> Result<()> {
    let pool = create_test_pool().await?;
    let source1 = create_test_source(&pool).await?;
    let source2 = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    // Create postings for source1
    for i in 0..3 {
        let posting = JobPosting::new(
            source1.id,
            format!("ext-{}", i),
            format!("Job {}", i),
            "Company".to_string(),
            "Desc".to_string(),
            Utc::now(),
            format!("hash{}", i),
        );
        repo.create(&posting).await?;
    }

    // Create postings for source2
    for i in 3..5 {
        let posting = JobPosting::new(
            source2.id,
            format!("ext-{}", i),
            format!("Job {}", i),
            "Company".to_string(),
            "Desc".to_string(),
            Utc::now(),
            format!("hash{}", i),
        );
        repo.create(&posting).await?;
    }

    // List by source1
    let source1_postings = repo.list(Some(source1.id), None, 10, 0).await?;
    assert_eq!(source1_postings.len(), 3);

    // List by source2
    let source2_postings = repo.list(Some(source2.id), None, 10, 0).await?;
    assert_eq!(source2_postings.len(), 2);

    // List all (no source filter)
    let all = repo.list(None, None, 10, 0).await?;
    assert_eq!(all.len(), 5);

    // Pagination
    let page1 = repo.list(None, None, 2, 0).await?;
    assert_eq!(page1.len(), 2);
    let page2 = repo.list(None, None, 2, 2).await?;
    assert_eq!(page2.len(), 2);
    let page3 = repo.list(None, None, 2, 4).await?;
    assert_eq!(page3.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_list_by_status() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let mut active = JobPosting::new(source.id, "ext-1".to_string(), "Active".to_string(), "Co".to_string(), "D".to_string(), Utc::now(), "h1".to_string());
    active.status = PostingStatus::Active;
    repo.create(&active).await?;

    let mut expired = JobPosting::new(source.id, "ext-2".to_string(), "Expired".to_string(), "Co".to_string(), "D".to_string(), Utc::now(), "h2".to_string());
    expired.status = PostingStatus::Expired;
    repo.create(&expired).await?;

    let mut applied = JobPosting::new(source.id, "ext-3".to_string(), "Applied".to_string(), "Co".to_string(), "D".to_string(), Utc::now(), "h3".to_string());
    applied.status = PostingStatus::Applied;
    repo.create(&applied).await?;

    let active_only = repo.list(None, Some(PostingStatus::Active), 10, 0).await?;
    assert_eq!(active_only.len(), 1);
    assert_eq!(active_only[0].title, "Active");

    let expired_only = repo.list(None, Some(PostingStatus::Expired), 10, 0).await?;
    assert_eq!(expired_only.len(), 1);

    let applied_only = repo.list(None, Some(PostingStatus::Applied), 10, 0).await?;
    assert_eq!(applied_only.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_count() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    assert_eq!(repo.count(None, None).await?, 0);
    assert_eq!(repo.count(Some(source.id), None).await?, 0);

    for i in 0..5 {
        let posting = JobPosting::new(
            source.id,
            format!("ext-{}", i),
            format!("Job {}", i),
            "Co".to_string(),
            "D".to_string(),
            Utc::now(),
            format!("h{}", i),
        );
        repo.create(&posting).await?;
    }

    assert_eq!(repo.count(None, None).await?, 5);
    assert_eq!(repo.count(Some(source.id), None).await?, 5);

    // Update one to expired
    let mut first = repo.list(Some(source.id), None, 1, 0).await?.remove(0);
    first.status = PostingStatus::Expired;
    repo.update(&first).await?;

    assert_eq!(repo.count(None, Some(PostingStatus::Active)).await?, 4);
    assert_eq!(repo.count(None, Some(PostingStatus::Expired)).await?, 1);

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_search() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let p1 = JobPosting::new(source.id, "e1".to_string(), "Rust Developer".to_string(), "TechCorp".to_string(), "We need Rust skills".to_string(), Utc::now(), "h1".to_string());
    let p2 = JobPosting::new(source.id, "e2".to_string(), "Python Engineer".to_string(), "DataInc".to_string(), "Python and ML".to_string(), Utc::now(), "h2".to_string());
    let p3 = JobPosting::new(source.id, "e3".to_string(), "Frontend Dev".to_string(), "WebCo".to_string(), "React and TypeScript".to_string(), Utc::now(), "h3".to_string());
    repo.create(&p1).await?;
    repo.create(&p2).await?;
    repo.create(&p3).await?;

    // Search by title
    let results = repo.search("Rust", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Rust Developer");

    // Search by company
    let results = repo.search("TechCorp", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].company, "TechCorp");

    // Search by description
    let results = repo.search("Python", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Python Engineer");

    // Search with no matches
    let results = repo.search("NonExistent", 10).await?;
    assert_eq!(results.len(), 0);

    // Search limit
    let results = repo.search("Dev", 2).await?;
    assert_eq!(results.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_update_status() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let posting = JobPosting::new(source.id, "ext".to_string(), "Job".to_string(), "Co".to_string(), "D".to_string(), Utc::now(), "h".to_string());
    repo.create(&posting).await?;

    repo.update_status(posting.id, PostingStatus::Applied).await?;

    let fetched = repo.get(posting.id).await?.unwrap();
    assert_eq!(fetched.status, PostingStatus::Applied);

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_delete_old() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let old_date = Utc::now() - chrono::Duration::days(100);
    let recent_date = Utc::now() - chrono::Duration::days(10);

    let mut old = JobPosting::new(source.id, "old".to_string(), "Old Job".to_string(), "Co".to_string(), "D".to_string(), old_date, "hold".to_string());
    old.scraped_at = old_date;
    old.status = PostingStatus::Active;
    repo.create(&old).await?;

    let mut recent = JobPosting::new(source.id, "recent".to_string(), "Recent Job".to_string(), "Co".to_string(), "D".to_string(), recent_date, "hnew".to_string());
    recent.scraped_at = recent_date;
    repo.create(&recent).await?;

    // Saved postings should not be deleted
    let mut saved = JobPosting::new(source.id, "saved".to_string(), "Saved Job".to_string(), "Co".to_string(), "D".to_string(), old_date, "hsaved".to_string());
    saved.scraped_at = old_date;
    saved.status = PostingStatus::Saved;
    repo.create(&saved).await?;

    let cutoff = Utc::now() - chrono::Duration::days(30);
    let deleted = repo.delete_old(cutoff).await?;

    assert_eq!(deleted, 1); // Only the old active one

    let remaining = repo.list(None, None, 10, 0).await?;
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().any(|p| p.source_posting_id == "recent"));
    assert!(remaining.iter().any(|p| p.source_posting_id == "saved"));
    assert!(!remaining.iter().any(|p| p.source_posting_id == "old"));

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_unique_constraint() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let p1 = JobPosting::new(source.id, "ext-dup".to_string(), "Job 1".to_string(), "Co".to_string(), "D1".to_string(), Utc::now(), "h1".to_string());
    repo.create(&p1).await?;

    // Duplicate source_id + source_posting_id should fail on create
    let p2 = JobPosting::new(source.id, "ext-dup".to_string(), "Job 2".to_string(), "Co".to_string(), "D2".to_string(), Utc::now(), "h2".to_string());
    let result = repo.create(&p2).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_all_fields() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let mut posting = JobPosting::new(
        source.id,
        "ext-full".to_string(),
        "Full Job".to_string(),
        "Full Company".to_string(),
        "Full description".to_string(),
        Utc::now(),
        "fullhash".to_string(),
    );

    // Set all optional fields
    posting.summary = Some("AI summary".to_string());
    posting.skills.0 = vec!["Rust".to_string(), "SQL".to_string()];
    posting.experience_required = Some("5 years".to_string());
    posting.education_required = Some("Master's".to_string());
    posting.languages.0 = vec!["English".to_string(), "French".to_string()];
    posting.location = Some("Tunis, Tunisia".to_string());
    posting.remote_policy = Some("remote".to_string());
    posting.job_type = Some("full-time".to_string());
    posting.salary_range = Some("50000-80000 TND".to_string());
    posting.deadline = Some(Utc::now() + chrono::Duration::days(30));
    posting.application_steps.0 = vec![
        ApplicationStep {
            order: 1,
            description: "Apply online".to_string(),
            action_type: ActionType::Link,
            target: "https://example.com/apply".to_string(),
            deadline: None,
            required: true,
            completed: false,
        }
    ];
    posting.application_email = Some("jobs@example.com".to_string());
    posting.application_url = Some("https://example.com/apply".to_string());
    posting.contact_person = Some("John Doe".to_string());
    posting.contact_phone = Some("+216 12 345 678".to_string());
    posting.posted_date = Some(Utc::now() - chrono::Duration::days(5));
    posting.raw_html = Some("<html>full</html>".to_string());

    repo.create(&posting).await?;

    let fetched = repo.get(posting.id).await?.unwrap();
    assert_eq!(fetched.summary, Some("AI summary".to_string()));
    assert_eq!(fetched.skills.0, vec!["Rust", "SQL"]);
    assert_eq!(fetched.experience_required, Some("5 years".to_string()));
    assert_eq!(fetched.education_required, Some("Master's".to_string()));
    assert_eq!(fetched.languages.0, vec!["English", "French"]);
    assert_eq!(fetched.location, Some("Tunis, Tunisia".to_string()));
    assert_eq!(fetched.remote_policy, Some("remote".to_string()));
    assert_eq!(fetched.job_type, Some("full-time".to_string()));
    assert_eq!(fetched.salary_range, Some("50000-80000 TND".to_string()));
    assert!(fetched.deadline.is_some());
    assert_eq!(fetched.application_steps.0.len(), 1);
    assert_eq!(fetched.application_email, Some("jobs@example.com".to_string()));
    assert_eq!(fetched.application_url, Some("https://example.com/apply".to_string()));
    assert_eq!(fetched.contact_person, Some("John Doe".to_string()));
    assert_eq!(fetched.contact_phone, Some("+216 12 345 678".to_string()));
    assert!(fetched.posted_date.is_some());
    assert_eq!(fetched.raw_html, Some("<html>full</html>".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_posting_repository_ordering() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    // Create postings with different scraped_at times
    let base_time = Utc::now();
    for i in 0..5 {
        let mut posting = JobPosting::new(
            source.id,
            format!("ext-{}", i),
            format!("Job {}", i),
            "Co".to_string(),
            "D".to_string(),
            base_time - chrono::Duration::days(i as i64),
            format!("h{}", i),
        );
        posting.scraped_at = base_time - chrono::Duration::days(i as i64);
        repo.create(&posting).await?;
    }

    // Default ordering should be by scraped_at DESC (most recent first)
    let all = repo.list(None, None, 10, 0).await?;
    assert_eq!(all.len(), 5);
    assert_eq!(all[0].source_posting_id, "ext-0"); // most recent
    assert_eq!(all[4].source_posting_id, "ext-4"); // oldest

    Ok(())
}

// ============================================================================
// Cross-Repository Integration Tests
// ============================================================================

#[tokio::test]
async fn test_source_posting_integration() -> Result<()> {
    let pool = create_test_pool().await?;

    let source_repo = SourceRepository::new(pool.clone());
    let posting_repo = PostingRepository::new(pool.clone());

    // 1. Create source
    let source = Source::new(
        "Integration Source".to_string(),
        SourceType::Url,
        SourceConfig::Url { url: "https://jobs.example.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None },
    );
    source_repo.create(&source).await?;

    // 2. Create postings for this source
    let p1 = JobPosting::new(source.id, "ext-1".to_string(), "Rust Engineer".to_string(), "TechCo".to_string(), "Rust backend role".to_string(), Utc::now(), "hash1".to_string());
    let p2 = JobPosting::new(source.id, "ext-2".to_string(), "Python Developer".to_string(), "DataCo".to_string(), "Python ML role".to_string(), Utc::now(), "hash2".to_string());
    posting_repo.create(&p1).await?;
    posting_repo.create(&p2).await?;

    // 3. List postings by source
    let postings = posting_repo.list(Some(source.id), None, 10, 0).await?;
    assert_eq!(postings.len(), 2);

    // 4. Update source status
    source_repo.update_status(source.id, SourceStatus::Active, None).await?;
    let updated_source = source_repo.get(source.id).await?.unwrap();
    assert_eq!(updated_source.status, SourceStatus::Active);

    // 5. Update posting status
    posting_repo.update_status(p1.id, PostingStatus::Saved).await?;
    let updated_posting = posting_repo.get(p1.id).await?.unwrap();
    assert_eq!(updated_posting.status, PostingStatus::Saved);

    // 6. Delete source should cascade (if FK constraints) or leave orphan postings
    // Current implementation doesn't have cascade delete, so postings remain
    source_repo.delete(source.id).await?;
    let postings_after = posting_repo.list(Some(source.id), None, 10, 0).await?;
    assert_eq!(postings_after.len(), 2); // Postings still exist

    Ok(())
}

#[tokio::test]
async fn test_upsert_idempotency() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    let posting = JobPosting::new(
        source.id,
        "ext-idempotent".to_string(),
        "Idempotent Job".to_string(),
        "Co".to_string(),
        "Description".to_string(),
        Utc::now(),
        "samehash".to_string(),
    );

    // Multiple upserts with identical data should not create duplicates
    for _ in 0..5 {
        let result = repo.upsert(&posting).await?;
        // First returns true (inserted), rest return false (updated/no-op)
    }

    let count = repo.count(Some(source.id), None).await?;
    assert_eq!(count, 1);

    // Verify the record is correct
    let fetched = repo.get_by_source_id(source.id, "ext-idempotent").await?.unwrap();
    assert_eq!(fetched.title, "Idempotent Job");
    assert_eq!(fetched.content_hash, "samehash");

    Ok(())
}

#[tokio::test]
async fn test_concurrent_upsert_simulation() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    // Simulate concurrent upserts by rapidly calling upsert
    // with slightly different content hashes
    for i in 0..10 {
        let mut posting = JobPosting::new(
            source.id,
            "ext-concurrent".to_string(),
            "Concurrent Job".to_string(),
            "Co".to_string(),
            format!("Description {}", i),
            Utc::now(),
            format!("hash{}", i),
        );
        let _ = repo.upsert(&posting).await?;
    }

    // Should only have one record (last write wins for content_hash changes)
    let count = repo.count(Some(source.id), None).await?;
    assert_eq!(count, 1);

    let fetched = repo.get_by_source_id(source.id, "ext-concurrent").await?.unwrap();
    // Last upsert should have won
    assert_eq!(fetched.content_hash, "hash9");
    assert_eq!(fetched.description, "Description 9");

    Ok(())
}