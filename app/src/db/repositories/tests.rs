//! Repository Integration Tests
//!
//! Comprehensive tests for all 6 repository modules using sqlx test fixtures
//! with a test SQLite database.

use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc, TimeZone};
use serde_json::json;

// Import all repositories and models
use super::{SourceRepository, PostingRepository, CVRepository, MatchRepository,
    SettingsRepository, ScanLogRepository};
use crate::models::source::{Source, SourceType, SourceConfig, SourceStatus, AuthData, AuthType};
use crate::models::posting::{JobPosting, PostingStatus, ApplicationStep, ActionType};
use crate::models::cv::{CVProfile, CVData, CVTemplate, CVTheme};
use crate::models::match_result::{MatchResult, MatchReason, MatchCategory};
use crate::models::settings::{Settings, LLMSettings, PushSettings, SyncSettings, LLMProvider, SyncMethod};
use crate::models::scan_log::{ScanLog, ScanStatus};

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
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}

/// Create a test source for use in posting/match tests
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

/// Create a test CV profile
async fn create_test_cv_profile(pool: &SqlitePool) -> Result<CVProfile> {
    let cv_data = CVData::default();
    let profile = CVProfile::new("Test CV".to_string(), cv_data);
    let repo = CVRepository::new(pool.clone());
    repo.create(&profile).await?;
    Ok(profile)
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
    source.auth_data = Some(Json(AuthData {
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

// ============================================================================
// CVRepository Tests
// ============================================================================

#[tokio::test]
async fn test_cv_repository_crud() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = CVRepository::new(pool.clone());

    // Create
    let cv_data = CVData {
        full_name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        phone: Some("+1234567890".to_string()),
        location: Some("Tunis, Tunisia".to_string()),
        linkedin: Some("linkedin.com/in/johndoe".to_string()),
        github: Some("github.com/johndoe".to_string()),
        portfolio: Some("johndoe.dev".to_string()),
        summary: Some("Experienced developer".to_string()),
        ..Default::default()
    };
    let profile = CVProfile::new("My CV".to_string(), cv_data);
    repo.create(&profile).await?;

    // Get
    let fetched = repo.get(profile.id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.name, "My CV");
    assert_eq!(fetched.data.0.full_name, "John Doe");
    assert_eq!(fetched.data.0.email, "john@example.com");
    assert_eq!(fetched.template, CVTemplate::Modern);
    assert_eq!(fetched.theme, CVTheme::Auto);
    assert!(!fetched.is_default);

    // Update
    let mut updated = fetched.clone();
    updated.name = "Updated CV".to_string();
    updated.data.0.full_name = "Jane Doe".to_string();
    updated.template = CVTemplate::Classic;
    updated.theme = CVTheme::Dark;
    repo.update(&updated).await?;

    let fetched = repo.get(profile.id).await?.unwrap();
    assert_eq!(fetched.name, "Updated CV");
    assert_eq!(fetched.data.0.full_name, "Jane Doe");
    assert_eq!(fetched.template, CVTemplate::Classic);
    assert_eq!(fetched.theme, CVTheme::Dark);

    // Delete
    repo.delete(profile.id).await?;
    let fetched = repo.get(profile.id).await?;
    assert!(fetched.is_none());

    Ok(())
}

#[tokio::test]
async fn test_cv_repository_get_default() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = CVRepository::new(pool.clone());

    // No default initially
    let default = repo.get_default().await?;
    assert!(default.is_none());

    // Create non-default
    let profile1 = CVProfile::new("CV 1".to_string(), CVData::default());
    repo.create(&profile1).await?;

    let default = repo.get_default().await?;
    assert!(default.is_none());

    // Set as default
    repo.set_default(profile1.id).await?;

    let default = repo.get_default().await?;
    assert!(default.is_some());
    assert_eq!(default.unwrap().id, profile1.id);

    // Create another and set as default
    let profile2 = CVProfile::new("CV 2".to_string(), CVData::default());
    repo.create(&profile2).await?;
    repo.set_default(profile2.id).await?;

    let default = repo.get_default().await?;
    assert!(default.is_some());
    assert_eq!(default.unwrap().id, profile2.id);

    // Verify profile1 is no longer default
    let p1 = repo.get(profile1.id).await?.unwrap();
    assert!(!p1.is_default);

    let p2 = repo.get(profile2.id).await?.unwrap();
    assert!(p2.is_default);

    Ok(())
}

#[tokio::test]
async fn test_cv_repository_list() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = CVRepository::new(pool.clone());

    let p1 = CVProfile::new("CV 1".to_string(), CVData::default());
    let p2 = CVProfile::new("CV 2".to_string(), CVData::default());
    let p3 = CVProfile::new("CV 3".to_string(), CVData::default());

    repo.create(&p1).await?;
    repo.create(&p2).await?;
    repo.create(&p3).await?;

    // Set p2 as default
    repo.set_default(p2.id).await?;

    let all = repo.list().await?;
    assert_eq!(all.len(), 3);

    // Default should be first
    assert_eq!(all[0].id, p2.id);
    assert!(all[0].is_default);

    Ok(())
}

#[tokio::test]
async fn test_cv_repository_update_sets_default() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = CVRepository::new(pool.clone());

    let p1 = CVProfile::new("CV 1".to_string(), CVData::default());
    let p2 = CVProfile::new("CV 2".to_string(), CVData::default());
    repo.create(&p1).await?;
    repo.create(&p2).await?;

    repo.set_default(p1.id).await?;

    // Update p2 to be default via update()
    let mut p2_updated = repo.get(p2.id).await?.unwrap();
    p2_updated.is_default = true;
    repo.update(&p2_updated).await?;

    let p1_check = repo.get(p1.id).await?.unwrap();
    assert!(!p1_check.is_default);

    let p2_check = repo.get(p2.id).await?.unwrap();
    assert!(p2_check.is_default);

    Ok(())
}

