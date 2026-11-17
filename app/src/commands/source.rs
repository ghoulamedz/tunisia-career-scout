//! Source Commands

use anyhow::Result;
use tauri::{State, command};
use uuid::Uuid;
use chrono::Utc;
use crate::db::DbPool;
use crate::db::repositories::{SourceRepository, ScanLogRepository};
use crate::models::source::{Source, SourceType, SourceConfig, SourceStatus, AuthData};
use crate::models::scan_log::ScanLog;
use crate::scrapers::{UrlAdapter, RssAdapter, GmailAdapter, SocialAdapter};
use crate::gmail::oauth::GmailOAuth;
use crate::models::settings::SettingsState;
use crate::scrapers::pipeline::ExtractionPipeline;
use crate::scrapers::rule_extractor::RuleExtractor;
use crate::scheduler::poller::SchedulerHandle;
use crate::sidecar::python::PythonSidecar;

#[command]
pub async fn list_sources(
    pool: State<'_, DbPool>,
    enabled_only: Option<bool>,
) -> Result<Vec<Source>, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    repo.list(enabled_only.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_source(
    pool: State<'_, DbPool>,
    id: Uuid,
) -> Result<Source, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    repo.get(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Source not found".to_string())
}

#[command]
pub async fn create_source(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
    name: String,
    source_type: SourceType,
    config: SourceConfig,
    schedule: Option<String>,
) -> Result<Source, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    let mut source = Source::new(name, source_type, config);
    if let Some(sched) = schedule {
        source.schedule = sched;
    }

    repo.create(&source).await.map_err(|e| e.to_string())?;

    // Schedule if enabled
    if source.enabled && source.status == SourceStatus::Active {
        scheduler.add_schedule(source.id, &source.schedule).await.map_err(|e| e.to_string())?;
    }

    Ok(source)
}

#[command]
pub async fn update_source(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
    id: Uuid,
    name: Option<String>,
    config: Option<SourceConfig>,
    schedule: Option<String>,
    enabled: Option<bool>,
) -> Result<Source, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    let mut source = repo.get(id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Source not found".to_string())?;

    if let Some(n) = name { source.name = n; }
    if let Some(c) = config { source.config = sqlx::types::Json(c); }
    if let Some(s) = schedule { source.schedule = s; }
    if let Some(e) = enabled {
        source.enabled = e;
        if !e { source.status = SourceStatus::Paused; }
    }
    source.updated_at = Utc::now();

    repo.update(&source).await.map_err(|e| e.to_string())?;

    // Update schedule
    if source.enabled && source.status == SourceStatus::Active {
        scheduler.add_schedule(source.id, &source.schedule).await.map_err(|e| e.to_string())?;
    } else {
        scheduler.remove_schedule(source.id).await.map_err(|e| e.to_string())?;
    }

    Ok(source)
}

