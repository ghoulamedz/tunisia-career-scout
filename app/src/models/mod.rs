//! Domain Models
//!
//! Core data structures shared between Rust backend and Svelte frontend.
//! Types here must be kept in sync with frontend `lib/types.ts`.

pub mod source;
pub mod posting;
pub mod cv;
pub mod match_result;
pub mod scan_log;
pub mod settings;

pub use source::{Source, SourceType, SourceConfig, SourceStatus, AuthData};
pub use posting::{JobPosting, ApplicationStep, ActionType};
pub use cv::{CVData, CVTemplate, CVTheme, CVProfile, Experience, Education, Skill, Language, Project, Certification};
pub use match_result::{MatchResult, MatchReason};
pub use scan_log::ScanLog;
pub use settings::{Settings, LLMSettings, PushSettings};