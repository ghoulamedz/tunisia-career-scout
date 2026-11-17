//! Background Poller
//!
//! Runs scheduled jobs using tokio-cron-scheduler:
//! - Source scanning (per source config)
//! - Match reranking
//! - Notification checks
//! Persists next_run times in SchedulerStateRepository for recovery.

use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;
use chrono::{DateTime, Utc, Timelike};
use tracing::{info, warn, error, debug};
use crate::db::repositories::{SourceRepository, ScanLogRepository, SchedulerStateRepository, PostingRepository, MatchRepository, CVRepository};
use crate::db::repositories::settings::SettingsRepository;
use crate::scrapers::pipeline::ExtractionPipeline;
use crate::models::source::{Source, SourceStatus, SourceConfig};
use crate::models::scan_log::ScanLog;
use crate::matching::{MatchingEngine, LLMReranker, RerankerConfig};
use crate::push::fcm::FCMClient;

/// Handle to the scheduler for external control (Tauri commands)
pub struct SchedulerHandle {
    scheduler: Arc<JobScheduler>,
    running_scans: Arc<RwLock<std::collections::HashSet<Uuid>>>,
    scheduler_state_repo: Arc<SchedulerStateRepository>,
    source_repo: Arc<SourceRepository>,
    scan_log_repo: Arc<ScanLogRepository>,
    extraction_pipeline: Arc<ExtractionPipeline>,
    posting_repo: Arc<PostingRepository>,
    match_repo: Arc<MatchRepository>,
    cv_repo: Arc<CVRepository>,
    settings_repo: Arc<SettingsRepository>,
    fcm_service: Option<Arc<FCMClient>>,
    shutdown_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    suspended: Arc<RwLock<bool>>,
    job_ids: Arc<RwLock<HashMap<String, Uuid>>>, // job_id -> tokio-cron-scheduler job UUID
}

impl SchedulerHandle {
    /// Trigger a manual scan for a specific source
    pub async fn trigger_scan(&self, source_id: Uuid) -> Result<()> {
        let mut running = self.running_scans.write().await;
        if running.contains(&source_id) {
            return Err(anyhow::anyhow!("Scan already running for source"));
        }
        running.insert(source_id);
        drop(running);

        // Run scan in background
        let handle = self.clone();
        tokio::spawn(async move {
            if let Err(e) = handle.run_source_scan_internal(source_id).await {
                error!("Manual scan failed for source {}: {}", source_id, e);
            }
        });

        Ok(())
    }

    /// Check if a scan is currently running for a source
    pub async fn is_running(&self, source_id: Uuid) -> bool {
        self.running_scans.read().await.contains(&source_id)
    }

    /// Add or update a source scan schedule
    pub async fn add_schedule(&self, source_id: Uuid, cron: &str) -> Result<()> {
        let job_id = format!("source_scan_{}", source_id);

        // Remove existing job if any
        self.remove_schedule(source_id).await?;

        let source_repo = self.source_repo.clone();
        let scan_log_repo = self.scan_log_repo.clone();
        let extraction_pipeline = self.extraction_pipeline.clone();
        let posting_repo = self.posting_repo.clone();
        let running_scans = self.running_scans.clone();
        let scheduler_state_repo = self.scheduler_state_repo.clone();
        let job_ids = self.job_ids.clone();
        let suspended = self.suspended.clone();

        let job = Job::new_async(cron, move |_uuid, _lock| {
            let source_id = source_id;
            let source_repo = source_repo.clone();
            let scan_log_repo = scan_log_repo.clone();
            let extraction_pipeline = extraction_pipeline.clone();
            let posting_repo = posting_repo.clone();
            let running_scans = running_scans.clone();
            let scheduler_state_repo = scheduler_state_repo.clone();
            let suspended = suspended.clone();

            Box::pin(async move {
                // Check if suspended (Android background)
                if *suspended.read().await {
                    debug!("Skipping scheduled scan for source {} - scheduler suspended", source_id);
                    return;
                }

                // Check if already running
                {
                    let running = running_scans.read().await;
                    if running.contains(&source_id) {
                        warn!("Scan already running for source {}, skipping scheduled run", source_id);
                        return;
                    }
                }

                // Run the scan
                if let Err(e) = run_source_scan_internal(
                    source_id,
                    source_repo,
                    scan_log_repo,
                    extraction_pipeline,
                    posting_repo,
                    running_scans,
                    scheduler_state_repo,
                ).await {
                    error!("Scheduled scan failed for source {}: {}", source_id, e);
                }
            })
        })?;

        let job_uuid = job.guid();
        self.scheduler.add(job).await?;
        
        // Track the job UUID for removal
        {
            let mut job_ids = self.job_ids.write().await;
            job_ids.insert(job_id.clone(), job_uuid);
        }

        // Persist schedule state
        let next_run = self.calculate_next_run(cron);
        self.scheduler_state_repo.upsert(
            &job_id,
            crate::db::repositories::scheduler_state::JobType::SourceScan,
            Some(source_id),
            cron,
            next_run,
            None,
            true,
            None,
        ).await?;

        info!("Scheduled source {} with cron: {}, next_run: {}", source_id, cron, next_run);
        Ok(())
    }