#[tokio::test]
async fn test_cv_repository_complex_data() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = CVRepository::new(pool.clone());

    let cv_data = CVData {
        full_name: "Test User".to_string(),
        email: "test@test.com".to_string(),
        experience: vec![
            crate::models::cv::Experience {
                id: Uuid::new_v4(),
                title: "Senior Engineer".to_string(),
                company: "Big Tech".to_string(),
                location: Some("Remote".to_string()),
                start_date: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
                end_date: None,
                current: true,
                description: "Leading backend team".to_string(),
                skills: vec!["Rust".to_string(), "PostgreSQL".to_string()],
            }
        ],
        education: vec![
            crate::models::cv::Education {
                id: Uuid::new_v4(),
                institution: "University".to_string(),
                degree: "MSc".to_string(),
                field_of_study: Some("Computer Science".to_string()),
                location: None,
                start_date: Utc.with_ymd_and_hms(2018, 9, 1, 0, 0, 0).unwrap(),
                end_date: Some(Utc.with_ymd_and_hms(2020, 6, 1, 0, 0, 0).unwrap()),
                current: false,
                description: None,
                grade: Some("A".to_string()),
            }
        ],
        skills: vec![
            crate::models::cv::Skill {
                id: Uuid::new_v4(),
                name: "Rust".to_string(),
                category: Some("technical".to_string()),
                level: Some(crate::models::cv::SkillLevel::Expert),
                years: Some(5),
            }
        ],
        languages: vec![
            crate::models::cv::Language {
                id: Uuid::new_v4(),
                name: "English".to_string(),
                proficiency: crate::models::cv::LanguageProficiency::Fluent,
            }
        ],
        ..Default::default()
    };

    let profile = CVProfile::new("Complex CV".to_string(), cv_data);
    repo.create(&profile).await?;

    let fetched = repo.get(profile.id).await?.unwrap();
    assert_eq!(fetched.data.0.experience.len(), 1);
    assert_eq!(fetched.data.0.education.len(), 1);
    assert_eq!(fetched.data.0.skills.len(), 1);
    assert_eq!(fetched.data.0.languages.len(), 1);
    assert_eq!(fetched.data.0.experience[0].skills, vec!["Rust", "PostgreSQL"]);

    Ok(())
}

// ============================================================================
// MatchRepository Tests
// ============================================================================

