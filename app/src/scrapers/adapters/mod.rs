//! Source Adapters
//!
//! Implementations for each source type.

pub mod url;
pub mod rss;
pub mod gmail;
pub mod social;

pub use url::UrlAdapter;
pub use rss::RssAdapter;
pub use gmail::GmailAdapter;
pub use social::SocialAdapter;