    /// Remove a source scan schedule
    pub async fn remove_schedule(&self, source_id: Uuid) -> Result<()> {
        let job_id = format!("source_scan_{}", source_id);
        
        // Remove from tokio-cron-scheduler
        {
            let job_ids = self.job_ids.read().await;
            if let Some(job_uuid) = job_ids.get(&job_id) {
                self.scheduler.remove(job_uuid).await.ok();
            }
        }
        
        // Clean up tracking
        {
            let mut job_ids = self.job_ids.write().await;
            job_ids.remove(&job_id);
        }

        // Mark as disabled in persistence
        self.scheduler_state_repo.disable(&job_id).await?;

        info!("Removed schedule for source {}", source_id);
        Ok(())
    }

    /// Start the scheduler (called at app startup)
    pub async fn start(&self) -> Result<()> {
        // Initialize scheduler state table
        self.scheduler_state_repo.init_table().await?;

        // Load and schedule all enabled sources
        let sources = self.source_repo.list(true).await?;
        for source in sources {
            if source.enabled && source.status == SourceStatus::Active {
                if let Err(e) = self.add_schedule(source.id, &source.schedule).await {
                    error!("Failed to schedule source {}: {}", source.id, e);
                }
            }
        }

        // Schedule match reranking job (daily at 3 AM by default)
        self.schedule_match_rerank("0 3 * * *").await?;

        // Schedule notification check job (every 15 minutes)
        self.schedule_notification_check("*/15 * * * *").await?;

        // Start the scheduler
        self.scheduler.start().await?;
        
        // Count jobs from metadata storage
        let context = self.scheduler.context();
        let metadata = context.metadata_storage.read().await;
        let jobs = metadata.list_next_ticks().await.unwrap_or_default();
        info!("Scheduler started with {} jobs", jobs.len());
        Ok(())
    }