#[tokio::test]
async fn test_match_repository_crud() -> Result<()> {
    let pool = create_test_pool().await?;
    let cv_profile = create_test_cv_profile(&pool).await?;
    let source = create_test_source(&pool).await?;
    let posting = create_test_posting(&pool, source.id).await?;
    let repo = MatchRepository::new(pool.clone());

    // Create
    let match_result = MatchResult::new(
        cv_profile.id,
        posting.id,
        85.5,
        vec!["Rust".to_string(), "SQL".to_string()],
        vec!["Python".to_string()],
        vec![
            MatchReason {
                category: MatchCategory::Skills,
                description: "Strong Rust match".to_string(),
                weight: 0.6,
                matched: true,
            },
            MatchReason {
                category: MatchCategory::Experience,
                description: "5 years experience".to_string(),
                weight: 0.4,
                matched: true,
            },
        ],
    );
    repo.create(&match_result).await?;

    // Get
    let fetched = repo.get(cv_profile.id, posting.id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.cv_profile_id, cv_profile.id);
    assert_eq!(fetched.posting_id, posting.id);
    assert_eq!(fetched.score, 85.5);
    assert_eq!(fetched.matched_skills.0, vec!["Rust", "SQL"]);
    assert_eq!(fetched.missing_skills.0, vec!["Python"]);
    assert_eq!(fetched.reasons.0.len(), 2);

    // Update via upsert (since no direct update method)
    let mut updated = fetched.clone();
    updated.score = 90.0;
    updated.matched_skills.0.push("Python".to_string());
    updated.missing_skills.0.clear();
    repo.upsert(&updated).await?;

    let fetched = repo.get(cv_profile.id, posting.id).await?.unwrap();
    assert_eq!(fetched.score, 90.0);
    assert_eq!(fetched.matched_skills.0.len(), 3);
    assert_eq!(fetched.missing_skills.0.len(), 0);

    // Delete for CV
    repo.delete_for_cv(cv_profile.id).await?;
    let fetched = repo.get(cv_profile.id, posting.id).await?;
    assert!(fetched.is_none());

    Ok(())
}

#[tokio::test]
async fn test_match_repository_upsert() -> Result<()> {
    let pool = create_test_pool().await?;
    let cv_profile = create_test_cv_profile(&pool).await?;
    let source = create_test_source(&pool).await?;
    let posting = create_test_posting(&pool, source.id).await?;
    let repo = MatchRepository::new(pool.clone());

    // First upsert (insert)
    let m1 = MatchResult::new(cv_profile.id, posting.id, 70.0, vec!["A".to_string()], vec!["B".to_string()], vec![]);
    repo.upsert(&m1).await?;

    // Second upsert (update) - same cv_profile_id + posting_id
    let m2 = MatchResult::new(cv_profile.id, posting.id, 80.0, vec!["A".to_string(), "B".to_string()], vec![], vec![]);
    repo.upsert(&m2).await?;

    let fetched = repo.get(cv_profile.id, posting.id).await?.unwrap();
    assert_eq!(fetched.score, 80.0);
    assert_eq!(fetched.matched_skills.0.len(), 2);

    // Verify only one record
    let all = repo.list_for_posting(posting.id).await?;
    assert_eq!(all.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_match_repository_list_for_cv() -> Result<()> {
    let pool = create_test_pool().await?;
    let cv_profile = create_test_cv_profile(&pool).await?;
    let source = create_test_source(&pool).await?;
    let repo = MatchRepository::new(pool.clone());

    // Create multiple postings and matches
    for i in 0..5 {
        let posting = create_test_posting(&pool, source.id).await?;
        let match_result = MatchResult::new(
            cv_profile.id,
            posting.id,
            50.0 + i as f32 * 10.0, // 50, 60, 70, 80, 90
            vec![],
            vec![],
            vec![],
        );
        repo.create(&match_result).await?;
    }

    // List all for CV (ordered by score DESC)
    let matches = repo.list_for_cv(cv_profile.id, None, 10, 0).await?;
    assert_eq!(matches.len(), 5);
    assert_eq!(matches[0].score, 90.0);
    assert_eq!(matches[4].score, 50.0);

    // List with min_score filter
    let matches = repo.list_for_cv(cv_profile.id, Some(75.0), 10, 0).await?;
    assert_eq!(matches.len(), 2); // 80 and 90
    assert!(matches.iter().all(|m| m.score >= 75.0));

    // Pagination
    let page1 = repo.list_for_cv(cv_profile.id, None, 2, 0).await?;
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].score, 90.0);
    assert_eq!(page1[1].score, 80.0);

    let page2 = repo.list_for_cv(cv_profile.id, None, 2, 2).await?;
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].score, 70.0);
    assert_eq!(page2[1].score, 60.0);

    Ok(())
}

#[tokio::test]
async fn test_match_repository_list_for_posting() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let posting = create_test_posting(&pool, source.id).await?;
    let repo = MatchRepository::new(pool.clone());

    // Create multiple CVs and matches for the same posting
    for i in 0..3 {
        let cv = create_test_cv_profile(&pool).await?;
        let match_result = MatchResult::new(
            cv.id,
            posting.id,
            60.0 + i as f32 * 10.0,
            vec![],
            vec![],
            vec![],
        );
        repo.create(&match_result).await?;
    }

    let matches = repo.list_for_posting(posting.id).await?;
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].score, 80.0);
    assert_eq!(matches[2].score, 60.0);

    Ok(())
}

