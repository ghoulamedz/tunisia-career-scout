//! Push Notifications Module
//!
//! FCM integration for Android push notifications.

pub mod fcm;

pub use fcm::{FCMClient, register_device, send_notification};