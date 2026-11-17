//! Scheduler Module
//!
//! Background job polling using tokio-cron-scheduler.

pub mod poller;

pub use poller::{SchedulerHandle, start_scheduler, SchedulerStatus, ScheduledJobInfo};