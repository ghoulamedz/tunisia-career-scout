//! CV Templates
//!
//! Typst template definitions for Modern, Classic, Minimal styles.

pub struct CVTemplateEngine;

impl CVTemplateEngine {
    pub fn modern() -> &'static str {
        include_str!("../../templates/cv/modern.typ")
    }

    pub fn classic() -> &'static str {
        include_str!("../../templates/cv/classic.typ")
    }

    pub fn minimal() -> &'static str {
        include_str!("../../templates/cv/minimal.typ")
    }

    pub fn get_template(template_type: crate::models::cv::CVTemplate) -> &'static str {
        match template_type {
            crate::models::cv::CVTemplate::Modern => Self::modern(),
            crate::models::cv::CVTemplate::Classic => Self::classic(),
            crate::models::cv::CVTemplate::Minimal => Self::minimal(),
        }
    }
}

pub enum TemplateType {
    Modern,
    Classic,
    Minimal,
}