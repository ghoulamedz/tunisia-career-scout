//! Scan Log Models
//!
//! Records of source scanning activity for debugging and monitoring.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "scan_status", rename_all = "lowercase")]
pub enum ScanStatus {
    Running,
    Completed,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScanLog {
    pub id: Uuid,
    pub source_id: Uuid,
    pub status: ScanStatus,
    pub postings_found: u32,
    pub postings_new: u32,
    pub postings_updated: u32,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub raw_response_sample: Option<String>, // First 1000 chars for debugging
}

impl ScanLog {
    pub fn new(source_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_id,
            status: ScanStatus::Running,
            postings_found: 0,
            postings_new: 0,
            postings_updated: 0,
            error_message: None,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            raw_response_sample: None,
        }
    }

    pub fn complete(&mut self, status: ScanStatus, found: u32, new: u32, updated: u32, error: Option<String>, sample: Option<String>) {
        self.status = status;
        self.postings_found = found;
        self.postings_new = new;
        self.postings_updated = updated;
        self.error_message = error;
        self.completed_at = Some(Utc::now());
        self.duration_ms = self.completed_at.map(|end| {
            (end - self.started_at).num_milliseconds() as u64
        });
        self.raw_response_sample = sample;
    }
}