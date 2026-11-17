//! CV Models
//!
//! Structured CV data, templates, and profiles.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CVTemplate {
    Modern,
    Classic,
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CVTheme {
    Light,
    Dark,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: Uuid,
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub current: bool,
    pub description: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Education {
    pub id: Uuid,
    pub institution: String,
    pub degree: String,
    pub field_of_study: Option<String>,
    pub location: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub current: bool,
    pub description: Option<String>,
    pub grade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,      // technical, soft, language, tool
    pub level: Option<SkillLevel>,
    pub years: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub id: Uuid,
    pub name: String,
    pub proficiency: LanguageProficiency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LanguageProficiency {
    Native,
    Fluent,
    Professional,
    Conversational,
    Basic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub url: Option<String>,
    pub github: Option<String>,
    pub technologies: Vec<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certification {
    pub id: Uuid,
    pub name: String,
    pub issuer: String,
    pub date: DateTime<Utc>,
    pub expiry: Option<DateTime<Utc>>,
    pub url: Option<String>,
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CVData {
    // Personal
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub linkedin: Option<String>,
    pub github: Option<String>,
    pub portfolio: Option<String>,
    pub summary: Option<String>,

    // Sections
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    pub skills: Vec<Skill>,
    pub languages: Vec<Language>,
    pub projects: Vec<Project>,
    pub certifications: Vec<Certification>,

    // Rendering
    pub template: CVTemplate,
    pub theme: CVTheme,
}

impl Default for CVData {
    fn default() -> Self {
        Self {
            full_name: String::new(),
            email: String::new(),
            phone: None,
            location: None,
            linkedin: None,
            github: None,
            portfolio: None,
            summary: None,
            experience: Vec::new(),
            education: Vec::new(),
            skills: Vec::new(),
            languages: Vec::new(),
            projects: Vec::new(),
            certifications: Vec::new(),
            template: CVTemplate::Modern,
            theme: CVTheme::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CVProfile {
    pub id: Uuid,
    pub name: String,                  // User-facing name (e.g., "Software Engineer", "Data Analyst")
    pub data: Json<CVData>,
    pub template: CVTemplate,
    pub theme: CVTheme,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CVProfile {
    pub fn new(name: String, data: CVData) -> Self {
        let now = Utc::now();
        let template = data.template.clone();
        let theme = data.theme.clone();
        Self {
            id: Uuid::new_v4(),
            name,
            data: Json(data),
            template,
            theme,
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }
}