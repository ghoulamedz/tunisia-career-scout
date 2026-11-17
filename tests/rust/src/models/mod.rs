//! Test Models

pub mod source;
pub mod posting;

pub use source::{Source, SourceType, SourceConfig, SourceStatus, AuthData, AuthType, PaginationConfig, PaginationType, SocialPlatform, SocialTarget};
pub use posting::{JobPosting, PostingStatus, ApplicationStep, ActionType};