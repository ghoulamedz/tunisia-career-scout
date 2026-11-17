//! Gmail Module
//!
//! OAuth2 flow and Gmail API integration.

pub mod oauth;
pub mod api;

pub use oauth::GmailOAuth;
pub use oauth::{GmailAuthStartResult, AuthStatus};
pub use api::{GmailApi, fetch_messages};