#[command]
pub async fn delete_source(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
    id: Uuid,
) -> Result<(), String> {
    let repo = SourceRepository::new(pool.inner().clone());
    scheduler.remove_schedule(id).await.map_err(|e| e.to_string())?;
    repo.delete(id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn trigger_source_scan(
    pool: State<'_, DbPool>,
    app: tauri::AppHandle,
    settings: State<'_, SettingsState>,
    id: Uuid,
) -> Result<ScanLog, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    let source = repo.get(id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Source not found".to_string())?;

    // Create scan log
    let scan_log_repo = ScanLogRepository::new(pool.inner().clone());
    let mut scan_log = ScanLog::new(id);
    scan_log_repo.create(&scan_log).await.map_err(|e| e.to_string())?;

    // Fetch content using appropriate adapter
    let raw_contents = match &source.config.0 {
        crate::models::source::SourceConfig::Url { .. } => {
            let adapter = UrlAdapter::new();
            adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
        }
        crate::models::source::SourceConfig::Rss { .. } => {
            let adapter = RssAdapter::new();
            adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
        }
        crate::models::source::SourceConfig::Gmail { .. } => {
            // Gmail requires auth - get from source.auth_data
            if let Some(auth) = &source.auth_data {
                let oauth = GmailOAuth::from_settings(&settings).await.map_err(|e| e.to_string())?;
                let gmail_adapter = GmailAdapter::new(oauth);
                gmail_adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
            } else {
                return Err("Gmail source not authenticated".to_string());
            }
        }
        crate::models::source::SourceConfig::Social { .. } => {
            // Social requires sidecar - use PythonSidecar from app handle
            let sidecar = PythonSidecar::new(&app)
                .or_else::<_, anyhow::Error>(|_| Ok(PythonSidecar::from_script_path(
                    std::env::var("SIDECAR_PATH").unwrap_or_else(|_| "sidecar/python-scrapers/src/__main__.py".to_string())
                )))?;
            let adapter = SocialAdapter::new(sidecar);
            adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
        }
    };

    // Extract postings
    let extraction_pipeline = ExtractionPipeline::with_defaults(
        RuleExtractor::new(),
        None, // LLM extractor optional
    );

    let posting_repo = crate::db::repositories::PostingRepository::new(pool.inner().clone());
    let mut new_count = 0;
    let mut updated_count = 0;

    for raw in raw_contents {
        let result = extraction_pipeline.extract(&raw, id).await.map_err(|e| e.to_string())?;
        let inserted = posting_repo.upsert(&result.posting).await.map_err(|e| e.to_string())?;
        if inserted { new_count += 1; } else { updated_count += 1; }
    }

    // Complete scan log
    scan_log.complete(
        crate::models::scan_log::ScanStatus::Completed,
        raw_contents.len() as u32,
        new_count,
        updated_count,
        None,
        None,
    );
    scan_log_repo.update(&scan_log).await.map_err(|e| e.to_string())?;

    // Update source last scan
    repo.update_last_scan(id, Utc::now()).await.map_err(|e| e.to_string())?;

    Ok(scan_log)
}

#[command]
pub async fn scan_all_sources(
    pool: State<'_, DbPool>,
    app: tauri::AppHandle,
    settings: State<'_, SettingsState>,
) -> Result<Vec<ScanLog>, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    let sources = repo.list(true).await.map_err(|e| e.to_string())?;

    let mut scan_logs = Vec::new();

    for source in sources {
        if source.enabled && source.status == SourceStatus::Active {
            match trigger_single_source_scan(&pool.inner(), &app, &settings, source.id).await {
                Ok(scan_log) => scan_logs.push(scan_log),
                Err(e) => {
                    tracing::error!("Failed to scan source {}: {}", source.id, e);
                }
            }
        }
    }

    Ok(scan_logs)
}

#[command]
pub async fn toggle_source_enabled(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
    id: Uuid,
) -> Result<Source, String> {
    let repo = SourceRepository::new(pool.inner().clone());
    let mut source = repo.get(id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Source not found".to_string())?;

    source.enabled = !source.enabled;
    if !source.enabled {
        source.status = SourceStatus::Paused;
        scheduler.remove_schedule(id).await.map_err(|e| e.to_string())?;
    } else if source.status == SourceStatus::Paused {
        source.status = SourceStatus::Active;
        scheduler.add_schedule(id, &source.schedule).await.map_err(|e| e.to_string())?;
    }
    source.updated_at = Utc::now();

    repo.update(&source).await.map_err(|e| e.to_string())?;

    Ok(source)
}

#[command]
pub async fn get_source_logs(
    pool: State<'_, DbPool>,
    source_id: Uuid,
    limit: Option<u32>,
) -> Result<Vec<ScanLog>, String> {
    let repo = ScanLogRepository::new(pool.inner().clone());
    repo.list_for_source(source_id, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

async fn trigger_single_source_scan(
    pool: &DbPool,
    app: &tauri::AppHandle,
    settings: &SettingsState,
    source_id: Uuid,
) -> Result<ScanLog, String> {
    let repo = SourceRepository::new(pool.clone());
    let source = repo.get(source_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Source not found".to_string())?;

    // Create scan log
    let scan_log_repo = ScanLogRepository::new(pool.clone());
    let mut scan_log = ScanLog::new(source_id);
    scan_log_repo.create(&scan_log).await.map_err(|e| e.to_string())?;

    // Fetch content using appropriate adapter
    let raw_contents = match &source.config.0 {
        crate::models::source::SourceConfig::Url { .. } => {
            let adapter = UrlAdapter::new();
            adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
        }
        crate::models::source::SourceConfig::Rss { .. } => {
            let adapter = RssAdapter::new();
            adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
        }
        crate::models::source::SourceConfig::Gmail { .. } => {
            if let Some(auth) = &source.auth_data {
                let oauth = GmailOAuth::from_settings(settings).await.map_err(|e| e.to_string())?;
                let gmail_adapter = GmailAdapter::new(oauth);
                gmail_adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
            } else {
                return Err("Gmail source not authenticated".to_string());
            }
        }
        crate::models::source::SourceConfig::Social { .. } => {
            let sidecar = PythonSidecar::new(app)
                .or_else::<_, anyhow::Error>(|_| Ok(PythonSidecar::from_script_path(
                    std::env::var("SIDECAR_PATH").unwrap_or_else(|_| "sidecar/python-scrapers/src/__main__.py".to_string())
                )))?;
            let adapter = SocialAdapter::new(sidecar);
            adapter.fetch(&source.config.0).await.map_err(|e| e.to_string())?
        }
    };

    // Extract postings
    let extraction_pipeline = ExtractionPipeline::with_defaults(
        RuleExtractor::new(),
        None, // LLM extractor optional
    );

    let posting_repo = crate::db::repositories::PostingRepository::new(pool.clone());
    let mut new_count = 0;
    let mut updated_count = 0;

    for raw in raw_contents {
        let result = extraction_pipeline.extract(&raw, source_id).await.map_err(|e| e.to_string())?;
        let inserted = posting_repo.upsert(&result.posting).await.map_err(|e| e.to_string())?;
        if inserted { new_count += 1; } else { updated_count += 1; }
    }

    // Complete scan log
    scan_log.complete(
        crate::models::scan_log::ScanStatus::Completed,
        raw_contents.len() as u32,
        new_count,
        updated_count,
        None,
        None,
    );
    scan_log_repo.update(&scan_log).await.map_err(|e| e.to_string())?;

    // Update source last scan
    repo.update_last_scan(source_id, Utc::now()).await.map_err(|e| e.to_string())?;

    Ok(scan_log)
}

#[command]
pub async fn login_social(
    app: tauri::AppHandle,
    platform: String,
    url: String,
) -> Result<AuthData, String> {
    let platform = match platform.to_lowercase().as_str() {
        "facebook" => crate::models::source::SocialPlatform::Facebook,
        "tiktok" => crate::models::source::SocialPlatform::TikTok,
        "linkedin" => crate::models::source::SocialPlatform::LinkedIn,
        _ => return Err(format!("Unsupported platform: {}", platform)),
    };

    let sidecar = PythonSidecar::new(&app)
        .or_else::<_, anyhow::Error>(|_| Ok(PythonSidecar::from_script_path(
            std::env::var("SIDECAR_PATH").unwrap_or_else(|_| "sidecar/python-scrapers/src/__main__.py".to_string())
        )))?;
    let adapter = SocialAdapter::new(sidecar);
    let cookies = adapter.login(&platform, &url).await.map_err(|e| e.to_string())?;
    Ok(adapter.create_auth_data(cookies))
}