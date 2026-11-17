//! Repository Pattern for Database Access
//!
//! Each repository encapsulates queries for a single entity.

pub mod source;
pub mod posting;
pub mod cv;
pub mod match_result;
pub mod settings;
pub mod scan_log;
pub mod scheduler_state;

pub use source::SourceRepository;
pub use posting::PostingRepository;
pub use cv::CVRepository;
pub use match_result::MatchRepository;
pub use settings::SettingsRepository;
pub use scan_log::ScanLogRepository;
pub use scheduler_state::SchedulerStateRepository;

#[cfg(test)]
mod tests;