#[tokio::test]
async fn test_match_repository_delete_old() -> Result<()> {
    let pool = create_test_pool().await?;
    let cv_profile = create_test_cv_profile(&pool).await?;
    let source = create_test_source(&pool).await?;
    let repo = MatchRepository::new(pool.clone());

    let old_date = Utc::now() - chrono::Duration::days(100);
    let _recent_date = Utc::now() - chrono::Duration::days(10);

    // Old match
    let mut old_posting = create_test_posting(&pool, source.id).await?;
    old_posting.scraped_at = old_date;
    // Can't easily change created_at on match_result, so we test with direct SQL
    // For now, just verify the method exists and runs
    let posting = create_test_posting(&pool, source.id).await?;
    let match_result = MatchResult::new(cv_profile.id, posting.id, 80.0, vec![], vec![], vec![]);
    repo.create(&match_result).await?;

    let cutoff = Utc::now() - chrono::Duration::days(30);
    let deleted = repo.delete_old(cutoff).await?;
    // The match was created now, so shouldn't be deleted
    assert_eq!(deleted, 0);

    Ok(())
}

#[tokio::test]
async fn test_match_repository_unique_constraint() -> Result<()> {
    let pool = create_test_pool().await?;
    let cv_profile = create_test_cv_profile(&pool).await?;
    let source = create_test_source(&pool).await?;
    let posting = create_test_posting(&pool, source.id).await?;
    let repo = MatchRepository::new(pool.clone());

    let m1 = MatchResult::new(cv_profile.id, posting.id, 70.0, vec![], vec![], vec![]);
    repo.create(&m1).await?;

    // Duplicate should fail on create
    let m2 = MatchResult::new(cv_profile.id, posting.id, 80.0, vec![], vec![], vec![]);
    let result = repo.create(&m2).await;
    assert!(result.is_err());

    Ok(())
}

// ============================================================================
// SettingsRepository Tests
// ============================================================================

#[tokio::test]
async fn test_settings_repository_get_set() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SettingsRepository::new(pool.clone());

    // Get non-existent
    let value = repo.get("nonexistent").await?;
    assert!(value.is_none());

    // Set and get
    repo.set("test_key", json!("test_value")).await?;
    let value = repo.get("test_key").await?;
    assert_eq!(value, Some(json!("test_value")));

    // Update
    repo.set("test_key", json!("updated_value")).await?;
    let value = repo.get("test_key").await?;
    assert_eq!(value, Some(json!("updated_value")));

    // Set complex JSON
    let complex = json!({
        "nested": {
            "array": [1, 2, 3],
            "bool": true
        },
        "string": "value"
    });
    repo.set("complex", complex.clone()).await?;
    let value = repo.get("complex").await?;
    assert_eq!(value, Some(complex));

    Ok(())
}

#[tokio::test]
async fn test_settings_repository_get_all() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SettingsRepository::new(pool.clone());

    // Default settings when empty
    let settings = repo.get_all().await?;
    assert_eq!(settings.theme, "auto");
    assert_eq!(settings.language, "en");
    assert_eq!(settings.scan_concurrency, 3);
    assert_eq!(settings.retention_days, 90);

    // Set some values
    repo.set("theme", json!("dark")).await?;
    repo.set("language", json!("fr")).await?;
    repo.set("scan_concurrency", json!(5)).await?;
    repo.set("retention_days", json!(30)).await?;

    let settings = repo.get_all().await?;
    assert_eq!(settings.theme, "dark");
    assert_eq!(settings.language, "fr");
    assert_eq!(settings.scan_concurrency, 5);
    assert_eq!(settings.retention_days, 30);

    // LLM settings
    let llm_settings = json!({
        "provider": "ollama",
        "model": "llama3.2",
        "base_url": "http://localhost:11434",
        "api_key": null,
        "temperature": 0.5,
        "max_tokens": 2048,
        "timeout_seconds": 30
    });
    repo.set("llm", llm_settings.clone()).await?;

    let settings = repo.get_all().await?;
    assert_eq!(settings.llm.provider, LLMProvider::Ollama);
    assert_eq!(settings.llm.model, "llama3.2");
    assert_eq!(settings.llm.temperature, 0.5);

    Ok(())
}

#[tokio::test]
async fn test_settings_repository_save_all() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SettingsRepository::new(pool.clone());

    let mut settings = Settings::default();
    settings.theme = "light".to_string();
    settings.language = "ar".to_string();
    settings.scan_concurrency = 2;
    settings.retention_days = 60;
    settings.llm.model = "custom-model".to_string();
    settings.llm.temperature = 0.7;

    repo.save_all(&settings).await?;

    let loaded = repo.get_all().await?;
    assert_eq!(loaded.theme, "light");
    assert_eq!(loaded.language, "ar");
    assert_eq!(loaded.scan_concurrency, 2);
    assert_eq!(loaded.retention_days, 60);
    assert_eq!(loaded.llm.model, "custom-model");
    assert_eq!(loaded.llm.temperature, 0.7);

    Ok(())
}

