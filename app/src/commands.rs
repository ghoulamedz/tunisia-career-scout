//! Tauri Command Handlers
//!
//! All frontend-invoked commands organized by domain.

pub mod source;
pub mod posting;
pub mod settings;
pub mod matching;
pub mod gmail;
pub mod cv;
pub mod push;
pub mod scheduler;

use tauri::State;
use crate::db::DbPool;

// Re-export all command modules for Tauri invocation
pub use source::*;
pub use posting::*;
pub use settings::*;
pub use matching::*;
pub use gmail::*;
pub use cv::*;
pub use push::*;
pub use scheduler::*;