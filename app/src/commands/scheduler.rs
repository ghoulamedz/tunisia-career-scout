//! Scheduler Commands
//!
//! Tauri commands for scheduler control and status.

use anyhow::Result;
use tauri::{State, command};
use crate::db::DbPool;
use crate::db::repositories::{SourceRepository, ScanLogRepository, SchedulerStateRepository, PostingRepository, MatchRepository, CVRepository, SettingsRepository};
use crate::scrapers::pipeline::ExtractionPipeline;
use crate::scheduler::poller::SchedulerHandle;
use crate::push::fcm::FCMClient;
use std::sync::Arc;

#[command]
pub async fn start_scheduler(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
) -> Result<(), String> {
    // The scheduler is already started in main.rs, this is a no-op
    // but we can use it to restart if needed
    let status = scheduler.get_status().await.map_err(|e| e.to_string())?;
    if !status.running {
        // Re-start by re-adding jobs
        let sources = SourceRepository::new(pool.inner().clone()).list(true).await.map_err(|e| e.to_string())?;
        for source in sources {
            if source.enabled && source.status == crate::models::source::SourceStatus::Active {
                scheduler.add_schedule(source.id, &source.schedule).await.map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[command]
pub async fn stop_scheduler(
    scheduler: State<'_, SchedulerHandle>,
) -> Result<(), String> {
    scheduler.stop().await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_scheduler_status(
    scheduler: State<'_, SchedulerHandle>,
) -> Result<crate::scheduler::poller::SchedulerStatus, String> {
    scheduler.get_status().await.map_err(|e| e.to_string())
}

#[command]
pub async fn suspend_scheduler(
    scheduler: State<'_, SchedulerHandle>,
) -> Result<(), String> {
    scheduler.suspend().await.map_err(|e| e.to_string())
}

#[command]
pub async fn resume_scheduler(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
) -> Result<(), String> {
    scheduler.resume().await.map_err(|e| e.to_string())
}

#[command]
pub async fn trigger_source_scan_scheduled(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
    source_id: uuid::Uuid,
) -> Result<(), String> {
    scheduler.trigger_scan(source_id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn update_source_schedule(
    pool: State<'_, DbPool>,
    scheduler: State<'_, SchedulerHandle>,
    source_id: uuid::Uuid,
    cron: String,
) -> Result<(), String> {
    // Update source in DB
    let repo = SourceRepository::new((*pool).clone());
    let mut source = repo.get(source_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Source not found".to_string())?;
    
    source.schedule = cron.clone();
    source.updated_at = chrono::Utc::now();
    repo.update(&source).await.map_err(|e| e.to_string())?;
    
    // Update scheduler
    if source.enabled && source.status == crate::models::source::SourceStatus::Active {
        scheduler.add_schedule(source_id, &cron).await.map_err(|e| e.to_string())?;
    } else {
        scheduler.remove_schedule(source_id).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

/// Initialize the scheduler state (called at app startup)
pub async fn init_scheduler(
    pool: &DbPool,
    extraction_pipeline: Arc<ExtractionPipeline>,
    fcm_service: Option<Arc<FCMClient>>,
) -> Result<SchedulerHandle> {
    let source_repo = Arc::new(SourceRepository::new((*pool).clone()));
    let scan_log_repo = Arc::new(ScanLogRepository::new((*pool).clone()));
    let scheduler_state_repo = Arc::new(SchedulerStateRepository::new((*pool).clone()));
    let posting_repo = Arc::new(PostingRepository::new((*pool).clone()));
    let match_repo = Arc::new(MatchRepository::new((*pool).clone()));
    let cv_repo = Arc::new(CVRepository::new((*pool).clone()));
    let settings_repo = Arc::new(SettingsRepository::new((*pool).clone()));

    crate::scheduler::poller::start_scheduler(
        source_repo,
        scan_log_repo,
        scheduler_state_repo,
        extraction_pipeline,
        posting_repo,
        match_repo,
        cv_repo,
        settings_repo,
        fcm_service,
    ).await
}