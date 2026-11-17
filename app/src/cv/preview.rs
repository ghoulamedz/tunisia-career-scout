//! CV Preview Commands
//!
//! Tauri commands for live CV PDF preview generation.

use anyhow::Result;
use tauri::command;
use std::env;
use crate::models::cv::{CVData, CVTemplate};
use crate::cv::compiler::CVCompiler;

/// Compile CV data to PDF bytes for live preview.
/// Takes CV data and an optional template override (for template selector in preview).
#[command]
pub async fn compile_cv_preview(
    cv_data: CVData,
    template: Option<CVTemplate>,
) -> Result<Vec<u8>, String> {
    // Apply template override if provided
    let mut data = cv_data;
    if let Some(t) = template {
        data.template = t;
    }

    let template_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates/cv");
    let font_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/fonts");
    let compiler = CVCompiler::from_env(template_fallback, font_fallback);
    compiler.compile(&data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cv::{CVData, CVTemplate, CVTheme, Experience, Education, Skill, Language, Project, Certification, SkillLevel, LanguageProficiency};
    use uuid::Uuid;
    use chrono::{Utc, TimeZone};

    fn sample_cv() -> CVData {
        CVData {
            full_name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            phone: Some("+1-555-0123".to_string()),
            location: Some("San Francisco, CA".to_string()),
            linkedin: Some("linkedin.com/in/johndoe".to_string()),
            github: Some("github.com/johndoe".to_string()),
            portfolio: Some("johndoe.dev".to_string()),
            summary: Some("Experienced software engineer with a passion for building scalable systems.".to_string()),
            experience: vec![
                Experience {
                    id: Uuid::new_v4(),
                    title: "Senior Software Engineer".to_string(),
                    company: "TechCorp Inc.".to_string(),
                    location: Some("San Francisco, CA".to_string()),
                    start_date: Utc.with_ymd_and_hms(2021, 1, 15, 0, 0, 0).unwrap(),
                    end_date: None,
                    current: true,
                    description: "Lead backend development for high-traffic APIs. Designed and implemented microservices architecture.".to_string(),
                    skills: vec!["Rust".to_string(), "PostgreSQL".to_string(), "Kubernetes".to_string()],
                },
            ],
            education: vec![
                Education {
                    id: Uuid::new_v4(),
                    institution: "Stanford University".to_string(),
                    degree: "M.S. Computer Science".to_string(),
                    field_of_study: Some("Distributed Systems".to_string()),
                    location: Some("Stanford, CA".to_string()),
                    start_date: Utc.with_ymd_and_hms(2016, 9, 1, 0, 0, 0).unwrap(),
                    end_date: Some(Utc.with_ymd_and_hms(2018, 6, 15, 0, 0, 0).unwrap()),
                    current: false,
                    description: Some("Thesis: Optimizing Consensus in Distributed Databases".to_string()),
                    grade: Some("3.9/4.0".to_string()),
                },
            ],
            skills: vec![
                Skill { id: Uuid::new_v4(), name: "Rust".to_string(), category: Some("Technical".to_string()), level: Some(SkillLevel::Expert), years: Some(5) },
                Skill { id: Uuid::new_v4(), name: "Python".to_string(), category: Some("Technical".to_string()), level: Some(SkillLevel::Advanced), years: Some(7) },
            ],
            languages: vec![
                Language { id: Uuid::new_v4(), name: "English".to_string(), proficiency: LanguageProficiency::Native },
            ],
            projects: vec![],
            certifications: vec![],
            template: CVTemplate::Modern,
            theme: CVTheme::Light,
        }
    }

    #[test]
    #[ignore = "requires fonts and templates to be present"]
    fn test_compile_cv_preview_all_templates() {
        let cv = sample_cv();
        
        for template in [CVTemplate::Modern, CVTemplate::Classic, CVTemplate::Minimal] {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let pdf = rt.block_on(compile_cv_preview(cv.clone(), Some(template))).expect(&format!("Failed to compile {} template", template));
            
            assert_eq!(&pdf[0..4], b"%PDF", "PDF should start with %PDF magic bytes for {}", template);
            assert!(pdf.len() > 100, "PDF should have substantial content");
        }
    }
}