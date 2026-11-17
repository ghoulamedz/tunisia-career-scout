//! Tunisia Career Scout — Main Entry Point
//!
//! Universal job scouting, CV engine, and matching application.
//! Runs on Desktop (Linux/macOS/Windows) and Android via Tauri v2.

mod commands;
mod db;
mod models;
mod scrapers;
mod scheduler;
mod cv;
mod matching;
mod gmail;
mod push;
mod sidecar;
mod plugins;

use anyhow::Result;
use tauri::{Manager, AppHandle};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use crate::scheduler::poller::SchedulerHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize tokio runtime for async initialization
    let rt = tokio::runtime::Runtime::new()?;

    // Initialize database
    let db_pool = rt.block_on(async {
        let pool = db::connection::init_pool().await?;
        db::connection::run_migrations(&pool).await?;
        Ok::<_, anyhow::Error>(pool)
    })?;

    // Initialize settings state
    let settings_state = rt.block_on(async {
        let repo = db::repositories::SettingsRepository::new(db_pool.clone());
        let settings = repo.get_all().await.unwrap_or_default();
        std::sync::Arc::new(tokio::sync::RwLock::new(settings))
    });

    // Initialize extraction pipeline
    let extraction_pipeline = Arc::new(crate::scrapers::pipeline::ExtractionPipeline::new(
        crate::scrapers::rule_extractor::RuleExtractor::new(),
        None, // LLM extractor optional
    ));

    // Initialize FCM push service (optional)
    let fcm_service = rt.block_on(async {
        let settings = settings_state.read().await;
        if settings.push.enabled && settings.push.fcm_token.is_some() {
            // FCM service would be initialized here with proper config
            // For now, return None as FCM requires server-side setup
            None
        } else {
            None
        }
    });

    // Initialize scheduler
    let scheduler = rt.block_on(async {
        commands::scheduler::init_scheduler(&db_pool, extraction_pipeline.clone(), fcm_service.clone()).await
    })?;

    // Build Tauri app
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::default().build())
        .plugin(crate::plugins::android_foreground_service::init())
        .manage(db_pool)
        .manage(settings_state.clone())
        .manage(scheduler.clone())
        .invoke_handler(tauri::generate_handler![
            // Source commands
            commands::source::list_sources,
            commands::source::get_source,
            commands::source::create_source,
            commands::source::update_source,
            commands::source::delete_source,
            commands::source::trigger_source_scan,
            commands::source::scan_all_sources,
            commands::source::toggle_source_enabled,
            commands::source::get_source_logs,
            commands::source::login_social,
            // Posting commands
            commands::posting::list_postings,
            commands::posting::get_posting,
            commands::posting::delete_posting,
            commands::posting::re_extract_posting,
            commands::posting::search_postings,
            commands::posting::update_posting_status,
            // CV commands
            commands::cv::get_cv,
            commands::cv::list_cvs,
            commands::cv::save_cv,
            commands::cv::update_cv,
            commands::cv::delete_cv,
            commands::cv::set_default_cv,
            commands::cv::generate_pdf,
            commands::cv::list_templates,
            commands::cv::preview_template,
            commands::cv::compile_cv_preview,
            // Matching commands
            commands::matching::match_cv_to_postings,
            commands::matching::get_match_results,
            // Gmail commands
            commands::gmail::gmail_auth_start,
            commands::gmail::gmail_auth_callback,
            commands::gmail::gmail_auth_android_callback,
            commands::gmail::gmail_disconnect,
            commands::gmail::gmail_status,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::update_llm_settings,
            commands::settings::update_push_settings,
            commands::settings::update_sync_settings,
            commands::settings::reset_settings,
            commands::settings::gmail_configured,
            // Push commands
            commands::push::register_device,
            commands::push::unregister_device,
            // Scheduler commands
            commands::scheduler::start_scheduler,
            commands::scheduler::stop_scheduler,
            commands::scheduler::get_scheduler_status,
            commands::scheduler::suspend_scheduler,
            commands::scheduler::resume_scheduler,
            commands::scheduler::trigger_source_scan_scheduled,
            commands::scheduler::update_source_schedule,
        ])
        .setup(|app| {
            // Initialize sidecar Python scrapers
            sidecar::python::ensure_python_sidecar(app.handle())?;

            // Request notification permission on mobile
            #[cfg(mobile)]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = tauri_plugin_notification::request_permission(&handle).await {
                        tracing::warn!("Notification permission denied: {}", e);
                    }
                });
            }

            // Set up Android lifecycle handlers
            #[cfg(mobile)]
            {
                let scheduler = scheduler.clone();
                let app_handle = app.handle().clone();
                
                // Listen for pause/resume events
                app_handle.plugin(tauri_plugin_process::init());
                
                // Note: Tauri v2 mobile lifecycle events are handled via
                // the tauri-plugin-process or custom event listeners
                // We'll use a simple approach with app state events
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Handle window events for desktop suspend/resume simulation
            if let tauri::WindowEvent::Focused(focused) = event {
                let scheduler = window.state::<SchedulerHandle>();
                let scheduler = scheduler.inner().clone();
                tauri::async_runtime::spawn(async move {
                    if focused {
                        // Window gained focus - resume
                        if let Err(e) = scheduler.resume().await {
                            tracing::warn!("Failed to resume scheduler: {}", e);
                        }
                    } else {
                        // Window lost focus - suspend (optional for desktop)
                        // Only suspend on mobile
                        #[cfg(mobile)]
                        if let Err(e) = scheduler.suspend().await {
                            tracing::warn!("Failed to suspend scheduler: {}", e);
                        }
                    }
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

fn main() {
    run().expect("Failed to run application");
}