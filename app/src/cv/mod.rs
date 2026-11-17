//! CV Engine Module
//!
//! CV data management, Typst templates, and PDF compilation.

pub mod compiler;
pub mod preview;
pub mod templates;

pub use compiler::{CVCompiler, compile_cv_to_pdf, compile_cv_to_pdf_from_env};
pub use preview::compile_cv_preview;
pub use templates::{CVTemplateEngine, TemplateType};