#[tokio::test]
async fn test_settings_repository_reset() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SettingsRepository::new(pool.clone());

    repo.set("theme", json!("dark")).await?;
    repo.set("custom_key", json!("custom_value")).await?;

    let before = repo.get_all().await?;
    assert_eq!(before.theme, "dark");

    repo.reset().await?;

    let after = repo.get_all().await?;
    assert_eq!(after.theme, "auto"); // back to default
    assert!(repo.get("custom_key").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_settings_repository_push_settings() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SettingsRepository::new(pool.clone());

    let push_settings = json!({
        "enabled": true,
        "fcm_token": "test-token",
        "min_match_score": 80.0,
        "quiet_hours_start": "23:00",
        "quiet_hours_end": "07:00"
    });
    repo.set("push", push_settings).await?;

    let settings = repo.get_all().await?;
    assert!(settings.push.enabled);
    assert_eq!(settings.push.fcm_token, Some("test-token".to_string()));
    assert_eq!(settings.push.min_match_score, 80.0);
    assert_eq!(settings.push.quiet_hours_start, Some("23:00".to_string()));
    assert_eq!(settings.push.quiet_hours_end, Some("07:00".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_settings_repository_sync_settings() -> Result<()> {
    let pool = create_test_pool().await?;
    let repo = SettingsRepository::new(pool.clone());

    let sync_settings = json!({
        "enabled": true,
        "method": "local-network",
        "peer_id": "peer-123",
        "last_sync": "2024-01-15T10:30:00Z"
    });
    repo.set("sync", sync_settings).await?;

    let settings = repo.get_all().await?;
    assert!(settings.sync.enabled);
    assert_eq!(settings.sync.method, SyncMethod::LocalNetwork);
    assert_eq!(settings.sync.peer_id, Some("peer-123".to_string()));
    assert!(settings.sync.last_sync.is_some());

    Ok(())
}

// ============================================================================
// ScanLogRepository Tests
// ============================================================================

#[tokio::test]
async fn test_scan_log_repository_crud() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = ScanLogRepository::new(pool.clone());

    // Create
    let mut log = ScanLog::new(source.id);
    repo.create(&log).await?;

    // Get
    let fetched = repo.get(log.id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.source_id, source.id);
    assert_eq!(fetched.status, ScanStatus::Running);
    assert_eq!(fetched.postings_found, 0);
    assert!(fetched.completed_at.is_none());
    assert!(fetched.duration_ms.is_none());

    // Update to completed
    log.complete(ScanStatus::Completed, 10, 5, 2, None, Some("sample response".to_string()));
    repo.update(&log).await?;

    let fetched = repo.get(log.id).await?.unwrap();
    assert_eq!(fetched.status, ScanStatus::Completed);
    assert_eq!(fetched.postings_found, 10);
    assert_eq!(fetched.postings_new, 5);
    assert_eq!(fetched.postings_updated, 2);
    assert!(fetched.completed_at.is_some());
    assert!(fetched.duration_ms.is_some());
    assert_eq!(fetched.raw_response_sample, Some("sample response".to_string()));

    // Update to failed
    let mut log2 = ScanLog::new(source.id);
    repo.create(&log2).await?;
    log2.complete(ScanStatus::Failed, 0, 0, 0, Some("Network error".to_string()), None);
    repo.update(&log2).await?;

    let fetched = repo.get(log2.id).await?.unwrap();
    assert_eq!(fetched.status, ScanStatus::Failed);
    assert_eq!(fetched.error_message, Some("Network error".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_scan_log_repository_list_for_source() -> Result<()> {
    let pool = create_test_pool().await?;
    let source1 = create_test_source(&pool).await?;
    let source2 = create_test_source(&pool).await?;
    let repo = ScanLogRepository::new(pool.clone());

    // Create logs for source1
    for i in 0..3 {
        let mut log = ScanLog::new(source1.id);
        log.complete(ScanStatus::Completed, i, i, 0, None, None);
        repo.create(&log).await?;
        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Create logs for source2
    for i in 0..2 {
        let mut log = ScanLog::new(source2.id);
        log.complete(ScanStatus::Completed, i, i, 0, None, None);
        repo.create(&log).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // List for source1
    let logs1 = repo.list_for_source(source1.id, 10).await?;
    assert_eq!(logs1.len(), 3);
    // Should be ordered by started_at DESC (most recent first)
    assert!(logs1[0].started_at >= logs1[1].started_at);
    assert!(logs1[1].started_at >= logs1[2].started_at);

    // List for source2
    let logs2 = repo.list_for_source(source2.id, 10).await?;
    assert_eq!(logs2.len(), 2);

    // Limit
    let limited = repo.list_for_source(source1.id, 2).await?;
    assert_eq!(limited.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_scan_log_repository_list_recent() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = ScanLogRepository::new(pool.clone());

    for i in 0..5 {
        let mut log = ScanLog::new(source.id);
        log.complete(ScanStatus::Completed, i, i, 0, None, None);
        repo.create(&log).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let recent = repo.list_recent(3).await?;
    assert_eq!(recent.len(), 3);
    // Most recent first
    assert!(recent[0].started_at >= recent[1].started_at);

    Ok(())
}

#[tokio::test]
async fn test_scan_log_repository_delete_old() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = ScanLogRepository::new(pool.clone());

    let old_date = Utc::now() - chrono::Duration::days(100);
    let recent_date = Utc::now() - chrono::Duration::days(10);

    // We can't easily set started_at on create, so we'll insert directly
    let old_log = ScanLog {
        id: Uuid::new_v4(),
        source_id: source.id,
        status: ScanStatus::Completed,
        postings_found: 5,
        postings_new: 3,
        postings_updated: 2,
        error_message: None,
        started_at: old_date,
        completed_at: Some(old_date + chrono::Duration::minutes(5)),
        duration_ms: Some(300000),
        raw_response_sample: None,
    };
    repo.create(&old_log).await?;

    let recent_log = ScanLog {
        id: Uuid::new_v4(),
        source_id: source.id,
        status: ScanStatus::Completed,
        postings_found: 10,
        postings_new: 5,
        postings_updated: 3,
        error_message: None,
        started_at: recent_date,
        completed_at: Some(recent_date + chrono::Duration::minutes(3)),
        duration_ms: Some(180000),
        raw_response_sample: None,
    };
    repo.create(&recent_log).await?;

    let cutoff = Utc::now() - chrono::Duration::days(30);
    let deleted = repo.delete_old(cutoff).await?;

    assert_eq!(deleted, 1);

    let remaining = repo.list_for_source(source.id, 10).await?;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, recent_log.id);

    Ok(())
}

#[tokio::test]
async fn test_scan_log_repository_partial_status() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = ScanLogRepository::new(pool.clone());

    let mut log = ScanLog::new(source.id);
    log.complete(ScanStatus::Partial, 100, 50, 10, Some("Some items failed".to_string()), Some("partial".to_string()));
    repo.create(&log).await?;

    let fetched = repo.get(log.id).await?.unwrap();
    assert_eq!(fetched.status, ScanStatus::Partial);
    assert_eq!(fetched.postings_found, 100);
    assert_eq!(fetched.postings_new, 50);
    assert_eq!(fetched.postings_updated, 10);

    Ok(())
}

// ============================================================================
// Cross-Repository Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_workflow_source_posting_match() -> Result<()> {
    let pool = create_test_pool().await?;

    let source_repo = SourceRepository::new(pool.clone());
    let posting_repo = PostingRepository::new(pool.clone());
    let cv_repo = CVRepository::new(pool.clone());
    let match_repo = MatchRepository::new(pool.clone());

    // 1. Create source
    let source = Source::new(
        "Integration Source".to_string(),
        SourceType::Url,
        SourceConfig::Url { url: "https://jobs.example.com".to_string(), selector: None, detail_selector: None, pagination: None, headers: None },
    );
    source_repo.create(&source).await?;

    // 2. Create postings
    let p1 = JobPosting::new(source.id, "ext-1".to_string(), "Rust Engineer".to_string(), "TechCo".to_string(), "Rust backend role".to_string(), Utc::now(), "hash1".to_string());
    let p2 = JobPosting::new(source.id, "ext-2".to_string(), "Python Developer".to_string(), "DataCo".to_string(), "Python ML role".to_string(), Utc::now(), "hash2".to_string());
    posting_repo.create(&p1).await?;
    posting_repo.create(&p2).await?;

    // 3. Create CV
    let cv_data = CVData {
        full_name: "Dev User".to_string(),
        email: "dev@example.com".to_string(),
        skills: vec![
            crate::models::cv::Skill { id: Uuid::new_v4(), name: "Rust".to_string(), category: Some("tech".to_string()), level: Some(crate::models::cv::SkillLevel::Expert), years: Some(5) },
            crate::models::cv::Skill { id: Uuid::new_v4(), name: "Python".to_string(), category: Some("tech".to_string()), level: Some(crate::models::cv::SkillLevel::Advanced), years: Some(3) },
        ],
        ..Default::default()
    };
    let cv = CVProfile::new("My Profile".to_string(), cv_data);
    cv_repo.create(&cv).await?;

    // 4. Create matches
    let m1 = MatchResult::new(cv.id, p1.id, 95.0, vec!["Rust".to_string()], vec![], vec![]);
    let m2 = MatchResult::new(cv.id, p2.id, 80.0, vec!["Python".to_string()], vec![], vec![]);
    match_repo.create(&m1).await?;
    match_repo.create(&m2).await?;

    // 5. Query matches for CV
    let matches = match_repo.list_for_cv(cv.id, None, 10, 0).await?;
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].score, 95.0); // Rust match higher
    assert_eq!(matches[1].score, 80.0);

    // 6. Query postings by source
    let source_postings = posting_repo.list(Some(source.id), None, 10, 0).await?;
    assert_eq!(source_postings.len(), 2);

    // 7. Update posting status
    posting_repo.update_status(p1.id, PostingStatus::Applied).await?;
    let updated = posting_repo.get(p1.id).await?.unwrap();
    assert_eq!(updated.status, PostingStatus::Applied);

    // 8. Verify cascade delete (source -> postings -> matches)
    source_repo.delete(source.id).await?;
    let postings_after = posting_repo.list(Some(source.id), None, 10, 0).await?;
    assert_eq!(postings_after.len(), 0); // CASCADE DELETE
    let matches_after = match_repo.list_for_cv(cv.id, None, 10, 0).await?;
    assert_eq!(matches_after.len(), 0); // CASCADE DELETE

    Ok(())
}

#[tokio::test]
async fn test_concurrent_access_simulation() -> Result<()> {
    let pool = create_test_pool().await?;
    let source = create_test_source(&pool).await?;
    let repo = PostingRepository::new(pool.clone());

    // Simulate concurrent upserts by running multiple in sequence
    // (true concurrency testing would require spawning tasks)
    let mut handles = vec![];

    for i in 0..20 {
        let repo = repo.clone();
        let source_id = source.id;
        let handle = tokio::spawn(async move {
            let posting = JobPosting::new(
                source_id,
                format!("ext-concurrent-{}", i),
                format!("Job {}", i),
                "Co".to_string(),
                "Desc".to_string(),
                Utc::now(),
                format!("hash{}", i),
            );
            repo.upsert(&posting).await
        });
        handles.push(handle);
    }

    let mut inserted_count = 0;
    for handle in handles {
        let result = handle.await??;
        if result { inserted_count += 1; }
    }

    assert_eq!(inserted_count, 20);

    let all = repo.list(Some(source.id), None, 50, 0).await?;
    assert_eq!(all.len(), 20);

    Ok(())
}

#[tokio::test]
async fn test_foreign_key_constraints() -> Result<()> {
    let pool = create_test_pool().await?;
    let posting_repo = PostingRepository::new(pool.clone());
    let match_repo = MatchRepository::new(pool.clone());
    let scan_log_repo = ScanLogRepository::new(pool.clone());

    let source = create_test_source(&pool).await?;
    let posting = create_test_posting(&pool, source.id).await?;
    let cv = create_test_cv_profile(&pool).await?;

    // Try to create posting with non-existent source_id
    let bad_posting = JobPosting::new(Uuid::new_v4(), "bad".to_string(), "Bad".to_string(), "Co".to_string(), "D".to_string(), Utc::now(), "h".to_string());
    let result = posting_repo.create(&bad_posting).await;
    assert!(result.is_err());

    // Try to create match with non-existent cv_profile_id
    let bad_match = MatchResult::new(Uuid::new_v4(), posting.id, 50.0, vec![], vec![], vec![]);
    let result = match_repo.create(&bad_match).await;
    assert!(result.is_err());

    // Try to create match with non-existent posting_id
    let bad_match = MatchResult::new(cv.id, Uuid::new_v4(), 50.0, vec![], vec![], vec![]);
    let result = match_repo.create(&bad_match).await;
    assert!(result.is_err());

    // Try to create scan_log with non-existent source_id
    let bad_log = ScanLog::new(Uuid::new_v4());
    let result = scan_log_repo.create(&bad_log).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_json_serialization_roundtrip() -> Result<()> {
    let pool = create_test_pool().await?;
    let source_repo = SourceRepository::new(pool.clone());
    let posting_repo = PostingRepository::new(pool.clone());
    let cv_repo = CVRepository::new(pool.clone());
    let match_repo = MatchRepository::new(pool.clone());

    // Test SourceConfig roundtrip
    let source = Source::new(
        "Config Test".to_string(),
        SourceType::Url,
        SourceConfig::Url {
            url: "https://test.com".to_string(),
            selector: Some(".job".to_string()),
            detail_selector: Some(".detail".to_string()),
            pagination: Some(crate::models::source::PaginationConfig {
                type_: crate::models::source::PaginationType::NextButton,
                selector: Some(".next".to_string()),
                max_pages: Some(10),
            }),
            headers: Some(json!({"User-Agent": "test"})),
        },
    );
    source_repo.create(&source).await?;
    let fetched = source_repo.get(source.id).await?.unwrap();
    match &fetched.config.0 {
        SourceConfig::Url { url, selector, detail_selector, pagination, headers } => {
            assert_eq!(url, "https://test.com");
            assert_eq!(selector, &Some(".job".to_string()));
            assert_eq!(detail_selector, &Some(".detail".to_string()));
            assert!(pagination.is_some());
            assert_eq!(pagination.as_ref().unwrap().max_pages, Some(10));
            assert!(headers.is_some());
        }
        _ => panic!("Wrong config type"),
    }

    // Test JobPosting JSON fields roundtrip
    let source2 = create_test_source(&pool).await?;
    let mut posting = JobPosting::new(
        source2.id,
        "json-test".to_string(),
        "JSON Test".to_string(),
        "Co".to_string(),
        "Desc".to_string(),
        Utc::now(),
        "jsonhash".to_string(),
    );
    posting.skills.0 = vec!["Rust".to_string(), "SQL".to_string()];
    posting.languages.0 = vec!["English".to_string(), "French".to_string()];
    posting.application_steps.0 = vec![
        ApplicationStep { order: 1, description: "Apply online".to_string(), action_type: ActionType::Link, target: "https://apply.com".to_string(), deadline: None, required: true, completed: false },
    ];
    posting_repo.create(&posting).await?;
    let fetched = posting_repo.get(posting.id).await?.unwrap();
    assert_eq!(fetched.skills.0, vec!["Rust", "SQL"]);
    assert_eq!(fetched.languages.0, vec!["English", "French"]);
    assert_eq!(fetched.application_steps.0.len(), 1);
    assert_eq!(fetched.application_steps.0[0].action_type, ActionType::Link);

    // Test CVData roundtrip
    let cv_data = CVData {
        full_name: "Test".to_string(),
        email: "test@test.com".to_string(),
        skills: vec![
            crate::models::cv::Skill { id: Uuid::new_v4(), name: "Rust".to_string(), category: Some("tech".to_string()), level: Some(crate::models::cv::SkillLevel::Expert), years: Some(5) },
        ],
        template: CVTemplate::Minimal,
        theme: CVTheme::Light,
        ..Default::default()
    };
    let cv = CVProfile::new("JSON CV".to_string(), cv_data);
    cv_repo.create(&cv).await?;
    let fetched = cv_repo.get(cv.id).await?.unwrap();
    assert_eq!(fetched.data.0.skills.len(), 1);
    assert_eq!(fetched.data.0.skills[0].name, "Rust");
    assert_eq!(fetched.template, CVTemplate::Minimal);
    assert_eq!(fetched.theme, CVTheme::Light);

    // Test MatchResult JSON fields roundtrip
    let source3 = create_test_source(&pool).await?;
    let posting2 = create_test_posting(&pool, source3.id).await?;
    let cv2 = create_test_cv_profile(&pool).await?;
    let match_result = MatchResult::new(
        cv2.id,
        posting2.id,
        88.0,
        vec!["Rust".to_string(), "PostgreSQL".to_string()],
        vec!["Python".to_string()],
        vec![
            MatchReason { category: MatchCategory::Skills, description: "Skills match".to_string(), weight: 0.7, matched: true },
            MatchReason { category: MatchCategory::Location, description: "Remote OK".to_string(), weight: 0.3, matched: true },
        ],
    );
    match_repo.create(&match_result).await?;
    let fetched = match_repo.get(cv2.id, posting2.id).await?.unwrap();
    assert_eq!(fetched.matched_skills.0, vec!["Rust", "PostgreSQL"]);
    assert_eq!(fetched.missing_skills.0, vec!["Python"]);
    assert_eq!(fetched.reasons.0.len(), 2);
    assert_eq!(fetched.reasons.0[0].category, MatchCategory::Skills);
    assert_eq!(fetched.reasons.0[1].category, MatchCategory::Location);

    Ok(())
}