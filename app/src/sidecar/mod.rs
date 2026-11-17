//! Sidecar Module
//!
//! Manages Python sidecar for Playwright-based social scraping.

pub mod python;

pub use python::{PythonSidecar, ensure_python_sidecar};