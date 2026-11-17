//! Match Result Models
//!
//! Represents a CV-to-job-posting match with score and reasoning.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchReason {
    pub category: MatchCategory,
    pub description: String,
    pub weight: f32,                   // Contribution to score (0-1)
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchCategory {
    Skills,
    Experience,
    Education,
    Location,
    Keywords,
    Language,
    Salary,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchResult {
    pub id: Uuid,
    pub cv_profile_id: Uuid,
    pub posting_id: Uuid,
    pub score: f32,                    // 0-100
    pub matched_skills: Json<Vec<String>>,
    pub missing_skills: Json<Vec<String>>,
    pub reasons: Json<Vec<MatchReason>>,
    pub tfidf_score: Option<f32>,
    pub llm_score: Option<f32>,
    pub created_at: DateTime<Utc>,
}

impl MatchResult {
    pub fn new(
        cv_profile_id: Uuid,
        posting_id: Uuid,
        score: f32,
        matched_skills: Vec<String>,
        missing_skills: Vec<String>,
        reasons: Vec<MatchReason>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            cv_profile_id,
            posting_id,
            score,
            matched_skills: Json(matched_skills),
            missing_skills: Json(missing_skills),
            reasons: Json(reasons),
            tfidf_score: None,
            llm_score: None,
            created_at: Utc::now(),
        }
    }
}