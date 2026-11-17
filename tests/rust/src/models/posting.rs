//! Job Posting Models
//!
//! Canonical structured job record extracted from any source.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Link,      // Open URL in browser
    Email,     // Open mailto:
    Form,      // Fill web form
    Upload,    // Upload file
    Other,     // Custom action
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStep {
    pub order: u32,
    pub description: String,
    pub action_type: ActionType,
    pub target: String,                // URL, email, form selector, etc.
    pub deadline: Option<DateTime<Utc>>,
    pub required: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "posting_status", rename_all = "lowercase")]
pub enum PostingStatus {
    Active,
    Expired,
    Filled,
    Hidden,
    Applied,
    Saved,
}

impl PostingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PostingStatus::Active => "active",
            PostingStatus::Expired => "expired",
            PostingStatus::Filled => "filled",
            PostingStatus::Hidden => "hidden",
            PostingStatus::Applied => "applied",
            PostingStatus::Saved => "saved",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobPosting {
    pub id: Uuid,
    pub source_id: Uuid,
    pub source_posting_id: String,     // External ID for deduplication
    pub title: String,
    pub company: String,
    pub description: String,
    pub summary: Option<String>,       // LLM-generated summary
    pub skills: Json<Vec<String>>,     // Extracted skills
    pub experience_required: Option<String>,
    pub education_required: Option<String>,
    pub languages: Json<Vec<String>>,
    pub location: Option<String>,
    pub remote_policy: Option<String>, // remote, hybrid, onsite
    pub job_type: Option<String>,      // full-time, part-time, contract, internship
    pub salary_range: Option<String>,
    pub deadline: Option<DateTime<Utc>>,
    pub application_steps: Json<Vec<ApplicationStep>>,
    pub application_email: Option<String>,
    pub application_url: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub posted_date: Option<DateTime<Utc>>,
    pub scraped_at: DateTime<Utc>,
    pub content_hash: String,          // For deduplication
    pub raw_html: Option<String>,      // Original HTML (optional, large)
    pub status: PostingStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl JobPosting {
    pub fn new(
        source_id: Uuid,
        source_posting_id: String,
        title: String,
        company: String,
        description: String,
        scraped_at: DateTime<Utc>,
        content_hash: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            source_id,
            source_posting_id,
            title,
            company,
            description,
            summary: None,
            skills: Json(Vec::new()),
            experience_required: None,
            education_required: None,
            languages: Json(Vec::new()),
            location: None,
            remote_policy: None,
            job_type: None,
            salary_range: None,
            deadline: None,
            application_steps: Json(Vec::new()),
            application_email: None,
            application_url: None,
            contact_person: None,
            contact_phone: None,
            posted_date: None,
            scraped_at,
            content_hash,
            raw_html: None,
            status: PostingStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if a field is empty/missing. Used for confidence scoring and missing field tracking.
    pub fn field_is_empty(&self, field: &str) -> bool {
        match field {
            "title" => self.title.is_empty(),
            "company" => self.company.is_empty(),
            "description" => self.description.is_empty(),
            "summary" => self.summary.is_none(),
            "skills" => self.skills.0.is_empty(),
            "location" => self.location.is_none(),
            "experience_required" => self.experience_required.is_none(),
            "education_required" => self.education_required.is_none(),
            "languages" => self.languages.0.is_empty(),
            "remote_policy" => self.remote_policy.is_none(),
            "job_type" => self.job_type.is_none(),
            "salary_range" => self.salary_range.is_none(),
            "deadline" => self.deadline.is_none(),
            "application_steps" => self.application_steps.0.is_empty(),
            "application_email" => self.application_email.is_none(),
            "application_url" => self.application_url.is_none(),
            "contact_person" => self.contact_person.is_none(),
            "contact_phone" => self.contact_phone.is_none(),
            "posted_date" => self.posted_date.is_none(),
            _ => false,
        }
    }

    /// Set a field by name. Used by rule-based extraction.
    pub fn set_field(&mut self, field: &str, value: String) {
        match field {
            "title" => self.title = value,
            "company" => self.company = value,
            "description" => self.description = value,
            "location" => self.location = Some(value),
            "job_type" => self.job_type = Some(value),
            "remote_policy" => self.remote_policy = Some(value),
            "experience_required" => self.experience_required = Some(value),
            "education_required" => self.education_required = Some(value),
            "salary_range" => self.salary_range = Some(value),
            "summary" => self.summary = Some(value),
            _ => {}
        }
    }

    /// Add a skill if not already present.
    pub fn add_skill(&mut self, skill: String) {
        if !self.skills.0.contains(&skill) {
            self.skills.0.push(skill);
        }
    }

    /// Add a language if not already present.
    pub fn add_language(&mut self, language: String) {
        if !self.languages.0.contains(&language) {
            self.languages.0.push(language);
        }
    }
}