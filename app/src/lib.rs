//! Tunisia Career Scout — Library Root
//!
//! Re-exports all public modules for Tauri command handlers and tests.

pub mod commands;
pub mod db;
pub mod models;
pub mod scrapers;
pub mod scheduler;
pub mod cv;
pub mod matching;
pub mod gmail;
pub mod push;
pub mod sidecar;

// Re-export commonly used types
pub use db::connection::DbPool;
pub use models::source::{Source, SourceType, SourceConfig, SourceStatus};
pub use models::posting::{JobPosting, ApplicationStep, ActionType};
pub use models::cv::{CVData, CVTemplate, CVTheme};
pub use models::match_result::{MatchResult, MatchReason};
pub use scrapers::pipeline::ExtractionPipeline;
pub use scheduler::poller::SchedulerHandle;