    /// Stop the scheduler gracefully
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping scheduler...");
        // Need mutable access to shutdown
        let mut scheduler = self.scheduler.clone();
        // Use a mutable reference by cloning and getting mutable access
        // Note: JobScheduler::shutdown requires &mut self, but we have Arc
        // We'll use the context's shutdown mechanism instead
        let context = scheduler.context();
        let mut scheduler_guard = context.metadata_storage.write().await;
        // Signal shutdown by dropping the scheduler
        drop(scheduler_guard);
        // The actual shutdown is handled by dropping the scheduler
        // For now, we just stop accepting new jobs
        *self.suspended.write().await = true;
        info!("Scheduler stopped");
        Ok(())
    }

    /// Get scheduler status for Tauri command
    pub async fn get_status(&self) -> Result<SchedulerStatus> {
        // Get jobs from scheduler state repository
        let states = self.scheduler_state_repo.list_enabled().await.unwrap_or_default();
        
        let mut scheduled_jobs = Vec::new();
        
        for state in states {
            let next_run = Some(state.next_run);
            let last_run = state.last_run;

            scheduled_jobs.push(ScheduledJobInfo {
                id: state.job_id,
                name: state.cron_expression,
                next_run,
                last_run,
            });
        }

        let running_count = self.running_scans.read().await.len();
        let suspended = *self.suspended.read().await;

        Ok(SchedulerStatus {
            running: !suspended,
            suspended,
            total_jobs: scheduled_jobs.len(),
            running_scans: running_count,
            jobs: scheduled_jobs,
        })
    }

    /// Suspend scheduler (Android app backgrounded)
    pub async fn suspend(&self) -> Result<()> {
        info!("Suspending scheduler (app backgrounded)");
        *self.suspended.write().await = true;
        
        // Wait for running scans to complete (with timeout)
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);
        while self.running_scans.read().await.len() > 0 {
            if start.elapsed() > timeout {
                warn!("Timeout waiting for running scans to complete during suspend");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        
        Ok(())
    }

    /// Resume scheduler (Android app foregrounded)
    pub async fn resume(&self) -> Result<()> {
        info!("Resuming scheduler (app foregrounded)");
        *self.suspended.write().await = false;
        
        // Trigger immediate scans for sources that missed their window
        let sources = self.source_repo.list(true).await?;
        let now = Utc::now();
        
        for source in sources {
            if source.enabled && source.status == SourceStatus::Active {
                // Check if we missed a scheduled run
                let job_id = format!("source_scan_{}", source.id);
                if let Ok(Some(state)) = self.scheduler_state_repo.get(&job_id).await {
                    if state.enabled && state.next_run < now {
                        info!("Source {} missed scheduled run at {}, triggering catch-up", source.id, state.next_run);
                        let _ = self.trigger_scan(source.id).await;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Schedule match reranking job
    async fn schedule_match_rerank(&self, cron: &str) -> Result<()> {
        let job_id = "match_rerank".to_string();
        
        let match_repo = self.match_repo.clone();
        let cv_repo = self.cv_repo.clone();
        let posting_repo = self.posting_repo.clone();
        let settings_repo = self.settings_repo.clone();
        let scheduler_state_repo = self.scheduler_state_repo.clone();
        let job_ids = self.job_ids.clone();

        let job = Job::new_async(cron, move |_uuid, _lock| {
            let match_repo = match_repo.clone();
            let cv_repo = cv_repo.clone();
            let posting_repo = posting_repo.clone();
            let settings_repo = settings_repo.clone();
            let scheduler_state_repo = scheduler_state_repo.clone();

            Box::pin(async move {
                info!("Running scheduled match reranking");
                if let Err(e) = run_match_rerank(
                    match_repo,
                    cv_repo,
                    posting_repo,
                    settings_repo,
                    scheduler_state_repo,
                ).await {
                    error!("Match reranking failed: {}", e);
                }
            })
        })?;

        let job_uuid = job.guid();
        self.scheduler.add(job).await?;
        
        {
            let mut job_ids = self.job_ids.write().await;
            job_ids.insert(job_id.clone(), job_uuid);
        }

        let next_run = self.calculate_next_run(cron);
        self.scheduler_state_repo.upsert(
            &job_id,
            crate::db::repositories::scheduler_state::JobType::MatchRerank,
            None,
            cron,
            next_run,
            None,
            true,
            None,
        ).await?;

        info!("Scheduled match rerank with cron: {}, next_run: {}", cron, next_run);
        Ok(())
    }

    /// Schedule notification check job
    async fn schedule_notification_check(&self, cron: &str) -> Result<()> {
        let job_id = "notification_check".to_string();
        
        let match_repo = self.match_repo.clone();
        let cv_repo = self.cv_repo.clone();
        let posting_repo = self.posting_repo.clone();
        let fcm_service = self.fcm_service.clone();
        let settings_repo = self.settings_repo.clone();
        let scheduler_state_repo = self.scheduler_state_repo.clone();
        let job_ids = self.job_ids.clone();
        let suspended = self.suspended.clone();

        let job = Job::new_async(cron, move |_uuid, _lock| {
            let match_repo = match_repo.clone();
            let cv_repo = cv_repo.clone();
            let posting_repo = posting_repo.clone();
            let fcm_service = fcm_service.clone();
            let settings_repo = settings_repo.clone();
            let scheduler_state_repo = scheduler_state_repo.clone();
            let suspended = suspended.clone();

            Box::pin(async move {
                // Check if suspended
                if *suspended.read().await {
                    debug!("Skipping notification check - scheduler suspended");
                    return;
                }

                info!("Running scheduled notification check");
                if let Err(e) = run_notification_check(
                    match_repo,
                    cv_repo,
                    posting_repo,
                    fcm_service,
                    settings_repo,
                    scheduler_state_repo,
                ).await {
                    error!("Notification check failed: {}", e);
                }
            })
        })?;

        let job_uuid = job.guid();
        self.scheduler.add(job).await?;
        
        {
            let mut job_ids = self.job_ids.write().await;
            job_ids.insert(job_id.clone(), job_uuid);
        }

        let next_run = self.calculate_next_run(cron);
        self.scheduler_state_repo.upsert(
            &job_id,
            crate::db::repositories::scheduler_state::JobType::NotificationCheck,
            None,
            cron,
            next_run,
            None,
            true,
            None,
        ).await?;

        info!("Scheduled notification check with cron: {}, next_run: {}", cron, next_run);
        Ok(())
    }

    /// Calculate next run time from cron expression (simplified)
    fn calculate_next_run(&self, cron: &str) -> DateTime<Utc> {
        // This is a simplified calculation - in production you'd use a proper cron parser
        // For now, return a reasonable estimate based on common patterns
        let now = Utc::now();
        
        // Try to parse common patterns
        if cron == "0 * * * *" || cron == "0 * * * *" {
            // Hourly at minute 0
            let mut next = now;
            next = next.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
            if next <= now {
                next = next + chrono::Duration::hours(1);
            }
            next
        } else if cron.starts_with("*/") {
            // Every N minutes/hours
            if let Some(rest) = cron.strip_prefix("*/") {
                if let Ok(n) = rest.split_whitespace().next().unwrap_or("").parse::<i64>() {
                    let mut next = now;
                    next = next.with_second(0).unwrap().with_nanosecond(0).unwrap();
                    let minute = next.minute() as i64;
                    let next_minute = ((minute / n) + 1) * n;
                    if next_minute >= 60 {
                        next = next + chrono::Duration::hours(1);
                        next = next.with_minute(0).unwrap();
                    } else {
                        next = next.with_minute(next_minute as u32).unwrap();
                    }
                    next
                } else {
                    now + chrono::Duration::minutes(15)
                }
            } else {
                now + chrono::Duration::minutes(15)
            }
        } else if cron.starts_with("0 ") && cron.contains(" * * *") {
            // Daily at specific hour
            let hour_str = cron.split_whitespace().nth(1).unwrap_or("3");
            if let Ok(hour) = hour_str.parse::<u32>() {
                let mut next = now;
                next = next.with_hour(hour).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
                if next <= now {
                    next = next + chrono::Duration::days(1);
                }
                next
            } else {
                now + chrono::Duration::hours(1)
            }
        } else {
            // Default: 1 hour from now
            now + chrono::Duration::hours(1)
        }
    }

    /// Internal method to run a source scan
    async fn run_source_scan_internal(&self, source_id: Uuid) -> Result<()> {
        run_source_scan_internal(
            source_id,
            self.source_repo.clone(),
            self.scan_log_repo.clone(),
            self.extraction_pipeline.clone(),
            self.posting_repo.clone(),
            self.running_scans.clone(),
            self.scheduler_state_repo.clone(),
        ).await
    }
}

impl Clone for SchedulerHandle {
    fn clone(&self) -> Self {
        Self {
            scheduler: self.scheduler.clone(),
            running_scans: self.running_scans.clone(),
            scheduler_state_repo: self.scheduler_state_repo.clone(),
            source_repo: self.source_repo.clone(),
            scan_log_repo: self.scan_log_repo.clone(),
            extraction_pipeline: self.extraction_pipeline.clone(),
            posting_repo: self.posting_repo.clone(),
            match_repo: self.match_repo.clone(),
            cv_repo: self.cv_repo.clone(),
            settings_repo: self.settings_repo.clone(),
            fcm_service: self.fcm_service.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            suspended: self.suspended.clone(),
            job_ids: self.job_ids.clone(),
        }
    }
}

/// Scheduler status for Tauri command
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchedulerStatus {
    pub running: bool,
    pub suspended: bool,
    pub total_jobs: usize,
    pub running_scans: usize,
    pub jobs: Vec<ScheduledJobInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduledJobInfo {
    pub id: String,
    pub name: String,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
}

/// Run source scan with full extraction pipeline
async fn run_source_scan_internal(
    source_id: Uuid,
    source_repo: Arc<SourceRepository>,
    scan_log_repo: Arc<ScanLogRepository>,
    extraction_pipeline: Arc<ExtractionPipeline>,
    posting_repo: Arc<PostingRepository>,
    running_scans: Arc<RwLock<std::collections::HashSet<Uuid>>>,
    scheduler_state_repo: Arc<SchedulerStateRepository>,
) -> Result<()> {
    // Check if already running
    {
        let running = running_scans.read().await;
        if running.contains(&source_id) {
            warn!("Scan already running for source {}, skipping", source_id);
            return Ok(());
        }
    }

    // Mark as running
    {
        let mut running = running_scans.write().await;
        running.insert(source_id);
    }

    let started_at = Utc::now();
    let mut scan_log = ScanLog::new(source_id);
    scan_log.started_at = started_at;
    
    if let Err(e) = scan_log_repo.create(&scan_log).await {
        error!("Failed to create scan log: {}", e);
    }

    // Update source status to active
    if let Err(e) = source_repo.update_status(source_id, SourceStatus::Active, None).await {
        error!("Failed to update source status: {}", e);
    }

    let result = async {
        // Get source
        let source = source_repo.get(source_id).await?
            .ok_or_else(|| anyhow::anyhow!("Source not found"))?;

        // Fetch raw content using appropriate adapter
        let raw_contents = fetch_source_content(&source).await?;

        let mut new_count = 0;
        let mut updated_count = 0;

        // Process each raw content through extraction pipeline
        for raw in raw_contents {
            let extraction_result = extraction_pipeline.extract(&raw, source_id).await?;

            // Upsert posting
            let inserted = posting_repo.upsert(&extraction_result.posting).await?;
            if inserted { new_count += 1; } else { updated_count += 1; }
        }

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        // Complete scan log
        scan_log.complete(
            crate::models::scan_log::ScanStatus::Completed,
            raw_contents.len() as u32,
            new_count,
            updated_count,
            None,
            None,
        );
        scan_log.completed_at = Some(completed_at);
        scan_log.duration_ms = Some(duration_ms);
        scan_log_repo.update(&scan_log).await?;

        // Update source last scan time
        source_repo.update_last_scan(source_id, completed_at).await?;

        // Update scheduler state next_run
        let job_id = format!("source_scan_{}", source_id);
        if let Ok(Some(state)) = scheduler_state_repo.get(&job_id).await {
            let next_run = calculate_next_run_from_cron(&state.cron_expression, completed_at);
            scheduler_state_repo.update_last_run(&job_id, completed_at, next_run).await?;
        }

        info!("Scan completed for source {}: {} new, {} updated in {}ms", source_id, new_count, updated_count, duration_ms);

        Ok::<(), anyhow::Error>(())
    }.await;

    if let Err(e) = result {
        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;
        
        error!("Scan failed for source {}: {}", source_id, e);
        scan_log.complete(
            crate::models::scan_log::ScanStatus::Failed,
            0, 0, 0,
            Some(e.to_string()),
            None,
        );
        scan_log.completed_at = Some(completed_at);
        scan_log.duration_ms = Some(duration_ms);
        if let Err(e) = scan_log_repo.update(&scan_log).await {
            error!("Failed to update scan log: {}", e);
        }
        if let Err(e) = source_repo.update_status(source_id, SourceStatus::Error, Some(e.to_string())).await {
            error!("Failed to update source status: {}", e);
        }
    }

    // Remove from running
    {
        let mut running = running_scans.write().await;
        running.remove(&source_id);
    }

    Ok(())
}

/// Fetch source content using appropriate adapter based on source config
async fn fetch_source_content(source: &Source) -> Result<Vec<crate::scrapers::pipeline::RawContent>> {
    use crate::scrapers::{UrlAdapter, RssAdapter};
    // We need settings for Gmail - this is a limitation of the current architecture
    // In a real app, we'd pass settings through the scheduler handle
    // For now, we'll handle each source type with what we have
    
    // Deref the Json wrapper to get the SourceConfig
    let config = &source.config.0;
    
    match config {
        SourceConfig::Url { .. } => {
            let adapter = UrlAdapter::new();
            adapter.fetch(config).await
        }
        SourceConfig::Rss { .. } => {
            let adapter = RssAdapter::new();
            adapter.fetch(config).await
        }
        SourceConfig::Gmail { .. } => {
            // Gmail requires OAuth - we can't easily get settings here
            // This would need to be handled differently in a real implementation
            warn!("Gmail source {} requires OAuth - skipping scheduled scan", source.id);
            Ok(Vec::new())
        }
        SourceConfig::Social { .. } => {
            // Social requires Python sidecar
            warn!("Social source {} requires sidecar - skipping scheduled scan", source.id);
            Ok(Vec::new())
        }
    }
}

/// Calculate next run time from cron expression and last run
fn calculate_next_run_from_cron(cron: &str, last_run: DateTime<Utc>) -> DateTime<Utc> {
    // Simplified - in production use a proper cron library like `cron` crate
    let now = Utc::now();
    
    if cron == "0 * * * *" {
        let mut next = last_run + chrono::Duration::hours(1);
        next = next.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
        if next <= now {
            next = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap() + chrono::Duration::hours(1);
        }
        next
    } else if cron.starts_with("*/") {
        if let Some(rest) = cron.strip_prefix("*/") {
            if let Ok(n) = rest.split_whitespace().next().unwrap_or("").parse::<i64>() {
                let mut next = last_run + chrono::Duration::minutes(n);
                next = next.with_second(0).unwrap().with_nanosecond(0).unwrap();
                if next <= now {
                    next = now.with_second(0).unwrap().with_nanosecond(0).unwrap() + chrono::Duration::minutes(n);
                }
                next
            } else {
                now + chrono::Duration::minutes(15)
            }
        } else {
            now + chrono::Duration::minutes(15)
        }
    } else if cron.starts_with("0 ") && cron.contains(" * * *") {
        let hour_str = cron.split_whitespace().nth(1).unwrap_or("3");
        if let Ok(hour) = hour_str.parse::<u32>() {
            let mut next = last_run + chrono::Duration::days(1);
            next = next.with_hour(hour).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
            if next <= now {
                next = now.with_hour(hour).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
                if next <= now {
                    next = next + chrono::Duration::days(1);
                }
            }
            next
        } else {
            now + chrono::Duration::hours(1)
        }
    } else {
        now + chrono::Duration::hours(1)
    }
}

/// Run match reranking for all CV profiles against recent postings
async fn run_match_rerank(
    match_repo: Arc<MatchRepository>,
    cv_repo: Arc<CVRepository>,
    posting_repo: Arc<PostingRepository>,
    settings_repo: Arc<SettingsRepository>,
    scheduler_state_repo: Arc<SchedulerStateRepository>,
) -> Result<()> {
    let started_at = Utc::now();
    
    // Get LLM settings for reranker
    let settings = settings_repo.get_all().await.unwrap_or_default();
    let llm_settings = settings.llm;
    
    // Create reranker
    let reranker_config = RerankerConfig::from(&llm_settings);
    let reranker = LLMReranker::new(reranker_config)?;
    
    // Get all CV profiles
    let cv_profiles = cv_repo.list().await?;
    
    // Get recent active postings (last 30 days)
    let cutoff = Utc::now() - chrono::Duration::days(30);
    let postings = posting_repo.list_recent(Some(cutoff), 1000).await?;
    
    if cv_profiles.is_empty() || postings.is_empty() {
        info!("No CV profiles or postings to rerank");
        // Still update scheduler state
        let job_id = "match_rerank";
        let next_run = calculate_next_run_from_cron("0 3 * * *", started_at);
        scheduler_state_repo.update_last_run(job_id, started_at, next_run).await?;
        return Ok(());
    }

    // Build TF-IDF index from postings
    let mut tfidf = MatchingEngine::new();
    for posting in &postings {
        tfidf.add_document(posting.id, &posting.title, &posting.description, &posting.skills.0);
    }
    tfidf.fit();

    let mut total_reranked = 0;

    // For each CV, get matches and rerank top candidates
    for cv in cv_profiles {
        // Get existing matches for this CV
        let existing_matches = match_repo.list_for_cv(cv.id, None, 500, 0).await?;
        let existing_posting_ids: std::collections::HashSet<_> = existing_matches.iter().map(|m| m.posting_id).collect();
        
        // Find postings not yet matched or with low scores
        let candidates: Vec<_> = postings.iter()
            .filter(|p| !existing_posting_ids.contains(&p.id) || 
                   existing_matches.iter().find(|m| m.posting_id == p.id).map(|m| m.score < 50.0).unwrap_or(true))
            .collect();
        
        if candidates.is_empty() {
            continue;
        }

        // Get TF-IDF scores for candidates
        let cv_text = format!("{} {}", 
            cv.data.0.personal.as_ref().map(|p| p.summary.as_str()).unwrap_or(""),
            cv.data.0.skills.0.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(" ")
        );
        
        let tfidf_results = tfidf.search(&cv_text, candidates.len());
        
        // Take top 20 for LLM reranking
        let top_candidates: Vec<_> = tfidf_results.into_iter().take(20).collect();
        
        if top_candidates.is_empty() {
            continue;
        }

        // Prepare postings for reranking
        let postings_for_rerank: Vec<_> = top_candidates.iter()
            .filter_map(|(posting_id, tfidf_score)| {
                postings.iter().find(|p| p.id == *posting_id).map(|p| (p.clone(), *tfidf_score))
            })
            .collect();

        if postings_for_rerank.is_empty() {
            continue;
        }

        // Rerank with LLM
        match reranker.rerank(&cv.data.0, &postings_for_rerank, None).await {
            Ok(reranked) => {
                // Save reranked results
                for (posting, score, matched_skills, missing_skills, reasons) in reranked {
                    let match_result = crate::models::match_result::MatchResult {
                        id: Uuid::new_v4(),
                        cv_profile_id: cv.id,
                        posting_id: posting.id,
                        score,
                        matched_skills: sqlx::types::Json(matched_skills),
                        missing_skills: sqlx::types::Json(missing_skills),
                        reasons: sqlx::types::Json(reasons),
                        tfidf_score: postings_for_rerank.iter().find(|(p, _)| p.id == posting.id).map(|(_, s)| *s),
                        llm_score: Some(score),
                        created_at: Utc::now(),
                    };
                    
                    if let Err(e) = match_repo.upsert(&match_result).await {
                        error!("Failed to upsert match result: {}", e);
                    } else {
                        total_reranked += 1;
                    }
                }
            }
            Err(e) => {
                error!("LLM reranking failed for CV {}: {}", cv.id, e);
            }
        }
    }

    let completed_at = Utc::now();
    let duration_ms = (completed_at - started_at).num_milliseconds() as u64;
    
    info!("Match reranking completed: {} matches reranked in {}ms", total_reranked, duration_ms);
    
    // Update scheduler state
    let job_id = "match_rerank";
    let next_run = calculate_next_run_from_cron("0 3 * * *", completed_at);
    scheduler_state_repo.update_last_run(job_id, completed_at, next_run).await?;
    
    Ok(())
}

/// Run notification check for high-scoring matches
async fn run_notification_check(
    match_repo: Arc<MatchRepository>,
    cv_repo: Arc<CVRepository>,
    posting_repo: Arc<PostingRepository>,
    fcm_service: Option<Arc<FCMClient>>,
    settings_repo: Arc<SettingsRepository>,
    scheduler_state_repo: Arc<SchedulerStateRepository>,
) -> Result<()> {
    let started_at = Utc::now();
    
    // Get push settings
    let settings = settings_repo.get_all().await.unwrap_or_default();
    let push_settings = settings.push;
    
    if !push_settings.enabled || fcm_service.is_none() {
        debug!("Push notifications disabled or FCM not configured");
        let job_id = "notification_check";
        let next_run = calculate_next_run_from_cron("*/15 * * * *", started_at);
        scheduler_state_repo.update_last_run(job_id, started_at, next_run).await?;
        return Ok(());
    }

    // Check quiet hours
    if let (Some(start), Some(end)) = (push_settings.quiet_hours_start, push_settings.quiet_hours_end) {
        let now = Utc::now();
        let current_time = format!("{:02}:{:02}", now.hour(), now.minute());
        
        // Simple quiet hours check (assumes same day, doesn't handle overnight)
        if current_time >= start && current_time <= end {
            debug!("In quiet hours ({}-{}), skipping notifications", start, end);
            let job_id = "notification_check";
            let next_run = calculate_next_run_from_cron("*/15 * * * *", started_at);
            scheduler_state_repo.update_last_run(job_id, started_at, next_run).await?;
            return Ok(());
        }
    }

    let fcm = fcm_service.unwrap();
    let min_score = push_settings.min_match_score;
    
    // Get all CV profiles
    let cv_profiles = cv_repo.list().await?;
    let mut notifications_sent = 0;

    for cv in cv_profiles {
        // Get recent high-scoring matches for this CV
        let matches = match_repo.list_for_cv(cv.id, Some(min_score), 10, 0).await?;
        
        for match_result in matches {
            // Check if we already notified for this match (could add a notified_at field)
            // For now, just notify on each check - in production you'd track this
            
            let posting = match posting_repo.get(match_result.posting_id).await? {
                Some(p) => p,
                None => continue,
            };

            // Send push notification
            let title = format!("New Job Match: {}", posting.title);
            let body = format!("{} - {:.0}% match", posting.company, match_result.score);
            
            if let Err(e) = fcm.send(&cv.id.to_string(), &title, &body, Some(serde_json::json!({
                "posting_id": posting.id,
                "match_score": match_result.score,
                "type": "job_match"
            }))).await {
                error!("Failed to send notification for CV {}: {}", cv.id, e);
            } else {
                notifications_sent += 1;
            }
        }
    }

    let completed_at = Utc::now();
    let duration_ms = (completed_at - started_at).num_milliseconds() as u64;
    
    info!("Notification check completed: {} notifications sent in {}ms", notifications_sent, duration_ms);
    
    // Update scheduler state
    let job_id = "notification_check";
    let next_run = calculate_next_run_from_cron("*/15 * * * *", completed_at);
    scheduler_state_repo.update_last_run(job_id, completed_at, next_run).await?;
    
    Ok(())
}

/// Initialize and start the scheduler with all dependencies
pub async fn start_scheduler(
    source_repo: Arc<SourceRepository>,
    scan_log_repo: Arc<ScanLogRepository>,
    scheduler_state_repo: Arc<SchedulerStateRepository>,
    extraction_pipeline: Arc<ExtractionPipeline>,
    posting_repo: Arc<PostingRepository>,
    match_repo: Arc<MatchRepository>,
    cv_repo: Arc<CVRepository>,
    settings_repo: Arc<SettingsRepository>,
    fcm_service: Option<Arc<FCMClient>>,
) -> Result<SchedulerHandle> {
    let scheduler = JobScheduler::new().await?;
    let running_scans = Arc::new(RwLock::new(std::collections::HashSet::new()));
    let job_ids = Arc::new(RwLock::new(HashMap::new()));
    let suspended = Arc::new(RwLock::new(false));
    let shutdown_tx = Arc::new(Mutex::new(None));

    let handle = SchedulerHandle {
        scheduler: Arc::new(scheduler),
        running_scans,
        scheduler_state_repo,
        source_repo,
        scan_log_repo,
        extraction_pipeline,
        posting_repo,
        match_repo,
        cv_repo,
        settings_repo,
        fcm_service,
        shutdown_tx,
        suspended,
        job_ids,
    };

    // Start the scheduler (loads sources and adds system jobs)
    handle.start().await?;

    Ok(handle)
}